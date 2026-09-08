use anyhow::Result;
use image::DynamicImage;
use mirajazz::types::HidDeviceInfo;
use std::sync::Arc;
use tokio::sync::{
    Mutex as TokioMutex,
    mpsc::{UnboundedReceiver, unbounded_channel},
};
use tokio_util::sync::CancellationToken;

use crate::{
    m3::{inputs::DeviceStateUpdate, inputs::input_reader, layout::Kind},
    utils::image_util::{ImageFormat, encode_image, rotated_dimensions},
};

mod mirajazz_backend;

pub(crate) use mirajazz_backend::MirajazzBackend;

pub(crate) struct DeviceTransport {
    mirajazz: Arc<MirajazzBackend>,
    pub kind: Kind,
    pub events: Arc<TokioMutex<Option<UnboundedReceiver<DeviceStateUpdate>>>>,
}

impl DeviceTransport {
    pub(crate) async fn open(info: &HidDeviceInfo, kind: Kind, token: CancellationToken) -> Result<Self> {
        let mirajazz = Arc::new(MirajazzBackend::connect(info).await?);

        let (event_tx, event_rx) = unbounded_channel::<DeviceStateUpdate>();
        input_reader::spawn_poller(mirajazz.clone(), event_tx, token);

        Ok(Self { mirajazz, kind, events: Arc::new(TokioMutex::new(Some(event_rx))) })
    }

    pub(crate) async fn wake_screen(&self) -> Result<()> {
        self.mirajazz.heartbeat().await
    }

    pub(crate) async fn set_brightness(&self, percent: u8) -> Result<()> {
        self.mirajazz.set_brightness(percent).await
    }

    pub(crate) async fn set_key_image(&self, key: u8, image_format: ImageFormat, image: &DynamicImage) -> Result<()> {
        self.mirajazz.set_key_image(key, image_format, image).await
    }

    pub(crate) async fn clear_key_image(&self, key: u8) -> Result<()> {
        self.mirajazz.clear_key_image(key).await
    }

    pub(crate) async fn flush(&self) -> Result<()> {
        self.mirajazz.flush().await
    }

    pub(crate) async fn set_background_region(&self, image_format: ImageFormat, image: &DynamicImage, x: u16, y: u16, fb_layer: u8) -> Result<()> {
        let data = encode_image(image_format, image).await?;
        let (width, height) = rotated_dimensions(image_format.size, image_format.rotation);
        self.mirajazz.set_background_region(&data, x, y, width, height, fb_layer).await
    }

    pub(crate) async fn clear_background_region(&self, fb_layer: u8) -> Result<()> {
        self.mirajazz.clear_background_region(fb_layer).await
    }

    pub(crate) async fn heartbeat(&self) -> Result<()> {
        self.mirajazz.heartbeat().await
    }

    pub(crate) async fn shutdown(&self) -> Result<()> {
        self.mirajazz.shutdown().await
    }

    pub(crate) fn firmware_version(&self) -> Option<String> {
        self.mirajazz.firmware_version().map(str::to_string)
    }
}
