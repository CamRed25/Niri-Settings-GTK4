// settings_ui/input_pages.rs — Behaviour, input, and layout settings pages.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, DropDown, Entry, Label, Orientation, Scale, Separator};

use super::helpers_ui::*;
use crate::settings_backend::{
    AccelProfile, CenterFocusedColumn, ClickMethod, DragSetting, MouseScrollMethod, SettingsConfig,
    TapButtonMap, TouchpadScrollMethod, TrackLayout,
};

// ── Behaviour page ────────────────────────────────────────────────────────────

pub fn build_behaviour_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header("behaviour", "compositor & input settings"));

    let comp_card = card("compositor");
    append_toggle(
        &comp_card,
        "prefer no decorations",
        "ask apps to omit client-side decorations (prefer-no-csd)",
        cfg.borrow().misc.prefer_no_csd,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.misc.prefer_no_csd = v),
    );
    append_toggle(
        &comp_card,
        "focus follows mouse",
        "focus windows as the pointer moves over them",
        cfg.borrow().input.focus_follows_mouse,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.input.focus_follows_mouse = v),
    );
    append_toggle(
        &comp_card,
        "workspace auto back-and-forth",
        "re-selecting the current workspace switches back to the previous one",
        cfg.borrow().input.workspace_auto_back_and_forth,
        cfg_toggle(Rc::clone(&cfg), |c, v| {
            c.input.workspace_auto_back_and_forth = v;
        }),
    );
    append_toggle(
        &comp_card,
        "warp mouse to focus",
        "move the cursor to newly focused windows",
        cfg.borrow().input.warp_mouse_to_focus,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.input.warp_mouse_to_focus = v),
    );
    append_toggle(
        &comp_card,
        "skip hotkey overlay at startup",
        "hide the 'Important Hotkeys' overlay when niri starts",
        cfg.borrow().misc.skip_hotkey_overlay,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.misc.skip_hotkey_overlay = v),
    );
    append_toggle(
        &comp_card,
        "disable power key handling",
        "let systemd/logind handle the power button instead of niri",
        cfg.borrow().input.disable_power_key_handling,
        cfg_toggle(Rc::clone(&cfg), |c, v| {
            c.input.disable_power_key_handling = v;
        }),
    );
    page.append(&comp_card);

    page.append(&save_button(cfg));
    page
}

// ── Input page ────────────────────────────────────────────────────────────────

pub fn build_input_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header("input", "keyboard, touchpad & mouse"));

    // ── Card: keyboard ────────────────────────────────────────────────────────
    let kb_card = card("keyboard");

    type XkbRow = (
        &'static str,
        &'static str,
        fn(&SettingsConfig) -> &str,
        fn(&mut SettingsConfig, String),
    );
    let xkb_rows: &[XkbRow] = &[
        (
            "layout",
            "e.g. us,de",
            |c| &c.input.keyboard_xkb_layout,
            |c, v| c.input.keyboard_xkb_layout = v,
        ),
        (
            "variant",
            "e.g. dvorak",
            |c| &c.input.keyboard_xkb_variant,
            |c, v| c.input.keyboard_xkb_variant = v,
        ),
        (
            "model",
            "e.g. pc105",
            |c| &c.input.keyboard_xkb_model,
            |c, v| c.input.keyboard_xkb_model = v,
        ),
        (
            "rules",
            "leave blank for default",
            |c| &c.input.keyboard_xkb_rules,
            |c, v| c.input.keyboard_xkb_rules = v,
        ),
        (
            "options",
            "e.g. compose:ralt,caps:escape",
            |c| &c.input.keyboard_xkb_options,
            |c, v| c.input.keyboard_xkb_options = v,
        ),
    ];
    for (label, hint, getter, setter) in xkb_rows {
        let sep = Separator::new(Orientation::Horizontal);
        sep.add_css_class("row-sep");
        kb_card.append(&sep);

        let row = row_box();
        let lbl_col = label_col(label, hint);
        lbl_col.set_hexpand(true);
        row.append(&lbl_col);

        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_text(getter(&cfg.borrow()));
        entry.set_width_chars(20);
        entry.set_max_width_chars(40);
        let cfg_e = Rc::clone(&cfg);
        let setter_copy = *setter;
        entry.connect_changed(move |e| {
            setter_copy(&mut cfg_e.borrow_mut(), e.text().to_string());
        });
        row.append(&entry);
        kb_card.append(&row);
    }

    // Repeat delay slider
    {
        kb_card.append(&Separator::new(Orientation::Horizontal));
        let row = row_box();
        let lc = label_col(
            "repeat delay",
            "key-held delay before repeat starts (ms, 0=default 600)",
        );
        lc.set_hexpand(true);
        row.append(&lc);

        let init_d = cfg.borrow().input.keyboard_repeat_delay;
        let val_lbl = Label::new(Some(&if init_d == 0 {
            "default".to_string()
        } else {
            format!("{init_d}ms")
        }));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(8);

        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 2000.0, 50.0);
        scale.set_value(init_d as f64);
        scale.set_width_request(140);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = s.value().round() as u32;
            cfg_s.borrow_mut().input.keyboard_repeat_delay = v;
            val_lbl_c.set_text(&if v == 0 {
                "default".to_string()
            } else {
                format!("{v}ms")
            });
        });

        row.append(&scale);
        row.append(&val_lbl);
        kb_card.append(&row);
    }

    // Repeat rate slider
    {
        kb_card.append(&Separator::new(Orientation::Horizontal));
        let row = row_box();
        let lc = label_col(
            "repeat rate",
            "key repeat speed in characters per second (0=default 25)",
        );
        lc.set_hexpand(true);
        row.append(&lc);

        let init_r = cfg.borrow().input.keyboard_repeat_rate;
        let val_lbl = Label::new(Some(&if init_r == 0 {
            "default".to_string()
        } else {
            format!("{init_r} cps")
        }));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(9);

        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
        scale.set_value(init_r as f64);
        scale.set_width_request(140);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = s.value().round() as u32;
            cfg_s.borrow_mut().input.keyboard_repeat_rate = v;
            val_lbl_c.set_text(&if v == 0 {
                "default".to_string()
            } else {
                format!("{v} cps")
            });
        });

        row.append(&scale);
        row.append(&val_lbl);
        kb_card.append(&row);
    }

    // Track layout dropdown
    {
        kb_card.append(&Separator::new(Orientation::Horizontal));
        let row = row_box();
        let lc = label_col(
            "track layout",
            "which layout to use when multiple are configured",
        );
        lc.set_hexpand(true);
        row.append(&lc);

        let options = gtk4::StringList::new(&["global", "window"]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.keyboard_track_layout {
            TrackLayout::Global => 0,
            TrackLayout::Window => 1,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.keyboard_track_layout = match d.selected() {
                1 => TrackLayout::Window,
                _ => TrackLayout::Global,
            };
        });
        row.append(&dd);
        kb_card.append(&row);
    }

    // Numlock toggle
    kb_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &kb_card,
        "enable Num Lock at startup",
        "turn on Num Lock automatically when niri starts",
        cfg.borrow().input.numlock,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.input.numlock = v),
    );

    page.append(&kb_card);

    // ── Card: touchpad ────────────────────────────────────────────────────────
    let tp_card = card("touchpad");

    append_toggle(
        &tp_card,
        "disable touchpad",
        "turn off the touchpad entirely",
        cfg.borrow().input.touchpad_off,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.input.touchpad_off = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "tap to click",
        "register a click when tapping the touchpad surface",
        cfg.borrow().input.touchpad_tap,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.input.touchpad_tap = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "disable while typing",
        "pause touchpad input while keys are pressed (dwt)",
        cfg.borrow().input.touchpad_dwt,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.input.touchpad_dwt = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "disable while trackpointing",
        "pause touchpad while an external pointing device is in use (dwtp)",
        cfg.borrow().input.touchpad_dwtp,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.input.touchpad_dwtp = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));

    // Drag dropdown
    {
        let row = row_box();
        let lc = label_col("drag", "enable or disable tap-to-drag");
        lc.set_hexpand(true);
        row.append(&lc);

        let options = gtk4::StringList::new(&["default", "enabled", "disabled"]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.touchpad_drag {
            DragSetting::Default => 0,
            DragSetting::Enabled => 1,
            DragSetting::Disabled => 2,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.touchpad_drag = match d.selected() {
                1 => DragSetting::Enabled,
                2 => DragSetting::Disabled,
                _ => DragSetting::Default,
            };
        });
        row.append(&dd);
        tp_card.append(&row);
    }

    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "drag lock",
        "keep drag active after lifting the finger until tapped again",
        cfg.borrow().input.touchpad_drag_lock,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.input.touchpad_drag_lock = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "natural scroll",
        "invert scroll direction so content follows the finger",
        cfg.borrow().input.touchpad_natural_scroll,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.input.touchpad_natural_scroll = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));

    // Accel speed slider
    {
        let row = row_box();
        let lc = label_col(
            "accel speed",
            "pointer acceleration speed (-1.0 to 1.0, 0=default)",
        );
        lc.set_hexpand(true);
        row.append(&lc);

        let init = cfg.borrow().input.touchpad_accel_speed;
        let val_lbl = Label::new(Some(&format!("{init:.1}")));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(5);

        let scale = Scale::with_range(Orientation::Horizontal, -1.0, 1.0, 0.1);
        scale.set_value(init);
        scale.set_width_request(140);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = (s.value() * 10.0).round() / 10.0;
            cfg_s.borrow_mut().input.touchpad_accel_speed = v;
            val_lbl_c.set_text(&format!("{v:.1}"));
        });

        row.append(&scale);
        row.append(&val_lbl);
        tp_card.append(&row);
    }

    tp_card.append(&Separator::new(Orientation::Horizontal));

    // Accel profile dropdown
    {
        let row = row_box();
        let lc = label_col("accel profile", "pointer acceleration algorithm");
        lc.set_hexpand(true);
        row.append(&lc);

        let options = gtk4::StringList::new(&["default", "adaptive", "flat"]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.touchpad_accel_profile {
            AccelProfile::Default => 0,
            AccelProfile::Adaptive => 1,
            AccelProfile::Flat => 2,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.touchpad_accel_profile = match d.selected() {
                1 => AccelProfile::Adaptive,
                2 => AccelProfile::Flat,
                _ => AccelProfile::Default,
            };
        });
        row.append(&dd);
        tp_card.append(&row);
    }

    tp_card.append(&Separator::new(Orientation::Horizontal));

    // Scroll method dropdown
    {
        let row = row_box();
        let lc = label_col("scroll method", "how scroll events are generated");
        lc.set_hexpand(true);
        row.append(&lc);

        let options = gtk4::StringList::new(&[
            "default",
            "two-finger",
            "edge",
            "on-button-down",
            "no-scroll",
        ]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.touchpad_scroll_method {
            TouchpadScrollMethod::Default => 0,
            TouchpadScrollMethod::TwoFinger => 1,
            TouchpadScrollMethod::Edge => 2,
            TouchpadScrollMethod::OnButtonDown => 3,
            TouchpadScrollMethod::NoScroll => 4,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.touchpad_scroll_method = match d.selected() {
                1 => TouchpadScrollMethod::TwoFinger,
                2 => TouchpadScrollMethod::Edge,
                3 => TouchpadScrollMethod::OnButtonDown,
                4 => TouchpadScrollMethod::NoScroll,
                _ => TouchpadScrollMethod::Default,
            };
        });
        row.append(&dd);
        tp_card.append(&row);
    }

    tp_card.append(&Separator::new(Orientation::Horizontal));

    // Tap button map dropdown
    {
        let row = row_box();
        let lc = label_col("tap button map", "mapping for 1/2/3 finger taps");
        lc.set_hexpand(true);
        row.append(&lc);

        let options = gtk4::StringList::new(&["default", "left-right-middle", "left-middle-right"]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.touchpad_tap_button_map {
            TapButtonMap::Default => 0,
            TapButtonMap::LeftRightMiddle => 1,
            TapButtonMap::LeftMiddleRight => 2,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.touchpad_tap_button_map = match d.selected() {
                1 => TapButtonMap::LeftRightMiddle,
                2 => TapButtonMap::LeftMiddleRight,
                _ => TapButtonMap::Default,
            };
        });
        row.append(&dd);
        tp_card.append(&row);
    }

    tp_card.append(&Separator::new(Orientation::Horizontal));

    // Click method dropdown
    {
        let row = row_box();
        let lc = label_col("click method", "how physical button clicks are emitted");
        lc.set_hexpand(true);
        row.append(&lc);

        let options = gtk4::StringList::new(&["default", "button-areas", "clickfinger"]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.touchpad_click_method {
            ClickMethod::Default => 0,
            ClickMethod::ButtonAreas => 1,
            ClickMethod::Clickfinger => 2,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.touchpad_click_method = match d.selected() {
                1 => ClickMethod::ButtonAreas,
                2 => ClickMethod::Clickfinger,
                _ => ClickMethod::Default,
            };
        });
        row.append(&dd);
        tp_card.append(&row);
    }

    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "disabled on external mouse",
        "turn off touchpad when a mouse is plugged in",
        cfg.borrow().input.touchpad_disabled_on_external_mouse,
        cfg_toggle(Rc::clone(&cfg), |c, v| {
            c.input.touchpad_disabled_on_external_mouse = v;
        }),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "left-handed mode",
        "swap left and right buttons",
        cfg.borrow().input.touchpad_left_handed,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.input.touchpad_left_handed = v),
    );
    tp_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &tp_card,
        "middle button emulation",
        "press left+right simultaneously to emit a middle click",
        cfg.borrow().input.touchpad_middle_emulation,
        cfg_toggle(Rc::clone(&cfg), |c, v| {
            c.input.touchpad_middle_emulation = v;
        }),
    );

    page.append(&tp_card);

    // ── Card: mouse ───────────────────────────────────────────────────────────
    let ms_card = card("mouse");

    append_toggle(
        &ms_card,
        "disable mouse",
        "turn off all mice",
        cfg.borrow().input.mouse_off,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.input.mouse_off = v),
    );
    ms_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &ms_card,
        "natural scroll",
        "invert the mouse wheel scrolling direction",
        cfg.borrow().input.mouse_natural_scroll,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.input.mouse_natural_scroll = v),
    );
    ms_card.append(&Separator::new(Orientation::Horizontal));

    // Mouse accel speed
    {
        let row = row_box();
        let lc = label_col(
            "accel speed",
            "pointer acceleration speed (-1.0 to 1.0, 0=default)",
        );
        lc.set_hexpand(true);
        row.append(&lc);

        let init = cfg.borrow().input.mouse_accel_speed;
        let val_lbl = Label::new(Some(&format!("{init:.1}")));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_width_chars(5);

        let scale = Scale::with_range(Orientation::Horizontal, -1.0, 1.0, 0.1);
        scale.set_value(init);
        scale.set_width_request(140);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = (s.value() * 10.0).round() / 10.0;
            cfg_s.borrow_mut().input.mouse_accel_speed = v;
            val_lbl_c.set_text(&format!("{v:.1}"));
        });

        row.append(&scale);
        row.append(&val_lbl);
        ms_card.append(&row);
    }

    ms_card.append(&Separator::new(Orientation::Horizontal));

    // Mouse accel profile
    {
        let row = row_box();
        let lc = label_col("accel profile", "pointer acceleration algorithm");
        lc.set_hexpand(true);
        row.append(&lc);

        let options = gtk4::StringList::new(&["default", "adaptive", "flat"]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.mouse_accel_profile {
            AccelProfile::Default => 0,
            AccelProfile::Adaptive => 1,
            AccelProfile::Flat => 2,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.mouse_accel_profile = match d.selected() {
                1 => AccelProfile::Adaptive,
                2 => AccelProfile::Flat,
                _ => AccelProfile::Default,
            };
        });
        row.append(&dd);
        ms_card.append(&row);
    }

    ms_card.append(&Separator::new(Orientation::Horizontal));

    // Mouse scroll method
    {
        let row = row_box();
        let lc = label_col("scroll method", "how scroll events are generated");
        lc.set_hexpand(true);
        row.append(&lc);

        let options = gtk4::StringList::new(&[
            "default",
            "no-scroll",
            "two-finger",
            "edge",
            "on-button-down",
        ]);
        let dd = DropDown::new(Some(options), gtk4::Expression::NONE);
        dd.set_selected(match cfg.borrow().input.mouse_scroll_method {
            MouseScrollMethod::Default => 0,
            MouseScrollMethod::NoScroll => 1,
            MouseScrollMethod::TwoFinger => 2,
            MouseScrollMethod::Edge => 3,
            MouseScrollMethod::OnButtonDown => 4,
        });
        let cfg_d = Rc::clone(&cfg);
        dd.connect_selected_notify(move |d| {
            cfg_d.borrow_mut().input.mouse_scroll_method = match d.selected() {
                1 => MouseScrollMethod::NoScroll,
                2 => MouseScrollMethod::TwoFinger,
                3 => MouseScrollMethod::Edge,
                4 => MouseScrollMethod::OnButtonDown,
                _ => MouseScrollMethod::Default,
            };
        });
        row.append(&dd);
        ms_card.append(&row);
    }

    ms_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &ms_card,
        "left-handed mode",
        "swap left and right buttons",
        cfg.borrow().input.mouse_left_handed,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.input.mouse_left_handed = v),
    );
    ms_card.append(&Separator::new(Orientation::Horizontal));
    append_toggle(
        &ms_card,
        "middle button emulation",
        "press left+right simultaneously to emit a middle click",
        cfg.borrow().input.mouse_middle_emulation,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.input.mouse_middle_emulation = v),
    );

    page.append(&ms_card);

    // ── Card: mod keys ────────────────────────────────────────────────────────
    let mk_card = card("mod keys");

    for (label, hint, getter, setter) in [
        (
            "mod-key",
            "modifier key used for niri actions (e.g. Super)",
            (|c: &SettingsConfig| c.input.mod_key.as_str()) as fn(&SettingsConfig) -> &str,
            (|c: &mut SettingsConfig, v: String| c.input.mod_key = v)
                as fn(&mut SettingsConfig, String),
        ),
        (
            "mod-key-nested",
            "mod key to use inside nested niri (e.g. Alt)",
            (|c: &SettingsConfig| c.input.mod_key_nested.as_str()) as fn(&SettingsConfig) -> &str,
            (|c: &mut SettingsConfig, v: String| c.input.mod_key_nested = v)
                as fn(&mut SettingsConfig, String),
        ),
    ] {
        let sep = Separator::new(Orientation::Horizontal);
        sep.add_css_class("row-sep");
        mk_card.append(&sep);

        let row = row_box();
        let lc = label_col(label, hint);
        lc.set_hexpand(true);
        row.append(&lc);

        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_text(getter(&cfg.borrow()));
        entry.set_placeholder_text(Some("Super"));
        entry.set_width_chars(14);
        entry.set_max_width_chars(30);
        let cfg_e = Rc::clone(&cfg);
        entry.connect_changed(move |e| {
            setter(&mut cfg_e.borrow_mut(), e.text().to_string());
        });
        row.append(&entry);
        mk_card.append(&row);
    }

    page.append(&mk_card);

    page.append(&save_button(cfg));
    page
}

// ── Layout page ───────────────────────────────────────────────────────────────

pub fn build_layout_page(cfg: Rc<RefCell<SettingsConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header("layout", "window layout & appearance"));

    // Card: spacing
    let sp_card = card("spacing");

    // Gaps slider row
    {
        let row = row_box();
        let lc = label_col("gaps", "space between windows in logical pixels");
        lc.set_hexpand(true);
        row.append(&lc);

        let val_lbl = Label::new(Some(&format!("{:.0}px", cfg.borrow().layout.gaps)));
        val_lbl.add_css_class("row-sublabel");
        val_lbl.set_xalign(1.0);
        val_lbl.set_width_chars(5);

        let scale = Scale::with_range(Orientation::Horizontal, 0.0, 64.0, 2.0);
        scale.set_value(cfg.borrow().layout.gaps);
        scale.set_width_request(120);
        scale.set_draw_value(false);
        scale.set_hexpand(false);

        let cfg_s = Rc::clone(&cfg);
        let val_lbl_c = val_lbl.clone();
        scale.connect_value_changed(move |s| {
            let v = s.value().round();
            cfg_s.borrow_mut().layout.gaps = v;
            val_lbl_c.set_text(&format!("{v:.0}px"));
        });

        row.append(&scale);
        row.append(&val_lbl);
        sp_card.append(&row);
    }

    sp_card.append(&Separator::new(Orientation::Horizontal));

    // Center focused column
    {
        let row = row_box();
        let lbl = Label::new(Some("center focused column"));
        lbl.add_css_class("row-label");
        lbl.set_xalign(0.0);
        lbl.set_hexpand(true);
        row.append(&lbl);

        let current_idx = match cfg.borrow().layout.center_focused_column {
            CenterFocusedColumn::Never => 0u32,
            CenterFocusedColumn::OnOverflow => 1,
            CenterFocusedColumn::Always => 2,
        };
        let combo = DropDown::from_strings(&["never", "on overflow", "always"]);
        combo.set_selected(current_idx);

        let cfg_c = Rc::clone(&cfg);
        combo.connect_selected_notify(move |d| {
            let val = match d.selected() {
                1 => CenterFocusedColumn::OnOverflow,
                2 => CenterFocusedColumn::Always,
                _ => CenterFocusedColumn::Never,
            };
            cfg_c.borrow_mut().layout.center_focused_column = val;
        });

        row.append(&combo);
        sp_card.append(&row);
    }

    sp_card.append(&Separator::new(Orientation::Horizontal));

    append_toggle(
        &sp_card,
        "always center single column",
        "center a lone window regardless of the centering policy",
        cfg.borrow().layout.always_center_single_column,
        cfg_toggle(Rc::clone(&cfg), |c, v| {
            c.layout.always_center_single_column = v;
        }),
    );

    page.append(&sp_card);

    // Card: decoration
    let dec_card = card("decoration");

    append_color(
        &dec_card,
        "focus ring active color",
        &cfg.borrow().layout.focus_ring_active_color.clone(),
        cfg_text(Rc::clone(&cfg), |c, v| {
            c.layout.focus_ring_active_color = v;
        }),
    );

    dec_card.append(&Separator::new(Orientation::Horizontal));

    append_color(
        &dec_card,
        "focus ring inactive color",
        &cfg.borrow().layout.focus_ring_inactive_color.clone(),
        cfg_text(Rc::clone(&cfg), |c, v| {
            c.layout.focus_ring_inactive_color = v;
        }),
    );

    dec_card.append(&Separator::new(Orientation::Horizontal));

    append_toggle(
        &dec_card,
        "border",
        "draw borders around all windows (affects window sizing)",
        cfg.borrow().layout.border_on,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.layout.border_on = v),
    );

    dec_card.append(&Separator::new(Orientation::Horizontal));

    append_toggle(
        &dec_card,
        "shadow",
        "draw drop shadows behind windows",
        cfg.borrow().layout.shadow_on,
        cfg_toggle(Rc::clone(&cfg), |c, v| c.layout.shadow_on = v),
    );

    page.append(&dec_card);
    page.append(&save_button(cfg));
    page
}
