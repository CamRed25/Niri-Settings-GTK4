// settings_ui/rules_pages.rs — Keybindings, window rules, layer rules,
// and switch events pages.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    glib, Align, Box as GtkBox, Button, DropDown, Entry, Label, Orientation, ScrolledWindow,
    Separator, Switch,
};

use super::helpers::*;
use crate::settings_backend::{
    action_args_hint, action_needs_args, import_binds_from_niri_config, next_row_id,
    normalize_key_combo, BlockOutFrom, Keybind, LayerRule, SettingsConfig, SwitchEventsSettings,
    TriState, WindowRule, NIRI_ACTIONS,
};

type TriStateRow<T> = (
    &'static str,
    &'static str,
    fn(&T) -> &TriState,
    fn(&mut T, TriState),
);

fn recorded_combo(key: gtk4::gdk::Key, modifiers: gtk4::gdk::ModifierType) -> Option<String> {
    let raw_name = key.name()?.to_string();
    if matches!(
        raw_name.as_str(),
        "Shift_L"
            | "Shift_R"
            | "Control_L"
            | "Control_R"
            | "Alt_L"
            | "Alt_R"
            | "Super_L"
            | "Super_R"
            | "Meta_L"
            | "Meta_R"
    ) {
        return None;
    }

    let mut parts = Vec::new();
    if modifiers
        .intersects(gtk4::gdk::ModifierType::SUPER_MASK | gtk4::gdk::ModifierType::META_MASK)
    {
        parts.push("Super".to_owned());
    }
    if modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
        parts.push("Ctrl".to_owned());
    }
    if modifiers.contains(gtk4::gdk::ModifierType::ALT_MASK) {
        parts.push("Alt".to_owned());
    }
    if modifiers.contains(gtk4::gdk::ModifierType::SHIFT_MASK) {
        parts.push("Shift".to_owned());
    }
    let key_name = if raw_name.chars().count() == 1 {
        raw_name.to_uppercase()
    } else {
        raw_name
    };
    parts.push(key_name);
    Some(parts.join("+"))
}

// ── Keybindings page ──────────────────────────────────────────────────────────

pub fn build_keybindings_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header(
        "keybindings",
        "define key bindings written to the binds {} block",
    ));

    let list: Rc<GtkBox> = {
        let b = GtkBox::new(Orientation::Vertical, 8);
        b.set_margin_bottom(8);
        Rc::new(b)
    };

    fn rebuild(list: &Rc<GtkBox>, cfg: &Rc<RefCell<SettingsConfig>>) {
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }
        let binds = cfg.borrow().binds.clone();
        for (idx, bind) in binds.iter().enumerate() {
            let title = if !bind.key.is_empty() {
                format!("bind {} \u{2014} {}", idx + 1, bind.key)
            } else {
                format!("bind {}", idx + 1)
            };
            let c = card(&title);
            // Address this row by its stable id, not its index: deletions and
            // insertions shift indices but ids are permanent for the row's life.
            let rid = bind.id;

            // Key recorder. Escape cancels and Backspace clears the binding.
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col(
                    "key combo",
                    "activate, then press a shortcut · Escape cancels",
                );
                lc.set_hexpand(true);
                row.append(&lc);
                let recorder = Button::with_label(if bind.key.is_empty() {
                    "record shortcut"
                } else {
                    &bind.key
                });
                recorder.add_css_class("key-recorder");
                let recording = Rc::new(Cell::new(false));
                let original = Rc::new(RefCell::new(bind.key.clone()));
                // Generation counter: each recording session bumps it so a stale
                // timeout from an earlier session can't revert a newer one.
                let record_gen = Rc::new(Cell::new(0u64));
                let recording_click = Rc::clone(&recording);
                let original_click = Rc::clone(&original);
                let gen_click = Rc::clone(&record_gen);
                recorder.connect_clicked(move |button| {
                    *original_click.borrow_mut() = button.label().unwrap_or_default().to_string();
                    recording_click.set(true);
                    let generation = gen_click.get().wrapping_add(1);
                    gen_click.set(generation);
                    button.add_css_class("recording");
                    button.set_label("● press shortcut…");
                    button.grab_focus();

                    // Auto-cancel if no key arrives, so the recorder never gets
                    // stuck in the "press shortcut…" state indefinitely.
                    let recording_to = Rc::clone(&recording_click);
                    let gen_to = Rc::clone(&gen_click);
                    let original_to = Rc::clone(&original_click);
                    let button_to = button.clone();
                    glib::timeout_add_local_once(std::time::Duration::from_secs(8), move || {
                        if recording_to.get() && gen_to.get() == generation {
                            recording_to.set(false);
                            button_to.remove_css_class("recording");
                            button_to.set_label(&original_to.borrow());
                        }
                    });
                });

                let controller = gtk4::EventControllerKey::new();
                let recording_key = Rc::clone(&recording);
                let original_key = Rc::clone(&original);
                let cfg_key = Rc::clone(cfg);
                let list_key = Rc::clone(list);
                let recorder_key = recorder.clone();
                controller.connect_key_pressed(move |_, key, _, modifiers| {
                    if !recording_key.get() {
                        return glib::Propagation::Proceed;
                    }
                    if key == gtk4::gdk::Key::Escape {
                        recording_key.set(false);
                        recorder_key.remove_css_class("recording");
                        recorder_key.set_label(&original_key.borrow());
                        return glib::Propagation::Stop;
                    }
                    if key == gtk4::gdk::Key::BackSpace {
                        if let Some(b) = cfg_key.borrow_mut().bind_mut(rid) {
                            b.key.clear();
                        }
                        recording_key.set(false);
                        rebuild(&list_key, &cfg_key);
                        return glib::Propagation::Stop;
                    }
                    let Some(combo) = recorded_combo(key, modifiers) else {
                        return glib::Propagation::Stop;
                    };
                    if let Some(b) = cfg_key.borrow_mut().bind_mut(rid) {
                        b.key = combo;
                    }
                    recording_key.set(false);
                    rebuild(&list_key, &cfg_key);
                    glib::Propagation::Stop
                });
                recorder.add_controller(controller);
                row.append(&recorder);
                c.append(&row);

                let normalized = normalize_key_combo(&bind.key);
                let conflict = !normalized.is_empty()
                    && binds.iter().enumerate().any(|(other_idx, other)| {
                        other_idx != idx && normalize_key_combo(&other.key) == normalized
                    });
                if conflict {
                    // Highlight the offending control itself, not just a note.
                    recorder.add_css_class("conflict");
                    let warning = Label::new(Some("conflicts with another managed binding"));
                    warning.add_css_class("key-conflict");
                    warning.set_xalign(0.0);
                    warning.set_margin_start(14);
                    warning.set_margin_bottom(8);
                    c.append(&warning);
                }
            }

            // action
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("action", "what happens when the key is pressed");
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(NIRI_ACTIONS);
                dd.set_enable_search(true);
                let sel_idx = NIRI_ACTIONS
                    .iter()
                    .position(|a| *a == bind.action.as_str())
                    .unwrap_or(0) as u32;
                dd.set_selected(sel_idx);
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    let name = NIRI_ACTIONS
                        .get(d.selected() as usize)
                        .copied()
                        .unwrap_or("spawn");
                    if let Some(b) = cfg_c.borrow_mut().bind_mut(rid) {
                        b.action = name.to_string();
                    }
                });
                row.append(&dd);
                c.append(&row);
            }

            // action arguments
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let hint = action_args_hint(&bind.action);
                let desc = if hint.is_empty() {
                    "arguments for the chosen action (leave blank if none)"
                } else {
                    "arguments for the chosen action"
                };
                let lc = label_col("args", desc);
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                entry.set_text(&bind.action_args);
                if !hint.is_empty() {
                    entry.set_placeholder_text(Some(hint));
                }
                entry
                    .set_sensitive(action_needs_args(&bind.action) || !bind.action_args.is_empty());
                entry.set_width_chars(24);
                let cfg_c = Rc::clone(cfg);
                entry.connect_changed(move |e| {
                    if let Some(b) = cfg_c.borrow_mut().bind_mut(rid) {
                        b.action_args = e.text().to_string();
                    }
                });
                row.append(&entry);
                c.append(&row);
            }

            // options row: repeat + cooldown
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();

                let lc_repeat = label_col("repeat", "fire repeatedly while the key is held");
                lc_repeat.set_hexpand(true);
                row.append(&lc_repeat);
                let sw_repeat = Switch::new();
                sw_repeat.set_active(bind.repeat);
                let cfg_c = Rc::clone(cfg);
                sw_repeat.connect_active_notify(move |s| {
                    if let Some(b) = cfg_c.borrow_mut().bind_mut(rid) {
                        b.repeat = s.is_active();
                    }
                });
                row.append(&sw_repeat);

                let lc_cd = label_col(
                    "cooldown-ms",
                    "min milliseconds between fires  (0 = disabled)",
                );
                lc_cd.set_margin_start(16);
                row.append(&lc_cd);
                let cd_entry = Entry::new();
                cd_entry.add_css_class("color-entry");
                cd_entry.set_width_chars(6);
                cd_entry.set_placeholder_text(Some("0"));
                if bind.cooldown_ms > 0 {
                    cd_entry.set_text(&bind.cooldown_ms.to_string());
                }
                let cfg_c = Rc::clone(cfg);
                bind_u32_entry(&cd_entry, move |v| {
                    if let Some(b) = cfg_c.borrow_mut().bind_mut(rid) {
                        b.cooldown_ms = v;
                    }
                });
                row.append(&cd_entry);

                c.append(&row);
            }

            // allow-when-locked
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col(
                    "allow when locked",
                    "fire even while the screen is locked (spawn only)",
                );
                lc.set_hexpand(true);
                row.append(&lc);
                let sw = Switch::new();
                sw.set_active(bind.allow_when_locked);
                sw.set_sensitive(bind.action == "spawn");
                let cfg_c = Rc::clone(cfg);
                sw.connect_active_notify(move |s| {
                    if let Some(b) = cfg_c.borrow_mut().bind_mut(rid) {
                        b.allow_when_locked = s.is_active();
                    }
                });
                row.append(&sw);
                c.append(&row);
            }

            // delete button
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let btn = Button::with_label("delete bind");
                btn.add_css_class("destructive-action");
                let cfg_c = Rc::clone(cfg);
                let list_c = Rc::clone(list);
                btn.connect_clicked(move |_| {
                    cfg_c.borrow_mut().remove_bind(rid);
                    rebuild(&list_c, &cfg_c);
                });
                row.append(&btn);
                c.append(&row);
            }

            list.append(&c);
        }
    }

    rebuild(&list, &cfg);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let inner = GtkBox::new(Orientation::Vertical, 8);
    inner.set_margin_start(0);
    inner.set_margin_end(0);
    inner.append(&*list);

    // add bind button
    let add_btn = Button::with_label("+ add bind");
    add_btn.add_css_class("suggested-action-custom");
    add_btn.set_halign(Align::Start);
    let cfg_c = Rc::clone(&cfg);
    let list_c = Rc::clone(&list);
    add_btn.connect_clicked(move |_| {
        cfg_c.borrow_mut().binds.push(Keybind::default());
        rebuild(&list_c, &cfg_c);
    });

    // import from niri config button
    let import_btn = Button::with_label("import from niri config");
    import_btn.add_css_class("suggested-action-custom");
    import_btn.set_halign(Align::Start);
    let cfg_c = Rc::clone(&cfg);
    let list_c = Rc::clone(&list);
    import_btn.connect_clicked(move |_| {
        let imported = match crate::settings_backend::niri_config_path() {
            Ok(path) => import_binds_from_niri_config(&path),
            Err(e) => {
                log::warn!("settings: cannot find niri config: {e}");
                return;
            }
        };
        if imported.is_empty() {
            log::warn!("settings: no binds found in niri config");
            return;
        }
        let mut guard = cfg_c.borrow_mut();
        let mut seen_keys: std::collections::HashSet<String> =
            guard.binds.iter().map(|b| b.key.clone()).collect();
        let mut added = 0usize;
        for b in imported {
            if seen_keys.insert(b.key.clone()) {
                guard.binds.push(b);
                added += 1;
            }
        }
        drop(guard);
        log::info!("settings: imported {added} new binds from niri config");
        rebuild(&list_c, &cfg_c);
    });

    let btn_row = GtkBox::new(Orientation::Horizontal, 8);
    btn_row.append(&add_btn);
    btn_row.append(&import_btn);
    inner.append(&btn_row);

    scroll.set_child(Some(&inner));
    page.append(&scroll);
    page
}

// ── Window Rules page ─────────────────────────────────────────────────────────

pub fn build_window_rules_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    fn rebuild(rb: &Rc<GtkBox>, cfg: &Rc<RefCell<SettingsConfig>>) {
        while let Some(child) = rb.first_child() {
            rb.remove(&child);
        }
        let rules = cfg.borrow().window_rules.clone();
        for (idx, rule) in rules.iter().enumerate() {
            let title = if !rule.match_app_id.is_empty() {
                format!("rule {} \u{2014} {}", idx + 1, rule.match_app_id)
            } else {
                format!("rule {}", idx + 1)
            };
            let c = card(&title);
            // Address this row by its stable id, not its index.
            let rid = rule.id;

            // match criteria
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("app-id", "regex against window app ID (empty = any)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                entry.set_text(&rule.match_app_id);
                entry.set_placeholder_text(Some("e.g. ^firefox$"));
                entry.set_width_chars(18);
                let cfg_c = Rc::clone(cfg);
                bind_regex_entry(&entry, move |v| {
                    if let Some(r) = cfg_c.borrow_mut().window_rule_mut(rid) {
                        r.match_app_id = v;
                    }
                });
                row.append(&entry);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("title", "regex against window title (empty = any)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                entry.set_text(&rule.match_title);
                entry.set_placeholder_text(Some("e.g. ^Media viewer$"));
                entry.set_width_chars(18);
                let cfg_c = Rc::clone(cfg);
                bind_regex_entry(&entry, move |v| {
                    if let Some(r) = cfg_c.borrow_mut().window_rule_mut(rid) {
                        r.match_title = v;
                    }
                });
                row.append(&entry);
                c.append(&row);
            }

            // TriState dropdowns helper
            let tristate_rows: &[TriStateRow<WindowRule>] = &[
                (
                    "at startup",
                    "match only during first 60 s after niri starts",
                    |r| &r.match_at_startup,
                    |r, v| r.match_at_startup = v,
                ),
                (
                    "open maximized",
                    "column fills monitor width on open",
                    |r| &r.open_maximized,
                    |r, v| r.open_maximized = v,
                ),
                (
                    "open fullscreen",
                    "window opens in fullscreen mode",
                    |r| &r.open_fullscreen,
                    |r, v| r.open_fullscreen = v,
                ),
                (
                    "open floating",
                    "window opens in the floating layout",
                    |r| &r.open_floating,
                    |r, v| r.open_floating = v,
                ),
                (
                    "open focused",
                    "window receives keyboard focus when opened",
                    |r| &r.open_focused,
                    |r, v| r.open_focused = v,
                ),
            ];
            for (label, desc, getter, setter) in tristate_rows {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col(label, desc);
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(TriState::label_variants());
                dd.set_selected(getter(rule).to_index());
                let cfg_c = Rc::clone(cfg);
                let setter_copy = *setter;
                dd.connect_selected_notify(move |d| {
                    if let Some(r) = cfg_c.borrow_mut().window_rule_mut(rid) {
                        setter_copy(r, TriState::from_index(d.selected()));
                    }
                });
                row.append(&dd);
                c.append(&row);
            }

            // String entries for output/workspace
            for (label, desc, placeholder, getter, setter) in [
                (
                    "open on output",
                    "connector name e.g. HDMI-A-1  (empty = default)",
                    "connector or make/model",
                    (|r: &WindowRule| r.open_on_output.as_str()) as fn(&WindowRule) -> &str,
                    (|r: &mut WindowRule, v: String| r.open_on_output = v)
                        as fn(&mut WindowRule, String),
                ),
                (
                    "open on workspace",
                    "named workspace  (empty = default)",
                    "workspace name",
                    (|r: &WindowRule| r.open_on_workspace.as_str()) as fn(&WindowRule) -> &str,
                    (|r: &mut WindowRule, v: String| r.open_on_workspace = v)
                        as fn(&mut WindowRule, String),
                ),
            ] {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col(label, desc);
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                entry.set_text(getter(rule));
                entry.set_placeholder_text(Some(placeholder));
                entry.set_width_chars(18);
                let cfg_c = Rc::clone(cfg);
                entry.connect_changed(move |e| {
                    if let Some(r) = cfg_c.borrow_mut().window_rule_mut(rid) {
                        setter(r, e.text().to_string());
                    }
                });
                row.append(&entry);
                c.append(&row);
            }

            // Dynamic properties: opacity
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("opacity", "0.0\u{2013}1.0  (empty = no override)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                if rule.opacity != 0.0 {
                    entry.set_text(&format!("{:.2}", rule.opacity));
                }
                entry.set_placeholder_text(Some("0.0\u{2013}1.0"));
                entry.set_width_chars(8);
                let cfg_c = Rc::clone(cfg);
                bind_f64_entry(&entry, move |v| {
                    if let Some(r) = cfg_c.borrow_mut().window_rule_mut(rid) {
                        r.opacity = v;
                    }
                });
                row.append(&entry);
                c.append(&row);
            }

            // block-out-from
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("block-out-from", "replace with black in screencasts");
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(BlockOutFrom::label_variants());
                dd.set_selected(rule.block_out_from.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    if let Some(r) = cfg_c.borrow_mut().window_rule_mut(rid) {
                        r.block_out_from = BlockOutFrom::from_index(d.selected());
                    }
                });
                row.append(&dd);
                c.append(&row);
            }

            // More TriState dynamic properties
            let dynamic_tristate: &[TriStateRow<WindowRule>] = &[
                (
                    "draw border with background",
                    "draw border/focus-ring as filled rectangle",
                    |r| &r.draw_border_with_background,
                    |r, v| r.draw_border_with_background = v,
                ),
                (
                    "clip to geometry",
                    "clip window to visual geometry (rounds corners)",
                    |r| &r.clip_to_geometry,
                    |r, v| r.clip_to_geometry = v,
                ),
                (
                    "variable refresh rate",
                    "enable VRR on output when this window is displayed",
                    |r| &r.variable_refresh_rate,
                    |r, v| r.variable_refresh_rate = v,
                ),
            ];
            for (label, desc, getter, setter) in dynamic_tristate {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col(label, desc);
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(TriState::label_variants());
                dd.set_selected(getter(rule).to_index());
                let cfg_c = Rc::clone(cfg);
                let setter_copy = *setter;
                dd.connect_selected_notify(move |d| {
                    if let Some(r) = cfg_c.borrow_mut().window_rule_mut(rid) {
                        setter_copy(r, TriState::from_index(d.selected()));
                    }
                });
                row.append(&dd);
                c.append(&row);
            }

            // corner radius
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col(
                    "corner radius",
                    "window geometry corner radius  (0 = no override)",
                );
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                if rule.geometry_corner_radius != 0.0 {
                    entry.set_text(&format!("{:.1}", rule.geometry_corner_radius));
                }
                entry.set_placeholder_text(Some("px"));
                entry.set_width_chars(8);
                let cfg_c = Rc::clone(cfg);
                bind_f64_entry(&entry, move |v| {
                    if let Some(r) = cfg_c.borrow_mut().window_rule_mut(rid) {
                        r.geometry_corner_radius = v;
                    }
                });
                row.append(&entry);
                c.append(&row);
            }

            // size limits
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("min / max width", "logical pixels  (0 = no limit)");
                lc.set_hexpand(true);
                row.append(&lc);
                let min_w = Entry::new();
                min_w.add_css_class("color-entry");
                if rule.min_width != 0 {
                    min_w.set_text(&rule.min_width.to_string());
                }
                min_w.set_placeholder_text(Some("min px"));
                min_w.set_width_chars(7);
                let cfg_c = Rc::clone(cfg);
                bind_u32_entry(&min_w, move |v| {
                    if let Some(r) = cfg_c.borrow_mut().window_rule_mut(rid) {
                        r.min_width = v;
                    }
                });
                row.append(&min_w);
                let max_w = Entry::new();
                max_w.add_css_class("color-entry");
                if rule.max_width != 0 {
                    max_w.set_text(&rule.max_width.to_string());
                }
                max_w.set_placeholder_text(Some("max px"));
                max_w.set_width_chars(7);
                let cfg_c = Rc::clone(cfg);
                bind_u32_entry(&max_w, move |v| {
                    if let Some(r) = cfg_c.borrow_mut().window_rule_mut(rid) {
                        r.max_width = v;
                    }
                });
                row.append(&max_w);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col("min / max height", "logical pixels  (0 = no limit)");
                lc.set_hexpand(true);
                row.append(&lc);
                let min_h = Entry::new();
                min_h.add_css_class("color-entry");
                if rule.min_height != 0 {
                    min_h.set_text(&rule.min_height.to_string());
                }
                min_h.set_placeholder_text(Some("min px"));
                min_h.set_width_chars(7);
                let cfg_c = Rc::clone(cfg);
                bind_u32_entry(&min_h, move |v| {
                    if let Some(r) = cfg_c.borrow_mut().window_rule_mut(rid) {
                        r.min_height = v;
                    }
                });
                row.append(&min_h);
                let max_h = Entry::new();
                max_h.add_css_class("color-entry");
                if rule.max_height != 0 {
                    max_h.set_text(&rule.max_height.to_string());
                }
                max_h.set_placeholder_text(Some("max px"));
                max_h.set_width_chars(7);
                let cfg_c = Rc::clone(cfg);
                bind_u32_entry(&max_h, move |v| {
                    if let Some(r) = cfg_c.borrow_mut().window_rule_mut(rid) {
                        r.max_height = v;
                    }
                });
                row.append(&max_h);
                c.append(&row);
            }
            {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col(
                    "scroll factor",
                    "multiplies all scroll events  (0 = no override)",
                );
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                if rule.scroll_factor != 0.0 {
                    entry.set_text(&format!("{:.2}", rule.scroll_factor));
                }
                entry.set_placeholder_text(Some("e.g. 0.75"));
                entry.set_width_chars(8);
                let cfg_c = Rc::clone(cfg);
                bind_f64_entry(&entry, move |v| {
                    if let Some(r) = cfg_c.borrow_mut().window_rule_mut(rid) {
                        r.scroll_factor = v;
                    }
                });
                row.append(&entry);
                c.append(&row);
            }

            // delete button
            c.append(&Separator::new(Orientation::Horizontal));
            let del_row = row_box();
            let del_btn = Button::with_label("delete rule");
            del_btn.add_css_class("sidebar-item");
            del_btn.set_halign(Align::End);
            del_btn.set_hexpand(true);
            del_btn.set_margin_end(6);
            del_btn.set_margin_top(4);
            del_btn.set_margin_bottom(4);
            let cfg_d = Rc::clone(cfg);
            let rb_d = Rc::clone(rb);
            del_btn.connect_clicked(move |_| {
                cfg_d.borrow_mut().remove_window_rule(rid);
                rebuild(&rb_d, &cfg_d);
            });
            del_row.append(&del_btn);
            c.append(&del_row);

            rb.append(&c);
        }
    }

    let page = page_box();
    page.append(&page_header(
        "window rules",
        "override properties per window",
    ));

    page.append(&build_compact_rule_table(Rc::clone(&cfg)));

    let advanced_label = Label::new(Some("ADVANCED RULE PROPERTIES"));
    advanced_label.add_css_class("sidebar-section-label");
    advanced_label.set_xalign(0.0);
    advanced_label.set_margin_top(12);
    page.append(&advanced_label);

    let rules_box = Rc::new(GtkBox::new(Orientation::Vertical, 8));
    page.append(&*rules_box);
    rebuild(&rules_box, &cfg);

    let add_btn = Button::with_label("+ add rule");
    add_btn.add_css_class("save-btn");
    add_btn.set_halign(Align::Start);
    add_btn.set_margin_start(14);
    add_btn.set_margin_top(8);
    add_btn.set_margin_bottom(8);
    let cfg_a = Rc::clone(&cfg);
    let rb_a = Rc::clone(&rules_box);
    add_btn.connect_clicked(move |_| {
        cfg_a.borrow_mut().window_rules.push(WindowRule {
            id: next_row_id(),
            ..Default::default()
        });
        rebuild(&rb_a, &cfg_a);
    });
    page.append(&add_btn);
    page.append(&save_button(cfg));
    page
}

fn build_compact_rule_table(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let table = card("window rules");
    let header = GtkBox::new(Orientation::Horizontal, 8);
    header.add_css_class("rule-table-header");
    for (label, width) in [
        ("APP ID", 16),
        ("TITLE MATCH", 16),
        ("WORKSPACE", 10),
        ("LAYOUT", 9),
    ] {
        let column = Label::new(Some(label));
        column.add_css_class("rule-column-title");
        column.set_width_chars(width);
        column.set_xalign(0.0);
        if label == "TITLE MATCH" {
            column.set_hexpand(true);
        }
        header.append(&column);
    }
    table.append(&header);

    let rules = cfg.borrow().window_rules.clone();
    if rules.is_empty() {
        let empty = Label::new(Some("No managed rules yet — use “add rule” below."));
        empty.add_css_class("row-sublabel");
        empty.set_margin_top(12);
        empty.set_margin_bottom(12);
        table.append(&empty);
    }
    for rule in rules.iter() {
        // Stable id: this table is built once and shares `cfg` with the card
        // stack below, which may delete rules; addressing by index would go
        // stale, so resolve by id on every edit.
        let rid = rule.id;
        let row = GtkBox::new(Orientation::Horizontal, 8);
        row.add_css_class("rule-table-row");
        let app_id = Entry::new();
        app_id.add_css_class("table-entry");
        app_id.set_width_chars(16);
        app_id.set_text(&rule.match_app_id);
        let cfg_app = Rc::clone(&cfg);
        bind_regex_entry(&app_id, move |v| {
            if let Some(r) = cfg_app.borrow_mut().window_rule_mut(rid) {
                r.match_app_id = v;
            }
        });

        let title = Entry::new();
        title.add_css_class("table-entry");
        title.set_width_chars(16);
        title.set_hexpand(true);
        title.set_text(&rule.match_title);
        let cfg_title = Rc::clone(&cfg);
        bind_regex_entry(&title, move |v| {
            if let Some(r) = cfg_title.borrow_mut().window_rule_mut(rid) {
                r.match_title = v;
            }
        });

        let workspace = Entry::new();
        workspace.add_css_class("table-entry");
        workspace.set_width_chars(10);
        workspace.set_text(&rule.open_on_workspace);
        workspace.set_placeholder_text(Some("any"));
        let cfg_workspace = Rc::clone(&cfg);
        workspace.connect_changed(move |entry| {
            if let Some(r) = cfg_workspace.borrow_mut().window_rule_mut(rid) {
                r.open_on_workspace = entry.text().to_string();
            }
        });

        let layout = DropDown::from_strings(&["default", "tiled", "floating"]);
        layout.set_selected(match rule.open_floating {
            TriState::Off => 1,
            TriState::On => 2,
            TriState::Default => 0,
        });
        let cfg_layout = Rc::clone(&cfg);
        layout.connect_selected_notify(move |dropdown| {
            if let Some(r) = cfg_layout.borrow_mut().window_rule_mut(rid) {
                r.open_floating = match dropdown.selected() {
                    1 => TriState::Off,
                    2 => TriState::On,
                    _ => TriState::Default,
                };
            }
        });
        row.append(&app_id);
        row.append(&title);
        row.append(&workspace);
        row.append(&layout);
        table.append(&row);
    }
    table
}

// ── Layer Rules page ──────────────────────────────────────────────────────────

pub fn build_layer_rules_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    fn rebuild(rb: &Rc<GtkBox>, cfg: &Rc<RefCell<SettingsConfig>>) {
        while let Some(child) = rb.first_child() {
            rb.remove(&child);
        }
        let rules = cfg.borrow().layer_rules.clone();
        for (idx, rule) in rules.iter().enumerate() {
            let title = if !rule.match_namespace.is_empty() {
                format!("rule {} \u{2014} {}", idx + 1, rule.match_namespace)
            } else {
                format!("rule {}", idx + 1)
            };
            let c = card(&title);
            // Address this row by its stable id, not its index.
            let rid = rule.id;

            // namespace
            c.append(&Separator::new(Orientation::Horizontal));
            {
                let row = row_box();
                let lc = label_col(
                    "namespace",
                    "regex matching layer surface namespace (empty = any)",
                );
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                entry.set_text(&rule.match_namespace);
                entry.set_placeholder_text(Some("e.g. ^waybar$"));
                entry.set_width_chars(18);
                let cfg_c = Rc::clone(cfg);
                bind_regex_entry(&entry, move |v| {
                    if let Some(r) = cfg_c.borrow_mut().layer_rule_mut(rid) {
                        r.match_namespace = v;
                    }
                });
                row.append(&entry);
                c.append(&row);
            }

            // TriState dropdowns
            let tristate_rows: &[TriStateRow<LayerRule>] = &[
                (
                    "at startup",
                    "match only during first 60 s after niri starts",
                    |r| &r.match_at_startup,
                    |r, v| r.match_at_startup = v,
                ),
                (
                    "shadow",
                    "force shadow on / off for this surface",
                    |r| &r.shadow,
                    |r, v| r.shadow = v,
                ),
                (
                    "place within backdrop",
                    "show surface inside Overview / workspace-switch backdrop",
                    |r| &r.place_within_backdrop,
                    |r, v| r.place_within_backdrop = v,
                ),
            ];
            for (label, desc, getter, setter) in tristate_rows {
                c.append(&Separator::new(Orientation::Horizontal));
                let row = row_box();
                let lc = label_col(label, desc);
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(TriState::label_variants());
                dd.set_selected(getter(rule).to_index());
                let cfg_c = Rc::clone(cfg);
                let setter_copy = *setter;
                dd.connect_selected_notify(move |d| {
                    if let Some(r) = cfg_c.borrow_mut().layer_rule_mut(rid) {
                        setter_copy(r, TriState::from_index(d.selected()));
                    }
                });
                row.append(&dd);
                c.append(&row);
            }

            // opacity
            c.append(&Separator::new(Orientation::Horizontal));
            {
                let row = row_box();
                let lc = label_col("opacity", "0.0\u{2013}1.0  (empty = no override)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                if rule.opacity != 0.0 {
                    entry.set_text(&format!("{:.2}", rule.opacity));
                }
                entry.set_placeholder_text(Some("0.0\u{2013}1.0"));
                entry.set_width_chars(8);
                let cfg_c = Rc::clone(cfg);
                bind_f64_entry(&entry, move |v| {
                    if let Some(r) = cfg_c.borrow_mut().layer_rule_mut(rid) {
                        r.opacity = v;
                    }
                });
                row.append(&entry);
                c.append(&row);
            }

            // block-out-from
            c.append(&Separator::new(Orientation::Horizontal));
            {
                let row = row_box();
                let lc = label_col("block-out-from", "replace with black in screencasts");
                lc.set_hexpand(true);
                row.append(&lc);
                let dd = DropDown::from_strings(BlockOutFrom::label_variants());
                dd.set_selected(rule.block_out_from.to_index());
                let cfg_c = Rc::clone(cfg);
                dd.connect_selected_notify(move |d| {
                    if let Some(r) = cfg_c.borrow_mut().layer_rule_mut(rid) {
                        r.block_out_from = BlockOutFrom::from_index(d.selected());
                    }
                });
                row.append(&dd);
                c.append(&row);
            }

            // corner radius
            c.append(&Separator::new(Orientation::Horizontal));
            {
                let row = row_box();
                let lc = label_col("corner radius", "rounds shadow corners  (0 = no override)");
                lc.set_hexpand(true);
                row.append(&lc);
                let entry = Entry::new();
                entry.add_css_class("color-entry");
                if rule.geometry_corner_radius != 0.0 {
                    entry.set_text(&format!("{:.1}", rule.geometry_corner_radius));
                }
                entry.set_placeholder_text(Some("px"));
                entry.set_width_chars(8);
                let cfg_c = Rc::clone(cfg);
                bind_f64_entry(&entry, move |v| {
                    if let Some(r) = cfg_c.borrow_mut().layer_rule_mut(rid) {
                        r.geometry_corner_radius = v;
                    }
                });
                row.append(&entry);
                c.append(&row);
            }

            // delete button
            c.append(&Separator::new(Orientation::Horizontal));
            let del_row = row_box();
            let del_btn = Button::with_label("delete rule");
            del_btn.add_css_class("sidebar-item");
            del_btn.set_halign(Align::End);
            del_btn.set_hexpand(true);
            del_btn.set_margin_end(6);
            del_btn.set_margin_top(4);
            del_btn.set_margin_bottom(4);
            let cfg_d = Rc::clone(cfg);
            let rb_d = Rc::clone(rb);
            del_btn.connect_clicked(move |_| {
                cfg_d.borrow_mut().remove_layer_rule(rid);
                rebuild(&rb_d, &cfg_d);
            });
            del_row.append(&del_btn);
            c.append(&del_row);

            rb.append(&c);
        }
    }

    let page = page_box();
    page.append(&page_header(
        "layer rules",
        "override properties for layer-shell surfaces",
    ));

    let rules_box = Rc::new(GtkBox::new(Orientation::Vertical, 8));
    page.append(&*rules_box);
    rebuild(&rules_box, &cfg);

    let add_btn = Button::with_label("+ add rule");
    add_btn.add_css_class("save-btn");
    add_btn.set_halign(Align::Start);
    add_btn.set_margin_start(14);
    add_btn.set_margin_top(8);
    add_btn.set_margin_bottom(8);
    let cfg_a = Rc::clone(&cfg);
    let rb_a = Rc::clone(&rules_box);
    add_btn.connect_clicked(move |_| {
        cfg_a.borrow_mut().layer_rules.push(LayerRule {
            id: next_row_id(),
            ..Default::default()
        });
        rebuild(&rb_a, &cfg_a);
    });
    page.append(&add_btn);
    page.append(&save_button(cfg));
    page
}

// ── Switch Events page ───────────────────────────────────────────────────────

pub fn build_switch_events_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    fn add_event_row<F>(
        c: &GtkBox,
        label: &str,
        sub: &str,
        initial: Vec<String>,
        cfg: Rc<RefCell<SettingsConfig>>,
        setter: F,
    ) where
        F: Fn(&mut SwitchEventsSettings, Vec<String>) + 'static,
    {
        c.append(&Separator::new(Orientation::Horizontal));
        let row = row_box();
        let lc = label_col(label, sub);
        lc.set_hexpand(true);
        row.append(&lc);
        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_text(&initial.join(" "));
        entry.set_placeholder_text(Some("program arg1 arg2 \u{2026}"));
        entry.set_width_chars(26);
        entry.connect_changed(move |e| {
            let argv: Vec<String> = e
                .text()
                .to_string()
                .split_whitespace()
                .map(str::to_string)
                .collect();
            setter(&mut cfg.borrow_mut().switch_events, argv);
        });
        row.append(&entry);
        c.append(&row);
    }

    let page = page_box();
    page.append(&page_header(
        "switch events",
        "run commands when hardware switches change state",
    ));

    let c = card("switch events");
    let hint = Label::new(Some(
        "Commands are space-separated argv  (e.g. notify-send Lid Closed).  \
         Leave blank to disable.",
    ));
    hint.set_wrap(true);
    hint.add_css_class("row-sublabel");
    hint.set_margin_start(14);
    hint.set_margin_end(14);
    hint.set_margin_top(4);
    hint.set_margin_bottom(8);
    c.append(&hint);

    let se = cfg.borrow().switch_events.clone();
    add_event_row(
        &c,
        "lid-close",
        "laptop lid closed",
        se.lid_close.clone(),
        Rc::clone(&cfg),
        |s, v| s.lid_close = v,
    );
    add_event_row(
        &c,
        "lid-open",
        "laptop lid opened",
        se.lid_open.clone(),
        Rc::clone(&cfg),
        |s, v| s.lid_open = v,
    );
    add_event_row(
        &c,
        "tablet-mode-on",
        "convertible enters tablet mode",
        se.tablet_mode_on.clone(),
        Rc::clone(&cfg),
        |s, v| s.tablet_mode_on = v,
    );
    add_event_row(
        &c,
        "tablet-mode-off",
        "convertible leaves tablet mode",
        se.tablet_mode_off.clone(),
        Rc::clone(&cfg),
        |s, v| s.tablet_mode_off = v,
    );

    page.append(&c);
    page.append(&save_button(cfg));
    page
}
