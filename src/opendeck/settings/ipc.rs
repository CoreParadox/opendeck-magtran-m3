use std::{io::Write, net::TcpStream, process::Child, sync::LazyLock, sync::Mutex};

use tokio::io::AsyncBufReadExt;

use crate::opendeck::profile;

/// Commands sent from the settings window (child process) back to the plugin.
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub(crate) enum Command {
    Refresh,
}

static CHILD: LazyLock<Mutex<Option<Child>>> = LazyLock::new(|| Mutex::new(None));

/// Spawns the settings window as a child process, unless one is already running.
pub(crate) fn spawn_settings_window() -> anyhow::Result<()> {
    let mut child_guard = CHILD.lock().unwrap();
    if let Some(child) = child_guard.as_mut()
        && child.try_wait().is_ok_and(|s| s.is_none())
    {
        return Ok(());
    }
    *child_guard = None;

    let (listener, addr) = bind_settings_socket()?;

    let exe = std::env::current_exe()?;
    let child = std::process::Command::new(&exe)
        .env("OPENDECK_M3_SETTINGS_ADDR", &addr)
        .spawn()?;
    *child_guard = Some(child);

    start_settings_listener(listener, addr);

    Ok(())
}

fn bind_settings_socket() -> anyhow::Result<(tokio::net::TcpListener, String)> {
    let std_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let addr = std_listener.local_addr()?;
    std_listener.set_nonblocking(true)?;
    let listener = tokio::net::TcpListener::from_std(std_listener)?;

    Ok((listener, addr.to_string()))
}

fn start_settings_listener(listener: tokio::net::TcpListener, _addr: String) {
    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            let mut reader = tokio::io::BufReader::new(stream);
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line).await {
                if n == 0 {
                    break;
                }
                if let Ok(cmd) = serde_json::from_str::<Command>(line.trim())
                    && let Err(e) = apply_command(cmd).await
                {
                    log::error!("Failed to apply settings command: {e}");
                }
                line.clear();
            }
        }
    });
}

async fn apply_command(cmd: Command) -> anyhow::Result<()> {
    match cmd {
        Command::Refresh => profile::apply_profile_background_to_all().await,
    }
    Ok(())
}

/// Connects to the plugin's IPC socket. Called from the settings window child process.
pub(crate) fn connect() -> anyhow::Result<TcpStream> {
    let addr = std::env::var("OPENDECK_M3_SETTINGS_ADDR")?;
    let stream = TcpStream::connect(addr)?;
    stream.set_nonblocking(false).ok();
    Ok(stream)
}

pub(crate) fn send_command(stream: &mut TcpStream, cmd: Command) {
    if let Ok(json) = serde_json::to_string(&cmd) {
        let _ = writeln!(stream, "{json}");
        let _ = stream.flush();
    }
}
