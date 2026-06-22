# niri-settings

A GTK4 settings GUI for the [niri](https://github.com/YaLTeR/niri) Wayland compositor, written in Rust.

Provides a graphical sidebar interface for editing niri's configuration without touching KDL by hand. Changes are staged and applied atomically after `niri validate` succeeds; Niri watches the managed include and reloads it automatically.

## Features

**Settings pages:**

| Page | What you can configure |
|---|---|
| **Behaviour** | Compositor toggles (focus follows mouse, resize with right-click, etc.) |
| **Input** | Keyboard (repeat rate/delay, XKB layout, track layout), touchpad (tap, natural scroll, accel, click method, drag), mouse (scroll method, accel, speed), modifier key remapping |
| **Layout** | Window gaps (inner/outer), border decoration, focus ring style |
| **Keybindings** | Keyboard recorder, conflict detection, actions/options, and recursive import from the active config and its includes |
| **Outputs** | Per-monitor mode, scale, rotation, VRR, position, enable/disable |
| **Switch Events** | Actions for lid-close and tablet-mode switch events |
| **Window Rules** | Match by app-id/title, control opening geometry, floating, fullscreen, decorations, opacity, size limits |
| **Layer Rules** | Per-layer-surface screencast/screen-capture visibility |
| **Animations** | Enable/disable and tune individual window animations |
| **Workspaces** | Define named workspaces |
| **Gestures** | Touchpad and touchscreen gesture settings |
| **Recent Windows** | Recent-focus behaviour |
| **Debug** | Niri debug options |
| **Theme** | Tokyo Night token editor and live shell preview |
| **Behaviour** | Dock, panel, launcher, and notification behaviour |
| **Software** | Installed/missing status for optional integrations |
| **Advanced** | Input, layout, animations, workspaces, gestures, recent windows, layer rules, switch events, debug, and miscellaneous compositor settings |

**On save:**
- `~/.config/niri-shell/settings.kdl` is the app-managed Niri fragment and canonical source for primary editor pages.
- `~/.config/niri-shell/shell.kdl` stores shell theme, behaviour, DND, and Night Light preferences.
- `settings.json` remains only as a compatibility cache and migration source for older builds.

The tool patches your niri config to add an `include` directive the first time it runs — you don't need to edit your config manually.

### Configuration round-trip & safety

- **Unknown nodes are preserved.** When rewriting `settings.kdl`, any top-level
  node the editor does not manage is carried over unchanged, so hand-written
  sections in the managed include survive a save.
- **Comments inside managed sections are not preserved.** The managed fragment is
  regenerated from the typed model on each save, so comments placed *within*
  editor-owned blocks are dropped. Keep comments in your own top-level nodes or
  in the main `config.kdl`, which the editor never rewrites.
- **Validation is honest.** A save is only reported as "applied" once
  `niri validate` accepts it. If the `niri` binary is not on `PATH` (e.g. an
  offline/dev session) the parser-valid KDL is still written, but the result is
  reported as *"saved — niri unavailable, not validated"* rather than applied.
- **Input is validated in place.** Numeric and regex fields flag malformed input
  with a red border instead of silently coercing to zero, and keybinding
  conflicts are highlighted on the offending row.

## Requirements

- GTK 4.12 or later
- [niri](https://github.com/YaLTeR/niri) compositor (for the generated config to take effect)
- Rust / Cargo (to build)

## Build

```sh
cargo build --release
```

The binary ends up at `target/release/niri-settings`.

## Usage

```sh
niri-settings
niri-settings --page outputs
```

Or add it as a keybind in your niri config:

```kdl
binds {
    Mod+Comma { spawn "niri-settings"; }
}
```

Set `RUST_LOG=debug` for verbose logging.

## Architecture

```
src/
├── main.rs              — entry point and renderer setup
├── settings_backend/    — pure-Rust models, KDL editing and persistence
├── settings_ui/         — GTK4 sidebar, primary pages and advanced pages
├── ipc/                 — Niri IPC output detection
└── error.rs             — typed application errors
```

The backend and UI modules are intentionally decoupled: configuration and validation remain testable without GTK.
