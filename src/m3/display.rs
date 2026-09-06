use std::sync::Arc;
use anyhow::Result;
use crate::opendeck::SetImageEvent;
use crate::m3::device_transport::DeviceTransport;

mod background;
mod keys;

pub(crate) use background::Background;
pub(crate) use keys::Keys;

pub(crate) struct Display {
    background: Background,
    keys: Keys,
}

impl Display {
    pub(crate) fn new(transport: Arc<DeviceTransport>) -> Self {
        let keys = Keys::new(transport.clone());
        let background = Background::new(transport);
        Self { background, keys }
    }

    pub(crate) async fn apply_image_event(&self, evt: SetImageEvent) -> Result<()> {
        if let Some(controller) = &evt.controller
            && controller != "Keypad"
        {
            log::debug!("Skipping setImage for non-keypad controller: {}", controller);
            return Ok(());
        }

        if evt.position.is_none()
            && let Some(image) = evt.image
        {
            self.background.apply(image).await?;
            return self.keys.refresh().await;
        }

        if let Some(position) = evt.position {
            match evt.image.as_deref() {
                Some(image) if !image.is_empty() => self.keys.set(position, image).await?,
                _ => self.keys.clear(position).await?,
            }
        } else {
            self.keys.clear_all().await?;
        }

        Ok(())
    }

    pub(crate) async fn clear_all_keys(&self) -> Result<()> {
        self.keys.clear_all().await
    }

    pub(crate) async fn apply_background(&self, image: String) -> Result<()> {
        self.background.apply(image).await?;
        self.keys.refresh().await
    }

    pub(crate) async fn clear_background(&self) -> Result<()> {
        self.background.clear().await?;
        self.keys.refresh().await
    }
}
