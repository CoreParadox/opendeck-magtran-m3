use std::{sync::Arc, time::Duration};

use anyhow::Result;

use tokio::time::sleep;

use crate::{
    m3::{device_transport::DeviceTransport, image_cache, layout::{COL_COUNT, KEY_COUNT, ROW_COUNT}},
    utils::image_util::{ImageFormat, ImageMirroring, ImageMode, ImageRotation, decode_image_data_url},
};

#[derive(Clone)]
pub(crate) struct Keys {
    transport: Arc<DeviceTransport>,
}

impl Keys {
    pub(crate) fn new(transport: Arc<DeviceTransport>) -> Self {
        Self { transport }
    }

    pub(crate) async fn refresh(&self) -> Result<()> {
        let keys = image_cache::get_key_images(&self.transport.serial_number);

        log::info!("Refreshing {} keys over background", keys.len());
        if keys.is_empty() {
            return Ok(());
        }

        for (position, img) in &keys {
            let hw_key = self.opendeck_to_device(*position);
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

        image_cache::set_key_image(&self.transport.serial_number, position, image.clone());

        let hw_key = self.opendeck_to_device(position);
        self.transport.set_key_image(hw_key, self.image_format(), image.as_ref()).await?;
        self.transport.flush().await?;

        Ok(())
    }

    pub(crate) async fn clear(&self, position: u8) -> Result<()> {
        log::info!("Clearing image for key {position}");

        image_cache::remove_key_image(&self.transport.serial_number, position);

        let hw_key = self.opendeck_to_device(position);
        self.transport.clear_key_image(hw_key).await?;
        self.transport.flush().await?;

        Ok(())
    }

    pub(crate) async fn clear_all(&self) -> Result<()> {
        log::info!("Clearing all key images");

        image_cache::clear_key_images(&self.transport.serial_number);

        // transport_clear_all_keys() makes empty actions solid black instead of leaving them transparent over, so the hack is to just clear each key individually.
        // again, could be a better way or some firmware trick that I don't know about
        for position in 0..u8::try_from(KEY_COUNT).expect("KEY_COUNT fits in u8") {
            let hw_key = self.opendeck_to_device(position);
            self.transport.clear_key_image(hw_key).await?;
        }
        self.transport.flush().await?;

        Ok(())
    }

    fn opendeck_to_device(&self, key: u8) -> u8 {
        let cols = u8::try_from(COL_COUNT).expect("COL_COUNT fits in u8");
        let rows = u8::try_from(ROW_COUNT).expect("ROW_COUNT fits in u8");
        let row = key / cols;
        let col = key % cols;
        (rows - 1 - row) * cols + col
    }

    fn image_format(&self) -> ImageFormat {
        ImageFormat { mode: ImageMode::JPEG, size: (96, 96), rotation: ImageRotation::Rot90, mirror: ImageMirroring::Both }
    }
}
