use std::sync::Arc;

use anyhow::Result;

use crate::{
    m3::device_transport::DeviceTransport,
    utils::image_util::{ImageFormat, ImageMirroring, ImageMode, ImageRotation, decode_image_data_url},
};

use super::keys::Keys;

pub(crate) struct Background {
    transport: Arc<DeviceTransport>,
    keys: Keys,
}

impl Background {
    pub(crate) fn new(transport: Arc<DeviceTransport>, keys: Keys) -> Self {
        Self { transport, keys }
    }

    /// Possibly some improvement to be made here... BUT for now: always draw the background and keys unconditionally since
    /// we can't know the state of the screen on connect e.g. it may have just powered on, or replugged.
    /// On the other hand, it's just genrally nice to treat the device the same in all cases when possible!
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

        self.keys.refresh().await?;
        log::info!("Background image sent to device");
        Ok(())
    }

    pub(crate) async fn clear(&self) -> Result<()> {
        log::info!("clear_background");

        self.transport.clear_background_region(0).await?;
        self.keys.refresh().await?;
        log::info!("Background cleared");

        Ok(())
    }

    fn image_format(&self) -> ImageFormat {
        ImageFormat { mode: ImageMode::JPEG, size: (854, 480), rotation: ImageRotation::Rot270, mirror: ImageMirroring::None }
    }
}
