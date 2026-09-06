use std::{collections::HashMap, sync::Arc, sync::LazyLock, sync::Mutex};

use image::DynamicImage;

type KeyImageCache = HashMap<String, HashMap<u8, Arc<DynamicImage>>>;

static KEY_IMAGES: LazyLock<Mutex<KeyImageCache>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn get_key_images(serial: &str) -> HashMap<u8, Arc<DynamicImage>> {
    KEY_IMAGES.lock().unwrap().get(serial).cloned().unwrap_or_default()
}

pub(crate) fn set_key_image(serial: &str, position: u8, image: Arc<DynamicImage>) {
    let mut guard = KEY_IMAGES.lock().unwrap();
    let map = guard.entry(serial.to_string()).or_default();
    map.insert(position, image);
    log::info!("Key images cache now has {} entries", map.len());
}

pub(crate) fn remove_key_image(serial: &str, position: u8) {
    if let Some(map) = KEY_IMAGES.lock().unwrap().get_mut(serial) {
        map.remove(&position);
    }
}

pub(crate) fn clear_key_images(serial: &str) {
    KEY_IMAGES.lock().unwrap().remove(serial);
}
