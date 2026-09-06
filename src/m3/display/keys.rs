use std::{collections::HashMap, sync::{Arc, Mutex}, time::Duration};

use anyhow::Result;

use tokio::time::sleep;

use crate::{
    m3::{device_transport::DeviceTransport, layout::{self, KEY_COUNT}},
    utils::image_util::{ImageFormat, ImageMirroring, ImageMode, ImageRotation, decode_image_data_url},
};

pub(crate) struct Keys {
    transport: Arc<DeviceTransport>,
    images: Mutex<HashMap<u8, Arc<image::DynamicImage>>>,
}

impl Keys {
    pub(crate) fn new(transport: Arc<DeviceTransport>) -> Self {
        Self { transport, images: Mutex::new(HashMap::new()) }
    }

    pub(crate) async fn refresh(&self) -> Result<()> {
        let keys = {
            let guard = self.images.lock().expect("keys cache lock");
            guard.clone()
        };

        log::info!("Refreshing {} keys over background", keys.len());
        if keys.is_empty() {
            return Ok(());
        }

        for (position, img) in &keys {
            let hw_key = layout::opendeck_key_to_device(*position).expect("position fits in layout");
            log::info!("Refreshing key position {position} (hw {hw_key})");
            self.transport.set_key_image(hw_key, self.image_format(), img.as_ref()).await?;
            self.transport.flush().await?;
            sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    pub(crate) async fn set(&self, position: u8, image: &str) -> Result<()> {
        log::info!("Setting image for key {position}");

        let image = match decode_image_data_url(image) {
            Ok(img) => Arc::new(img),
            Err(err) => {
                log::error!("Failed to decode key image: {err}");
                return Ok(());
            }
        };

        {
            let mut guard = self.images.lock().expect("keys cache lock");
            guard.insert(position, image.clone());
        }

        let hw_key = layout::opendeck_key_to_device(position).expect("position fits in layout");
        self.transport.set_key_image(hw_key, self.image_format(), image.as_ref()).await?;
        self.transport.flush().await?;

        Ok(())
    }

    pub(crate) async fn clear(&self, position: u8) -> Result<()> {
        log::info!("Clearing image for key {position}");

        {
            let mut guard = self.images.lock().expect("keys cache lock");
            guard.remove(&position);
        }

        let hw_key = layout::opendeck_key_to_device(position).expect("position fits in layout");
        self.transport.clear_key_image(hw_key).await?;
        self.transport.flush().await?;

        Ok(())
    }

    pub(crate) async fn clear_all(&self) -> Result<()> {
        log::info!("Clearing all key images");

        {
            let mut guard = self.images.lock().expect("keys cache lock");
            guard.clear();
        }

        for position in 0..u8::try_from(KEY_COUNT).expect("KEY_COUNT fits in u8") {
            let hw_key = layout::opendeck_key_to_device(position).expect("position fits in layout");
            self.transport.clear_key_image(hw_key).await?;
        }
        self.transport.flush().await?;

        Ok(())
    }

    fn image_format(&self) -> ImageFormat {
        ImageFormat { mode: ImageMode::JPEG, size: (96, 96), rotation: ImageRotation::Rot90, mirror: ImageMirroring::Both }
    }
}
