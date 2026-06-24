// settings_backend/mod.rs — Pure Rust config model for the niri settings GUI.
//
// settings.json is this app's authoritative store: it is loaded and written
// back in full. settings.kdl is the fragment niri consumes; it is generated on
// save and validated, but on load only the externally-editable nodes (binds,
// output, window-rule, layer-rule) are parsed back from it and allowed to
// override the JSON copy (so hand edits to those are picked up). Every other
// managed section round-trips through settings.json by design — see
// `overlay_primary_kdl` and `MANAGED_TOP_LEVEL`.

pub mod kdl;
pub mod keybinds;
pub mod shell;
pub mod tools;
pub mod types;

// Re-export everything the rest of the crate uses.
pub use kdl::generate_kdl;
pub use keybinds::{
    action_args_hint, action_needs_args, binding_conflicts, import_binds_from_niri_config,
    normalize_key_combo, NIRI_ACTIONS,
};
pub use types::*;

use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialisation error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid managed KDL: {0}")]
    Kdl(String),
    #[error("niri rejected the configuration:\n{0}")]
    Validation(String),
}

// ── Paths ─────────────────────────────────────────────────────────────────────

/// Returns the XDG config base directory, falling back to `$HOME/.config`.
///
/// Returns an error if neither `XDG_CONFIG_HOME` nor `HOME` is set.
pub fn config_base() -> Result<PathBuf, SettingsError> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Ok(PathBuf::from(xdg));
        }
    }
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() => Ok(PathBuf::from(home).join(".config")),
        _ => Err(SettingsError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "neither XDG_CONFIG_HOME nor HOME is set",
        ))),
    }
}

fn shell_config_dir() -> Result<PathBuf, SettingsError> {
    Ok(config_base()?.join("niri-shell"))
}

fn json_path() -> Result<PathBuf, SettingsError> {
    Ok(shell_config_dir()?.join("settings.json"))
}

pub fn kdl_path() -> Result<PathBuf, SettingsError> {
    Ok(shell_config_dir()?.join("settings.kdl"))
}

pub fn niri_config_path() -> Result<PathBuf, SettingsError> {
    if let Some(path) = std::env::var_os("NIRI_CONFIG").filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    Ok(config_base()?.join("niri").join("config.kdl"))
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Loads settings from disk. Returns defaults if the file does not yet exist.
pub fn load() -> Result<SettingsConfig, SettingsError> {
    let json = json_path()?;
    let mut cfg = if json.exists() {
        let data = std::fs::read_to_string(&json)?;
        serde_json::from_str(&data)?
    } else {
        SettingsConfig::default()
    };
    // Normalise gaps to an integer value.
    cfg.layout.gaps = cfg.layout.gaps.round();
    // Normalise overview zoom to 2 decimal places.
    if cfg.misc.overview_zoom > 0.0 {
        cfg.misc.overview_zoom = (cfg.misc.overview_zoom * 20.0).round() / 20.0;
    }
    // Normalise accel speeds to 1 decimal place.
    cfg.input.touchpad_accel_speed = (cfg.input.touchpad_accel_speed * 10.0).round() / 10.0;
    cfg.input.mouse_accel_speed = (cfg.input.mouse_accel_speed * 10.0).round() / 10.0;
    let managed = kdl_path()?;
    if managed.exists() {
        let text = std::fs::read_to_string(managed)?;
        let doc: ::kdl::KdlDocument = text
            .parse()
            .map_err(|e: ::kdl::KdlError| SettingsError::Kdl(e.to_string()))?;
        overlay_primary_kdl(&mut cfg, &doc);
    }
    // Assign stable runtime ids so the UI can address rows across rebuilds.
    cfg.ensure_row_ids();
    Ok(cfg)
}

/// Overlays the externally-editable nodes from a parsed settings.kdl onto a
/// config already loaded from JSON. Only `binds`, `output`, `window-rule`, and
/// `layer-rule` are parsed back (they may be hand-edited with provenance); all
/// other [`MANAGED_TOP_LEVEL`] sections are JSON-authoritative and intentionally
/// not read here. Keep the set parsed here in sync with their writers in
/// `kdl.rs` so no field is silently dropped on reload.
fn overlay_primary_kdl(cfg: &mut SettingsConfig, doc: &::kdl::KdlDocument) {
    if let Some(children) = doc.get("binds").and_then(|node| node.children()) {
        cfg.binds = children
            .nodes()
            .iter()
            .filter_map(|node| keybinds::parse_bind_line(&node.to_string()))
            .collect();
    }

    let mut outputs = Vec::new();
    let mut rules = Vec::new();
    let mut layer_rules = Vec::new();
    for node in doc.nodes() {
        match node.name().value() {
            "output" => {
                let Some(name) = node.get(0).and_then(|e| e.value().as_string()) else {
                    continue;
                };
                let mut output = OutputConfig::new(name);
                if let Some(children) = node.children() {
                    output.off = children.get("off").is_some();
                    if let Some(mode) = child_string(children, "mode") {
                        parse_mode(mode, &mut output);
                    }
                    output.scale = child_number(children, "scale").unwrap_or(0.0);
                    if let Some(transform) = child_string(children, "transform") {
                        output.transform = match transform {
                            "90" => OutputTransform::Rotate90,
                            "180" => OutputTransform::Rotate180,
                            "270" => OutputTransform::Rotate270,
                            "flipped" => OutputTransform::Flipped,
                            "flipped-90" => OutputTransform::Flipped90,
                            "flipped-180" => OutputTransform::Flipped180,
                            "flipped-270" => OutputTransform::Flipped270,
                            _ => OutputTransform::Normal,
                        };
                    }
                    if let Some(position) = children.get("position") {
                        output.position_set = true;
                        output.position_x = property_i64(position, "x").unwrap_or(0) as i32;
                        output.position_y = property_i64(position, "y").unwrap_or(0) as i32;
                    }
                    if let Some(vrr) = children.get("variable-refresh-rate") {
                        output.vrr = if property_bool(vrr, "on-demand") == Some(true) {
                            VrrMode::OnDemand
                        } else {
                            VrrMode::On
                        };
                    }
                }
                outputs.push(output);
            }
            "window-rule" => {
                let Some(children) = node.children() else {
                    continue;
                };
                let mut rule = WindowRule::default();
                // The writer emits app-id, title, and at-startup as separate
                // `match` lines, so merge properties across every match node.
                for matcher in match_nodes(children) {
                    if let Some(app_id) = property_string(matcher, "app-id") {
                        rule.match_app_id = app_id;
                    }
                    if let Some(title) = property_string(matcher, "title") {
                        rule.match_title = title;
                    }
                    if matcher.get("at-startup").is_some() {
                        rule.match_at_startup = tri_property(matcher, "at-startup");
                    }
                }
                rule.open_maximized = tri_child(children, "open-maximized");
                rule.open_fullscreen = tri_child(children, "open-fullscreen");
                rule.open_floating = tri_child(children, "open-floating");
                rule.open_focused = tri_child(children, "open-focused");
                rule.open_on_output = child_string(children, "open-on-output")
                    .unwrap_or_default()
                    .to_owned();
                rule.open_on_workspace = child_string(children, "open-on-workspace")
                    .unwrap_or_default()
                    .to_owned();
                rule.opacity = child_number(children, "opacity").unwrap_or(0.0);
                rule.block_out_from =
                    parse_block_out_from(child_string(children, "block-out-from"));
                rule.draw_border_with_background =
                    tri_child(children, "draw-border-with-background");
                rule.geometry_corner_radius =
                    child_number(children, "geometry-corner-radius").unwrap_or(0.0);
                rule.clip_to_geometry = tri_child(children, "clip-to-geometry");
                rule.variable_refresh_rate = tri_child(children, "variable-refresh-rate");
                rule.min_width = child_i64(children, "min-width").unwrap_or(0).max(0) as u32;
                rule.max_width = child_i64(children, "max-width").unwrap_or(0).max(0) as u32;
                rule.min_height = child_i64(children, "min-height").unwrap_or(0).max(0) as u32;
                rule.max_height = child_i64(children, "max-height").unwrap_or(0).max(0) as u32;
                rule.scroll_factor = child_number(children, "scroll-factor").unwrap_or(0.0);
                rules.push(rule);
            }
            "layer-rule" => {
                let Some(children) = node.children() else {
                    continue;
                };
                let mut rule = LayerRule::default();
                for matcher in match_nodes(children) {
                    if let Some(namespace) = property_string(matcher, "namespace") {
                        rule.match_namespace = namespace;
                    }
                    if matcher.get("at-startup").is_some() {
                        rule.match_at_startup = tri_property(matcher, "at-startup");
                    }
                }
                rule.opacity = child_number(children, "opacity").unwrap_or(0.0);
                rule.block_out_from =
                    parse_block_out_from(child_string(children, "block-out-from"));
                rule.shadow = parse_shadow(children);
                rule.geometry_corner_radius =
                    child_number(children, "geometry-corner-radius").unwrap_or(0.0);
                rule.place_within_backdrop = tri_child(children, "place-within-backdrop");
                layer_rules.push(rule);
            }
            _ => {}
        }
    }
    cfg.outputs = outputs;
    cfg.window_rules = rules;
    cfg.layer_rules = layer_rules;
}

fn child_value<'a>(doc: &'a ::kdl::KdlDocument, name: &str) -> Option<&'a ::kdl::KdlValue> {
    doc.get(name)?.get(0).map(|entry| entry.value())
}

fn child_string<'a>(doc: &'a ::kdl::KdlDocument, name: &str) -> Option<&'a str> {
    child_value(doc, name).and_then(::kdl::KdlValue::as_string)
}

fn child_i64(doc: &::kdl::KdlDocument, name: &str) -> Option<i64> {
    child_value(doc, name).and_then(::kdl::KdlValue::as_i64)
}

fn child_number(doc: &::kdl::KdlDocument, name: &str) -> Option<f64> {
    child_value(doc, name).and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_i64().map(|number| number as f64))
    })
}

fn property_string(node: &::kdl::KdlNode, name: &str) -> Option<String> {
    node.get(name)?.value().as_string().map(str::to_owned)
}

fn property_i64(node: &::kdl::KdlNode, name: &str) -> Option<i64> {
    node.get(name)?.value().as_i64()
}

fn property_bool(node: &::kdl::KdlNode, name: &str) -> Option<bool> {
    node.get(name)?.value().as_bool()
}

fn tri_property(node: &::kdl::KdlNode, name: &str) -> TriState {
    match property_bool(node, name) {
        Some(true) => TriState::On,
        Some(false) => TriState::Off,
        None => TriState::Default,
    }
}

fn tri_child(doc: &::kdl::KdlDocument, name: &str) -> TriState {
    match child_value(doc, name).and_then(::kdl::KdlValue::as_bool) {
        Some(true) => TriState::On,
        Some(false) => TriState::Off,
        None => TriState::Default,
    }
}

/// Every `match` node inside a window/layer-rule block. The writer splits
/// matchers across separate lines, so callers must merge them.
fn match_nodes(doc: &::kdl::KdlDocument) -> impl Iterator<Item = &::kdl::KdlNode> {
    doc.nodes()
        .iter()
        .filter(|node| node.name().value() == "match")
}

/// Inverse of [`BlockOutFrom::as_kdl_str`].
fn parse_block_out_from(value: Option<&str>) -> BlockOutFrom {
    match value {
        Some("screencast") => BlockOutFrom::Screencast,
        Some("screen-capture") => BlockOutFrom::ScreenCapture,
        _ => BlockOutFrom::None,
    }
}

/// Reads a layer-rule `shadow { on|off }` block back into a tri-state.
fn parse_shadow(doc: &::kdl::KdlDocument) -> TriState {
    let Some(inner) = doc.get("shadow").and_then(::kdl::KdlNode::children) else {
        return TriState::Default;
    };
    if inner.get("on").is_some() {
        TriState::On
    } else if inner.get("off").is_some() {
        TriState::Off
    } else {
        TriState::Default
    }
}

fn parse_mode(mode: &str, output: &mut OutputConfig) {
    let (resolution, refresh) = mode.split_once('@').unwrap_or((mode, ""));
    let Some((width, height)) = resolution.split_once('x') else {
        return;
    };
    output.mode_width = width.parse().unwrap_or(0);
    output.mode_height = height.parse().unwrap_or(0);
    output.mode_refresh_mhz = refresh
        .parse::<f64>()
        .map(|hz| (hz * 1000.0).round() as u32)
        .unwrap_or(0);
}

/// Outcome of a successful [`save`]: whether the niri compositor actually
/// validated the written config, or whether it was saved without validation
/// because `niri` was not on `PATH` (development/offline session).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveStatus {
    /// `niri validate` ran and accepted the configuration.
    Validated,
    /// The KDL parsed cleanly and was saved, but `niri` was unavailable so the
    /// compositor never validated it.
    SavedWithoutValidation,
}

/// Atomically persists and validates the app-owned KDL include.
///
/// settings.json is the authoritative store and is rewritten in full. The KDL
/// fragment is the file consumed by niri and is always validated before it is
/// replaced. Unknown top-level nodes in the managed fragment are preserved.
///
/// Returns whether niri actually validated the result (see [`SaveStatus`]).
pub fn save(cfg: &SettingsConfig) -> Result<SaveStatus, SettingsError> {
    let dir = shell_config_dir()?;
    std::fs::create_dir_all(&dir)?;

    let kdl_content = merge_with_existing_kdl(&generate_kdl(cfg))?;
    let final_kdl_path = kdl_path()?;
    let tmp_kdl_path = temporary_sibling(&final_kdl_path, "candidate");
    std::fs::write(&tmp_kdl_path, &kdl_content)?;

    let status = match validate_candidate(&tmp_kdl_path) {
        Ok(status) => status,
        Err(error) => {
            if let Err(cleanup_error) = std::fs::remove_file(&tmp_kdl_path) {
                log::warn!("could not remove rejected candidate: {cleanup_error}");
            }
            return Err(error);
        }
    };

    std::fs::rename(&tmp_kdl_path, &final_kdl_path)?;
    ensure_include()?;

    // Authoritative store: rewrite in full, atomically.
    let json = serde_json::to_string_pretty(cfg)?;
    let json_final = json_path()?;
    let json_tmp = temporary_sibling(&json_final, "tmp");
    std::fs::write(&json_tmp, json)?;
    std::fs::rename(json_tmp, json_final)?;

    Ok(status)
}

/// Top-level nodes this app owns and replaces wholesale when writing
/// settings.kdl; any other node in the file is preserved untouched. Of these,
/// only `output`, `window-rule`, `layer-rule`, and `binds` are parsed back on
/// load (see `overlay_primary_kdl`); the rest are JSON-authoritative.
const MANAGED_TOP_LEVEL: &[&str] = &[
    "input",
    "layout",
    "hotkey-overlay",
    "screenshot-path",
    "cursor",
    "clipboard",
    "overview",
    "xwayland-satellite",
    "config-notification",
    "animations",
    "workspace",
    "gestures",
    "recent-windows",
    "debug",
    "output",
    "window-rule",
    "layer-rule",
    "switch-events",
    "binds",
    "prefer-no-csd",
];

fn merge_with_existing_kdl(generated: &str) -> Result<String, SettingsError> {
    let path = kdl_path()?;
    if !path.exists() {
        return merge_kdl_text(generated, None);
    }

    let existing = std::fs::read_to_string(path)?;
    merge_kdl_text(generated, Some(&existing))
}

fn merge_kdl_text(generated: &str, existing: Option<&str>) -> Result<String, SettingsError> {
    let mut candidate: ::kdl::KdlDocument = generated
        .parse()
        .map_err(|e: ::kdl::KdlError| SettingsError::Kdl(e.to_string()))?;
    let Some(existing) = existing else {
        return Ok(candidate.to_string());
    };
    let existing: ::kdl::KdlDocument = existing
        .parse()
        .map_err(|e: ::kdl::KdlError| SettingsError::Kdl(e.to_string()))?;
    for node in existing.nodes() {
        if !MANAGED_TOP_LEVEL.contains(&node.name().value()) {
            candidate.nodes_mut().push(node.clone());
        }
    }
    Ok(candidate.to_string())
}

fn validate_candidate(candidate: &std::path::Path) -> Result<SaveStatus, SettingsError> {
    let config = niri_config_path()?;
    let config_dir = config.parent().ok_or_else(|| {
        SettingsError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "niri config has no parent directory",
        ))
    })?;
    std::fs::create_dir_all(config_dir)?;

    let current = if config.exists() {
        std::fs::read_to_string(&config)?
    } else {
        String::new()
    };
    let final_path = kdl_path()?;
    let mut validation_root = current
        .lines()
        .filter(|line| !line.contains(final_path.to_string_lossy().as_ref()))
        .collect::<Vec<_>>()
        .join("\n");
    validation_root.push_str(&format!(
        "\ninclude \"{}\"\n",
        candidate.to_string_lossy().replace('"', "\\\"")
    ));

    let validation_path = config_dir.join(format!(
        ".niri-settings-validation-{}.kdl",
        std::process::id()
    ));
    std::fs::write(&validation_path, validation_root)?;
    let output = Command::new("niri")
        .args(["validate", "--config"])
        .arg(&validation_path)
        .output();
    if let Err(error) = std::fs::remove_file(&validation_path) {
        log::warn!("could not remove validation file: {error}");
    }

    match output {
        Ok(output) if output.status.success() => Ok(SaveStatus::Validated),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            Err(SettingsError::Validation(if stderr.is_empty() {
                stdout
            } else {
                stderr
            }))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // The KDL parser has already accepted the fragment. Allow editing in
            // a development/offline session where the compositor is absent, but
            // report that the compositor never validated it.
            log::warn!("niri validate unavailable; saving parser-valid KDL only");
            Ok(SaveStatus::SavedWithoutValidation)
        }
        Err(error) => Err(SettingsError::Io(error)),
    }
}

/// Adds `include "~/.config/niri-shell/settings.kdl"` to the main niri config
/// if it is not already present.
fn ensure_include() -> Result<(), SettingsError> {
    let config_path = niri_config_path()?;
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = std::fs::read_to_string(&config_path).unwrap_or_default();
    let abs_kdl = kdl_path()?;
    let include_line = format!(r#"include "{}""#, abs_kdl.to_string_lossy());
    if content.contains(&include_line) {
        return Ok(());
    }
    // Remove any old `~`-based line to avoid duplicate includes.
    let cleaned = content
        .lines()
        .filter(|l| !l.contains(r#"include "~/.config/niri-shell/settings.kdl""#))
        .collect::<Vec<_>>()
        .join("\n");
    let new_content = format!(
        "{}\n\n// Added by niri-settings — overrides settings above\n{}\n",
        cleaned.trim_end(),
        include_line
    );
    if config_path.exists() {
        let backup = config_path.with_extension("kdl.niri-settings.bak");
        if !backup.exists() {
            std::fs::copy(&config_path, backup)?;
        }
    }
    let tmp = temporary_sibling(&config_path, "tmp");
    std::fs::write(&tmp, new_content)?;
    std::fs::rename(tmp, &config_path)?;
    log::info!("settings: added include line to niri config");
    Ok(())
}

fn temporary_sibling(path: &std::path::Path, purpose: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("niri-settings");
    path.with_file_name(format!(".{file_name}.{purpose}.{}", std::process::id()))
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::kdl::{sanitise_color, sanitise_keybind_key};
    use super::keybinds::parse_bind_line;
    use super::*;

    #[test]
    fn default_config_round_trips() {
        let cfg = SettingsConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: SettingsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.layout.gaps, cfg.layout.gaps);
    }

    #[test]
    fn parse_mode_handles_integer_and_fractional_refresh() {
        let mut out = OutputConfig::new("DP-1");
        parse_mode("2560x1440@144", &mut out);
        assert_eq!((out.mode_width, out.mode_height), (2560, 1440));
        // mHz = Hz * 1000, so 144 Hz → 144000 mHz.
        assert_eq!(out.mode_refresh_mhz, 144_000);

        // Fractional refresh must not be silently truncated to a wrong value.
        let mut out = OutputConfig::new("DP-2");
        parse_mode("3840x2160@59.94", &mut out);
        assert_eq!((out.mode_width, out.mode_height), (3840, 2160));
        assert_eq!(out.mode_refresh_mhz, 59_940);
    }

    #[test]
    fn parse_mode_rejects_malformed_strings_without_panicking() {
        let mut out = OutputConfig::new("DP-3");
        // No 'x' separator: leaves dimensions at zero, does not panic.
        parse_mode("garbage", &mut out);
        assert_eq!((out.mode_width, out.mode_height), (0, 0));

        // Non-numeric dimensions parse to zero rather than aborting.
        let mut out = OutputConfig::new("DP-4");
        parse_mode("axb@60", &mut out);
        assert_eq!((out.mode_width, out.mode_height), (0, 0));
    }

    #[test]
    fn key_combos_normalise_and_conflict() {
        assert_eq!(normalize_key_combo("Shift+Ctrl+T"), "Ctrl+Shift+t");
        let binds = vec![
            Keybind {
                key: "Ctrl+Shift+T".into(),
                ..Keybind::default()
            },
            Keybind {
                key: "shift+control+t".into(),
                ..Keybind::default()
            },
        ];
        let conflicts = binding_conflicts(&binds);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].1, vec![0, 1]);
    }

    #[test]
    fn managed_kdl_preserves_unknown_nodes() {
        let generated = generate_kdl(&SettingsConfig::default());
        let existing = "// custom data\nfuture-niri-option \"keep-me\"\nlayout {\n    gaps 99\n}\n";
        let merged = merge_kdl_text(&generated, Some(existing)).unwrap();
        assert!(merged.contains("future-niri-option \"keep-me\""));
        assert!(!merged.contains("gaps 99"));
    }

    #[test]
    fn generated_default_is_accepted_by_installed_niri() {
        if std::process::Command::new("niri")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let path = std::env::temp_dir().join(format!(
            "niri-settings-test-{}-{}.kdl",
            std::process::id(),
            std::thread::current().name().unwrap_or("worker")
        ));
        std::fs::write(&path, generate_kdl(&SettingsConfig::default())).unwrap();
        let output = std::process::Command::new("niri")
            .args(["validate", "--config"])
            .arg(&path)
            .output()
            .unwrap();
        let _ = std::fs::remove_file(path);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn primary_kdl_fields_round_trip_into_model() {
        let mut source = SettingsConfig::default();
        source.binds.push(Keybind {
            key: "Mod+T".into(),
            action: "spawn".into(),
            action_args: "foot".into(),
            ..Keybind::default()
        });
        source.outputs.push(OutputConfig {
            name: "DP-1".into(),
            mode_width: 2560,
            mode_height: 1440,
            mode_refresh_mhz: 144_000,
            scale: 1.5,
            ..OutputConfig::new("DP-1")
        });
        source.window_rules.push(WindowRule {
            match_app_id: "^firefox$".into(),
            open_on_workspace: "2".into(),
            open_floating: TriState::Off,
            ..WindowRule::default()
        });
        let document: ::kdl::KdlDocument = generate_kdl(&source).parse().unwrap();
        let mut restored = SettingsConfig::default();
        overlay_primary_kdl(&mut restored, &document);
        assert_eq!(restored.binds[0].key, "Mod+T");
        assert_eq!(restored.outputs[0].mode_refresh_mhz, 144_000);
        assert_eq!(restored.window_rules[0].open_on_workspace, "2");
        assert_eq!(restored.window_rules[0].open_floating, TriState::Off);
    }

    #[test]
    fn kdl_contains_gaps() {
        let mut cfg = SettingsConfig::default();
        cfg.layout.gaps = 24.0;
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("gaps 24"), "expected 'gaps 24' in: {kdl}");
    }

    #[test]
    fn kdl_touchpad_tap() {
        let mut cfg = SettingsConfig::default();
        cfg.input.touchpad_tap = true;
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("tap"), "expected 'tap' in: {kdl}");
    }

    #[test]
    fn kdl_prefer_no_csd() {
        let mut cfg = SettingsConfig::default();
        cfg.misc.prefer_no_csd = true;
        let kdl = generate_kdl(&cfg);
        assert!(
            kdl.contains("prefer-no-csd"),
            "expected prefer-no-csd: {kdl}"
        );
    }

    #[test]
    fn sanitise_color_strips_junk() {
        assert_eq!(sanitise_color("#7fc8ff"), "#7fc8ff");
        assert_eq!(sanitise_color("#fff"), "#fff");
        assert_eq!(sanitise_color("#7fc8ff80"), "#7fc8ff80");
        assert_eq!(sanitise_color("#7fc8ff; DROP TABLE"), "#ffffff");
        assert_eq!(sanitise_color("red"), "#ffffff");
        assert_eq!(sanitise_color(""), "#ffffff");
    }

    #[test]
    fn center_focused_column_variants() {
        assert_eq!(CenterFocusedColumn::Never.as_kdl_str(), "never");
        assert_eq!(CenterFocusedColumn::Always.as_kdl_str(), "always");
        assert_eq!(CenterFocusedColumn::OnOverflow.as_kdl_str(), "on-overflow");
    }

    #[test]
    fn screenshot_path_null_case_insensitive() {
        for variant in &["null", "NULL", "Null", "nUlL"] {
            let mut cfg = SettingsConfig::default();
            cfg.misc.screenshot_path = variant.to_string();
            let kdl = generate_kdl(&cfg);
            assert!(
                kdl.contains("screenshot-path null\n"),
                "'{}' should produce unquoted null but got:\n{kdl}",
                variant,
            );
            assert!(
                !kdl.contains("screenshot-path \""),
                "'{}' must not produce a quoted path: {kdl}",
                variant,
            );
        }
    }

    #[test]
    fn screenshot_path_regular_value() {
        let mut cfg = SettingsConfig::default();
        cfg.misc.screenshot_path = "~/Pictures/Screenshots/%Y-%m-%d.png".to_string();
        let kdl = generate_kdl(&cfg);
        assert!(
            kdl.contains("screenshot-path \"~/Pictures/Screenshots/"),
            "expected quoted path in: {kdl}",
        );
    }

    #[test]
    fn overview_backdrop_invalid_color_skipped() {
        let mut cfg = SettingsConfig::default();
        cfg.misc.overview_backdrop_color = "#26262".to_string();
        let kdl = generate_kdl(&cfg);
        assert!(
            !kdl.contains("backdrop-color"),
            "invalid color should be omitted entirely, got:\n{kdl}",
        );
    }

    #[test]
    fn overview_backdrop_valid_color_written() {
        let mut cfg = SettingsConfig::default();
        cfg.misc.overview_backdrop_color = "#262626".to_string();
        let kdl = generate_kdl(&cfg);
        assert!(
            kdl.contains("backdrop-color \"#262626\""),
            "expected backdrop-color: {kdl}",
        );
    }

    #[test]
    fn overview_zoom_float_normalised_on_load() {
        let mut cfg = SettingsConfig::default();
        cfg.misc.overview_zoom = 0.300_000_000_000_000_04_f64;
        if cfg.misc.overview_zoom > 0.0 {
            cfg.misc.overview_zoom = (cfg.misc.overview_zoom * 20.0).round() / 20.0;
        }
        let kdl = generate_kdl(&cfg);
        assert!(
            kdl.contains("zoom 0.30"),
            "expected zoom 0.30 but got:\n{kdl}",
        );
    }

    #[test]
    fn cursor_block_only_written_when_non_default() {
        let cfg = SettingsConfig::default();
        let kdl = generate_kdl(&cfg);
        assert!(
            !kdl.contains("cursor {"),
            "cursor block should be absent for default config",
        );
    }

    #[test]
    fn xwayland_off_takes_priority_over_path() {
        let mut cfg = SettingsConfig::default();
        cfg.misc.xwayland_off = true;
        cfg.misc.xwayland_path = "/usr/local/bin/xwayland-satellite".to_string();
        let kdl = generate_kdl(&cfg);
        assert!(
            kdl.contains("xwayland-satellite {\n    off"),
            "expected off block: {kdl}",
        );
        assert!(
            !kdl.contains("path"),
            "path should be suppressed when off is set: {kdl}",
        );
    }

    #[test]
    fn switch_events_written_when_set() {
        let mut cfg = SettingsConfig::default();
        cfg.switch_events.lid_close = vec!["notify-send".into(), "Lid closed".into()];
        cfg.switch_events.tablet_mode_on = vec!["wvkbd-mobintl".into()];
        let kdl = generate_kdl(&cfg);
        assert!(
            kdl.contains("switch-events {"),
            "expected switch-events block"
        );
        assert!(kdl.contains(r#"lid-close { spawn "notify-send" "Lid closed"; }"#));
        assert!(kdl.contains(r#"tablet-mode-on { spawn "wvkbd-mobintl"; }"#));
        assert!(
            !kdl.contains("lid-open"),
            "lid-open should be absent when empty"
        );
    }

    #[test]
    fn switch_events_absent_for_default() {
        let cfg = SettingsConfig::default();
        let kdl = generate_kdl(&cfg);
        assert!(
            !kdl.contains("switch-events"),
            "switch-events absent when all empty"
        );
    }

    #[test]
    fn window_rule_written_with_match_and_dynamic_props() {
        let mut cfg = SettingsConfig::default();
        let mut r = WindowRule::default();
        r.match_app_id = "^firefox$".into();
        r.open_floating = TriState::On;
        r.opacity = 0.85;
        r.block_out_from = BlockOutFrom::Screencast;
        cfg.window_rules.push(r);
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("window-rule {"), "window-rule block expected");
        assert!(kdl.contains(r#"match app-id="^firefox$""#));
        assert!(kdl.contains("open-floating true"));
        assert!(kdl.contains("opacity 0.85"));
        assert!(kdl.contains(r#"block-out-from "screencast""#));
    }

    #[test]
    fn window_rule_size_limits_written() {
        let mut cfg = SettingsConfig::default();
        let mut r = WindowRule::default();
        r.match_app_id = "obs".into();
        r.min_width = 876;
        cfg.window_rules.push(r);
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("min-width 876"));
    }

    #[test]
    fn window_rule_empty_when_all_defaults() {
        let mut cfg = SettingsConfig::default();
        cfg.window_rules.push(WindowRule::default());
        let kdl = generate_kdl(&cfg);
        assert!(
            !kdl.contains("window-rule"),
            "empty rule should not produce KDL block"
        );
    }

    #[test]
    fn layer_rule_written_with_shadow_and_radius() {
        let mut cfg = SettingsConfig::default();
        let mut r = LayerRule::default();
        r.match_namespace = "^launcher$".into();
        r.shadow = TriState::On;
        r.geometry_corner_radius = 12.0;
        cfg.layer_rules.push(r);
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("layer-rule {"), "layer-rule block expected");
        assert!(kdl.contains(r#"match namespace="^launcher$""#));
        assert!(kdl.contains("shadow {\n        on\n    }"));
        assert!(kdl.contains("geometry-corner-radius 12.0"));
    }

    #[test]
    fn layer_rule_block_out_from_screen_capture() {
        let mut cfg = SettingsConfig::default();
        let mut r = LayerRule::default();
        r.match_namespace = "^notifications$".into();
        r.block_out_from = BlockOutFrom::ScreenCapture;
        cfg.layer_rules.push(r);
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains(r#"block-out-from "screen-capture""#));
    }

    #[test]
    fn window_rule_full_roundtrip() {
        let mut source = SettingsConfig::default();
        source.window_rules.push(WindowRule {
            match_app_id: "^firefox$".into(),
            match_title: "^Library$".into(),
            match_at_startup: TriState::On,
            open_maximized: TriState::On,
            open_fullscreen: TriState::Off,
            open_floating: TriState::On,
            open_focused: TriState::Off,
            open_on_output: "DP-1".into(),
            open_on_workspace: "3".into(),
            opacity: 0.85,
            block_out_from: BlockOutFrom::ScreenCapture,
            draw_border_with_background: TriState::On,
            geometry_corner_radius: 8.0,
            clip_to_geometry: TriState::Off,
            variable_refresh_rate: TriState::On,
            min_width: 100,
            max_width: 1920,
            min_height: 200,
            max_height: 1080,
            scroll_factor: 1.5,
            ..WindowRule::default()
        });
        let document: ::kdl::KdlDocument = generate_kdl(&source).parse().unwrap();
        let mut restored = SettingsConfig::default();
        overlay_primary_kdl(&mut restored, &document);
        let r = &restored.window_rules[0];
        // app-id and title live on separate `match` lines: both must survive.
        assert_eq!(r.match_app_id, "^firefox$");
        assert_eq!(r.match_title, "^Library$");
        assert_eq!(r.match_at_startup, TriState::On);
        assert_eq!(r.open_maximized, TriState::On);
        assert_eq!(r.open_fullscreen, TriState::Off);
        assert_eq!(r.open_floating, TriState::On);
        assert_eq!(r.open_focused, TriState::Off);
        assert_eq!(r.open_on_output, "DP-1");
        assert_eq!(r.open_on_workspace, "3");
        assert_eq!(r.opacity, 0.85);
        // Previously dropped on read:
        assert_eq!(r.block_out_from, BlockOutFrom::ScreenCapture);
        assert_eq!(r.draw_border_with_background, TriState::On);
        assert_eq!(r.geometry_corner_radius, 8.0);
        assert_eq!(r.clip_to_geometry, TriState::Off);
        assert_eq!(r.variable_refresh_rate, TriState::On);
        assert_eq!(r.min_width, 100);
        assert_eq!(r.max_width, 1920);
        assert_eq!(r.min_height, 200);
        assert_eq!(r.max_height, 1080);
        assert_eq!(r.scroll_factor, 1.5);
    }

    #[test]
    fn layer_rule_full_roundtrip() {
        let mut source = SettingsConfig::default();
        source.layer_rules.push(LayerRule {
            match_namespace: "^launcher$".into(),
            match_at_startup: TriState::On,
            opacity: 0.90,
            block_out_from: BlockOutFrom::Screencast,
            shadow: TriState::On,
            geometry_corner_radius: 12.0,
            place_within_backdrop: TriState::On,
            ..LayerRule::default()
        });
        let document: ::kdl::KdlDocument = generate_kdl(&source).parse().unwrap();
        let mut restored = SettingsConfig::default();
        overlay_primary_kdl(&mut restored, &document);
        // Layer rules were entirely write-only before this fix.
        assert_eq!(restored.layer_rules.len(), 1);
        let r = &restored.layer_rules[0];
        assert_eq!(r.match_namespace, "^launcher$");
        assert_eq!(r.match_at_startup, TriState::On);
        assert_eq!(r.opacity, 0.90);
        assert_eq!(r.block_out_from, BlockOutFrom::Screencast);
        assert_eq!(r.shadow, TriState::On);
        assert_eq!(r.geometry_corner_radius, 12.0);
        assert_eq!(r.place_within_backdrop, TriState::On);
    }

    #[test]
    fn window_rule_parses_combined_match_line() {
        // niri also accepts one `match` node carrying several properties;
        // the reader must capture every property even when not split.
        let document: ::kdl::KdlDocument = r#"
window-rule {
    match app-id="firefox" title="Library"
}
"#
        .parse()
        .unwrap();
        let mut restored = SettingsConfig::default();
        overlay_primary_kdl(&mut restored, &document);
        assert_eq!(restored.window_rules[0].match_app_id, "firefox");
        assert_eq!(restored.window_rules[0].match_title, "Library");
    }

    #[test]
    fn tristate_roundtrip() {
        assert_eq!(TriState::from_index(0), TriState::Default);
        assert_eq!(TriState::from_index(1), TriState::On);
        assert_eq!(TriState::from_index(2), TriState::Off);
        assert_eq!(TriState::On.to_index(), 1);
        assert_eq!(TriState::Off.to_index(), 2);
    }

    #[test]
    fn block_out_from_roundtrip() {
        assert_eq!(BlockOutFrom::from_index(0), BlockOutFrom::None);
        assert_eq!(BlockOutFrom::from_index(1), BlockOutFrom::Screencast);
        assert_eq!(BlockOutFrom::from_index(2), BlockOutFrom::ScreenCapture);
        assert_eq!(BlockOutFrom::Screencast.as_kdl_str(), "screencast");
        assert_eq!(BlockOutFrom::ScreenCapture.as_kdl_str(), "screen-capture");
    }

    #[test]
    fn keybind_spawn_written_correctly() {
        let mut cfg = SettingsConfig::default();
        let mut b = Keybind::default();
        b.key = "Mod+T".into();
        b.action = "spawn".into();
        b.action_args = "alacritty -e fish".into();
        cfg.binds.push(b);
        let kdl = generate_kdl(&cfg);
        assert!(
            kdl.contains(r#"Mod+T { spawn "alacritty" "-e" "fish"; }"#),
            "got:\n{kdl}",
        );
    }

    #[test]
    fn keybind_spawn_sh_written_correctly() {
        let mut cfg = SettingsConfig::default();
        let mut b = Keybind::default();
        b.key = "Mod+E".into();
        b.action = "spawn-sh".into();
        b.action_args = "notify-send hello world".into();
        cfg.binds.push(b);
        let kdl = generate_kdl(&cfg);
        assert!(
            kdl.contains(r#"Mod+E { spawn-sh "notify-send hello world"; }"#),
            "got:\n{kdl}",
        );
    }

    #[test]
    fn keybind_no_arg_action() {
        let mut cfg = SettingsConfig::default();
        let mut b = Keybind::default();
        b.key = "Mod+Left".into();
        b.action = "focus-column-left".into();
        cfg.binds.push(b);
        let kdl = generate_kdl(&cfg);
        assert!(
            kdl.contains("Mod+Left { focus-column-left; }"),
            "got:\n{kdl}",
        );
    }

    #[test]
    fn keybind_cooldown_and_no_repeat() {
        let mut cfg = SettingsConfig::default();
        let mut b = Keybind::default();
        b.key = "Mod+WheelScrollDown".into();
        b.action = "focus-workspace-down".into();
        b.repeat = false;
        b.cooldown_ms = 150;
        cfg.binds.push(b);
        let kdl = generate_kdl(&cfg);
        assert!(
            kdl.contains(
                "Mod+WheelScrollDown repeat=false cooldown-ms=150 \
                 { focus-workspace-down; }"
            ),
            "got:\n{kdl}",
        );
    }

    #[test]
    fn keybind_allow_when_locked() {
        let mut cfg = SettingsConfig::default();
        let mut b = Keybind::default();
        b.key = "XF86AudioMute".into();
        b.action = "spawn".into();
        b.action_args = "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle".into();
        b.allow_when_locked = true;
        cfg.binds.push(b);
        let kdl = generate_kdl(&cfg);
        assert!(kdl.contains("allow-when-locked=true"), "got:\n{kdl}",);
    }

    #[test]
    fn keybind_empty_key_skipped() {
        let mut cfg = SettingsConfig::default();
        let mut b = Keybind::default();
        b.key = "".into();
        b.action = "close-window".into();
        cfg.binds.push(b);
        let kdl = generate_kdl(&cfg);
        assert!(
            !kdl.contains("binds {"),
            "empty-key bind must be skipped: {kdl}",
        );
    }

    #[test]
    fn keybind_spawn_empty_args_skipped() {
        let mut cfg = SettingsConfig::default();
        let mut b = Keybind::default();
        b.key = "Mod+T".into();
        b.action = "spawn".into();
        b.action_args = "".into();
        cfg.binds.push(b);
        let kdl = generate_kdl(&cfg);
        assert!(
            !kdl.contains("binds {"),
            "spawn with no args must be skipped: {kdl}",
        );
    }

    #[test]
    fn sanitise_keybind_key_strips_injection() {
        assert_eq!(sanitise_keybind_key(r#"Mod+T { evil; }"#), "Mod+Tevil",);
        assert_eq!(sanitise_keybind_key("XF86AudioMute"), "XF86AudioMute",);
        assert_eq!(sanitise_keybind_key("ISO_Level3_Shift"), "ISO_Level3_Shift",);
        assert_eq!(sanitise_keybind_key("Super+Alt+L"), "Super+Alt+L");
    }

    #[test]
    fn action_args_hint_spawn_nonempty() {
        assert!(!action_args_hint("spawn").is_empty());
        assert!(!action_args_hint("spawn-sh").is_empty());
        assert!(action_args_hint("close-window").is_empty());
    }

    #[test]
    fn parse_bind_line_simple_action() {
        let b = parse_bind_line("Mod+Left { focus-column-left; }").unwrap();
        assert_eq!(b.key, "Mod+Left");
        assert_eq!(b.action, "focus-column-left");
        assert!(b.action_args.is_empty());
        assert!(b.repeat);
        assert_eq!(b.cooldown_ms, 0);
        assert!(!b.allow_when_locked);
    }

    #[test]
    fn parse_bind_line_spawn_args() {
        let b = parse_bind_line(r#"Mod+T { spawn "alacritty"; }"#).unwrap();
        assert_eq!(b.key, "Mod+T");
        assert_eq!(b.action, "spawn");
        assert_eq!(b.action_args, "alacritty");
    }

    #[test]
    fn parse_bind_line_spawn_multi_args() {
        let b = parse_bind_line(
            r#"XF86MonBrightnessUp { spawn "brightnessctl" "--class=backlight" "set" "+10%"; }"#,
        )
        .unwrap();
        assert_eq!(b.action, "spawn");
        assert_eq!(b.action_args, "brightnessctl --class=backlight set +10%");
    }

    #[test]
    fn parse_bind_line_spawn_sh() {
        let b = parse_bind_line(
            r#"XF86AudioMute allow-when-locked=true { spawn-sh "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"; }"#,
        )
        .unwrap();
        assert_eq!(b.action, "spawn-sh");
        assert_eq!(b.action_args, "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle");
        assert!(b.allow_when_locked);
    }

    #[test]
    fn parse_bind_line_repeat_false() {
        let b = parse_bind_line("Mod+Q repeat=false { close-window; }").unwrap();
        assert_eq!(b.key, "Mod+Q");
        assert!(!b.repeat);
    }

    #[test]
    fn parse_bind_line_cooldown() {
        let b = parse_bind_line(
            "Mod+WheelScrollDown cooldown-ms=150 \
             { focus-workspace-down; }",
        )
        .unwrap();
        assert_eq!(b.cooldown_ms, 150);
        assert_eq!(b.action, "focus-workspace-down");
    }

    #[test]
    fn parse_bind_line_action_with_index() {
        let b = parse_bind_line("Mod+1 { focus-workspace 1; }").unwrap();
        assert_eq!(b.action, "focus-workspace");
        assert_eq!(b.action_args, "1");
    }

    #[test]
    fn parse_bind_line_comment_returns_none() {
        assert!(parse_bind_line("// Mod+T { spawn \"alacritty\"; }").is_none());
        assert!(parse_bind_line("   // commented out").is_none());
        assert!(parse_bind_line("").is_none());
    }

    #[test]
    fn parse_bind_line_hotkey_overlay_title_ignored() {
        let b = parse_bind_line(
            r#"Mod+T hotkey-overlay-title="Open alacritty" { spawn "alacritty"; }"#,
        )
        .unwrap();
        assert_eq!(b.key, "Mod+T");
        assert_eq!(b.action, "spawn");
        assert_eq!(b.action_args, "alacritty");
    }
}
