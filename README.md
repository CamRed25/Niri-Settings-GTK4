# niri-settings

A GTK4 configuration editor for the [Niri](https://github.com/YaLTeR/niri) Wayland compositor. Changes remain in a draft until Apply validates and atomically writes the app-managed include.

## Pages

The primary navigation contains:

| Page | Purpose |
|---|---|
| Keybindings | GDK key capture, normalized combinations, conflict detection, actions, and recursive import |
| Window Rules | Compact match/layout table with expanded advanced properties |
| Outputs | IPC-detected monitor canvas with mode, scale, transform, VRR, and position controls |
| Theme | Validated Tokyo Night tokens and live shell preview |
| Behaviour | Dock, workspace, media, network-speed, launcher, and notification preferences |
| Software | Read-only integration availability and missing-tool guidance |

Advanced contains Input, Layout, Animations, Workspaces, Gestures, Recent Windows, Miscellaneous, Debug, Layer Rules, and Switch Events.

## Persistence and safety

- `~/.config/niri-shell/settings.json` is the authoritative store: it is loaded and rewritten in full on every Apply.
- `~/.config/niri-shell/settings.kdl` is the generated fragment niri consumes. On load, only the externally-editable nodes (`binds`, `output`, `window-rule`, `layer-rule`) are parsed back from it and override the JSON copy, so hand edits to those are honoured. The remaining managed sections (input, layout, animations, workspaces, gestures, recent-windows, debug, switch-events, miscellaneous, `prefer-no-csd`) round-trip through `settings.json` by design.
- `~/.config/niri-shell/shell.kdl` stores theme, behaviour, Do Not Disturb, and Night Light preferences.
- `NIRI_CONFIG` takes precedence over XDG and HOME defaults for the active compositor config.
- Apply updates managed nodes, validates a same-directory candidate, creates the first-edit backup, and atomically replaces accepted files.
- Failed validation leaves the active files untouched and presents the compositor diagnostics.
- Unknown top-level nodes survive managed rewrites. Comments within editor-owned blocks are regenerated; comments elsewhere remain untouched.
- External bindings stay visible with provenance. Editing one promotes it into the managed override.

When the `niri` executable is unavailable, parser-valid KDL can still be saved for offline development, but the UI explicitly reports that compositor validation did not run.

## Architecture

```text
src/
├── main.rs                 entry point and renderer setup
├── settings_backend/       pure Rust models, KDL parsing, validation, and persistence
├── settings_ui.rs          GTK application and navigation shell
├── settings_ui/*_ui.rs     GTK page implementations and widget helpers
├── ipc/                    typed Niri output queries
└── error.rs                application error types
```

Configuration parsing, recursive include traversal, conflict detection, IPC queries, validation, and disk writes remain outside the GTK page modules.

## Build and usage

The editor requires GTK4 and Niri for live output detection and compositor validation.

```sh
cargo build --release
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check

niri-settings
niri-settings --page outputs
```

Example Niri binding:

```kdl
binds {
    Mod+Comma { spawn "niri-settings"; }
}
```

Set `RUST_LOG=debug` for diagnostic logging.
