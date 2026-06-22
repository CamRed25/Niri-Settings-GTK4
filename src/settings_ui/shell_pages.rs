//! Theme, shell behaviour and optional-software pages.

use std::cell::RefCell;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, CssProvider, Entry, Label, Orientation, Separator};

use super::helpers::*;
use crate::settings_backend::shell::{valid_color, ShellConfig};

pub fn build_theme_page(shell: Rc<RefCell<ShellConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header(
        "theme",
        "Tokyo Night tokens with a live shell preview",
    ));

    let tokens = card("theme tokens");
    let preview_provider = CssProvider::new();
    let preview = build_preview(&shell.borrow(), &preview_provider);

    type ColorAccess = (
        &'static str,
        fn(&ShellConfig) -> &str,
        fn(&mut ShellConfig, String),
    );
    let colors: &[ColorAccess] = &[
        ("accent", |c| &c.theme.accent, |c, v| c.theme.accent = v),
        ("surface", |c| &c.theme.surface, |c, v| c.theme.surface = v),
        ("text", |c| &c.theme.text, |c, v| c.theme.text = v),
        ("muted", |c| &c.theme.muted, |c, v| c.theme.muted = v),
        ("success", |c| &c.theme.success, |c, v| c.theme.success = v),
        ("warning", |c| &c.theme.warning, |c, v| c.theme.warning = v),
        ("danger", |c| &c.theme.danger, |c, v| c.theme.danger = v),
        ("purple", |c| &c.theme.purple, |c, v| c.theme.purple = v),
    ];
    for (index, (name, get, set)) in colors.iter().enumerate() {
        if index > 0 {
            tokens.append(&Separator::new(Orientation::Horizontal));
        }
        let row = row_box();
        let label = Label::new(Some(name));
        label.add_css_class("row-label");
        label.set_hexpand(true);
        label.set_xalign(0.0);
        let swatch = Label::new(None);
        swatch.add_css_class("token-swatch");
        let entry = Entry::new();
        entry.add_css_class("color-entry");
        entry.set_width_chars(10);
        entry.set_text(get(&shell.borrow()));
        let shell_for_entry = Rc::clone(&shell);
        let preview_for_entry = preview.clone();
        let provider_for_entry = preview_provider.clone();
        let setter = *set;
        entry.connect_changed(move |entry| {
            let value = entry.text().to_string();
            if valid_color(&value) {
                entry.remove_css_class("invalid");
                setter(&mut shell_for_entry.borrow_mut(), value);
                update_preview(
                    &preview_for_entry,
                    &provider_for_entry,
                    &shell_for_entry.borrow(),
                );
            } else {
                entry.add_css_class("invalid");
            }
        });
        row.append(&label);
        row.append(&swatch);
        row.append(&entry);
        tokens.append(&row);
    }
    page.append(&tokens);

    let appearance = card("appearance");
    let radius_row = row_box();
    let radius_label = label_col("corner radius", "panel radius in logical pixels");
    radius_label.set_hexpand(true);
    let radius = gtk4::Scale::with_range(Orientation::Horizontal, 0.0, 32.0, 1.0);
    radius.set_draw_value(true);
    radius.set_value(f64::from(shell.borrow().theme.radius));
    radius.set_width_request(180);
    let shell_for_radius = Rc::clone(&shell);
    let preview_for_radius = preview.clone();
    let provider_for_radius = preview_provider.clone();
    radius.connect_value_changed(move |scale| {
        shell_for_radius.borrow_mut().theme.radius = scale.value().round() as u32;
        update_preview(
            &preview_for_radius,
            &provider_for_radius,
            &shell_for_radius.borrow(),
        );
    });
    radius_row.append(&radius_label);
    radius_row.append(&radius);
    appearance.append(&radius_row);
    page.append(&appearance);

    let preview_title = Label::new(Some("LIVE PREVIEW"));
    preview_title.add_css_class("sidebar-section-label");
    preview_title.set_xalign(0.0);
    page.append(&preview_title);
    page.append(&preview);
    page
}

pub fn build_shell_behaviour_page(shell: Rc<RefCell<ShellConfig>>) -> GtkBox {
    let page = page_box();
    page.append(&page_header(
        "behaviour",
        "niri-shell component visibility and interaction",
    ));
    let card = card("shell behaviour");

    type BehaviourAccess = (
        &'static str,
        &'static str,
        fn(&ShellConfig) -> bool,
        fn(&mut ShellConfig, bool),
    );
    let rows: &[BehaviourAccess] = &[
        (
            "dock autohide",
            "hide the dock after the pointer leaves",
            |c| c.behaviour.dock_autohide,
            |c, v| c.behaviour.dock_autohide = v,
        ),
        (
            "show workspace indicator",
            "show workspaces in the top panel",
            |c| c.behaviour.show_workspace_indicator,
            |c, v| c.behaviour.show_workspace_indicator = v,
        ),
        (
            "show media controls",
            "show the active MPRIS player in the panel",
            |c| c.behaviour.show_media_controls,
            |c, v| c.behaviour.show_media_controls = v,
        ),
        (
            "show network speeds",
            "show upload and download rates",
            |c| c.behaviour.show_network_speeds,
            |c, v| c.behaviour.show_network_speeds = v,
        ),
        (
            "launcher recent files",
            "include recently opened documents in launcher results",
            |c| c.behaviour.launcher_recent_files,
            |c, v| c.behaviour.launcher_recent_files = v,
        ),
        (
            "notification sounds",
            "play the desktop message sound for new notifications",
            |c| c.behaviour.notification_sounds,
            |c, v| c.behaviour.notification_sounds = v,
        ),
    ];
    for (label, description, get, set) in rows {
        let initial = get(&shell.borrow());
        let shell_for_toggle = Rc::clone(&shell);
        let setter = *set;
        append_toggle(&card, label, description, initial, move |value| {
            setter(&mut shell_for_toggle.borrow_mut(), value);
        });
    }
    page.append(&card);
    page
}

pub fn build_software_page() -> GtkBox {
    let page = page_box();
    page.append(&page_header(
        "software",
        "optional integrations discovered from the current PATH",
    ));
    let tools = card("tools & software");
    for (name, description) in [
        ("nmcli", "NetworkManager Wi-Fi, Ethernet and VPN control"),
        ("bluetoothctl", "BlueZ Bluetooth adapter control"),
        ("wpctl", "PipeWire volume control"),
        ("pactl", "PulseAudio-compatible volume fallback"),
        ("brightnessctl", "laptop display brightness"),
        ("ddcutil", "external monitor brightness fallback"),
        ("gammastep", "night-light colour temperature"),
        ("hyprlock", "preferred lock screen"),
        ("swaylock", "lock-screen fallback"),
        ("cliphist", "launcher clipboard history"),
        ("grim", "screenshot capture"),
        ("slurp", "interactive screenshot regions"),
        ("canberra-gtk-play", "notification event sounds"),
    ] {
        let installed = program_exists(name);
        let row = row_box();
        let status = Label::new(Some(if installed { "✓" } else { "—" }));
        status.add_css_class(if installed {
            "tool-installed"
        } else {
            "tool-missing"
        });
        status.set_width_chars(2);
        let labels = label_col(name, description);
        labels.set_hexpand(true);
        let badge = Label::new(Some(if installed {
            "installed"
        } else {
            "not installed"
        }));
        badge.add_css_class(if installed {
            "installed-badge"
        } else {
            "missing-badge"
        });
        row.append(&status);
        row.append(&labels);
        row.append(&badge);
        tools.append(&row);
    }
    page.append(&tools);
    page
}

fn build_preview(config: &ShellConfig, provider: &CssProvider) -> GtkBox {
    let preview = GtkBox::new(Orientation::Horizontal, 10);
    preview.add_css_class("theme-preview");
    preview.set_valign(Align::Center);
    for _ in 0..3 {
        let dot = Label::new(Some("●"));
        dot.add_css_class("preview-dot");
        preview.append(&dot);
    }
    let title = Label::new(Some("Roads — Portishead"));
    title.set_hexpand(true);
    title.set_xalign(0.0);
    title.add_css_class("preview-text");
    let time = Label::new(Some("14:32"));
    time.add_css_class("preview-time");
    preview.append(&title);
    preview.append(&time);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
        );
    }
    update_preview(&preview, provider, config);
    preview
}

fn update_preview(preview: &GtkBox, provider: &CssProvider, config: &ShellConfig) {
    provider.load_from_string(&format!(
        ".theme-preview {{ background: {}; border-color: {}; border-radius: {}px; }}\n\
         .theme-preview .preview-dot {{ color: {}; }}\n\
         .theme-preview .preview-text, .theme-preview .preview-time {{ color: {}; }}",
        config.theme.surface,
        config.theme.muted,
        config.theme.radius.min(20),
        config.theme.accent,
        config.theme.text,
    ));
    preview.queue_draw();
}

fn program_exists(name: &str) -> bool {
    if name.contains('/') {
        return executable(Path::new(name));
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|directory| directory.join(name))
        .any(|path| executable(&path))
}

fn executable(path: &Path) -> bool {
    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_is_discovered_from_path() {
        assert!(program_exists("sh"));
    }

    #[test]
    fn nonsense_tool_is_missing() {
        assert!(!program_exists("niri-shell-definitely-not-a-program"));
    }
}
