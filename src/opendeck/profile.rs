use std::{collections::HashMap, path::PathBuf, sync::Arc, sync::LazyLock};

use tokio::sync::{Mutex, RwLock};

use crate::{core::state::DEVICES, m3::device::Device, opendeck::config};

static CURRENT: LazyLock<RwLock<HashMap<String, String>>> = LazyLock::new(|| RwLock::new(HashMap::new()));
static LAST: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

fn profile_path() -> PathBuf {
    config::plugin_dir().join("active_profile")
}

pub(crate) fn profile_store_dir(device_id: &str) -> PathBuf {
    config::opendeck_config_dir().join("profiles").join(device_id)
}

pub(crate) async fn list(device_id: &str) -> Vec<String> {
    let dir = profile_store_dir(device_id);
    let mut names = Vec::new();
    if let Ok(mut entries) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(name) = entry.file_name().into_string()
                && let Some(stem) = name.strip_suffix(".json")
            {
                names.push(stem.to_string());
            }
        }
    }
    names.sort_by_key(|a| a.to_lowercase());
    names
}

pub(crate) async fn set(device: &str, profile: &str) -> Option<String> {
    let old = CURRENT.write().await.insert(device.to_string(), profile.to_string());
    *LAST.lock().await = Some(profile.to_string());
    let _ = tokio::fs::write(profile_path(), profile).await;
    old
}

pub(crate) async fn get(device: &str) -> Option<String> {
    CURRENT.read().await.get(device).cloned()
}

pub(crate) async fn current() -> Option<String> {
    if let Some(p) = LAST.lock().await.clone() {
        return Some(p);
    }
    tokio::fs::read_to_string(profile_path()).await.ok().map(|s| s.trim().to_string())
}

fn selected_profile_path(device_id: &str) -> PathBuf {
    config::opendeck_config_dir().join("profiles").join(format!("{device_id}.json"))
}

async fn read_selected_profile(device_id: &str) -> Option<String> {
    let text = tokio::fs::read_to_string(selected_profile_path(device_id)).await.ok()?;
    serde_json::from_str::<serde_json::Value>(&text).ok()?.get("selected_profile").and_then(|p| p.as_str()).map(String::from)
}

pub(crate) async fn sync_profile_background(device: &Device, device_id: &str) {
    if detect_profile_change(device, device_id).await {
        return;
    }
    apply(device, device_id).await;
}

async fn detect_profile_change(device: &Device, device_id: &str) -> bool {
    let Some(new_profile) = read_selected_profile(device_id).await else {
        return false;
    };
    let old_profile = get(device_id).await;
    if old_profile.as_deref() == Some(new_profile.as_str()) {
        return false;
    }

    log::info!("Profile changed for {device_id}: {old_profile:?} -> {new_profile}");
    set(device_id, &new_profile).await;
    apply(device, device_id).await;
    true
}

// Obviously file writes may lag behind opendeck profile change notif a tiny bit, so we retry for a short window
// so we can get the actual active profile to set the right background
pub(crate) fn watch_for_profile_change(device_id: String) {
    tokio::spawn(async move {
        let plugin_config = config::load_or_default().await;
        let retry_window = plugin_config.profile_retry_window();
        let deadline = tokio::time::Instant::now() + retry_window;
        while tokio::time::Instant::now() < deadline {
            let device = DEVICES.read().await.get(&device_id).cloned();
            if let Some(device) = device
                && detect_profile_change(&device, &device_id).await
            {
                return;
            }
            tokio::time::sleep(plugin_config.profile_retry_interval()).await;
        }
        log::debug!("No profile change detected for {device_id} within {retry_window:?}");
    });
}

pub(crate) async fn apply(device: &Device, device_id: &str) {
    let Some(profile) = get(device_id).await else {
        return;
    };

    let config = match config::load_or_create().await {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to load config: {e}");
            return;
        }
    };

    let background = config.profile_backgrounds.get(&profile).cloned().or(config.base_background);
    apply_background_choice(device, background).await;
}

async fn apply_background_choice(device: &Device, background: Option<String>) {
    if let Some(image) = background.filter(|b| !b.is_empty()) {
        if let Err(e) = device.apply_background(image).await {
            log::error!("Failed to apply profile background: {e}");
        }
    } else if let Err(e) = device.clear_background().await {
        log::error!("Failed to clear profile background: {e}");
    }
}

pub(crate) async fn apply_profile_background_to_all() {
    let devices: Vec<(String, Arc<Device>)> = DEVICES.read().await.iter().map(|(id, d)| (id.clone(), d.clone())).collect();
    for (id, device) in &devices {
        apply(device, id).await;
    }
}
