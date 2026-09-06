use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use image::DynamicImage;
use mirajazz::{
    device::Device as MzDevice,
    error::MirajazzError,
    state::DeviceStateReader,
    types::{
        DeviceInput, HidDeviceInfo, ImageFormat as MzImageFormat, ImageMirroring as MzImageMirroring, ImageMode as MzImageMode,
        ImageRotation as MzImageRotation,
    },
};

use crate::{
    m3::layout::{ENCODER_COUNT, KEY_COUNT},
    utils::image_util::{ImageFormat, ImageMirroring, ImageMode, ImageRotation},
};

/// the device reports separate press/release events for both buttons and encoders. this apparently "protocol version 3".
/// This protocol uses 1024-byte packets plus a 1-byte repot-id
const PROTOCOL_VERSION: usize = 3;
const REPORT_PAYLOAD: usize = 1024;

/// We have to decode raw input events ourselves (see comment on input_reader::decode_input_report)
/// This is only here to satisfy the signature of `Device::get_reader`.
fn stub_process_input(_input: u8, _state: u8) -> Result<DeviceInput, MirajazzError> {
    Ok(DeviceInput::NoData)
}

fn to_mirajazz_format(format: ImageFormat) -> MzImageFormat {
    MzImageFormat {
        mode: match format.mode {
            ImageMode::JPEG => MzImageMode::JPEG,
        },
        size: (usize::from(format.size.0), usize::from(format.size.1)),
        rotation: match format.rotation {
            ImageRotation::Rot90 => MzImageRotation::Rot90,
            ImageRotation::Rot270 => MzImageRotation::Rot270,
        },
        mirror: match format.mirror {
            ImageMirroring::None => MzImageMirroring::None,
            ImageMirroring::Both => MzImageMirroring::Both,
        },
    }
}

pub(crate) struct MirajazzBackend {
    device: MzDevice,
    reader: Arc<DeviceStateReader>,
}

impl MirajazzBackend {
    pub(crate) async fn connect(info: &HidDeviceInfo) -> Result<Self> {
        let device = MzDevice::connect(info, PROTOCOL_VERSION, KEY_COUNT, ENCODER_COUNT).await.map_err(|e| anyhow!("mirajazz connect failed: {e}"))?;
        let reader = device.get_reader(stub_process_input);
        Ok(Self { device, reader })
    }

    pub(crate) fn firmware_version(&self) -> Option<&str> {
        self.device.firmware_version.as_deref()
    }

    pub(crate) async fn set_brightness(&self, percent: u8) -> Result<()> {
        self.device.set_brightness(percent).await.map_err(|e| anyhow!("{e}"))
    }

    pub(crate) async fn set_key_image(&self, key: u8, image_format: ImageFormat, image: &DynamicImage) -> Result<()> {
        self.device.set_button_image(key, to_mirajazz_format(image_format), image.clone()).await.map_err(|e| anyhow!("{e}"))
    }

    pub(crate) async fn clear_key_image(&self, key: u8) -> Result<()> {
        self.device.clear_button_image(key).await.map_err(|e| anyhow!("{e}"))
    }

    pub(crate) async fn flush(&self) -> Result<()> {
        self.device.flush().await.map_err(|e| anyhow!("{e}"))
    }

    pub(crate) async fn heartbeat(&self) -> Result<()> {
        self.device.keep_alive().await.map_err(|e| anyhow!("{e}"))
    }

    // The background layer (BGPIC/BGCLE) isn't implemented via mirajazz
    // I captured the wire format while using the vendor sdk with a write interceptor

    /// Draws JPEG data into a background layer.
    /// Wire format captured from using sdk: 
    /// 1. `[0x00] "CRT" 00 00 "BGPIC" LEN32 X16 Y16 W16 H16 LAYER` (big-endian),
    /// 2. followed by the JPEG payload in `[0x00]` + 1024-byte chunks (same as mirajazz keyimages).
    pub(crate) async fn set_background_region(&self, data: &[u8], x: u16, y: u16, width: u16, height: u16, fb_layer: u8) -> Result<()> {
        self.device.keep_alive().await.map_err(|e| anyhow!("{e}"))?;

        let mut header = vec![0x00, 0x43, 0x52, 0x54, 0x00, 0x00];
        header.extend_from_slice(b"BGPIC");
        header.extend_from_slice(&(data.len() as u32).to_be_bytes());
        header.extend_from_slice(&x.to_be_bytes());
        header.extend_from_slice(&y.to_be_bytes());
        header.extend_from_slice(&width.to_be_bytes());
        header.extend_from_slice(&height.to_be_bytes());
        header.push(fb_layer);
        self.device.write_extended_data(&mut header).await.map_err(|e| anyhow!("{e}"))?;

        self.write_image_data(data).await
    }

    /// Clears a background layer.
    /// Wire format captured using sdk: `[0x00] "CRT" 00 00 "BGCLE" LAYER`.
    pub(crate) async fn clear_background_region(&self, fb_layer: u8) -> Result<()> {
        self.device.keep_alive().await.map_err(|e| anyhow!("{e}"))?;

        let mut buf = vec![0x00, 0x43, 0x52, 0x54, 0x00, 0x00];
        buf.extend_from_slice(b"BGCLE");
        buf.push(fb_layer);
        self.device.write_extended_data(&mut buf).await.map_err(|e| anyhow!("{e}"))
    }

    /// Streams image bytes as `[0x00]` + payload chunks
    async fn write_image_data(&self, data: &[u8]) -> Result<()> {
        let mut buf = Vec::with_capacity(1 + REPORT_PAYLOAD);
        for chunk in data.chunks(REPORT_PAYLOAD) {
            buf.clear();
            buf.push(0x00);
            buf.extend_from_slice(chunk);
            buf.resize(1 + REPORT_PAYLOAD, 0);
            self.device.write_data(&buf).await.map_err(|e| anyhow!("{e}"))?;
        }
        Ok(())
    }

    pub(crate) async fn poll_input(&self, timeout: Duration) -> Result<Option<Vec<u8>>> {
        self.reader.raw_read_data_with_timeout(512, timeout).await.map_err(|e| anyhow!("{e}"))
    }
}
