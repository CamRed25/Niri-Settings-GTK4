// settings_backend/keybinds.rs — Keybinding definitions, NIRI_ACTIONS list,
// bind-line parser, and niri config import.

use super::types::Keybind;
use std::path::PathBuf;

/// Canonicalise a user-facing key combination for conflict detection.
pub fn normalize_key_combo(input: &str) -> String {
    let mut modifiers = Vec::new();
    let mut keys = Vec::new();
    for part in input
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let canonical = match part.to_ascii_lowercase().as_str() {
            "control" | "ctrl" => "Ctrl",
            "alt" => "Alt",
            "shift" => "Shift",
            "super" | "logo" | "meta" => "Super",
            "mod" => "Mod",
            _ => {
                keys.push(part.to_ascii_lowercase());
                continue;
            }
        };
        if !modifiers.contains(&canonical) {
            modifiers.push(canonical);
        }
    }
    modifiers.sort_by_key(|modifier| match *modifier {
        "Mod" => 0,
        "Super" => 1,
        "Ctrl" => 2,
        "Alt" => 3,
        "Shift" => 4,
        _ => 5,
    });
    let mut parts: Vec<String> = modifiers.into_iter().map(str::to_owned).collect();
    parts.extend(keys);
    parts.join("+")
}

/// Return all duplicate key combinations and the indexes which use them.
pub fn binding_conflicts(binds: &[Keybind]) -> Vec<(String, Vec<usize>)> {
    let mut by_key: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, bind) in binds.iter().enumerate() {
        let key = normalize_key_combo(&bind.key);
        if !key.is_empty() {
            by_key.entry(key).or_default().push(index);
        }
    }
    by_key
        .into_iter()
        .filter(|(_, indexes)| indexes.len() > 1)
        .collect()
}

// ── Action catalogue ──────────────────────────────────────────────────────────

/// Full ordered list of niri action names available for keybinding.
pub const NIRI_ACTIONS: &[&str] = &[
    // Launch
    "spawn",
    "spawn-sh",
    // Window
    "close-window",
    "fullscreen-window",
    "toggle-windowed-fullscreen",
    "toggle-window-floating",
    "move-window-to-floating",
    "move-window-to-tiling",
    "focus-floating",
    "focus-tiling",
    "switch-focus-between-floating-and-tiling",
    "toggle-window-rule-opacity",
    "toggle-window-urgent",
    "set-window-urgent",
    "unset-window-urgent",
    "maximize-window-to-edges",
    "set-window-width",
    "set-window-height",
    "reset-window-height",
    "switch-preset-window-width",
    "switch-preset-window-width-back",
    "switch-preset-window-height",
    "switch-preset-window-height-back",
    "move-window-down",
    "move-window-up",
    "move-window-down-or-to-workspace-down",
    "move-window-up-or-to-workspace-up",
    "move-window-to-workspace",
    "move-window-to-workspace-down",
    "move-window-to-workspace-up",
    "consume-or-expel-window-left",
    "consume-or-expel-window-right",
    "consume-window-into-column",
    "expel-window-from-column",
    "swap-window-left",
    "swap-window-right",
    "move-floating-window",
    // Focus
    "focus-window-previous",
    "focus-window-down",
    "focus-window-up",
    "focus-window-down-or-column-left",
    "focus-window-down-or-column-right",
    "focus-window-up-or-column-left",
    "focus-window-up-or-column-right",
    "focus-window-or-monitor-up",
    "focus-window-or-monitor-down",
    "focus-window-or-workspace-down",
    "focus-window-or-workspace-up",
    "focus-window-top",
    "focus-window-bottom",
    "focus-window-down-or-top",
    "focus-window-up-or-bottom",
    // Column
    "focus-column-left",
    "focus-column-right",
    "focus-column-first",
    "focus-column-last",
    "focus-column-right-or-first",
    "focus-column-left-or-last",
    "focus-column-or-monitor-left",
    "focus-column-or-monitor-right",
    "move-column-left",
    "move-column-right",
    "move-column-to-first",
    "move-column-to-last",
    "move-column-left-or-to-monitor-left",
    "move-column-right-or-to-monitor-right",
    "move-column-to-monitor-left",
    "move-column-to-monitor-right",
    "move-column-to-monitor-down",
    "move-column-to-monitor-up",
    "move-column-to-monitor-previous",
    "move-column-to-monitor-next",
    "move-column-to-workspace",
    "move-column-to-workspace-down",
    "move-column-to-workspace-up",
    "toggle-column-tabbed-display",
    "set-column-display",
    "set-column-width",
    "switch-preset-column-width",
    "switch-preset-column-width-back",
    "maximize-column",
    "expand-column-to-available-width",
    // Workspace
    "focus-workspace",
    "focus-workspace-down",
    "focus-workspace-up",
    "focus-workspace-previous",
    "move-workspace-to-monitor-left",
    "move-workspace-to-monitor-right",
    "move-workspace-to-monitor-down",
    "move-workspace-to-monitor-up",
    "move-workspace-to-monitor-previous",
    "move-workspace-to-monitor-next",
    "move-workspace-to-monitor",
    // Overlay / UI
    "toggle-overview",
    "open-overview",
    "close-overview",
    "show-hotkey-overlay",
    "toggle-keyboard-shortcuts-inhibit",
    // Screenshot
    "screenshot",
    "screenshot-screen",
    "screenshot-window",
    "do-screen-transition",
    // Layout / misc
    "switch-layout",
    "power-off-monitors",
    "power-on-monitors",
    "set-dynamic-cast-window",
    "set-dynamic-cast-monitor",
    "clear-dynamic-cast-target",
    "load-config-file",
    // Debug
    "toggle-debug-tint",
    "debug-toggle-opaque-regions",
    "debug-toggle-damage",
    // Session
    "quit",
];

/// Returns a short hint describing what to put in `action_args` for a given action.
pub fn action_args_hint(action: &str) -> &'static str {
    match action {
        "spawn" => "program  arg1  arg2 …",
        "spawn-sh" => "shell command string",
        "focus-workspace" | "move-window-to-workspace" | "move-column-to-workspace" => {
            "workspace index  (1, 2, …)"
        }
        "set-column-width" | "set-window-width" => "size  e.g. 50%  or  1200",
        "set-window-height" => "size  e.g. 50%  or  800",
        "set-column-display" => "normal  or  tabbed",
        "switch-layout" => "next  /  prev  /  index",
        "do-screen-transition" => "delay-ms=250  (optional)",
        "quit" => "skip-confirmation=true  (optional)",
        "move-workspace-to-monitor" | "move-column-to-monitor" => "monitor name or index",
        "move-floating-window" => "dx dy  (pixels, e.g. 0 50)",
        _ => "",
    }
}

/// Returns `true` if the given action takes mandatory or useful arguments.
pub fn action_needs_args(action: &str) -> bool {
    matches!(
        action,
        "spawn"
            | "spawn-sh"
            | "focus-workspace"
            | "move-window-to-workspace"
            | "move-column-to-workspace"
            | "move-workspace-to-monitor"
            | "move-column-to-monitor"
            | "set-column-width"
            | "set-window-width"
            | "set-window-height"
            | "set-column-display"
            | "switch-layout"
            | "move-floating-window"
    )
}

// ── Bind-line parser ──────────────────────────────────────────────────────────

/// Parse a single bind line from niri config KDL.
///
/// Expected form (all on one line):
/// ```text
/// KeyCombo [prop=val …] { action [args]; }
/// ```
///
/// Returns `None` for comments, blank lines, or lines that can't be parsed.
pub fn parse_bind_line(line: &str) -> Option<Keybind> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return None;
    }
    let brace_open = trimmed.find('{')?;
    let brace_close = trimmed.rfind('}')?;
    if brace_close <= brace_open {
        return None;
    }

    let before_brace = trimmed[..brace_open].trim();
    let action_part = trimmed[brace_open + 1..brace_close]
        .trim()
        .trim_end_matches(';')
        .trim();

    if action_part.is_empty() || before_brace.is_empty() {
        return None;
    }

    let mut tokens = before_brace.split_whitespace();
    let key = tokens.next()?.to_string();
    if key.is_empty() || key.contains('"') {
        return None;
    }

    let mut repeat = true;
    let mut cooldown_ms: u32 = 0;
    let mut allow_when_locked = false;
    for tok in tokens {
        if tok == "repeat=false" {
            repeat = false;
        } else if let Some(ms) = tok.strip_prefix("cooldown-ms=") {
            match ms.parse() {
                Ok(val) => cooldown_ms = val,
                Err(_) => {
                    log::warn!("settings: invalid cooldown-ms value '{ms}' in bind: {line}");
                    return None;
                }
            }
        } else if tok == "allow-when-locked=true" {
            allow_when_locked = true;
        }
    }

    let (action, action_args) = parse_action_and_args(action_part);

    Some(Keybind {
        id: super::types::next_row_id(),
        key,
        action,
        action_args,
        repeat,
        cooldown_ms,
        allow_when_locked,
    })
}

/// Split `action [arg1 arg2 …]` into the action name and a single
/// space-joined argument string.
fn parse_action_and_args(s: &str) -> (String, String) {
    let s = s.trim();
    let split = s.find(|c: char| c.is_whitespace()).unwrap_or(s.len());
    let action = s[..split].to_string();
    let rest = s[split..].trim();
    if rest.is_empty() {
        return (action, String::new());
    }

    let mut args: Vec<String> = Vec::new();
    let mut chars = rest.chars().peekable();
    while chars.peek().is_some() {
        while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            chars.next();
        }
        match chars.peek() {
            None => break,
            Some('"') => {
                chars.next();
                let mut buf = String::new();
                for c in chars.by_ref() {
                    if c == '"' {
                        break;
                    }
                    buf.push(c);
                }
                if !buf.is_empty() {
                    args.push(buf);
                }
            }
            _ => {
                let mut buf = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() {
                        break;
                    }
                    chars.next();
                    buf.push(c);
                }
                if !buf.is_empty() {
                    args.push(buf);
                }
            }
        }
    }

    (action, args.join(" "))
}

// ── Config import ─────────────────────────────────────────────────────────────

/// Read `~/.config/niri/config.kdl` and extract all bind entries from the
/// first `binds { … }` block.
pub fn import_binds_from_niri_config(niri_config_path: &std::path::Path) -> Vec<Keybind> {
    let mut visited = std::collections::HashSet::new();
    import_binds_recursive(niri_config_path, &mut visited, 0)
}

fn import_binds_recursive(
    config_path: &std::path::Path,
    visited: &mut std::collections::HashSet<PathBuf>,
    depth: usize,
) -> Vec<Keybind> {
    if depth > 16 {
        log::warn!(
            "settings: include nesting exceeds 16 at {}",
            config_path.display()
        );
        return Vec::new();
    }
    let identity = config_path
        .canonicalize()
        .unwrap_or_else(|_| config_path.to_path_buf());
    if !visited.insert(identity) {
        return Vec::new();
    }
    let content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("settings: could not read niri config for bind import: {e}");
            return Vec::new();
        }
    };

    if let Ok(document) = content.parse::<kdl::KdlDocument>() {
        let mut binds = Vec::new();
        for node in document.nodes() {
            match node.name().value() {
                "include" => {
                    let Some(include) = node.get(0).and_then(|entry| entry.value().as_string())
                    else {
                        continue;
                    };
                    let include = expand_include(config_path, include);
                    binds.extend(import_binds_recursive(&include, visited, depth + 1));
                }
                "binds" => {
                    if let Some(children) = node.children() {
                        binds.extend(
                            children
                                .nodes()
                                .iter()
                                .filter_map(|bind| parse_bind_line(&bind.to_string())),
                        );
                    }
                }
                _ => {}
            }
        }
        log::info!(
            "settings: imported {} binds from {}",
            binds.len(),
            config_path.display()
        );
        return binds;
    }

    // Tolerant fallback for a partially edited config.
    let mut binds = Vec::new();
    let mut in_binds = false;
    let mut brace_depth: u32 = 0;
    for line in content.lines() {
        let trimmed = line.trim();
        if !in_binds {
            if trimmed.starts_with("binds {") {
                in_binds = true;
                brace_depth = 1;
            }
            continue;
        }
        let opens = trimmed.chars().filter(|&c| c == '{').count() as u32;
        let closes = trimmed.chars().filter(|&c| c == '}').count() as u32;
        if trimmed == "}" {
            brace_depth = brace_depth.saturating_sub(1);
            if brace_depth == 0 {
                break;
            }
            continue;
        }
        brace_depth = brace_depth.saturating_add(opens).saturating_sub(closes);
        if let Some(bind) = parse_bind_line(trimmed) {
            binds.push(bind);
        }
    }
    binds
}

fn expand_include(parent_config: &std::path::Path, include: &str) -> PathBuf {
    if let Some(rest) = include.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    let path = PathBuf::from(include);
    if path.is_absolute() {
        path
    } else {
        parent_config
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(path)
    }
}
