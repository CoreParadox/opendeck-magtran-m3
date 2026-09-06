use std::{
    collections::HashMap,
    time::Instant,
};

use eframe::egui;
use tokio::runtime::Handle;

use crate::opendeck::config;

use super::{
    ipc::Command,
    thumbnails::{self, LoadedThumbnail, PendingThumbnail},
};

pub(crate) struct SettingsApp {
    pub(crate) rt: Handle,
    pub(crate) tx: Box<dyn FnMut(Command) + Send>,
    pub(crate) config: config::PluginConfig,
    pub(crate) current_profile: Option<String>,
    pub(crate) last_refresh: Instant,
    pub(crate) device_id: String,
    pub(crate) profile_names: Vec<String>,
    pub(crate) thumbnails: HashMap<String, LoadedThumbnail>,
    pub(crate) pending_thumbnails: HashMap<String, PendingThumbnail>,
    pub(crate) base_thumbnail: Option<LoadedThumbnail>,
    pub(crate) base_pending: Option<PendingThumbnail>,
}

pub(crate) fn run(rt: Handle, tx: Box<dyn FnMut(Command) + Send>) -> anyhow::Result<()> {
    let app = SettingsApp::new(rt, tx);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("M3 Settings").with_inner_size([720.0, 540.0]).with_visible(true),
        ..Default::default()
    };

    eframe::run_native("M3 Settings", options, Box::new(|_cc| Ok(Box::new(app)))).map_err(|e| anyhow::anyhow!("eframe error: {e}"))
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
