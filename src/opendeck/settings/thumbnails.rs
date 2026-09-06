use std::{collections::HashMap, path::Path};

use base64::Engine;
use eframe::egui;

use crate::utils::image_util::decode_image_data_url;

const THUMBNAIL_SIZE: u32 = 120;

/// A texture that has finished uploading to the GPU, tagged with the data URL it was
/// decoded from so we can tell whether it's stale.
pub(crate) type LoadedThumbnail = (String, egui::TextureHandle);

/// A decoded image waiting to be uploaded as a texture on the next frame, tagged with
/// the data URL it was decoded from.
pub(crate) type PendingThumbnail = (String, egui::ColorImage);

pub(crate) fn thumbnail_from_url(url: &str) -> anyhow::Result<egui::ColorImage> {
    let img = decode_image_data_url(url)?.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE);
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    Ok(egui::ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw()))
}

pub(crate) fn encode_file_as_data_url(path: &str) -> anyhow::Result<String> {
    let path = Path::new(path);
    let bytes = std::fs::read(path)?;
    let mime = match path.extension().and_then(|s| s.to_str()) {
        Some("png") => "image/png",
        _ => "image/jpeg",
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{b64}"))
}

/// Refreshes a single optional thumbnail slot (e.g. the base background). Skips
/// reloading if a thumbnail for `url` is already loaded or in flight, and clears the
/// slot entirely if `url` is `None`.
pub(crate) fn refresh_slot(url: Option<&String>, loaded: &mut Option<LoadedThumbnail>, pending: &mut Option<PendingThumbnail>, label: &str) {
    let Some(url) = url else {
        *loaded = None;
        *pending = None;
        return;
    };

    let already_loading = loaded.as_ref().map(|(u, _)| u) == Some(url) || pending.as_ref().map(|(u, _)| u) == Some(url);
    if already_loading {
        return;
    }

    *loaded = None;
    match thumbnail_from_url(url) {
        Ok(img) => *pending = Some((url.clone(), img)),
        Err(e) => log::error!("Failed to decode {label} thumbnail: {e}"),
    }
}

/// Refreshes a map of thumbnails keyed by name (e.g. per-profile backgrounds), one
/// entry per `keys`. Entries no longer present in `urls` are cleared; the caller is
/// responsible for pruning keys that have disappeared entirely (see `retain`).
pub(crate) fn refresh_map<'a>(
    keys: impl Iterator<Item = &'a String>, urls: &HashMap<String, String>, loaded: &mut HashMap<String, LoadedThumbnail>,
    pending: &mut HashMap<String, PendingThumbnail>,
) {
    for key in keys {
        match urls.get(key) {
            Some(url) => {
                let already_loading = loaded.get(key).map(|(u, _)| u) == Some(url) || pending.get(key).map(|(u, _)| u) == Some(url);
                if already_loading {
                    continue;
                }

                loaded.remove(key);
                match thumbnail_from_url(url) {
                    Ok(img) => {
                        pending.insert(key.clone(), (url.clone(), img));
                    }
                    Err(e) => log::error!("Failed to decode thumbnail for {key}: {e}"),
                }
            }
            None => {
                loaded.remove(key);
                pending.remove(key);
            }
        }
    }
}
