use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use tokio::runtime::Handle;

use crate::{
    m3::layout,
    opendeck::{config, profile},
};

use super::{app::SettingsApp, ipc::Command, thumbnails};

pub(crate) fn find_device_id() -> Option<String> {
    let profiles_dir = config::opendeck_config_dir().join("profiles");
    let entries = std::fs::read_dir(&profiles_dir).ok()?;
    let prefix = format!("{}-", layout::DEVICE_NAMESPACE);
    for entry in entries.flatten() {
        if entry.file_type().ok()?.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&prefix) {
                return Some(name);
            }
        }
    }
    None
}

pub(crate) fn load_app_state(rt: &Handle) -> (config::PluginConfig, Option<String>, String, Vec<String>) {
    let config = rt.block_on(config::load_or_default());
    let current_profile = rt.block_on(profile::current());
    let device_id = find_device_id().unwrap_or_else(|| {
        log::warn!("Could not find M3 device profile directory");
        format!("{}-unknown", layout::DEVICE_NAMESPACE)
    });
    let profile_names = rt.block_on(profile::list(&device_id));
    (config, current_profile, device_id, profile_names)
}

impl SettingsApp {
    pub(crate) fn new(rt: Handle, tx: Box<dyn FnMut(Command) + Send>) -> Self {
        let (config, current_profile, device_id, profile_names) = load_app_state(&rt);

        let mut app = Self {
            rt,
            tx,
            config,
            current_profile,
            // Long enough ago that the first `refresh()` call always runs, regardless of interval.
            last_refresh: Instant::now() - Duration::from_secs(3600),
            device_id,
            profile_names,
            thumbnails: HashMap::new(),
            pending_thumbnails: HashMap::new(),
            base_thumbnail: None,
            base_pending: None,
        };
        app.refresh();
        app
    }

    pub(crate) fn refresh(&mut self) {
        if self.last_refresh.elapsed() < self.config.settings_refresh_interval() {
            return;
        }
        self.last_refresh = Instant::now();

        self.refresh_config();
        self.refresh_profile_thumbnails();
        self.refresh_base_thumbnail();
    }

    fn refresh_config(&mut self) {
        self.config = self.rt.block_on(config::load_or_create()).unwrap_or_else(|e| {
            log::error!("Failed to load config: {e}");
            self.config.clone()
        });
        self.current_profile = self.rt.block_on(profile::current());
        self.profile_names = self.rt.block_on(profile::list(&self.device_id));
    }

    fn refresh_profile_thumbnails(&mut self) {
        thumbnails::refresh_map(self.profile_names.iter(), &self.config.profile_backgrounds, &mut self.thumbnails, &mut self.pending_thumbnails);

        self.thumbnails.retain(|p, _| self.profile_names.contains(p));
        self.pending_thumbnails.retain(|p, _| self.profile_names.contains(p));
    }

    fn refresh_base_thumbnail(&mut self) {
        thumbnails::refresh_slot(self.config.base_background.as_ref(), &mut self.base_thumbnail, &mut self.base_pending, "base");
    }

    pub(crate) fn notify(&mut self) {
        (self.tx)(Command::Refresh);
    }

    pub(crate) fn update_config(&mut self, mutate: impl FnOnce(&mut config::PluginConfig)) {
        mutate(&mut self.config);
        if let Err(e) = self.rt.block_on(config::save(&self.config)) {
            log::error!("Failed to save config: {e}");
        }
        self.notify();
    }

    pub(crate) fn set_base(&mut self, data_url: String) {
        self.update_config(|cfg| cfg.base_background = Some(data_url));
    }

    pub(crate) fn set_profile(&mut self, profile: String, data_url: String) {
        self.update_config(|cfg| {
            cfg.profile_backgrounds.insert(profile, data_url);
        });
    }

    pub(crate) fn clear_base(&mut self) {
        self.update_config(|cfg| cfg.base_background = None);
    }

    pub(crate) fn clear_profile(&mut self, profile: &str) {
        self.update_config(|cfg| {
            cfg.profile_backgrounds.remove(profile);
        });
    }

    pub(crate) fn pick_file(title: &str) -> Option<String> {
        let file = rfd::FileDialog::new().set_title(title).add_filter("Images", ["png", "jpg", "jpeg"].as_slice()).pick_file();
        file.map(|p| p.to_string_lossy().into_owned())
    }
}
