//! Pure-Rust model for `~/.config/niri-shell/shell.kdl`.

use kdl::{KdlDocument, KdlValue};

use super::{config_base, SettingsError};

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeTokens {
    pub accent: String,
    pub surface: String,
    pub text: String,
    pub muted: String,
    pub success: String,
    pub warning: String,
    pub danger: String,
    pub purple: String,
    pub font_ui: String,
    pub font_mono: String,
    pub radius: u32,
    pub panel_opacity: f64,
}

impl Default for ThemeTokens {
    fn default() -> Self {
        Self {
            accent: "#7aa2f7".into(),
            surface: "#0d0e17".into(),
            text: "#c0caf5".into(),
            muted: "#565f89".into(),
            success: "#9ece6a".into(),
            warning: "#e0af68".into(),
            danger: "#f7768e".into(),
            purple: "#bb9af7".into(),
            font_ui: "Inter, Noto Sans, sans-serif".into(),
            font_mono: "JetBrains Mono, monospace".into(),
            radius: 14,
            panel_opacity: 0.90,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellBehaviour {
    pub dock_autohide: bool,
    pub show_workspace_indicator: bool,
    pub show_media_controls: bool,
    pub show_network_speeds: bool,
    pub launcher_recent_files: bool,
    pub notification_sounds: bool,
}

impl Default for ShellBehaviour {
    fn default() -> Self {
        Self {
            dock_autohide: true,
            show_workspace_indicator: true,
            show_media_controls: true,
            show_network_speeds: true,
            launcher_recent_files: false,
            notification_sounds: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuickSettingsPrefs {
    pub dnd: bool,
    pub night_light: bool,
    pub night_light_temperature: u32,
}

impl Default for QuickSettingsPrefs {
    fn default() -> Self {
        Self {
            dnd: false,
            night_light: false,
            night_light_temperature: 3200,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShellConfig {
    pub theme: ThemeTokens,
    pub behaviour: ShellBehaviour,
    pub quick_settings: QuickSettingsPrefs,
}

pub fn path() -> Result<std::path::PathBuf, SettingsError> {
    Ok(config_base()?.join("niri-shell/shell.kdl"))
}

pub fn load() -> Result<ShellConfig, SettingsError> {
    let path = path()?;
    if !path.exists() {
        return Ok(ShellConfig::default());
    }
    parse(&std::fs::read_to_string(path)?)
}

pub fn parse(text: &str) -> Result<ShellConfig, SettingsError> {
    let doc: KdlDocument = text
        .parse()
        .map_err(|e: kdl::KdlError| SettingsError::Kdl(e.to_string()))?;
    let mut cfg = ShellConfig::default();
    if let Some(theme) = doc.get("theme").and_then(|n| n.children()) {
        for (name, target) in [
            ("accent", &mut cfg.theme.accent),
            ("surface", &mut cfg.theme.surface),
            ("text", &mut cfg.theme.text),
            ("muted", &mut cfg.theme.muted),
            ("success", &mut cfg.theme.success),
            ("warning", &mut cfg.theme.warning),
            ("danger", &mut cfg.theme.danger),
            ("purple", &mut cfg.theme.purple),
        ] {
            if let Some(value) = string_arg(theme, name).filter(|v| valid_color(v)) {
                *target = value.to_owned();
            }
        }
        if let Some(value) = integer_arg(theme, "radius") {
            cfg.theme.radius = value.clamp(0, 32) as u32;
        }
        if let Some(value) = number_arg(theme, "panel-opacity") {
            cfg.theme.panel_opacity = value.clamp(0.4, 1.0);
        }
        if let Some(value) = string_arg(theme, "font-ui").filter(|value| valid_font(value)) {
            cfg.theme.font_ui = value.to_owned();
        }
        if let Some(value) = string_arg(theme, "font-mono").filter(|value| valid_font(value)) {
            cfg.theme.font_mono = value.to_owned();
        }
    }
    if let Some(behaviour) = doc.get("behaviour").and_then(|n| n.children()) {
        read_bool(behaviour, "dock-autohide", &mut cfg.behaviour.dock_autohide);
        read_bool(
            behaviour,
            "show-workspace-indicator",
            &mut cfg.behaviour.show_workspace_indicator,
        );
        read_bool(
            behaviour,
            "show-media-controls",
            &mut cfg.behaviour.show_media_controls,
        );
        read_bool(
            behaviour,
            "show-network-speeds",
            &mut cfg.behaviour.show_network_speeds,
        );
        read_bool(
            behaviour,
            "launcher-recent-files",
            &mut cfg.behaviour.launcher_recent_files,
        );
        read_bool(
            behaviour,
            "notification-sounds",
            &mut cfg.behaviour.notification_sounds,
        );
    }
    if let Some(qs) = doc.get("quick-settings").and_then(|n| n.children()) {
        read_bool(qs, "do-not-disturb", &mut cfg.quick_settings.dnd);
        read_bool(qs, "night-light", &mut cfg.quick_settings.night_light);
        if let Some(value) = integer_arg(qs, "night-light-temperature") {
            cfg.quick_settings.night_light_temperature = value.clamp(1000, 10_000) as u32;
        }
    }
    Ok(cfg)
}

pub fn save(cfg: &ShellConfig) -> Result<(), SettingsError> {
    let path = path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("shell.kdl");
    let tmp = path.with_file_name(format!(".{file_name}.tmp.{}", std::process::id()));
    std::fs::write(&tmp, render(cfg))?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

pub fn render(cfg: &ShellConfig) -> String {
    format!(
        "// niri-shell preferences. Safe to edit while the shell is running.\n\n\
theme {{\n    accent \"{}\"\n    surface \"{}\"\n    text \"{}\"\n    muted \"{}\"\n\
    success \"{}\"\n    warning \"{}\"\n    danger \"{}\"\n    purple \"{}\"\n\
    font-ui \"{}\"\n    font-mono \"{}\"\n    radius {}\n    panel-opacity {:.2}\n}}\n\n\
behaviour {{\n    dock-autohide {}\n    show-workspace-indicator {}\n    show-media-controls {}\n\
    show-network-speeds {}\n    launcher-recent-files {}\n    notification-sounds {}\n}}\n\n\
quick-settings {{\n    do-not-disturb {}\n    night-light {}\n    night-light-temperature {}\n}}\n",
        escape(&cfg.theme.accent),
        escape(&cfg.theme.surface),
        escape(&cfg.theme.text),
        escape(&cfg.theme.muted),
        escape(&cfg.theme.success),
        escape(&cfg.theme.warning),
        escape(&cfg.theme.danger),
        escape(&cfg.theme.purple),
        escape(&cfg.theme.font_ui),
        escape(&cfg.theme.font_mono),
        cfg.theme.radius,
        cfg.theme.panel_opacity,
        cfg.behaviour.dock_autohide,
        cfg.behaviour.show_workspace_indicator,
        cfg.behaviour.show_media_controls,
        cfg.behaviour.show_network_speeds,
        cfg.behaviour.launcher_recent_files,
        cfg.behaviour.notification_sounds,
        cfg.quick_settings.dnd,
        cfg.quick_settings.night_light,
        cfg.quick_settings.night_light_temperature,
    )
}

pub fn valid_color(value: &str) -> bool {
    value.starts_with('#')
        && matches!(value.len(), 4 | 5 | 7 | 9)
        && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn valid_font(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | ','))
}

fn string_arg<'a>(doc: &'a KdlDocument, name: &str) -> Option<&'a str> {
    doc.get_arg(name).and_then(KdlValue::as_string)
}

fn integer_arg(doc: &KdlDocument, name: &str) -> Option<i64> {
    doc.get_arg(name).and_then(KdlValue::as_i64)
}

fn number_arg(doc: &KdlDocument, name: &str) -> Option<f64> {
    doc.get_arg(name)
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)))
}

fn read_bool(doc: &KdlDocument, name: &str, target: &mut bool) {
    if let Some(value) = doc.get_arg(name).and_then(KdlValue::as_bool) {
        *target = value;
    }
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_config_round_trip() {
        let cfg = ShellConfig::default();
        assert_eq!(parse(&render(&cfg)).unwrap(), cfg);
    }

    #[test]
    fn invalid_color_is_rejected() {
        assert!(!valid_color("red; include bad"));
    }
}
