# niri-settings

A GTK4 settings GUI for the [niri](https://github.com/YaLTeR/niri) Wayland compositor, written in Rust.

Provides a graphical sidebar interface for editing niri's configuration without touching KDL by hand. Changes are saved immediately on close and take effect on the next niri reload.

## Features

**Settings pages:**

| Page | What you can configure |
|---|---|
| **Behaviour** | Compositor toggles (focus follows mouse, resize with right-click, etc.) |
| **Input** | Keyboard (repeat rate/delay, XKB layout, track layout), touchpad (tap, natural scroll, accel, click method, drag), mouse (scroll method, accel, speed), modifier key remapping |
| **Layout** | Window gaps (inner/outer), border decoration, focus ring style |
| **Keybindings** | Full keybind editor — add, remove, set key combo + action + args, repeat settings, allow-when-locked; imports existing binds from `~/.config/niri/config.kdl` |
| **Outputs** | Per-monitor mode, scale, rotation, VRR, position, enable/disable |
| **Switch Events** | Actions for lid-close and tablet-mode switch events |
| **Window Rules** | Match by app-id/title, control opening geometry, floating, fullscreen, decorations, opacity, size limits |
| **Layer Rules** | Per-layer-surface screencast/screen-capture visibility |
| **Animations** | Enable/disable and tune individual window animations |
| **Workspaces** | Define named workspaces |
| **Gestures** | Touchpad and touchscreen gesture settings |
| **Recent Windows** | Recent-focus behaviour |
| **Debug** | Niri debug options |
| **Misc** | Miscellaneous compositor settings |

**On save, writes two files:**
- `~/.config/niri-shell/settings.json` — GUI state (persisted across launches)
- `~/.config/niri-shell/settings.kdl` — generated KDL fragment, auto-included in `~/.config/niri/config.kdl`

The tool patches your niri config to add an `include` directive the first time it runs — you don't need to edit your config manually.

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
├── main.rs              — entry point, env_logger init
├── settings_backend.rs  — pure-Rust config model, KDL codegen, serde persistence (zero GTK)
├── settings_ui.rs       — GTK4 window: sidebar navigation + page stack
├── ipc/                 — niri IPC helpers (output detection, config import)
└── error.rs             — shared error type
```

`settings_backend.rs` and `settings_ui.rs` are intentionally decoupled: the backend has no GTK dependency and can be tested or reused independently.
