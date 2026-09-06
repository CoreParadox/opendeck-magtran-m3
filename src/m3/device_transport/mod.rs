use async_hid::DeviceId;
use image::DynamicImage;
use mirajazz::types::HidDeviceInfo;
use util::hidraw_path_from_syspath;
use worker::transport_worker;
use device_commands::Command;
use anyhow::{Result, bail};
use std::{
    ffi::CString,
    sync::{Arc, mpsc},
    thread,
};
use tokio::sync::{
    Mutex as TokioMutex,
    mpsc::{UnboundedReceiver, unbounded_channel},
    oneshot,
};
use crate::{
    m3::{inputs::DeviceStateUpdate, layout::Kind, transport::LibTransport},
    utils::image_util::{ImageFormat, encode_image, rotated_dimensions},
};

pub(crate) mod util;
pub(crate) mod worker;
pub(crate) mod device_commands;

pub(crate) struct DeviceTransport {
    tx: mpsc::Sender<Command>,
    pub kind: Kind,
    pub serial_number: String,
    pub events: Arc<TokioMutex<Option<UnboundedReceiver<DeviceStateUpdate>>>>,
}

impl DeviceTransport {
    pub(crate) async fn open(info: &HidDeviceInfo, kind: Kind) -> Result<Self> {
        let transport = LibTransport::global()?;

        let DeviceId::DevPath(syspath) = &info.id else {
            bail!("Unsupported device identifier (expected a hidraw path)");
        };

        let devnode = hidraw_path_from_syspath(syspath)?;

        let path_c = CString::new(devnode.to_string_lossy().into_owned())?;

        let (command_tx, command_rx) = mpsc::channel::<Command>();
        let (event_tx, event_rx) = unbounded_channel::<DeviceStateUpdate>();

        let vid = info.vendor_id;
        let pid = info.product_id;
        let usage_page = info.usage_page;
        let usage = info.usage_id;
        let serial = info.serial_number.clone().unwrap_or_default();

        thread::spawn(move || transport_worker(transport, command_rx, event_tx));

        let (reply_tx, reply_rx) = oneshot::channel();
        command_tx.send(Command::Create { path: path_c, vid, pid, usage_page, usage, reply: reply_tx })?;
        reply_rx.await??;

        Ok(Self { tx: command_tx, kind, serial_number: serial, events: Arc::new(TokioMutex::new(Some(event_rx))) })
    }

    pub(crate) async fn set_brightness(&self, percent: u8) -> Result<()> {
        self.call(|reply| Command::SetBrightness(percent, reply)).await
    }

    pub(crate) async fn wake_screen(&self) -> Result<()> {
        self.call(Command::WakeScreen).await
    }

    pub(crate) async fn clear_key_image(&self, key: u8) -> Result<()> {
        self.call(|reply| Command::ClearKeyImage(key, reply)).await
    }

    pub(crate) async fn flush(&self) -> Result<()> {
        self.call(Command::Flush).await
    }

    pub(crate) async fn set_key_image(&self, key: u8, image_format: ImageFormat, image: &DynamicImage) -> Result<()> {
        let data = encode_image(image_format, image).await?;
        self.call(|reply| Command::SetKeyImage(key, data, reply)).await
    }

    pub(crate) async fn set_background_region(&self, image_format: ImageFormat, image: &DynamicImage, x: u16, y: u16, fb_layer: u8) -> Result<()> {
        let data = encode_image(image_format, image).await?;
        let (width, height) = rotated_dimensions(image_format.size, image_format.rotation);
        self.call(|reply| Command::SetBackgroundRegion { data, width, height, x, y, fb_layer, reply }).await
    }

    pub(crate) async fn clear_background_region(&self, fb_layer: u8) -> Result<()> {
        self.call(|reply| Command::ClearBackgroundRegion(fb_layer, reply)).await
    }

    pub(crate) async fn heartbeat(&self) -> Result<()> {
        self.call(Command::Heartbeat).await
    }

    pub(crate) async fn get_firmware_version(&self) -> Result<String> {
        self.call(Command::GetFirmwareVersion).await
    }

    /// Sends a command built from a fresh reply channel and awaits its response.
    async fn call<T>(&self, make: impl FnOnce(oneshot::Sender<Result<T>>) -> Command) -> Result<T> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(make(reply))?;
        rx.await?
    }
}

impl Drop for DeviceTransport {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Destroy);
    }
}
