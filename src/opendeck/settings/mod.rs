use crate::opendeck::config;

mod app;
mod ipc;
mod thumbnails;

/// Opens the settings window, spawning it as a child process if one isn't already running.
pub(crate) fn show() {
    log::info!("Opening settings window");
    if let Err(e) = ipc::spawn_settings_window() {
        log::error!("Failed to spawn settings window: {e}");
    }
}

/// Entry point for the settings window child process. Must be called directly from
/// `main()` with no thread spawned in between: winit/eframe's AppKit (macOS) backend
/// requires the event loop to run on the process's actual main thread.
pub(crate) fn run() -> anyhow::Result<()> {
    let mut stream = ipc::connect()?;

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    rt.block_on(config::init())?;
    let handle = rt.handle().clone();

    app::run(handle, Box::new(move |cmd| ipc::send_command(&mut stream, cmd)))
}
