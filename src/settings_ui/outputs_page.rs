// settings_ui/outputs_page.rs — Per-display output configuration page.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, DropDown, Entry, Label, Orientation, Scale, Separator};

use super::helpers::*;
use crate::settings_backend::{OutputConfig, OutputTransform, SettingsConfig, VrrMode};

/// Runs a blocking closure on a background thread and returns the result
/// to the GTK main thread via a oneshot channel awaited as a future.
pub async fn gio_run_blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (tx, rx) = futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.await
        .map_err(|_| "gio_run_blocking: sender dropped".to_string())
}

/// Render an inline error explaining why niri could not be queried, then fall
/// back to drawing whatever outputs the saved config already knows about.
fn show_query_error(cards: &GtkBox, canvas: &GtkBox, cfg: &SettingsConfig, message: &str) {
    log::warn!("outputs IPC query failed: {message}");
    let err_lbl = Label::new(Some(&format!("Could not query niri: {message}")));
    err_lbl.add_css_class("row-sublabel");
    err_lbl.set_margin_start(18);
    cards.append(&err_lbl);
    for output in &cfg.outputs {
        append_monitor_visual(canvas, &output.name, output.mode_width, output.mode_height);
    }
}

pub fn build_outputs_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header(
        "outputs",
        "per-display mode, scale, transform & position",
    ));

    let visual_card = card("display arrangement");
    let monitor_canvas = GtkBox::new(Orientation::Horizontal, 10);
    monitor_canvas.add_css_class("monitor-canvas");
    monitor_canvas.set_halign(gtk4::Align::Center);
    visual_card.append(&monitor_canvas);
    page.append(&visual_card);

    let status_label = Label::new(Some("Querying connected outputs\u{2026}"));
    status_label.add_css_class("row-sublabel");
    status_label.set_margin_start(18);
    status_label.set_margin_top(8);
    page.append(&status_label);

    let cards_box = GtkBox::new(Orientation::Vertical, 12);
    page.append(&cards_box);

    let cfg_outer = Rc::clone(&cfg);
    let page_clone = page.clone();
    let status_clone = status_label.clone();
    let cards_clone = cards_box.clone();
    let canvas_clone = monitor_canvas.clone();

    glib::spawn_future_local(async move {
        let result = gio_run_blocking(crate::ipc::query_outputs).await;
        page_clone.remove(&status_clone);

        // `gio_run_blocking` may fail (worker dropped) or the IPC call itself
        // may fail (no socket / timeout); both degrade to the same fallback:
        // show why, then render whatever outputs are already in the config.
        let detected = match result {
            Ok(Ok(v)) => v,
            Err(e) => {
                show_query_error(&cards_clone, &canvas_clone, &cfg_outer.borrow(), &e);
                return;
            }
            Ok(Err(e)) => {
                show_query_error(
                    &cards_clone,
                    &canvas_clone,
                    &cfg_outer.borrow(),
                    &e.to_string(),
                );
                return;
            }
        };
        {
            let mut c = cfg_outer.borrow_mut();
            for out in &detected {
                if !c.outputs.iter().any(|o| o.name == out.name) {
                    c.outputs.push(OutputConfig::new(&out.name));
                }
            }
        }

        for output in &detected {
            let (width, height) = output
                .current_mode
                .and_then(|index| output.modes.get(index))
                .map(|mode| (mode.width, mode.height))
                .unwrap_or((0, 0));
            append_monitor_visual(&canvas_clone, &output.name, width, height);
        }

        for ipc_out in detected {
            let out_idx = {
                let c = cfg_outer.borrow();
                c.outputs
                    .iter()
                    .position(|o| o.name == ipc_out.name)
                    .unwrap_or(0)
            };

            let title = format!(
                "{} \u{2014} {} {}",
                ipc_out.name, ipc_out.make, ipc_out.model
            );
            let out_card = card(&title);

            // off toggle
            let init_off = cfg_outer.borrow().outputs[out_idx].off;
            append_toggle(
                &out_card,
                "disable output",
                "turn this display off",
                init_off,
                cfg_toggle(Rc::clone(&cfg_outer), move |c, v| {
                    c.outputs[out_idx].off = v;
                }),
            );
            out_card.append(&Separator::new(Orientation::Horizontal));

            // mode dropdown
            {
                let row = row_box();
                let lbl_col = label_col("mode", "resolution and refresh rate");
                lbl_col.set_hexpand(true);
                row.append(&lbl_col);

                let mut mode_labels: Vec<String> = vec!["auto".to_string()];
                for m in &ipc_out.modes {
                    let hz = m.refresh_rate as f64 / 1000.0;
                    let label = format!(
                        "{}\u{00d7}{}  {:.3} Hz{}",
                        m.width,
                        m.height,
                        hz,
                        if m.is_preferred { " \u{2713}" } else { "" }
                    );
                    mode_labels.push(label);
                }
                let mode_strs: Vec<&str> = mode_labels.iter().map(|s| s.as_str()).collect();
                let dd = DropDown::from_strings(&mode_strs);
                dd.add_css_class("mini-dropdown");

                let saved_w = cfg_outer.borrow().outputs[out_idx].mode_width;
                let saved_h = cfg_outer.borrow().outputs[out_idx].mode_height;
                let saved_r = cfg_outer.borrow().outputs[out_idx].mode_refresh_mhz;
                let selected = if saved_w == 0 {
                    0u32
                } else {
                    ipc_out
                        .modes
                        .iter()
                        .position(|m| {
                            m.width == saved_w
                                && m.height == saved_h
                                && (saved_r == 0 || m.refresh_rate == saved_r)
                        })
                        .map(|i| i as u32 + 1)
                        .unwrap_or(0)
                };
                dd.set_selected(selected);

                let modes_copy = ipc_out.modes.clone();
                let cfg_dd = Rc::clone(&cfg_outer);
                dd.connect_selected_notify(move |d| {
                    let sel = d.selected() as usize;
                    let mut c = cfg_dd.borrow_mut();
                    if sel == 0 {
                        c.outputs[out_idx].mode_width = 0;
                        c.outputs[out_idx].mode_height = 0;
                        c.outputs[out_idx].mode_refresh_mhz = 0;
                    } else if let Some(m) = modes_copy.get(sel - 1) {
                        c.outputs[out_idx].mode_width = m.width;
                        c.outputs[out_idx].mode_height = m.height;
                        c.outputs[out_idx].mode_refresh_mhz = m.refresh_rate;
                    }
                });
                row.append(&dd);
                out_card.append(&row);
            }
            out_card.append(&Separator::new(Orientation::Horizontal));

            // scale
            {
                let row = row_box();
                let lbl_col = label_col("scale", "fractional scaling factor (0 = auto)");
                lbl_col.set_hexpand(true);
                row.append(&lbl_col);

                let init_scale = cfg_outer.borrow().outputs[out_idx].scale;
                let val_lbl = Label::new(Some(&if init_scale == 0.0 {
                    "auto".to_string()
                } else {
                    format!("{init_scale:.2}\u{00d7}")
                }));
                val_lbl.add_css_class("row-sublabel");
                val_lbl.set_width_chars(7);

                let scale_w = Scale::with_range(Orientation::Horizontal, 0.0, 4.0, 0.25);
                scale_w.set_value(init_scale);
                scale_w.set_width_request(140);
                scale_w.set_draw_value(false);
                scale_w.set_hexpand(false);

                let cfg_sc = Rc::clone(&cfg_outer);
                let val_lbl_c = val_lbl.clone();
                scale_w.connect_value_changed(move |s| {
                    let v = (s.value() * 4.0).round() / 4.0;
                    cfg_sc.borrow_mut().outputs[out_idx].scale = v;
                    val_lbl_c.set_text(&if v == 0.0 {
                        "auto".to_string()
                    } else {
                        format!("{v:.2}\u{00d7}")
                    });
                });
                row.append(&scale_w);
                row.append(&val_lbl);
                out_card.append(&row);
            }
            out_card.append(&Separator::new(Orientation::Horizontal));

            // transform
            {
                let row = row_box();
                let lbl_col = label_col("transform", "rotation / flip applied to the display");
                lbl_col.set_hexpand(true);
                row.append(&lbl_col);

                let dd = DropDown::from_strings(OutputTransform::label_variants());
                dd.add_css_class("mini-dropdown");
                dd.set_selected(cfg_outer.borrow().outputs[out_idx].transform.to_index());
                let cfg_tr = Rc::clone(&cfg_outer);
                dd.connect_selected_notify(move |d| {
                    cfg_tr.borrow_mut().outputs[out_idx].transform =
                        OutputTransform::from_index(d.selected());
                });
                row.append(&dd);
                out_card.append(&row);
            }
            out_card.append(&Separator::new(Orientation::Horizontal));

            // VRR
            {
                let vrr_note = if ipc_out.vrr_supported {
                    "variable refresh rate"
                } else {
                    "variable refresh rate (not supported by this \
                             display)"
                };
                let row = row_box();
                let lbl_col = label_col("variable refresh rate", vrr_note);
                lbl_col.set_hexpand(true);
                row.append(&lbl_col);

                let dd = DropDown::from_strings(VrrMode::label_variants());
                dd.add_css_class("mini-dropdown");
                dd.set_selected(cfg_outer.borrow().outputs[out_idx].vrr.to_index());
                dd.set_sensitive(ipc_out.vrr_supported);
                let cfg_vrr = Rc::clone(&cfg_outer);
                dd.connect_selected_notify(move |d| {
                    cfg_vrr.borrow_mut().outputs[out_idx].vrr = VrrMode::from_index(d.selected());
                });
                row.append(&dd);
                out_card.append(&row);
            }
            out_card.append(&Separator::new(Orientation::Horizontal));

            // position
            {
                let init_set = cfg_outer.borrow().outputs[out_idx].position_set;
                append_toggle(
                    &out_card,
                    "override position",
                    "manually set this display's position in the \
                             global space",
                    init_set,
                    cfg_toggle(Rc::clone(&cfg_outer), move |c, v| {
                        c.outputs[out_idx].position_set = v;
                    }),
                );

                let pos_row = row_box();
                let x_lbl = Label::new(Some("x"));
                x_lbl.add_css_class("row-sublabel");
                pos_row.append(&x_lbl);

                let x_entry = Entry::new();
                x_entry.add_css_class("color-entry");
                x_entry.set_width_chars(6);
                x_entry.set_text(&cfg_outer.borrow().outputs[out_idx].position_x.to_string());
                let cfg_px = Rc::clone(&cfg_outer);
                bind_i32_entry(&x_entry, move |v| {
                    cfg_px.borrow_mut().outputs[out_idx].position_x = v;
                });
                pos_row.append(&x_entry);

                let y_lbl = Label::new(Some("y"));
                y_lbl.add_css_class("row-sublabel");
                y_lbl.set_margin_start(8);
                pos_row.append(&y_lbl);

                let y_entry = Entry::new();
                y_entry.add_css_class("color-entry");
                y_entry.set_width_chars(6);
                y_entry.set_text(&cfg_outer.borrow().outputs[out_idx].position_y.to_string());
                let cfg_py = Rc::clone(&cfg_outer);
                bind_i32_entry(&y_entry, move |v| {
                    cfg_py.borrow_mut().outputs[out_idx].position_y = v;
                });
                pos_row.append(&y_entry);
                out_card.append(&pos_row);
            }

            cards_clone.append(&out_card);
        }

        let save_btn = save_button(cfg_outer);
        cards_clone.append(&save_btn);
    });

    page
}

fn append_monitor_visual(canvas: &GtkBox, name: &str, width: u32, height: u32) {
    let button = gtk4::Button::new();
    button.add_css_class("monitor-block");
    button.set_tooltip_text(Some("Select this monitor's controls below"));
    let aspect = if width > 0 && height > 0 {
        width as f64 / height as f64
    } else {
        16.0 / 9.0
    };
    let visual_height = 58;
    button.set_size_request(
        (f64::from(visual_height) * aspect).round() as i32,
        visual_height,
    );
    let labels = GtkBox::new(Orientation::Vertical, 2);
    let name_label = Label::new(Some(name));
    name_label.add_css_class("monitor-name");
    let resolution = if width > 0 && height > 0 {
        format!("{width}×{height}")
    } else {
        "automatic".into()
    };
    let resolution_label = Label::new(Some(&resolution));
    resolution_label.add_css_class("monitor-resolution");
    labels.append(&name_label);
    labels.append(&resolution_label);
    button.set_child(Some(&labels));
    canvas.append(&button);
}
