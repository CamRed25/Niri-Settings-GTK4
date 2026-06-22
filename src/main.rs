mod error;
mod ipc;
mod settings_backend;
mod settings_ui;

fn main() {
    env_logger::init();

    // SAFETY: called before any threads are spawned.
    // Prefer the GL renderer — the Vulkan backend emits VK_ERROR_OUT_OF_DATE_KHR
    // on every resize and has no benefit for a simple settings dialog.
    if std::env::var_os("GSK_RENDERER").is_none() {
        // SAFETY: single-threaded at this point, no other threads exist yet.
        unsafe { std::env::set_var("GSK_RENDERER", "gl") };
    }

    if let Err(e) = settings_ui::run() {
        log::error!("niri-settings: {e}");
        std::process::exit(1);
    }
}
