// settings_backend/kdl.rs — KDL config generation and string sanitisation.

use super::types::*;

// ── Sanitisation helpers ──────────────────────────────────────────────────────

/// Validates that a colour string is a safe `#rrggbb` / `#rrggbbaa` hex literal.
/// Returns the unchanged string if valid, or a safe white fallback otherwise.
pub fn sanitise_color(s: &str) -> String {
    let s = s.trim();
    if is_valid_color(s) {
        s.to_string()
    } else {
        "#ffffff".to_string()
    }
}

/// Returns `true` when `s` is a valid `#rgb` / `#rgba` / `#rrggbb` / `#rrggbbaa`
/// hex colour string.
pub fn is_valid_color(s: &str) -> bool {
    let s = s.trim();
    s.starts_with('#')
        && matches!(s.len(), 4 | 5 | 7 | 9)
        && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Strips characters that could break a KDL string literal (double-quote,
/// backslash, and control characters). Safe to embed inside `"..."` in KDL.
pub fn sanitise_path(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '"' && *c != '\\' && !c.is_control())
        .collect()
}

/// Sanitises a regex string for safe embedding in a KDL string literal.
pub fn sanitise_regex(s: &str) -> String {
    s.chars().filter(|c| *c != '"' && !c.is_control()).collect()
}

/// Strips characters not valid in a niri key-combo string.
/// Allows ASCII alphanumerics, `+`, `-`, and `_`.
pub fn sanitise_keybind_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '_'))
        .collect()
}

// ── KDL generation ────────────────────────────────────────────────────────────

/// Generates a valid niri KDL config fragment from typed settings.
pub fn generate_kdl(cfg: &SettingsConfig) -> String {
    let mut out = String::from(
        "// Managed by niri-settings. Manual KDL edits are preserved where possible.\n\n",
    );
    write_input(&mut out, cfg);
    write_layout(&mut out, cfg);
    write_misc(&mut out, cfg);
    write_animations(&mut out, cfg);
    write_workspaces(&mut out, cfg);
    write_gestures(&mut out, cfg);
    write_recent_windows(&mut out, cfg);
    write_debug(&mut out, cfg);
    write_outputs(&mut out, cfg);
    write_window_rules(&mut out, cfg);
    write_layer_rules(&mut out, cfg);
    write_switch_events(&mut out, cfg);
    write_binds(&mut out, cfg);
    out
}

// ── Section writers ───────────────────────────────────────────────────────────

fn write_input(out: &mut String, cfg: &SettingsConfig) {
    let i = &cfg.input;
    let mut body = String::new();

    // keyboard sub-block
    {
        let mut kb = String::new();
        let mut xkb = String::new();
        let xkb_fields: &[(&str, &str)] = &[
            ("layout", &i.keyboard_xkb_layout),
            ("variant", &i.keyboard_xkb_variant),
            ("model", &i.keyboard_xkb_model),
            ("rules", &i.keyboard_xkb_rules),
            ("options", &i.keyboard_xkb_options),
        ];
        for (name, val) in xkb_fields {
            if !val.is_empty() {
                xkb.push_str(&format!(
                    "            {} \"{}\"\n",
                    name,
                    val.replace('"', "\\\""),
                ));
            }
        }
        if !xkb.is_empty() {
            kb.push_str("        xkb {\n");
            kb.push_str(&xkb);
            kb.push_str("        }\n");
        }
        if i.keyboard_repeat_delay > 0 {
            kb.push_str(&format!(
                "        repeat-delay {}\n",
                i.keyboard_repeat_delay
            ));
        }
        if i.keyboard_repeat_rate > 0 {
            kb.push_str(&format!("        repeat-rate {}\n", i.keyboard_repeat_rate));
        }
        if i.keyboard_track_layout == TrackLayout::Window {
            kb.push_str("        track-layout \"window\"\n");
        }
        if i.numlock {
            kb.push_str("        numlock\n");
        }
        if !kb.is_empty() {
            body.push_str("    keyboard {\n");
            body.push_str(&kb);
            body.push_str("    }\n");
        }
    }

    // touchpad sub-block
    {
        let mut tp = String::new();
        if i.touchpad_off {
            tp.push_str("        off\n");
        }
        if i.touchpad_tap {
            tp.push_str("        tap\n");
        }
        if i.touchpad_dwt {
            tp.push_str("        dwt\n");
        }
        if i.touchpad_dwtp {
            tp.push_str("        dwtp\n");
        }
        match &i.touchpad_drag {
            DragSetting::Enabled => tp.push_str("        drag true\n"),
            DragSetting::Disabled => tp.push_str("        drag false\n"),
            DragSetting::Default => {}
        }
        if i.touchpad_drag_lock {
            tp.push_str("        drag-lock\n");
        }
        if i.touchpad_natural_scroll {
            tp.push_str("        natural-scroll\n");
        }
        if i.touchpad_accel_speed != 0.0 {
            tp.push_str(&format!(
                "        accel-speed {:.1}\n",
                i.touchpad_accel_speed
            ));
        }
        if let Some(s) = i.touchpad_accel_profile.as_kdl_str() {
            tp.push_str(&format!("        accel-profile \"{s}\"\n"));
        }
        if let Some(s) = i.touchpad_scroll_method.as_kdl_str() {
            tp.push_str(&format!("        scroll-method \"{s}\"\n"));
        }
        if let Some(s) = i.touchpad_tap_button_map.as_kdl_str() {
            tp.push_str(&format!("        tap-button-map \"{s}\"\n"));
        }
        if let Some(s) = i.touchpad_click_method.as_kdl_str() {
            tp.push_str(&format!("        click-method \"{s}\"\n"));
        }
        if i.touchpad_disabled_on_external_mouse {
            tp.push_str("        disabled-on-external-mouse\n");
        }
        if i.touchpad_left_handed {
            tp.push_str("        left-handed\n");
        }
        if i.touchpad_middle_emulation {
            tp.push_str("        middle-emulation\n");
        }
        if !tp.is_empty() {
            body.push_str("    touchpad {\n");
            body.push_str(&tp);
            body.push_str("    }\n");
        }
    }

    // mouse sub-block
    {
        let mut ms = String::new();
        if i.mouse_off {
            ms.push_str("        off\n");
        }
        if i.mouse_natural_scroll {
            ms.push_str("        natural-scroll\n");
        }
        if i.mouse_accel_speed != 0.0 {
            ms.push_str(&format!("        accel-speed {:.1}\n", i.mouse_accel_speed));
        }
        if let Some(s) = i.mouse_accel_profile.as_kdl_str() {
            ms.push_str(&format!("        accel-profile \"{s}\"\n"));
        }
        if let Some(s) = i.mouse_scroll_method.as_kdl_str() {
            ms.push_str(&format!("        scroll-method \"{s}\"\n"));
        }
        if i.mouse_left_handed {
            ms.push_str("        left-handed\n");
        }
        if i.mouse_middle_emulation {
            ms.push_str("        middle-emulation\n");
        }
        if !ms.is_empty() {
            body.push_str("    mouse {\n");
            body.push_str(&ms);
            body.push_str("    }\n");
        }
    }

    // general flags
    if i.focus_follows_mouse {
        body.push_str("    focus-follows-mouse\n");
    }
    if i.workspace_auto_back_and_forth {
        body.push_str("    workspace-auto-back-and-forth\n");
    }
    if i.warp_mouse_to_focus {
        body.push_str("    warp-mouse-to-focus\n");
    }
    if i.disable_power_key_handling {
        body.push_str("    disable-power-key-handling\n");
    }
    if !i.mod_key.is_empty() {
        body.push_str(&format!(
            "    mod-key \"{}\"\n",
            i.mod_key.replace('"', "\\\"")
        ));
    }
    if !i.mod_key_nested.is_empty() {
        body.push_str(&format!(
            "    mod-key-nested \"{}\"\n",
            i.mod_key_nested.replace('"', "\\\""),
        ));
    }

    if !body.is_empty() {
        out.push_str("input {\n");
        out.push_str(&body);
        out.push_str("}\n\n");
    }
}

fn write_layout(out: &mut String, cfg: &SettingsConfig) {
    let l = &cfg.layout;
    out.push_str("layout {\n");
    out.push_str(&format!("    gaps {}\n", l.gaps.round() as i64));
    out.push_str(&format!(
        "    center-focused-column \"{}\"\n",
        l.center_focused_column.as_kdl_str(),
    ));
    if l.always_center_single_column {
        out.push_str("    always-center-single-column\n");
    }
    out.push_str("    focus-ring {\n");
    out.push_str(&format!(
        "        active-color \"{}\"\n",
        sanitise_color(&l.focus_ring_active_color),
    ));
    out.push_str(&format!(
        "        inactive-color \"{}\"\n",
        sanitise_color(&l.focus_ring_inactive_color),
    ));
    out.push_str("    }\n");
    if l.border_on {
        out.push_str("    border {\n        on\n    }\n");
    }
    if l.shadow_on {
        out.push_str("    shadow {\n        on\n    }\n");
    }
    out.push_str("}\n\n");
}

fn write_misc(out: &mut String, cfg: &SettingsConfig) {
    let m = &cfg.misc;
    if m.prefer_no_csd {
        out.push_str("prefer-no-csd\n\n");
    }

    // screenshot-path
    let shot_path = m.screenshot_path.trim();
    if shot_path.eq_ignore_ascii_case("null") {
        out.push_str("screenshot-path null\n\n");
    } else if !shot_path.is_empty() {
        out.push_str(&format!(
            "screenshot-path \"{}\"\n\n",
            sanitise_path(shot_path)
        ));
    }

    // cursor {}
    {
        let mut body = String::new();
        if !m.cursor_theme.is_empty() {
            body.push_str(&format!(
                "    xcursor-theme \"{}\"\n",
                sanitise_path(&m.cursor_theme)
            ));
        }
        if m.cursor_size > 0 {
            body.push_str(&format!("    xcursor-size {}\n", m.cursor_size.min(512)));
        }
        if m.cursor_hide_when_typing {
            body.push_str("    hide-when-typing\n");
        }
        if m.cursor_hide_after_inactive_ms > 0 {
            body.push_str(&format!(
                "    hide-after-inactive-ms {}\n",
                m.cursor_hide_after_inactive_ms,
            ));
        }
        if !body.is_empty() {
            out.push_str("cursor {\n");
            out.push_str(&body);
            out.push_str("}\n\n");
        }
    }

    // overview {}
    {
        let mut body = String::new();
        if m.overview_zoom > 0.0 {
            body.push_str(&format!(
                "    zoom {:.2}\n",
                m.overview_zoom.clamp(0.01, 0.75)
            ));
        }
        if is_valid_color(&m.overview_backdrop_color) {
            body.push_str(&format!(
                "    backdrop-color \"{}\"\n",
                sanitise_color(&m.overview_backdrop_color),
            ));
        }
        if !body.is_empty() {
            out.push_str("overview {\n");
            out.push_str(&body);
            out.push_str("}\n\n");
        }
    }

    if m.clipboard_disable_primary {
        out.push_str("clipboard {\n    disable-primary\n}\n\n");
    }

    // hotkey-overlay {}
    {
        let mut body = String::new();
        if m.skip_hotkey_overlay {
            body.push_str("    skip-at-startup\n");
        }
        if m.hotkey_overlay_hide_not_bound {
            body.push_str("    hide-not-bound\n");
        }
        if !body.is_empty() {
            out.push_str("hotkey-overlay {\n");
            out.push_str(&body);
            out.push_str("}\n\n");
        }
    }

    if m.config_notification_disable_failed {
        out.push_str("config-notification {\n    disable-failed\n}\n\n");
    }

    // xwayland-satellite {}
    {
        let mut body = String::new();
        if m.xwayland_off {
            body.push_str("    off\n");
        } else if !m.xwayland_path.is_empty() {
            body.push_str(&format!(
                "    path \"{}\"\n",
                sanitise_path(&m.xwayland_path)
            ));
        }
        if !body.is_empty() {
            out.push_str("xwayland-satellite {\n");
            out.push_str(&body);
            out.push_str("}\n\n");
        }
    }
}

fn write_animations(out: &mut String, cfg: &SettingsConfig) {
    let a = &cfg.anim;
    if a.global_off {
        out.push_str("animations {\n    off\n}\n\n");
        return;
    }
    let mut body = String::new();
    if a.slowdown != 0.0 {
        body.push_str(&format!("    slowdown {:.1}\n", a.slowdown));
    }
    let anim_flags: &[(&str, bool)] = &[
        ("workspace-switch", a.workspace_switch_off),
        ("window-open", a.window_open_off),
        ("window-close", a.window_close_off),
        ("horizontal-view-movement", a.horizontal_view_movement_off),
        ("window-movement", a.window_movement_off),
        ("window-resize", a.window_resize_off),
        (
            "config-notification-open-close",
            a.config_notification_open_close_off,
        ),
        (
            "exit-confirmation-open-close",
            a.exit_confirmation_open_close_off,
        ),
        ("screenshot-ui-open", a.screenshot_ui_open_off),
        ("overview-open-close", a.overview_open_close_off),
        ("recent-windows-close", a.recent_windows_close_off),
    ];
    for (name, is_off) in anim_flags {
        if *is_off {
            body.push_str(&format!("    {name} {{\n        off\n    }}\n"));
        }
    }
    if !body.is_empty() {
        out.push_str("animations {\n");
        out.push_str(&body);
        out.push_str("}\n\n");
    }
}

fn write_workspaces(out: &mut String, cfg: &SettingsConfig) {
    for ws in &cfg.workspaces {
        let name = ws.name.replace('"', "\\\"");
        if ws.open_on_output.is_empty() {
            out.push_str(&format!("workspace \"{name}\"\n"));
        } else {
            let on = ws.open_on_output.replace('"', "\\\"");
            out.push_str(&format!(
                "workspace \"{name}\" {{\n    open-on-output \"{on}\"\n}}\n",
            ));
        }
    }
    if !cfg.workspaces.is_empty() {
        out.push('\n');
    }
}

fn write_gestures(out: &mut String, cfg: &SettingsConfig) {
    let g = &cfg.gestures;
    let mut body = String::new();

    if g.dnd_view_trigger_width != 0 || g.dnd_view_delay_ms != 0 || g.dnd_view_max_speed != 0 {
        body.push_str("    dnd-edge-view-scroll {\n");
        if g.dnd_view_trigger_width != 0 {
            body.push_str(&format!(
                "        trigger-width {}\n",
                g.dnd_view_trigger_width
            ));
        }
        if g.dnd_view_delay_ms != 0 {
            body.push_str(&format!("        delay-ms {}\n", g.dnd_view_delay_ms));
        }
        if g.dnd_view_max_speed != 0 {
            body.push_str(&format!("        max-speed {}\n", g.dnd_view_max_speed));
        }
        body.push_str("    }\n");
    }

    if g.dnd_ws_trigger_height != 0 || g.dnd_ws_delay_ms != 0 || g.dnd_ws_max_speed != 0 {
        body.push_str("    dnd-edge-workspace-switch {\n");
        if g.dnd_ws_trigger_height != 0 {
            body.push_str(&format!(
                "        trigger-height {}\n",
                g.dnd_ws_trigger_height
            ));
        }
        if g.dnd_ws_delay_ms != 0 {
            body.push_str(&format!("        delay-ms {}\n", g.dnd_ws_delay_ms));
        }
        if g.dnd_ws_max_speed != 0 {
            body.push_str(&format!("        max-speed {}\n", g.dnd_ws_max_speed));
        }
        body.push_str("    }\n");
    }

    if g.hot_corners_off {
        body.push_str("    hot-corners {\n        off\n    }\n");
    } else if g.hot_corners_top_left
        || g.hot_corners_top_right
        || g.hot_corners_bottom_left
        || g.hot_corners_bottom_right
    {
        let mut hc = String::new();
        if g.hot_corners_top_left {
            hc.push_str("        top-left\n");
        }
        if g.hot_corners_top_right {
            hc.push_str("        top-right\n");
        }
        if g.hot_corners_bottom_left {
            hc.push_str("        bottom-left\n");
        }
        if g.hot_corners_bottom_right {
            hc.push_str("        bottom-right\n");
        }
        body.push_str("    hot-corners {\n");
        body.push_str(&hc);
        body.push_str("    }\n");
    }

    if !body.is_empty() {
        out.push_str("gestures {\n");
        out.push_str(&body);
        out.push_str("}\n\n");
    }
}

fn write_recent_windows(out: &mut String, cfg: &SettingsConfig) {
    let r = &cfg.recent_windows;
    if r.off {
        out.push_str("recent-windows {\n    off\n}\n\n");
        return;
    }
    let mut body = String::new();
    if r.debounce_ms != 0 {
        body.push_str(&format!("    debounce-ms {}\n", r.debounce_ms));
    }
    if r.open_delay_ms != 0 {
        body.push_str(&format!("    open-delay-ms {}\n", r.open_delay_ms));
    }
    // highlight sub-block
    {
        let mut hl = String::new();
        if is_valid_color(&r.highlight_active_color) {
            hl.push_str(&format!(
                "        active-color \"{}\"\n",
                sanitise_color(&r.highlight_active_color),
            ));
        }
        if is_valid_color(&r.highlight_urgent_color) {
            hl.push_str(&format!(
                "        urgent-color \"{}\"\n",
                sanitise_color(&r.highlight_urgent_color),
            ));
        }
        if r.highlight_padding != 0 {
            hl.push_str(&format!("        padding {}\n", r.highlight_padding));
        }
        if r.highlight_corner_radius != 0 {
            hl.push_str(&format!(
                "        corner-radius {}\n",
                r.highlight_corner_radius
            ));
        }
        if !hl.is_empty() {
            body.push_str("    highlight {\n");
            body.push_str(&hl);
            body.push_str("    }\n");
        }
    }
    // previews sub-block
    {
        let mut pv = String::new();
        if r.previews_max_height != 0 {
            pv.push_str(&format!("        max-height {}\n", r.previews_max_height));
        }
        if r.previews_max_scale != 0.0 {
            pv.push_str(&format!("        max-scale {:.2}\n", r.previews_max_scale));
        }
        if !pv.is_empty() {
            body.push_str("    previews {\n");
            body.push_str(&pv);
            body.push_str("    }\n");
        }
    }
    if !body.is_empty() {
        out.push_str("recent-windows {\n");
        out.push_str(&body);
        out.push_str("}\n\n");
    }
}

fn write_debug(out: &mut String, cfg: &SettingsConfig) {
    let d = &cfg.debug;
    let mut body = String::new();
    let flags: &[(&str, bool)] = &[
        ("enable-overlay-planes", d.enable_overlay_planes),
        ("disable-cursor-plane", d.disable_cursor_plane),
        ("disable-direct-scanout", d.disable_direct_scanout),
        (
            "restrict-primary-scanout-to-matching-format",
            d.restrict_primary_scanout_to_matching_format,
        ),
        (
            "force-disable-connectors-on-resume",
            d.force_disable_connectors_on_resume,
        ),
        (
            "force-pipewire-invalid-modifier",
            d.force_pipewire_invalid_modifier,
        ),
        (
            "dbus-interfaces-in-non-session-instances",
            d.dbus_interfaces_in_non_session_instances,
        ),
        (
            "wait-for-frame-completion-before-queueing",
            d.wait_for_frame_completion_before_queueing,
        ),
        (
            "emulate-zero-presentation-time",
            d.emulate_zero_presentation_time,
        ),
        ("disable-resize-throttling", d.disable_resize_throttling),
        ("disable-transactions", d.disable_transactions),
        (
            "keep-laptop-panel-on-when-lid-is-closed",
            d.keep_laptop_panel_on_when_lid_is_closed,
        ),
        ("disable-monitor-names", d.disable_monitor_names),
        (
            "strict-new-window-focus-policy",
            d.strict_new_window_focus_policy,
        ),
        (
            "honor-xdg-activation-with-invalid-serial",
            d.honor_xdg_activation_with_invalid_serial,
        ),
        (
            "skip-cursor-only-updates-during-vrr",
            d.skip_cursor_only_updates_during_vrr,
        ),
        (
            "deactivate-unfocused-windows",
            d.deactivate_unfocused_windows,
        ),
    ];
    for (name, enabled) in flags {
        if *enabled {
            body.push_str(&format!("    {name}\n"));
        }
    }
    if !d.render_drm_device.is_empty() {
        body.push_str(&format!(
            "    render-drm-device \"{}\"\n",
            sanitise_path(&d.render_drm_device),
        ));
    }
    if !body.is_empty() {
        out.push_str("debug {\n");
        out.push_str(&body);
        out.push_str("}\n\n");
    }
}

fn write_outputs(out: &mut String, cfg: &SettingsConfig) {
    for o in &cfg.outputs {
        if o.name.is_empty() {
            continue;
        }
        let quoted = format!("\"{}\"", o.name.replace('"', "\\\""));
        let mut body = String::new();
        if o.off {
            body.push_str("    off\n");
        } else {
            if o.mode_width != 0 && o.mode_height != 0 {
                if o.mode_refresh_mhz != 0 {
                    let hz = o.mode_refresh_mhz as f64 / 1000.0;
                    body.push_str(&format!(
                        "    mode \"{}x{}@{:.3}\"\n",
                        o.mode_width, o.mode_height, hz,
                    ));
                } else {
                    body.push_str(&format!(
                        "    mode \"{}x{}\"\n",
                        o.mode_width, o.mode_height,
                    ));
                }
            }
            if o.scale != 0.0 {
                body.push_str(&format!("    scale {:.2}\n", o.scale));
            }
            if o.transform != OutputTransform::Normal {
                body.push_str(&format!("    transform \"{}\"\n", o.transform.as_kdl_str()));
            }
            if o.position_set {
                body.push_str(&format!(
                    "    position x={} y={}\n",
                    o.position_x, o.position_y,
                ));
            }
            match o.vrr {
                VrrMode::On => body.push_str("    variable-refresh-rate\n"),
                VrrMode::OnDemand => {
                    body.push_str("    variable-refresh-rate on-demand=true\n");
                }
                VrrMode::Default | VrrMode::Off => {}
            }
        }
        out.push_str(&format!("output {quoted} {{\n"));
        out.push_str(&body);
        out.push_str("}\n\n");
    }
}

fn write_window_rules(out: &mut String, cfg: &SettingsConfig) {
    for r in &cfg.window_rules {
        let mut body = String::new();
        if !r.match_app_id.is_empty() {
            body.push_str(&format!(
                "    match app-id=\"{}\"\n",
                sanitise_regex(&r.match_app_id),
            ));
        }
        if !r.match_title.is_empty() {
            body.push_str(&format!(
                "    match title=\"{}\"\n",
                sanitise_regex(&r.match_title),
            ));
        }
        r.match_at_startup.write_kdl_match(&mut body, "at-startup");
        r.open_maximized.write_kdl_bool(&mut body, "open-maximized");
        r.open_fullscreen
            .write_kdl_bool(&mut body, "open-fullscreen");
        r.open_floating.write_kdl_bool(&mut body, "open-floating");
        r.open_focused.write_kdl_bool(&mut body, "open-focused");
        if !r.open_on_output.is_empty() {
            body.push_str(&format!(
                "    open-on-output \"{}\"\n",
                sanitise_path(&r.open_on_output),
            ));
        }
        if !r.open_on_workspace.is_empty() {
            body.push_str(&format!(
                "    open-on-workspace \"{}\"\n",
                sanitise_path(&r.open_on_workspace),
            ));
        }
        if r.opacity != 0.0 {
            body.push_str(&format!("    opacity {:.2}\n", r.opacity));
        }
        if r.block_out_from != BlockOutFrom::None {
            body.push_str(&format!(
                "    block-out-from \"{}\"\n",
                r.block_out_from.as_kdl_str(),
            ));
        }
        r.draw_border_with_background
            .write_kdl_bool(&mut body, "draw-border-with-background");
        if r.geometry_corner_radius != 0.0 {
            body.push_str(&format!(
                "    geometry-corner-radius {:.1}\n",
                r.geometry_corner_radius,
            ));
        }
        r.clip_to_geometry
            .write_kdl_bool(&mut body, "clip-to-geometry");
        r.variable_refresh_rate
            .write_kdl_bool(&mut body, "variable-refresh-rate");
        if r.min_width != 0 {
            body.push_str(&format!("    min-width {}\n", r.min_width));
        }
        if r.max_width != 0 {
            body.push_str(&format!("    max-width {}\n", r.max_width));
        }
        if r.min_height != 0 {
            body.push_str(&format!("    min-height {}\n", r.min_height));
        }
        if r.max_height != 0 {
            body.push_str(&format!("    max-height {}\n", r.max_height));
        }
        if r.scroll_factor != 0.0 {
            body.push_str(&format!("    scroll-factor {:.2}\n", r.scroll_factor));
        }
        if !body.is_empty() {
            out.push_str("window-rule {\n");
            out.push_str(&body);
            out.push_str("}\n\n");
        }
    }
}

fn write_layer_rules(out: &mut String, cfg: &SettingsConfig) {
    for r in &cfg.layer_rules {
        let mut body = String::new();
        if !r.match_namespace.is_empty() {
            body.push_str(&format!(
                "    match namespace=\"{}\"\n",
                sanitise_regex(&r.match_namespace),
            ));
        }
        r.match_at_startup.write_kdl_match(&mut body, "at-startup");
        if r.opacity != 0.0 {
            body.push_str(&format!("    opacity {:.2}\n", r.opacity));
        }
        if r.block_out_from != BlockOutFrom::None {
            body.push_str(&format!(
                "    block-out-from \"{}\"\n",
                r.block_out_from.as_kdl_str(),
            ));
        }
        match &r.shadow {
            TriState::On => body.push_str("    shadow {\n        on\n    }\n"),
            TriState::Off => body.push_str("    shadow {\n        off\n    }\n"),
            TriState::Default => {}
        }
        if r.geometry_corner_radius != 0.0 {
            body.push_str(&format!(
                "    geometry-corner-radius {:.1}\n",
                r.geometry_corner_radius,
            ));
        }
        r.place_within_backdrop
            .write_kdl_bool(&mut body, "place-within-backdrop");
        if !body.is_empty() {
            out.push_str("layer-rule {\n");
            out.push_str(&body);
            out.push_str("}\n\n");
        }
    }
}

fn write_switch_events(out: &mut String, cfg: &SettingsConfig) {
    let se = &cfg.switch_events;
    let events: &[(&str, &Vec<String>)] = &[
        ("lid-close", &se.lid_close),
        ("lid-open", &se.lid_open),
        ("tablet-mode-on", &se.tablet_mode_on),
        ("tablet-mode-off", &se.tablet_mode_off),
    ];
    let mut body = String::new();
    for (name, argv) in events {
        if argv.is_empty() {
            continue;
        }
        let args = argv
            .iter()
            .map(|a| format!("\"{}\"", sanitise_path(a)))
            .collect::<Vec<_>>()
            .join(" ");
        body.push_str(&format!("    {name} {{ spawn {args}; }}\n"));
    }
    if !body.is_empty() {
        out.push_str("switch-events {\n");
        out.push_str(&body);
        out.push_str("}\n\n");
    }
}

fn write_binds(out: &mut String, cfg: &SettingsConfig) {
    let mut body = String::new();
    for b in &cfg.binds {
        if b.key.is_empty() || b.action.is_empty() {
            continue;
        }
        let key = sanitise_keybind_key(&b.key);
        if key.is_empty() {
            continue;
        }

        let mut props = String::new();
        if !b.repeat {
            props.push_str(" repeat=false");
        }
        if b.cooldown_ms > 0 {
            props.push_str(&format!(" cooldown-ms={}", b.cooldown_ms));
        }
        if b.allow_when_locked {
            props.push_str(" allow-when-locked=true");
        }

        let action_kdl = match b.action.as_str() {
            "spawn" => {
                let args: Vec<String> = b
                    .action_args
                    .split_whitespace()
                    .map(|a| format!("\"{}\"", sanitise_path(a)))
                    .collect();
                if args.is_empty() {
                    continue;
                }
                format!("spawn {}", args.join(" "))
            }
            "spawn-sh" => {
                if b.action_args.is_empty() {
                    continue;
                }
                format!("spawn-sh \"{}\"", sanitise_path(&b.action_args))
            }
            name if !b.action_args.is_empty() => {
                format!("{} {}", name, sanitise_path(&b.action_args))
            }
            name => name.to_string(),
        };
        body.push_str(&format!("    {key}{props} {{ {action_kdl}; }}\n"));
    }
    if !body.is_empty() {
        out.push_str("binds {\n");
        out.push_str(&body);
        out.push_str("}\n\n");
    }
}
