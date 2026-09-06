use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use eframe::egui;
use tokio::runtime::Handle;

use crate::{
    m3::layout,
    opendeck::{config, profile},
};

use super::{
    ipc::Command,
    thumbnails::{self, LoadedThumbnail, PendingThumbnail},
};

pub(crate) struct SettingsApp {
    rt: Handle,
    tx: Box<dyn FnMut(Command) + Send>,
    config: config::PluginConfig,
    current_profile: Option<String>,
    last_refresh: Instant,
    device_id: String,
    profile_names: Vec<String>,
    thumbnails: HashMap<String, LoadedThumbnail>,
    pending_thumbnails: HashMap<String, PendingThumbnail>,
    base_thumbnail: Option<LoadedThumbnail>,
    base_pending: Option<PendingThumbnail>,
}

pub(crate) fn run(rt: Handle, tx: Box<dyn FnMut(Command) + Send>) -> anyhow::Result<()> {
    let app = SettingsApp::new(rt, tx);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("M3 Settings").with_inner_size([720.0, 540.0]).with_visible(true),
        ..Default::default()
    };

    eframe::run_native("M3 Settings", options, Box::new(|_cc| Ok(Box::new(app)))).map_err(|e| anyhow::anyhow!("eframe error: {e}"))
}

fn find_device_id() -> Option<String> {
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

fn load_app_state(rt: &Handle) -> (config::PluginConfig, Option<String>, String, Vec<String>) {
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
    fn new(rt: Handle, tx: Box<dyn FnMut(Command) + Send>) -> Self {
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

    fn refresh(&mut self) {
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

    fn notify(&mut self) {
        (self.tx)(Command::Refresh);
    }

    fn update_config(&mut self, mutate: impl FnOnce(&mut config::PluginConfig)) {
        mutate(&mut self.config);
        if let Err(e) = self.rt.block_on(config::save(&self.config)) {
            log::error!("Failed to save config: {e}");
        }
        self.notify();
    }

    fn set_base(&mut self, data_url: String) {
        self.update_config(|cfg| cfg.base_background = Some(data_url));
    }

    fn set_profile(&mut self, profile: String, data_url: String) {
        self.update_config(|cfg| {
            cfg.profile_backgrounds.insert(profile, data_url);
        });
    }

    fn clear_base(&mut self) {
        self.update_config(|cfg| cfg.base_background = None);
    }

    fn clear_profile(&mut self, profile: &str) {
        self.update_config(|cfg| {
            cfg.profile_backgrounds.remove(profile);
        });
    }

    fn pick_file(title: &str) -> Option<String> {
        let file = rfd::FileDialog::new().set_title(title).add_filter("Images", ["png", "jpg", "jpeg"].as_slice()).pick_file();
        file.map(|p| p.to_string_lossy().into_owned())
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.refresh();
        self.upload_pending_textures(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MagTran M3 Backgrounds");
            ui.separator();

            ui.label(format!("Current active profile: {}", self.current_profile.as_deref().unwrap_or("(none)")));

            self.show_base_background_panel(ui);
            self.show_profile_backgrounds_panel(ui);
            self.show_advanced_settings_panel(ui);
        });
    }
}

impl SettingsApp {
    fn upload_pending_textures(&mut self, ctx: &egui::Context) {
        for (profile, (url, color_image)) in self.pending_thumbnails.drain() {
            let texture = ctx.load_texture(profile.clone(), color_image, egui::TextureOptions::default());
            self.thumbnails.insert(profile, (url, texture));
        }

        if let Some((url, color_image)) = self.base_pending.take() {
            let texture = ctx.load_texture("base".to_string(), color_image, egui::TextureOptions::default());
            self.base_thumbnail = Some((url, texture));
        }
    }

    fn show_base_background_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label("Base background");
            ui.horizontal(|ui| {
                if let Some((_, texture)) = &self.base_thumbnail {
                    ui.add(egui::Image::new(texture).max_width(80.0).max_height(80.0).corner_radius(4.0));
                } else {
                    ui.add_sized(egui::vec2(80.0, 80.0), egui::Label::new(egui::RichText::new("no image").small().weak()));
                }

                ui.vertical(|ui| {
                    if let Some(url) = &self.config.base_background {
                        let size = url.split(',').next_back().map(|s| s.len().to_string()).unwrap_or_default();
                        ui.label(format!("<set>, {size} bytes"));
                    } else {
                        ui.label("(none)");
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Set base...").clicked()
                            && let Some(path) = Self::pick_file("Pick base background")
                            && let Ok(data_url) = thumbnails::encode_file_as_data_url(&path)
                        {
                            self.set_base(data_url);
                        }
                        if ui.button("Clear").clicked() {
                            self.clear_base();
                        }
                    });
                });
            });
        });
    }

    fn show_profile_backgrounds_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label("Profile backgrounds");

            egui::ScrollArea::vertical().show(ui, |ui| {
                for profile in self.profile_names.clone() {
                    let is_active = self.current_profile.as_ref() == Some(&profile);
                    let has_override = self.config.profile_backgrounds.contains_key(&profile);

                    ui.horizontal(|ui| {
                        if let Some((_, texture)) = self.thumbnails.get(&profile) {
                            ui.add(egui::Image::new(texture).max_width(80.0).max_height(80.0).corner_radius(4.0));
                        } else {
                            ui.add_sized(egui::vec2(80.0, 80.0), egui::Label::new(egui::RichText::new("no image").small().weak()));
                        }

                        ui.vertical(|ui| {
                            if is_active {
                                ui.label(egui::RichText::new(&profile).strong());
                                ui.label(egui::RichText::new("(active)").small().italics());
                            } else {
                                ui.label(&profile);
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if has_override && ui.button("Clear").clicked() {
                                self.clear_profile(&profile);
                            }
                            if ui.button("Set...").clicked()
                                && let Some(path) = Self::pick_file("Pick background for profile")
                                && let Ok(data_url) = thumbnails::encode_file_as_data_url(&path)
                            {
                                self.set_profile(profile.clone(), data_url);
                            }
                        });
                    });

                    ui.separator();
                }
            });
        });
    }

    fn show_advanced_settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label("Advanced");
            ui.label(egui::RichText::new("Keepalive interval and default brightness only apply the next time a device connects.").small().weak());

            let mut keepalive = self.config.keepalive_interval_secs;
            if ui.add(egui::Slider::new(&mut keepalive, 1..=60).text("Keepalive interval (s)")).changed() {
                self.update_config(|cfg| cfg.keepalive_interval_secs = keepalive);
            }

            let mut brightness = self.config.default_brightness;
            if ui.add(egui::Slider::new(&mut brightness, 0..=100).text("Default brightness (%)")).changed() {
                self.update_config(|cfg| cfg.default_brightness = brightness);
            }

            let mut retry_window = self.config.profile_retry_window_ms;
            if ui.add(egui::Slider::new(&mut retry_window, 100..=5000).text("Profile change retry window (ms)")).changed() {
                self.update_config(|cfg| cfg.profile_retry_window_ms = retry_window);
            }

            let mut retry_interval = self.config.profile_retry_interval_ms;
            if ui.add(egui::Slider::new(&mut retry_interval, 10..=1000).text("Profile change retry interval (ms)")).changed() {
                self.update_config(|cfg| cfg.profile_retry_interval_ms = retry_interval);
            }

            let mut refresh_interval = self.config.settings_refresh_interval_ms;
            if ui.add(egui::Slider::new(&mut refresh_interval, 100..=5000).text("Settings window refresh interval (ms)")).changed() {
                self.update_config(|cfg| cfg.settings_refresh_interval_ms = refresh_interval);
            }
        });
    }
}
