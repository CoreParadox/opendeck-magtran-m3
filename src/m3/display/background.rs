use std::sync::Arc;

use anyhow::Result;

use crate::{
    m3::device_transport::DeviceTransport,
    utils::image_util::{ImageFormat, ImageMirroring, ImageMode, ImageRotation, decode_image_data_url},
};

pub(crate) struct Background {
    transport: Arc<DeviceTransport>,
}

impl Background {
    pub(crate) fn new(transport: Arc<DeviceTransport>) -> Self {
        Self { transport }
    }

    pub(crate) async fn apply(&self, image: String) -> Result<()> {
        log::info!("apply_background: {} bytes", image.len());

        let decoded = match decode_image_data_url(&image) {
            Ok(img) => {
                log::info!("Background image decoded to {}x{}", img.width(), img.height());
                img
            }
            Err(err) => {
                log::error!("Failed to decode background image: {err}");
                return Ok(());
            }
        };

        self.transport.set_background_region(self.image_format(), &decoded, 0, 0, 0).await?;
        log::info!("Background image sent to device");
        Ok(())
    }

    pub(crate) async fn clear(&self) -> Result<()> {
        log::info!("clear_background");

        self.transport.clear_background_region(0).await?;
        log::info!("Background cleared");

        Ok(())
    }

    fn image_format(&self) -> ImageFormat {
        ImageFormat { mode: ImageMode::JPEG, size: (854, 480), rotation: ImageRotation::Rot270, mirror: ImageMirroring::None }
    }
}
