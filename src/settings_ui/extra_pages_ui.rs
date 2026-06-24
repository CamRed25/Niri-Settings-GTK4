// settings_ui/extra_pages.rs — Animations, workspaces, gestures,
// recent-windows, debug, and miscellaneous pages.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Entry, Label, Orientation, Scale, Separator};

use super::helpers_ui::*;
use crate::settings_backend::{NamedWorkspace, SettingsConfig};

// ── Animations page ───────────────────────────────────────────────────────────

pub fn build_animations_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header(
        "animations",
        "enable, disable & slow down niri animations",
    ));

    let global_card = card("global");
    append_toggle(
        &global_card,
        "disable all animations",
        "turn off every animation at once (overrides individual settings below)",
        cfg.borrow().anim.global_off,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.anim.global_off = v),
    );
    global_card.append(&Separator::new(Orientation::Horizontal));
    {
        let row = row_box();
        let lbl_col = label_col(
            "slowdown factor",
            "multiply all animation durations  (0 = niri default; < 1 = faster)",
        );
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let init = cfg.borrow().anim.slowdown;
        let val_lbl = Label::new(Some(&if init == 0.0 {
            "default".to_string()
        } else {
            format!("{init:.1}\u{00d7}")
        }));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(8);

        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 10.0, 0.1);
        scale.set_value(init);
        scale.set_width_request(150);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = (s.value() * 10.0).round() / 10.0;
            cfg_s.borrow_mut().anim.slowdown = v;
            val_lbl_c.set_text(&if v == 0.0 {
                "default".to_string()
            } else {
                format!("{v:.1}\u{00d7}")
            });
        });

        row.append(&scale);
        row.append(&val_lbl);
        global_card.append(&row);
    }
    page.append(&global_card);

    let anim_card = card("individual animations");
    let anim_toggles: &[AnimBoolRow] = &[
        (
            "workspace-switch",
            "switching workspaces up/down",
            |a| a.workspace_switch_off,
            |a, v| a.workspace_switch_off = v,
        ),
        (
            "window-open",
            "window appearing on screen",
            |a| a.window_open_off,
            |a, v| a.window_open_off = v,
        ),
        (
            "window-close",
            "window disappearing",
            |a| a.window_close_off,
            |a, v| a.window_close_off = v,
        ),
        (
            "horizontal-view-movement",
            "camera scrolling left/right",
            |a| a.horizontal_view_movement_off,
            |a, v| a.horizontal_view_movement_off = v,
        ),
        (
            "window-movement",
            "windows moving within a workspace",
            |a| a.window_movement_off,
            |a, v| a.window_movement_off = v,
        ),
        (
            "window-resize",
            "window size changes",
            |a| a.window_resize_off,
            |a, v| a.window_resize_off = v,
        ),
        (
            "config-notification-open-close",
            "config error notification slide",
            |a| a.config_notification_open_close_off,
            |a, v| a.config_notification_open_close_off = v,
        ),
        (
            "exit-confirmation-open-close",
            "exit confirmation dialog",
            |a| a.exit_confirmation_open_close_off,
            |a, v| a.exit_confirmation_open_close_off = v,
        ),
        (
            "screenshot-ui-open",
            "screenshot picker fade-in",
            |a| a.screenshot_ui_open_off,
            |a, v| a.screenshot_ui_open_off = v,
        ),
        (
            "overview-open-close",
            "overview zoom in/out",
            |a| a.overview_open_close_off,
            |a, v| a.overview_open_close_off = v,
        ),
        (
            "recent-windows-close",
            "recent windows switcher fade-out",
            |a| a.recent_windows_close_off,
            |a, v| a.recent_windows_close_off = v,
        ),
    ];

    let mut first = true;
    for (name, desc, getter, setter) in anim_toggles {
        if !first {
            anim_card.append(&Separator::new(Orientation::Horizontal));
        }
        first = false;

        let label = format!("disable {name}");
        let init = getter(&cfg.borrow().anim);
        let setter_copy = *setter;
        append_toggle(
            &anim_card,
            &label,
            desc,
            init,
            cfg_toggle(Rc::clone(&cfg), move |c, v| {
                setter_copy(&mut c.anim, v);
            }),
        );
    }
    page.append(&anim_card);

    page.append(&save_button(cfg));
    page
}

// ── Workspaces page ───────────────────────────────────────────────────────────

pub fn build_workspaces_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header(
        "workspaces",
        "named workspaces that always exist",
    ));

    let list_card = Rc::new(card("named workspaces"));
    page.append(&*list_card);

    fn rebuild(card_box: &GtkBox, cfg: &Rc<RefCell<SettingsConfig>>) {
        let mut to_remove: Vec<gtk4::Widget> = Vec::new();
        let mut child = card_box.first_child();
        if let Some(ref w) = child {
            child = w.next_sibling();
        }
        while let Some(w) = child {
            to_remove.push(w.clone());
            child = w.next_sibling();
        }
        for w in to_remove {
            card_box.remove(&w);
        }

        let workspaces = cfg.borrow().workspaces.clone();
        for (idx, ws) in workspaces.iter().enumerate() {
            let sep = Separator::new(Orientation::Horizontal);
            sep.add_css_class("row-sep");
            card_box.append(&sep);

            let row = row_box();
            let name_entry = Entry::new();
            name_entry.add_css_class("color-entry");
            name_entry.set_text(&ws.name);
            name_entry.set_placeholder_text(Some("workspace name"));
            name_entry.set_width_chars(14);
            name_entry.set_hexpand(true);
            let cfg_n = Rc::clone(cfg);
            name_entry.connect_changed(move |e| {
                cfg_n.borrow_mut().workspaces[idx].name = e.text().to_string();
            });
            row.append(&name_entry);

            let out_lbl = Label::new(Some("on output:"));
            out_lbl.add_css_class("row-sublabel");
            out_lbl.set_margin_start(8);
            row.append(&out_lbl);

            let out_entry = Entry::new();
            out_entry.add_css_class("color-entry");
            out_entry.set_text(&ws.open_on_output);
            out_entry.set_placeholder_text(Some("any"));
            out_entry.set_width_chars(12);
            let cfg_o = Rc::clone(cfg);
            out_entry.connect_changed(move |e| {
                cfg_o.borrow_mut().workspaces[idx].open_on_output = e.text().to_string();
            });
            row.append(&out_entry);

            let del_btn = Button::with_label("\u{2715}");
            del_btn.add_css_class("sidebar-item");
            let cfg_d = Rc::clone(cfg);
            del_btn.connect_clicked(move |btn| {
                cfg_d.borrow_mut().workspaces.remove(idx);
                if let Some(card_w) = btn.parent().and_then(|r| r.parent()) {
                    if let Ok(cb) = card_w.downcast::<GtkBox>() {
                        rebuild(&cb, &cfg_d);
                    }
                }
            });
            row.append(&del_btn);
            card_box.append(&row);
        }
    }

    rebuild(&list_card, &cfg);

    let add_btn = Button::with_label("+ add workspace");
    add_btn.add_css_class("save-btn");
    add_btn.set_halign(Align::Start);
    add_btn.set_margin_start(14);
    add_btn.set_margin_top(8);
    add_btn.set_margin_bottom(8);
    let cfg_a = Rc::clone(&cfg);
    let list_card_c = Rc::clone(&list_card);
    add_btn.connect_clicked(move |_| {
        cfg_a.borrow_mut().workspaces.push(NamedWorkspace {
            name: String::new(),
            open_on_output: String::new(),
        });
        rebuild(&list_card_c, &cfg_a);
    });
    page.append(&add_btn);
    page.append(&save_button(cfg));
    page
}

// ── Gestures page ─────────────────────────────────────────────────────────────

pub fn build_gestures_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header(
        "gestures",
        "drag-and-drop scrolling & hot corners",
    ));

    let vs_card = card("dnd edge view scroll");
    let vs_rows: &[CfgU32Row] = &[
        (
            "trigger width",
            "px from edge that activates scrolling (0=default 30)",
            3000,
            |c| c.gestures.dnd_view_trigger_width,
            |c, v| c.gestures.dnd_view_trigger_width = v,
        ),
        (
            "delay",
            "ms before scrolling starts (0=default 100)",
            2000,
            |c| c.gestures.dnd_view_delay_ms,
            |c, v| c.gestures.dnd_view_delay_ms = v,
        ),
        (
            "max speed",
            "px/s maximum scroll speed (0=default 1500)",
            5000,
            |c| c.gestures.dnd_view_max_speed,
            |c, v| c.gestures.dnd_view_max_speed = v,
        ),
    ];
    append_u32_sliders(&vs_card, vs_rows, Rc::clone(&cfg));
    page.append(&vs_card);

    let ws_card = card("dnd edge workspace switch");
    let ws_rows: &[CfgU32Row] = &[
        (
            "trigger height",
            "px from edge that activates switching (0=default 50)",
            3000,
            |c| c.gestures.dnd_ws_trigger_height,
            |c, v| c.gestures.dnd_ws_trigger_height = v,
        ),
        (
            "delay",
            "ms before switching starts (0=default 100)",
            2000,
            |c| c.gestures.dnd_ws_delay_ms,
            |c, v| c.gestures.dnd_ws_delay_ms = v,
        ),
        (
            "max speed",
            "maximum switching speed (0=default 1500)",
            5000,
            |c| c.gestures.dnd_ws_max_speed,
            |c, v| c.gestures.dnd_ws_max_speed = v,
        ),
    ];
    append_u32_sliders(&ws_card, ws_rows, Rc::clone(&cfg));
    page.append(&ws_card);

    let hc_card = card("hot corners");
    append_toggle(
        &hc_card,
        "disable hot corners",
        "turn off the corner trigger for the overview",
        cfg.borrow().gestures.hot_corners_off,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.gestures.hot_corners_off = v),
    );

    for (label, desc, getter, setter) in [
        (
            "top-left",
            "top-left corner triggers overview",
            (|c: &SettingsConfig| c.gestures.hot_corners_top_left) as fn(&SettingsConfig) -> bool,
            (|c: &mut SettingsConfig, v: bool| {
                c.gestures.hot_corners_top_left = v;
            }) as fn(&mut SettingsConfig, bool),
        ),
        (
            "top-right",
            "top-right corner triggers overview",
            (|c: &SettingsConfig| c.gestures.hot_corners_top_right) as fn(&SettingsConfig) -> bool,
            (|c: &mut SettingsConfig, v: bool| {
                c.gestures.hot_corners_top_right = v;
            }) as fn(&mut SettingsConfig, bool),
        ),
        (
            "bottom-left",
            "bottom-left corner triggers overview",
            (|c: &SettingsConfig| c.gestures.hot_corners_bottom_left)
                as fn(&SettingsConfig) -> bool,
            (|c: &mut SettingsConfig, v: bool| {
                c.gestures.hot_corners_bottom_left = v;
            }) as fn(&mut SettingsConfig, bool),
        ),
        (
            "bottom-right",
            "bottom-right corner triggers overview",
            (|c: &SettingsConfig| c.gestures.hot_corners_bottom_right)
                as fn(&SettingsConfig) -> bool,
            (|c: &mut SettingsConfig, v: bool| {
                c.gestures.hot_corners_bottom_right = v;
            }) as fn(&mut SettingsConfig, bool),
        ),
    ] {
        hc_card.append(&Separator::new(Orientation::Horizontal));
        append_toggle(
            &hc_card,
            label,
            desc,
            getter(&cfg.borrow()),
            cfg_toggle(Rc::clone(&cfg), setter),
        );
    }
    page.append(&hc_card);

    page.append(&save_button(cfg));
    page
}

// ── Recent windows page ──────────────────────────────────────────────────────

pub fn build_recent_windows_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header(
        "recent windows",
        "Alt-Tab switcher configuration",
    ));

    let gen_card = card("general");
    append_toggle(
        &gen_card,
        "disable recent windows switcher",
        "turn off the Alt-Tab window switcher entirely",
        cfg.borrow().recent_windows.off,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.recent_windows.off = v),
    );
    gen_card.append(&Separator::new(Orientation::Horizontal));

    let rw_rows: &[CfgU32Row] = &[
        (
            "debounce delay",
            "ms before focusing a window commits it to the list (0=default 750)",
            2000,
            |c| c.recent_windows.debounce_ms,
            |c, v| c.recent_windows.debounce_ms = v,
        ),
        (
            "open delay",
            "ms between pressing Alt-Tab and the switcher appearing (0=default 150)",
            1000,
            |c| c.recent_windows.open_delay_ms,
            |c, v| c.recent_windows.open_delay_ms = v,
        ),
    ];
    append_u32_sliders(&gen_card, rw_rows, Rc::clone(&cfg));
    page.append(&gen_card);

    let hl_card = card("highlight");
    {
        let init = cfg.borrow().recent_windows.highlight_active_color.clone();
        append_color(
            &hl_card,
            "active color",
            &init,
            cfg_text(Rc::clone(&cfg), |c, v| {
                c.recent_windows.highlight_active_color = v;
            }),
        );
    }
    hl_card.append(&Separator::new(Orientation::Horizontal));
    {
        let init = cfg.borrow().recent_windows.highlight_urgent_color.clone();
        append_color(
            &hl_card,
            "urgent color",
            &init,
            cfg_text(Rc::clone(&cfg), |c, v| {
                c.recent_windows.highlight_urgent_color = v;
            }),
        );
    }
    hl_card.append(&Separator::new(Orientation::Horizontal));

    let hl_rows: &[CfgU32Row] = &[
        (
            "padding",
            "px of padding around the focused preview (0=default 30)",
            100,
            |c| c.recent_windows.highlight_padding,
            |c, v| c.recent_windows.highlight_padding = v,
        ),
        (
            "corner radius",
            "radius of the highlight corners in px",
            60,
            |c| c.recent_windows.highlight_corner_radius,
            |c, v| c.recent_windows.highlight_corner_radius = v,
        ),
    ];
    append_u32_sliders(&hl_card, hl_rows, Rc::clone(&cfg));
    page.append(&hl_card);

    let pv_card = card("previews");
    {
        let pv_max_height_row: &[CfgU32Row] = &[(
            "max height",
            "maximum height of window previews in px (0=default 480)",
            1440,
            |c| c.recent_windows.previews_max_height,
            |c, v| c.recent_windows.previews_max_height = v,
        )];
        append_u32_sliders(&pv_card, pv_max_height_row, Rc::clone(&cfg));
    }
    pv_card.append(&Separator::new(Orientation::Horizontal));
    {
        let row = row_box();
        let lbl_col = label_col(
            "max scale",
            "maximum scale factor for preview windows (0.0=default 0.5)",
        );
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let init = cfg.borrow().recent_windows.previews_max_scale;
        let val_lbl = Label::new(Some(&if init == 0.0 {
            "default".to_string()
        } else {
            format!("{init:.2}")
        }));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(8);

        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.01);
        scale.set_value(init);
        scale.set_width_request(140);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = (s.value() * 100.0).round() / 100.0;
            cfg_s.borrow_mut().recent_windows.previews_max_scale = v;
            val_lbl_c.set_text(&if v == 0.0 {
                "default".to_string()
            } else {
                format!("{v:.2}")
            });
        });

        row.append(&scale);
        row.append(&val_lbl);
        pv_card.append(&row);
    }
    page.append(&pv_card);

    page.append(&save_button(cfg));
    page
}

// ── Debug options page ────────────────────────────────────────────────────────

pub fn build_debug_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header(
        "debug",
        "experimental & diagnostic flags \u{2014} not covered by config \
         compatibility policy",
    ));

    let render_card = card("rendering");
    let render_toggles: &[CfgBoolRow] = &[
        (
            "enable overlay planes",
            "direct scanout into overlay planes (may cause frame drops)",
            |c| c.debug.enable_overlay_planes,
            |c, v| c.debug.enable_overlay_planes = v,
        ),
        (
            "disable cursor plane",
            "render cursor with the rest of the frame",
            |c| c.debug.disable_cursor_plane,
            |c, v| c.debug.disable_cursor_plane = v,
        ),
        (
            "disable direct scanout",
            "disable direct scanout to both primary and overlay planes",
            |c| c.debug.disable_direct_scanout,
            |c, v| c.debug.disable_direct_scanout = v,
        ),
        (
            "restrict primary scanout to matching format",
            "only scan out when the buffer format exactly matches",
            |c| c.debug.restrict_primary_scanout_to_matching_format,
            |c, v| {
                c.debug.restrict_primary_scanout_to_matching_format = v;
            },
        ),
        (
            "force disable connectors on resume",
            "force modeset/screen blank on all outputs when waking",
            |c| c.debug.force_disable_connectors_on_resume,
            |c, v| c.debug.force_disable_connectors_on_resume = v,
        ),
        (
            "force pipewire invalid modifier",
            "use the invalid DRM modifier for PipeWire screencasting",
            |c| c.debug.force_pipewire_invalid_modifier,
            |c, v| c.debug.force_pipewire_invalid_modifier = v,
        ),
        (
            "skip cursor-only updates during VRR",
            "skip redraws triggered only by cursor movement while VRR",
            |c| c.debug.skip_cursor_only_updates_during_vrr,
            |c, v| c.debug.skip_cursor_only_updates_during_vrr = v,
        ),
    ];

    let mut first = true;
    for (label, desc, getter, setter) in render_toggles {
        if !first {
            render_card.append(&Separator::new(Orientation::Horizontal));
        }
        first = false;
        let init = getter(&cfg.borrow());
        let setter_copy = *setter;
        append_toggle(
            &render_card,
            label,
            desc,
            init,
            cfg_toggle(Rc::clone(&cfg), setter_copy),
        );
    }

    render_card.append(&Separator::new(Orientation::Horizontal));
    {
        let row = row_box();
        let lbl_col = label_col(
            "render DRM device",
            "override the DRM device used for rendering (empty = auto)",
        );
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_text(&cfg.borrow().debug.render_drm_device);
        entry.set_placeholder_text(Some("/dev/dri/renderD129"));
        entry.set_width_chars(20);
        entry.set_max_width_chars(40);
        let cfg_e = Rc::clone(&cfg);
        entry.connect_changed(move |e| {
            cfg_e.borrow_mut().debug.render_drm_device = e.text().to_string();
        });
        row.append(&entry);
        render_card.append(&row);
    }
    page.append(&render_card);

    let wm_card = card("window management");
    let wm_toggles: &[CfgBoolRow] = &[
        (
            "disable resize throttling",
            "send resizes as fast as possible",
            |c| c.debug.disable_resize_throttling,
            |c, v| c.debug.disable_resize_throttling = v,
        ),
        (
            "disable transactions",
            "disable resize/close transactions",
            |c| c.debug.disable_transactions,
            |c, v| c.debug.disable_transactions = v,
        ),
        (
            "strict new window focus policy",
            "only focus windows with valid xdg-activation token",
            |c| c.debug.strict_new_window_focus_policy,
            |c, v| c.debug.strict_new_window_focus_policy = v,
        ),
        (
            "honor xdg-activation with invalid serial",
            "let apps like Discord steal focus via tray click",
            |c| c.debug.honor_xdg_activation_with_invalid_serial,
            |c, v| {
                c.debug.honor_xdg_activation_with_invalid_serial = v;
            },
        ),
        (
            "deactivate unfocused windows",
            "drop Activated state for unfocused windows",
            |c| c.debug.deactivate_unfocused_windows,
            |c, v| c.debug.deactivate_unfocused_windows = v,
        ),
        (
            "keep laptop panel on when lid is closed",
            "leave internal monitor on when lid is shut",
            |c| c.debug.keep_laptop_panel_on_when_lid_is_closed,
            |c, v| {
                c.debug.keep_laptop_panel_on_when_lid_is_closed = v;
            },
        ),
        (
            "disable monitor names",
            "ignore make/model/serial from EDID",
            |c| c.debug.disable_monitor_names,
            |c, v| c.debug.disable_monitor_names = v,
        ),
    ];

    first = true;
    for (label, desc, getter, setter) in wm_toggles {
        if !first {
            wm_card.append(&Separator::new(Orientation::Horizontal));
        }
        first = false;
        let init = getter(&cfg.borrow());
        let setter_copy = *setter;
        append_toggle(
            &wm_card,
            label,
            desc,
            init,
            cfg_toggle(Rc::clone(&cfg), setter_copy),
        );
    }
    page.append(&wm_card);

    let misc_card = card("d-bus & diagnostics");
    let misc_toggles: &[CfgBoolRow] = &[
        (
            "D-Bus interfaces in non-session instances",
            "create D-Bus interfaces even when not running as --session",
            |c| c.debug.dbus_interfaces_in_non_session_instances,
            |c, v| {
                c.debug.dbus_interfaces_in_non_session_instances = v;
            },
        ),
        (
            "wait for frame completion before queueing",
            "wait until every frame is done before handing to DRM",
            |c| c.debug.wait_for_frame_completion_before_queueing,
            |c, v| {
                c.debug.wait_for_frame_completion_before_queueing = v;
            },
        ),
        (
            "emulate zero presentation time",
            "emulate unknown DRM presentation time (NVIDIA)",
            |c| c.debug.emulate_zero_presentation_time,
            |c, v| c.debug.emulate_zero_presentation_time = v,
        ),
    ];

    first = true;
    for (label, desc, getter, setter) in misc_toggles {
        if !first {
            misc_card.append(&Separator::new(Orientation::Horizontal));
        }
        first = false;
        let init = getter(&cfg.borrow());
        let setter_copy = *setter;
        append_toggle(
            &misc_card,
            label,
            desc,
            init,
            cfg_toggle(Rc::clone(&cfg), setter_copy),
        );
    }
    page.append(&misc_card);

    page.append(&save_button(cfg));
    page
}

// ── Miscellaneous page ───────────────────────────────────────────────────────

pub fn build_miscellaneous_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header(
        "miscellaneous",
        "cursor, overview, screenshot, clipboard & more",
    ));

    // Screenshot
    let ss_card = card("screenshot");
    {
        let row = row_box();
        let lc = label_col(
            "save path",
            "strftime codes for date/time; empty = niri default; \
             \"null\" = disable saving",
        );
        lc.set_hexpand(true);
        row.append(&lc);
        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_width_chars(26);
        entry.set_max_width_chars(36);
        entry.set_placeholder_text(Some("~/Pictures/Screenshots/\u{2026}"));
        entry.set_text(&cfg.borrow().misc.screenshot_path);
        let cfg_s = Rc::clone(&cfg);
        entry.connect_changed(move |e| {
            cfg_s.borrow_mut().misc.screenshot_path = e.text().to_string();
        });
        row.append(&entry);
        ss_card.append(&row);
    }
    page.append(&ss_card);

    // Cursor
    let cur_card = card("cursor");
    {
        let row = row_box();
        let lc = label_col("xcursor theme", "cursor theme name  (e.g. breeze_cursors)");
        lc.set_hexpand(true);
        row.append(&lc);
        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_width_chars(16);
        entry.set_placeholder_text(Some("system default"));
        entry.set_text(&cfg.borrow().misc.cursor_theme);
        let cfg_s = Rc::clone(&cfg);
        entry.connect_changed(move |e| {
            cfg_s.borrow_mut().misc.cursor_theme = e.text().to_string();
        });
        row.append(&entry);
        cur_card.append(&row);
    }
    cur_card.append(&Separator::new(Orientation::Horizontal));
    {
        let row = row_box();
        let lc = label_col(
            "xcursor size",
            "cursor size in pixels  (0 = system default)",
        );
        lc.set_hexpand(true);
        row.append(&lc);
        let init = cfg.borrow().misc.cursor_size;
        let init_text = if init == 0 {
            String::from("default")
        } else {
            format!("{}px", init)
        };
        let val_lbl = Label::new(Some(&init_text));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(7);
        val_lbl.set_xalign(1.0);
        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 128.0, 1.0);
        scale.set_value(init as f64);
        scale.set_width_request(120);
        scale.set_draw_value(false);
        scale.set_hexpand(false);
        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = s.value().round() as u32;
            cfg_s.borrow_mut().misc.cursor_size = v;
            let t = if v == 0 {
                String::from("default")
            } else {
                format!("{}px", v)
            };
            val_lbl_c.set_text(&t);
        });
        row.append(&scale);
        row.append(&val_lbl);
        cur_card.append(&row);
    }
    cur_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &cur_card,
        "hide when typing",
        "hide the cursor while keys are being pressed",
        cfg.borrow().misc.cursor_hide_when_typing,
        cfg_toggle(Rc::clone(&cfg), |c, v| {
            c.misc.cursor_hide_when_typing = v;
        }),
    );
    cur_card.append(&Separator::new(Orientation::Horizontal));
    {
        let row = row_box();
        let lc = label_col(
            "hide after idle",
            "hide cursor after N ms with no movement  (0 = never)",
        );
        lc.set_hexpand(true);
        row.append(&lc);
        let init_ms = cfg.borrow().misc.cursor_hide_after_inactive_ms;
        let val_lbl = Label::new(Some(&idle_ms_label(init_ms)));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(8);
        val_lbl.set_xalign(1.0);
        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 10_000.0, 500.0);
        scale.set_value(init_ms as f64);
        scale.set_width_request(120);
        scale.set_draw_value(false);
        scale.set_hexpand(false);
        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = s.value().round() as u32;
            cfg_s.borrow_mut().misc.cursor_hide_after_inactive_ms = v;
            val_lbl_c.set_text(&idle_ms_label(v));
        });
        row.append(&scale);
        row.append(&val_lbl);
        cur_card.append(&row);
    }
    page.append(&cur_card);

    // Overview
    let ov_card = card("overview");
    {
        let row = row_box();
        let lc = label_col(
            "zoom",
            "how much workspaces shrink in the overview  (0 = niri default)",
        );
        lc.set_hexpand(true);
        row.append(&lc);
        let init_zoom = cfg.borrow().misc.overview_zoom;
        let val_lbl = Label::new(Some(&zoom_label(init_zoom)));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(8);
        val_lbl.set_xalign(1.0);
        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 0.75, 0.05);
        scale.set_value(init_zoom);
        scale.set_width_request(120);
        scale.set_draw_value(false);
        scale.set_hexpand(false);
        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = (s.value() * 20.0).round() / 20.0;
            cfg_s.borrow_mut().misc.overview_zoom = v;
            val_lbl_c.set_text(&zoom_label(v));
        });
        row.append(&scale);
        row.append(&val_lbl);
        ov_card.append(&row);
    }
    ov_card.append(&Separator::new(Orientation::Horizontal));
    {
        let init_color = cfg.borrow().misc.overview_backdrop_color.clone();
        append_color(
            &ov_card,
            "backdrop color",
            &init_color,
            cfg_text(Rc::clone(&cfg), |c, v| {
                c.misc.overview_backdrop_color = v;
            }),
        );
    }
    page.append(&ov_card);

    // Clipboard
    let clip_card = card("clipboard");
    append_toggle(
        &clip_card,
        "disable primary selection",
        "disable middle-click paste (primary clipboard)",
        cfg.borrow().misc.clipboard_disable_primary,
        cfg_toggle(Rc::clone(&cfg), |c, v| {
            c.misc.clipboard_disable_primary = v;
        }),
    );
    page.append(&clip_card);

    // Hotkey overlay
    let hk_card = card("hotkey overlay");
    append_toggle(
        &hk_card,
        "hide unbound actions",
        "only show hotkey overlay entries that are actually bound",
        cfg.borrow().misc.hotkey_overlay_hide_not_bound,
        cfg_toggle(Rc::clone(&cfg), |c, v| {
            c.misc.hotkey_overlay_hide_not_bound = v;
        }),
    );
    page.append(&hk_card);

    // Config notifications
    let notif_card = card("config notifications");
    append_toggle(
        &notif_card,
        "disable parse error notification",
        "don't show the 'Failed to parse config' notification",
        cfg.borrow().misc.config_notification_disable_failed,
        cfg_toggle(Rc::clone(&cfg), |c, v| {
            c.misc.config_notification_disable_failed = v;
        }),
    );
    page.append(&notif_card);

    // Xwayland
    let xwl_card = card("xwayland");
    append_toggle(
        &xwl_card,
        "disable xwayland-satellite",
        "turn off automatic Xwayland X11 integration",
        cfg.borrow().misc.xwayland_off,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.misc.xwayland_off = v),
    );
    xwl_card.append(&Separator::new(Orientation::Horizontal));
    {
        let row = row_box();
        let lc = label_col(
            "satellite path",
            "path to xwayland-satellite binary  (empty = auto-detect)",
        );
        lc.set_hexpand(true);
        row.append(&lc);
        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_width_chars(20);
        entry.set_placeholder_text(Some("xwayland-satellite"));
        entry.set_text(&cfg.borrow().misc.xwayland_path);
        let cfg_s = Rc::clone(&cfg);
        entry.connect_changed(move |e| {
            cfg_s.borrow_mut().misc.xwayland_path = e.text().to_string();
        });
        row.append(&entry);
        xwl_card.append(&row);
    }
    page.append(&xwl_card);

    page.append(&save_button(cfg));
    page
}
