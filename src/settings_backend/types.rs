// settings_backend/types.rs — All config structs and enums for niri settings.

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// Monotonic source of stable, process-unique row identifiers.
///
/// These ids are runtime-only (never serialized): the settings UI uses them to
/// address a specific rule/keybind across `rebuild()` cycles without relying on
/// vector indices, which shift when rows are inserted or deleted.
static NEXT_ROW_ID: AtomicU64 = AtomicU64::new(1);

/// Allocate a fresh, process-unique row id.
pub fn next_row_id() -> u64 {
    NEXT_ROW_ID.fetch_add(1, Ordering::Relaxed)
}

// ── Root config ───────────────────────────────────────────────────────────────

/// Root settings configuration managed by the settings GUI.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsConfig {
    #[serde(default)]
    pub input: InputSettings,
    #[serde(default)]
    pub layout: LayoutSettings,
    #[serde(default)]
    pub misc: MiscSettings,
    #[serde(default)]
    pub anim: AnimSettings,
    #[serde(default)]
    pub gestures: GestureSettings,
    #[serde(default)]
    pub workspaces: Vec<NamedWorkspace>,
    #[serde(default)]
    pub recent_windows: RecentWindowsSettings,
    #[serde(default)]
    pub debug: DebugSettings,
    #[serde(default)]
    pub outputs: Vec<OutputConfig>,
    #[serde(default)]
    pub window_rules: Vec<WindowRule>,
    #[serde(default)]
    pub layer_rules: Vec<LayerRule>,
    #[serde(default)]
    pub switch_events: SwitchEventsSettings,
    #[serde(default)]
    pub binds: Vec<Keybind>,
}

impl SettingsConfig {
    /// Assign a fresh runtime id to every keybind/window-rule/layer-rule that
    /// does not yet have one (ids are `#[serde(skip)]`, so loaded rows arrive
    /// with `id == 0`). Call this once after loading so the UI can address rows
    /// stably across `rebuild()` cycles. Idempotent: rows with a non-zero id are
    /// left untouched.
    pub fn ensure_row_ids(&mut self) {
        for b in &mut self.binds {
            if b.id == 0 {
                b.id = next_row_id();
            }
        }
        for r in &mut self.window_rules {
            if r.id == 0 {
                r.id = next_row_id();
            }
        }
        for r in &mut self.layer_rules {
            if r.id == 0 {
                r.id = next_row_id();
            }
        }
    }

    /// Mutable access to a keybind by its stable id, if it still exists.
    pub fn bind_mut(&mut self, id: u64) -> Option<&mut Keybind> {
        self.binds.iter_mut().find(|b| b.id == id)
    }

    /// Mutable access to a window rule by its stable id, if it still exists.
    pub fn window_rule_mut(&mut self, id: u64) -> Option<&mut WindowRule> {
        self.window_rules.iter_mut().find(|r| r.id == id)
    }

    /// Mutable access to a layer rule by its stable id, if it still exists.
    pub fn layer_rule_mut(&mut self, id: u64) -> Option<&mut LayerRule> {
        self.layer_rules.iter_mut().find(|r| r.id == id)
    }

    /// Remove a keybind by stable id. No-op if already gone.
    pub fn remove_bind(&mut self, id: u64) {
        self.binds.retain(|b| b.id != id);
    }

    /// Remove a window rule by stable id. No-op if already gone.
    pub fn remove_window_rule(&mut self, id: u64) {
        self.window_rules.retain(|r| r.id != id);
    }

    /// Remove a layer rule by stable id. No-op if already gone.
    pub fn remove_layer_rule(&mut self, id: u64) {
        self.layer_rules.retain(|r| r.id != id);
    }
}

// ── Shared enums ──────────────────────────────────────────────────────────────

/// Shared block-out-from value used by window and layer rules.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum BlockOutFrom {
    #[default]
    None,
    Screencast,
    ScreenCapture,
}

impl BlockOutFrom {
    pub fn label_variants() -> &'static [&'static str] {
        &["none", "screencast", "screen-capture"]
    }

    pub fn from_index(i: u32) -> Self {
        match i {
            1 => Self::Screencast,
            2 => Self::ScreenCapture,
            _ => Self::None,
        }
    }

    pub fn to_index(&self) -> u32 {
        match self {
            Self::None => 0,
            Self::Screencast => 1,
            Self::ScreenCapture => 2,
        }
    }

    pub fn as_kdl_str(&self) -> &'static str {
        match self {
            Self::Screencast => "screencast",
            Self::ScreenCapture => "screen-capture",
            Self::None => "",
        }
    }
}

/// Tri-state: use niri default / force on / force off.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum TriState {
    #[default]
    Default,
    On,
    Off,
}

impl TriState {
    pub fn label_variants() -> &'static [&'static str] {
        &["default", "on", "off"]
    }

    pub fn from_index(i: u32) -> Self {
        match i {
            1 => Self::On,
            2 => Self::Off,
            _ => Self::Default,
        }
    }

    pub fn to_index(&self) -> u32 {
        match self {
            Self::Default => 0,
            Self::On => 1,
            Self::Off => 2,
        }
    }

    /// Write a KDL `key true/false` line for this tri-state, if non-default.
    pub fn write_kdl_bool(&self, out: &mut String, key: &str) {
        match self {
            Self::On => {
                out.push_str("    ");
                out.push_str(key);
                out.push_str(" true\n");
            }
            Self::Off => {
                out.push_str("    ");
                out.push_str(key);
                out.push_str(" false\n");
            }
            Self::Default => {}
        }
    }

    /// Write a KDL `match key=true/false` line for this tri-state, if non-default.
    pub fn write_kdl_match(&self, out: &mut String, key: &str) {
        match self {
            Self::On => {
                out.push_str("    match ");
                out.push_str(key);
                out.push_str("=true\n");
            }
            Self::Off => {
                out.push_str("    match ");
                out.push_str(key);
                out.push_str("=false\n");
            }
            Self::Default => {}
        }
    }
}

// ── Window & layer rules ──────────────────────────────────────────────────────

/// A single window rule.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowRule {
    /// Runtime-only stable identifier (see [`next_row_id`]); never serialized.
    #[serde(skip)]
    pub id: u64,
    #[serde(default)]
    pub match_app_id: String,
    #[serde(default)]
    pub match_title: String,
    #[serde(default)]
    pub match_at_startup: TriState,
    #[serde(default)]
    pub open_maximized: TriState,
    #[serde(default)]
    pub open_fullscreen: TriState,
    #[serde(default)]
    pub open_floating: TriState,
    #[serde(default)]
    pub open_focused: TriState,
    #[serde(default)]
    pub open_on_output: String,
    #[serde(default)]
    pub open_on_workspace: String,
    #[serde(default)]
    pub opacity: f64,
    #[serde(default)]
    pub block_out_from: BlockOutFrom,
    #[serde(default)]
    pub draw_border_with_background: TriState,
    #[serde(default)]
    pub geometry_corner_radius: f64,
    #[serde(default)]
    pub clip_to_geometry: TriState,
    #[serde(default)]
    pub variable_refresh_rate: TriState,
    #[serde(default)]
    pub min_width: u32,
    #[serde(default)]
    pub max_width: u32,
    #[serde(default)]
    pub min_height: u32,
    #[serde(default)]
    pub max_height: u32,
    #[serde(default)]
    pub scroll_factor: f64,
}

/// A single layer-shell rule.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayerRule {
    /// Runtime-only stable identifier (see [`next_row_id`]); never serialized.
    #[serde(skip)]
    pub id: u64,
    #[serde(default)]
    pub match_namespace: String,
    #[serde(default)]
    pub match_at_startup: TriState,
    #[serde(default)]
    pub opacity: f64,
    #[serde(default)]
    pub block_out_from: BlockOutFrom,
    #[serde(default)]
    pub shadow: TriState,
    #[serde(default)]
    pub geometry_corner_radius: f64,
    #[serde(default)]
    pub place_within_backdrop: TriState,
}

/// Configuration for `switch-events {}`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SwitchEventsSettings {
    #[serde(default)]
    pub lid_close: Vec<String>,
    #[serde(default)]
    pub lid_open: Vec<String>,
    #[serde(default)]
    pub tablet_mode_on: Vec<String>,
    #[serde(default)]
    pub tablet_mode_off: Vec<String>,
}

// ── Key bindings ─────────────────────────────────────────────────────────────

fn default_true() -> bool {
    true
}

/// A single keybinding entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybind {
    /// Runtime-only stable identifier (see [`next_row_id`]); never serialized.
    #[serde(skip)]
    pub id: u64,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub action_args: String,
    #[serde(default = "default_true")]
    pub repeat: bool,
    #[serde(default)]
    pub cooldown_ms: u32,
    #[serde(default)]
    pub allow_when_locked: bool,
}

impl Default for Keybind {
    fn default() -> Self {
        Self {
            id: next_row_id(),
            key: String::new(),
            action: "spawn".into(),
            action_args: String::new(),
            repeat: true,
            cooldown_ms: 0,
            allow_when_locked: false,
        }
    }
}

// ── Output types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum OutputTransform {
    #[default]
    Normal,
    #[serde(rename = "90")]
    Rotate90,
    #[serde(rename = "180")]
    Rotate180,
    #[serde(rename = "270")]
    Rotate270,
    #[serde(rename = "flipped")]
    Flipped,
    #[serde(rename = "flipped-90")]
    Flipped90,
    #[serde(rename = "flipped-180")]
    Flipped180,
    #[serde(rename = "flipped-270")]
    Flipped270,
}

impl OutputTransform {
    pub fn as_kdl_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Rotate90 => "90",
            Self::Rotate180 => "180",
            Self::Rotate270 => "270",
            Self::Flipped => "flipped",
            Self::Flipped90 => "flipped-90",
            Self::Flipped180 => "flipped-180",
            Self::Flipped270 => "flipped-270",
        }
    }

    pub fn label_variants() -> &'static [&'static str] {
        &[
            "normal",
            "90°",
            "180°",
            "270°",
            "flipped",
            "flipped 90°",
            "flipped 180°",
            "flipped 270°",
        ]
    }

    pub fn from_index(i: u32) -> Self {
        match i {
            1 => Self::Rotate90,
            2 => Self::Rotate180,
            3 => Self::Rotate270,
            4 => Self::Flipped,
            5 => Self::Flipped90,
            6 => Self::Flipped180,
            7 => Self::Flipped270,
            _ => Self::Normal,
        }
    }

    pub fn to_index(&self) -> u32 {
        match self {
            Self::Normal => 0,
            Self::Rotate90 => 1,
            Self::Rotate180 => 2,
            Self::Rotate270 => 3,
            Self::Flipped => 4,
            Self::Flipped90 => 5,
            Self::Flipped180 => 6,
            Self::Flipped270 => 7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum VrrMode {
    #[default]
    Default,
    On,
    Off,
    OnDemand,
}

impl VrrMode {
    pub fn label_variants() -> &'static [&'static str] {
        &["default", "on", "off", "on-demand"]
    }

    pub fn from_index(i: u32) -> Self {
        match i {
            1 => Self::On,
            2 => Self::Off,
            3 => Self::OnDemand,
            _ => Self::Default,
        }
    }

    pub fn to_index(&self) -> u32 {
        match self {
            Self::Default => 0,
            Self::On => 1,
            Self::Off => 2,
            Self::OnDemand => 3,
        }
    }
}

/// Persisted configuration for one physical output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub name: String,
    #[serde(default)]
    pub off: bool,
    #[serde(default)]
    pub mode_width: u32,
    #[serde(default)]
    pub mode_height: u32,
    #[serde(default)]
    pub mode_refresh_mhz: u32,
    #[serde(default)]
    pub scale: f64,
    #[serde(default)]
    pub transform: OutputTransform,
    #[serde(default)]
    pub position_set: bool,
    #[serde(default)]
    pub position_x: i32,
    #[serde(default)]
    pub position_y: i32,
    #[serde(default)]
    pub vrr: VrrMode,
}

impl OutputConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            off: false,
            mode_width: 0,
            mode_height: 0,
            mode_refresh_mhz: 0,
            scale: 0.0,
            transform: OutputTransform::Normal,
            position_set: false,
            position_x: 0,
            position_y: 0,
            vrr: VrrMode::Default,
        }
    }
}

// ── Input enums ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum TrackLayout {
    #[default]
    Global,
    Window,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum AccelProfile {
    #[default]
    Default,
    Adaptive,
    Flat,
}

impl AccelProfile {
    pub fn as_kdl_str(&self) -> Option<&'static str> {
        match self {
            Self::Adaptive => Some("adaptive"),
            Self::Flat => Some("flat"),
            Self::Default => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum TouchpadScrollMethod {
    #[default]
    Default,
    TwoFinger,
    Edge,
    OnButtonDown,
    NoScroll,
}

impl TouchpadScrollMethod {
    pub fn as_kdl_str(&self) -> Option<&'static str> {
        match self {
            Self::TwoFinger => Some("two-finger"),
            Self::Edge => Some("edge"),
            Self::OnButtonDown => Some("on-button-down"),
            Self::NoScroll => Some("no-scroll"),
            Self::Default => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum MouseScrollMethod {
    #[default]
    Default,
    NoScroll,
    TwoFinger,
    Edge,
    OnButtonDown,
}

impl MouseScrollMethod {
    pub fn as_kdl_str(&self) -> Option<&'static str> {
        match self {
            Self::NoScroll => Some("no-scroll"),
            Self::TwoFinger => Some("two-finger"),
            Self::Edge => Some("edge"),
            Self::OnButtonDown => Some("on-button-down"),
            Self::Default => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum TapButtonMap {
    #[default]
    Default,
    LeftRightMiddle,
    LeftMiddleRight,
}

impl TapButtonMap {
    pub fn as_kdl_str(&self) -> Option<&'static str> {
        match self {
            Self::LeftRightMiddle => Some("left-right-middle"),
            Self::LeftMiddleRight => Some("left-middle-right"),
            Self::Default => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ClickMethod {
    #[default]
    Default,
    ButtonAreas,
    Clickfinger,
}

impl ClickMethod {
    pub fn as_kdl_str(&self) -> Option<&'static str> {
        match self {
            Self::ButtonAreas => Some("button-areas"),
            Self::Clickfinger => Some("clickfinger"),
            Self::Default => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum DragSetting {
    #[default]
    Default,
    Enabled,
    Disabled,
}

// ── Input settings ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputSettings {
    pub numlock: bool,
    #[serde(default)]
    pub keyboard_xkb_layout: String,
    #[serde(default)]
    pub keyboard_xkb_variant: String,
    #[serde(default)]
    pub keyboard_xkb_model: String,
    #[serde(default)]
    pub keyboard_xkb_rules: String,
    #[serde(default)]
    pub keyboard_xkb_options: String,
    #[serde(default)]
    pub keyboard_repeat_delay: u32,
    #[serde(default)]
    pub keyboard_repeat_rate: u32,
    #[serde(default)]
    pub keyboard_track_layout: TrackLayout,
    pub touchpad_tap: bool,
    pub touchpad_natural_scroll: bool,
    pub touchpad_dwt: bool,
    #[serde(default)]
    pub touchpad_off: bool,
    #[serde(default)]
    pub touchpad_dwtp: bool,
    #[serde(default)]
    pub touchpad_drag: DragSetting,
    #[serde(default)]
    pub touchpad_drag_lock: bool,
    #[serde(default)]
    pub touchpad_accel_speed: f64,
    #[serde(default)]
    pub touchpad_accel_profile: AccelProfile,
    #[serde(default)]
    pub touchpad_scroll_method: TouchpadScrollMethod,
    #[serde(default)]
    pub touchpad_tap_button_map: TapButtonMap,
    #[serde(default)]
    pub touchpad_click_method: ClickMethod,
    #[serde(default)]
    pub touchpad_disabled_on_external_mouse: bool,
    #[serde(default)]
    pub touchpad_left_handed: bool,
    #[serde(default)]
    pub touchpad_middle_emulation: bool,
    pub mouse_natural_scroll: bool,
    #[serde(default)]
    pub mouse_off: bool,
    #[serde(default)]
    pub mouse_accel_speed: f64,
    #[serde(default)]
    pub mouse_accel_profile: AccelProfile,
    #[serde(default)]
    pub mouse_scroll_method: MouseScrollMethod,
    #[serde(default)]
    pub mouse_left_handed: bool,
    #[serde(default)]
    pub mouse_middle_emulation: bool,
    pub focus_follows_mouse: bool,
    pub workspace_auto_back_and_forth: bool,
    pub warp_mouse_to_focus: bool,
    pub disable_power_key_handling: bool,
    #[serde(default)]
    pub mod_key: String,
    #[serde(default)]
    pub mod_key_nested: String,
}

// ── Layout settings ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSettings {
    pub gaps: f64,
    pub center_focused_column: CenterFocusedColumn,
    pub always_center_single_column: bool,
    pub shadow_on: bool,
    pub border_on: bool,
    pub focus_ring_active_color: String,
    pub focus_ring_inactive_color: String,
}

impl Default for LayoutSettings {
    fn default() -> Self {
        Self {
            gaps: 16.0,
            center_focused_column: CenterFocusedColumn::Never,
            always_center_single_column: false,
            shadow_on: false,
            border_on: false,
            focus_ring_active_color: "#7fc8ff".to_string(),
            focus_ring_inactive_color: "#505050".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum CenterFocusedColumn {
    #[default]
    Never,
    Always,
    OnOverflow,
}

impl CenterFocusedColumn {
    pub fn as_kdl_str(&self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Always => "always",
            Self::OnOverflow => "on-overflow",
        }
    }
}

// ── Misc settings ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MiscSettings {
    pub prefer_no_csd: bool,
    pub skip_hotkey_overlay: bool,
    #[serde(default)]
    pub hotkey_overlay_hide_not_bound: bool,
    #[serde(default)]
    pub screenshot_path: String,
    #[serde(default)]
    pub cursor_theme: String,
    #[serde(default)]
    pub cursor_size: u32,
    #[serde(default)]
    pub cursor_hide_when_typing: bool,
    #[serde(default)]
    pub cursor_hide_after_inactive_ms: u32,
    #[serde(default)]
    pub clipboard_disable_primary: bool,
    #[serde(default)]
    pub overview_zoom: f64,
    #[serde(default)]
    pub overview_backdrop_color: String,
    #[serde(default)]
    pub xwayland_off: bool,
    #[serde(default)]
    pub xwayland_path: String,
    #[serde(default)]
    pub config_notification_disable_failed: bool,
}

// ── Animation settings ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnimSettings {
    pub global_off: bool,
    #[serde(default)]
    pub slowdown: f64,
    #[serde(default)]
    pub workspace_switch_off: bool,
    #[serde(default)]
    pub window_open_off: bool,
    #[serde(default)]
    pub window_close_off: bool,
    #[serde(default)]
    pub horizontal_view_movement_off: bool,
    #[serde(default)]
    pub window_movement_off: bool,
    #[serde(default)]
    pub window_resize_off: bool,
    #[serde(default)]
    pub config_notification_open_close_off: bool,
    #[serde(default)]
    pub exit_confirmation_open_close_off: bool,
    #[serde(default)]
    pub screenshot_ui_open_off: bool,
    #[serde(default)]
    pub overview_open_close_off: bool,
    #[serde(default)]
    pub recent_windows_close_off: bool,
}

// ── Gesture settings ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GestureSettings {
    #[serde(default)]
    pub dnd_view_trigger_width: u32,
    #[serde(default)]
    pub dnd_view_delay_ms: u32,
    #[serde(default)]
    pub dnd_view_max_speed: u32,
    #[serde(default)]
    pub dnd_ws_trigger_height: u32,
    #[serde(default)]
    pub dnd_ws_delay_ms: u32,
    #[serde(default)]
    pub dnd_ws_max_speed: u32,
    #[serde(default)]
    pub hot_corners_off: bool,
    #[serde(default)]
    pub hot_corners_top_left: bool,
    #[serde(default)]
    pub hot_corners_top_right: bool,
    #[serde(default)]
    pub hot_corners_bottom_left: bool,
    #[serde(default)]
    pub hot_corners_bottom_right: bool,
}

// ── Named workspaces ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NamedWorkspace {
    pub name: String,
    #[serde(default)]
    pub open_on_output: String,
}

// ── Recent windows ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentWindowsSettings {
    pub off: bool,
    #[serde(default)]
    pub debounce_ms: u32,
    #[serde(default)]
    pub open_delay_ms: u32,
    #[serde(default)]
    pub highlight_active_color: String,
    #[serde(default)]
    pub highlight_urgent_color: String,
    #[serde(default)]
    pub highlight_padding: u32,
    #[serde(default)]
    pub highlight_corner_radius: u32,
    #[serde(default)]
    pub previews_max_height: u32,
    #[serde(default)]
    pub previews_max_scale: f64,
}

// ── Debug options ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebugSettings {
    #[serde(default)]
    pub enable_overlay_planes: bool,
    #[serde(default)]
    pub disable_cursor_plane: bool,
    #[serde(default)]
    pub disable_direct_scanout: bool,
    #[serde(default)]
    pub restrict_primary_scanout_to_matching_format: bool,
    #[serde(default)]
    pub force_disable_connectors_on_resume: bool,
    #[serde(default)]
    pub render_drm_device: String,
    #[serde(default)]
    pub force_pipewire_invalid_modifier: bool,
    #[serde(default)]
    pub dbus_interfaces_in_non_session_instances: bool,
    #[serde(default)]
    pub wait_for_frame_completion_before_queueing: bool,
    #[serde(default)]
    pub emulate_zero_presentation_time: bool,
    #[serde(default)]
    pub disable_resize_throttling: bool,
    #[serde(default)]
    pub disable_transactions: bool,
    #[serde(default)]
    pub keep_laptop_panel_on_when_lid_is_closed: bool,
    #[serde(default)]
    pub disable_monitor_names: bool,
    #[serde(default)]
    pub strict_new_window_focus_policy: bool,
    #[serde(default)]
    pub honor_xdg_activation_with_invalid_serial: bool,
    #[serde(default)]
    pub skip_cursor_only_updates_during_vrr: bool,
    #[serde(default)]
    pub deactivate_unfocused_windows: bool,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_row_ids_assigns_unique_nonzero_ids() {
        let mut cfg = SettingsConfig {
            window_rules: vec![
                WindowRule::default(),
                WindowRule::default(),
                WindowRule::default(),
            ],
            ..Default::default()
        };
        // Simulate a freshly deserialized config: ids start at 0.
        for r in &mut cfg.window_rules {
            r.id = 0;
        }
        cfg.ensure_row_ids();
        let ids: Vec<u64> = cfg.window_rules.iter().map(|r| r.id).collect();
        assert!(ids.iter().all(|&id| id != 0), "no id should remain zero");
        assert_eq!(
            ids.iter().collect::<std::collections::HashSet<_>>().len(),
            3,
            "ids must be unique"
        );
    }

    #[test]
    fn ensure_row_ids_is_idempotent() {
        let mut cfg = SettingsConfig {
            binds: vec![Keybind::default(), Keybind::default()],
            ..Default::default()
        };
        let before: Vec<u64> = cfg.binds.iter().map(|b| b.id).collect();
        cfg.ensure_row_ids();
        let after: Vec<u64> = cfg.binds.iter().map(|b| b.id).collect();
        assert_eq!(before, after, "existing ids must not be reassigned");
    }

    /// The core A1 regression: after removing a middle row by id, every
    /// surviving row's id still resolves to the *correct* element — which is
    /// exactly what the UI relies on instead of captured indices.
    #[test]
    fn removing_middle_row_keeps_id_resolution_correct() {
        let mut cfg = SettingsConfig::default();
        for app in ["alpha", "bravo", "charlie", "delta"] {
            cfg.window_rules.push(WindowRule {
                id: next_row_id(),
                match_app_id: app.to_string(),
                ..Default::default()
            });
        }
        let ids: Vec<u64> = cfg.window_rules.iter().map(|r| r.id).collect();

        // Remove the middle "bravo" by id.
        cfg.remove_window_rule(ids[1]);
        assert_eq!(cfg.window_rules.len(), 3);

        // The removed id no longer resolves; the others still map to *their*
        // original rule despite the index shift.
        assert!(cfg.window_rule_mut(ids[1]).is_none());
        assert_eq!(cfg.window_rule_mut(ids[0]).unwrap().match_app_id, "alpha");
        assert_eq!(cfg.window_rule_mut(ids[2]).unwrap().match_app_id, "charlie");
        assert_eq!(cfg.window_rule_mut(ids[3]).unwrap().match_app_id, "delta");

        // Mutating through a stale id is a no-op, never a panic / wrong write.
        if let Some(r) = cfg.window_rule_mut(ids[1]) {
            r.match_app_id = "should not happen".into();
        }
        assert!(cfg
            .window_rules
            .iter()
            .all(|r| r.match_app_id != "should not happen"));
    }

    #[test]
    fn remove_helpers_are_noops_for_unknown_ids() {
        let mut cfg = SettingsConfig::default();
        cfg.binds.push(Keybind::default());
        cfg.layer_rules.push(LayerRule {
            id: next_row_id(),
            ..Default::default()
        });
        let bind_count = cfg.binds.len();
        let layer_count = cfg.layer_rules.len();
        cfg.remove_bind(u64::MAX);
        cfg.remove_layer_rule(u64::MAX);
        assert_eq!(cfg.binds.len(), bind_count);
        assert_eq!(cfg.layer_rules.len(), layer_count);
    }
}
