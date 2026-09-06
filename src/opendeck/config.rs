use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;

// Fallback values for settings.json
fn default_keepalive_interval_secs() -> u64 {
    10
}
fn default_brightness() -> u8 {
    100
}
fn default_profile_retry_window_ms() -> u64 {
    500
}
fn default_profile_retry_interval_ms() -> u64 {
    50
}
fn default_settings_refresh_interval_ms() -> u64 {
    500
}

/// Plugin-wide settings and backgrounds
/// TODO: This should probably be per device in the future
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PluginConfig {
    pub base_background: Option<String>,
    #[serde(default)]
    pub profile_backgrounds: HashMap<String, String>,

    /// How often to send a keepalive pulse to a connected device.
    #[serde(default = "default_keepalive_interval_secs")]
    pub keepalive_interval_secs: u64,
    /// Brightness applied when a device first connects.
    #[serde(default = "default_brightness")]
    pub default_brightness: u8,
    /// Total time to wait for OpenDeck's profile-selection file to update after a
    /// profile switch is signalled.
    #[serde(default = "default_profile_retry_window_ms")]
    pub profile_retry_window_ms: u64,
    /// How often to poll while waiting for the profile-selection file to update.
    #[serde(default = "default_profile_retry_interval_ms")]
    pub profile_retry_interval_ms: u64,
    /// How often the settings window polls for config/profile changes.
    #[serde(default = "default_settings_refresh_interval_ms")]
    pub settings_refresh_interval_ms: u64,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            base_background: None,
            profile_backgrounds: HashMap::new(),
            keepalive_interval_secs: default_keepalive_interval_secs(),
            default_brightness: default_brightness(),
            profile_retry_window_ms: default_profile_retry_window_ms(),
            profile_retry_interval_ms: default_profile_retry_interval_ms(),
            settings_refresh_interval_ms: default_settings_refresh_interval_ms(),
        }
    }
}

impl PluginConfig {
    pub(crate) fn keepalive_interval(&self) -> Duration {
        Duration::from_secs(self.keepalive_interval_secs)
    }

    pub(crate) fn profile_retry_window(&self) -> Duration {
        Duration::from_millis(self.profile_retry_window_ms)
    }

    pub(crate) fn profile_retry_interval(&self) -> Duration {
        Duration::from_millis(self.profile_retry_interval_ms)
    }

    pub(crate) fn settings_refresh_interval(&self) -> Duration {
        Duration::from_millis(self.settings_refresh_interval_ms)
    }
}

pub(crate) fn opendeck_config_dir() -> PathBuf {
    dirs::config_dir().expect("user config directory").join("opendeck")
}

pub(crate) fn plugin_dir() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir()
        && cwd.join("manifest.json").exists()
    {
        return cwd;
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
        && dir.join("manifest.json").exists()
    {
        return dir.to_path_buf();
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub(crate) fn config_path() -> PathBuf {
    plugin_dir().join("settings.json")
}

pub(crate) async fn init() -> Result<()> {
    let _ = load_or_create().await?;
    Ok(())
}

pub(crate) async fn load_or_create() -> Result<PluginConfig> {
    let path = config_path();
    if path.exists() {
        let text = fs::read_to_string(&path).await.context("read settings.json")?;
        Ok(serde_json::from_str(&text).context("parse settings.json")?)
    } else {
        let config = PluginConfig::default();
        save(&config).await?;
        Ok(config)
    }
}

/// Similar to `load_or_create` but instead just returns the default and logs an error on fail.
/// Ideally this should only be used in places where loading the config is not critical AND you can't
/// meaningfully propagate an error. (Or you're me and feeling lazy)
pub(crate) async fn load_or_default() -> PluginConfig {
    match load_or_create().await {
        Ok(config) => config,
        Err(e) => {
            log::error!("Failed to load config: {e}");
            PluginConfig::default()
        }
    }
}

pub(crate) async fn save(config: &PluginConfig) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.context("create settings dir")?;
    }
    let text = serde_json::to_string_pretty(config).context("serialize settings")?;
    fs::write(&path, text).await.context("write settings.json")?;
    Ok(())
}
