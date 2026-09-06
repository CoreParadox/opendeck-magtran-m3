mod m3;
mod opendeck;
mod state;
mod utils;

use anyhow::Result;
use futures_lite::FutureExt;
use log::info;

use openaction::global_events::{
    GlobalEventHandler, SetBrightnessEvent, SetImageEvent, ShowSettingsInterfaceEvent, set_global_event_handler,
};

struct GlobalHandler;

static GLOBAL_HANDLER: GlobalHandler = GlobalHandler;

#[openaction::async_trait]
impl GlobalEventHandler for GlobalHandler {
    async fn plugin_ready(&self) -> openaction::OpenActionResult<()> {
        if let Err(e) = opendeck::config::init().await {
            log::error!("Failed to load config: {e}");
        }

        start_lifecycle().await;

        info!("Plugin initialized");
        Ok(())
    }

    async fn show_settings_interface(&self, _event: ShowSettingsInterfaceEvent) -> openaction::OpenActionResult<()> {
        opendeck::settings::show();
        Ok(())
    }

    async fn device_plugin_set_image(&self, event: SetImageEvent) -> openaction::OpenActionResult<()> {
        opendeck::handle_set_image(event).await;
        Ok(())
    }

    async fn device_plugin_set_brightness(&self, event: SetBrightnessEvent) -> openaction::OpenActionResult<()> {
        opendeck::handle_set_brightness(event).await;
        Ok(())
    }
}

fn init_logging() {
    let log_path = opendeck::config::plugin_dir().join("opendeck-m3.log");
    let log_file: Box<dyn std::io::Write + Send + 'static> =
        Box::new(std::io::LineWriter::new(std::fs::OpenOptions::new().create(true).append(true).open(log_path).unwrap()));

    let cfg = simplelog::ConfigBuilder::new().add_filter_allow_str("opendeck_m3").add_filter_allow_str("mirajazz").build();

    simplelog::CombinedLogger::init(vec![
        simplelog::TermLogger::new(simplelog::LevelFilter::Info, cfg.clone(), simplelog::TerminalMode::Stdout, simplelog::ColorChoice::Never),
        simplelog::WriteLogger::new(simplelog::LevelFilter::Info, cfg, log_file),
    ])
    .unwrap();
}

fn main() -> Result<()> {
    if std::env::var("OPENDECK_M3_SETTINGS_SOCKET").is_ok() {
        init_logging();
        return opendeck::settings::run();
    }

    init_logging();

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    rt.block_on(async {
        let result = run_plugin().or(wait_for_sigterm()).await;

        if let Err(err) = result {
            log::error!("M3 plugin terminated with error: {err:#}");
        }
        crate::state::shutdown().await;
    });
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
async fn wait_for_sigterm() -> Result<()> {
    let mut sig = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    sig.recv().await;
    Ok(())
}

#[cfg(target_os = "windows")]
async fn wait_for_sigterm() -> Result<()> {
    std::future::pending::<()>().await;
    Ok(())
}

async fn run_plugin() -> Result<()> {
    set_global_event_handler(&GLOBAL_HANDLER);
    openaction::run(std::env::args().collect()).await.map_err(anyhow::Error::from)
}

async fn start_lifecycle() {
    let token = tokio_util::sync::CancellationToken::new();
    let tracker = crate::state::TRACKER.lock().await.clone();
    let lifecycle_token = token.clone();
    tracker.spawn(async move {
        if let Err(e) = m3::device_lifecycle::run(lifecycle_token).await {
            log::error!("Device lifecycle manager exited with error: {e:#}");
        }
    });
    crate::state::TOKENS.write().await.insert("_lifecycle".to_string(), token);
}
