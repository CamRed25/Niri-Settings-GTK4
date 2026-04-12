# niri-settings

GTK4 settings GUI for the [niri](https://github.com/YaLTeR/niri) Wayland compositor.

Provides a graphical interface for editing niri's configuration. On save, writes:
- `~/.config/niri-shell/settings.json` — GUI state
- `~/.config/niri-shell/settings.kdl` — generated KDL fragment included by niri

## Build

```sh
cargo build --release
```

## Usage

```sh
niri-settings
```

## Architecture

- `settings_backend.rs` — Pure Rust config model, KDL codegen, serde persistence. Zero GTK.
- `settings_ui.rs` — GTK4 settings window (sidebar + page stack).
