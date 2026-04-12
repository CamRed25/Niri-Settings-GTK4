mod error;
mod ipc;
mod settings_backend;
mod settings_ui;

fn main() {
    env_logger::init();
    if let Err(e) = settings_ui::run() {
        log::error!("niri-settings: {e}");
        std::process::exit(1);
    }
}
