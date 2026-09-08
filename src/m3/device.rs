use std::{sync::Arc, time::Duration};

use anyhow::Result;
use futures_lite::FutureExt;
use mirajazz::types::HidDeviceInfo;
use tokio::time::interval;

use crate::opendeck::api;
use tokio_util::sync::CancellationToken;

use crate::{
    m3::{
        device_transport::DeviceTransport,
        display::Display,
        inputs::Inputs,
        layout::{COL_COUNT, DEVICE_TYPE, ENCODER_COUNT, Kind, ROW_COUNT},
    },
    opendeck::{config, profile},
    state::{DEVICES, TOKENS},
};

#[derive(Debug, Clone)]
pub(crate) struct DiscoveredDevice {
    pub id: String,
    pub dev: HidDeviceInfo,
    pub kind: Kind,
}

pub(crate) struct Device {
    id: String,
    transport: Arc<DeviceTransport>,
    keepalive_interval: Duration,
    inputs: Inputs,
    display: Display,
}

impl Device {
    pub(crate) async fn run(discovered: DiscoveredDevice, token: CancellationToken) {
        log::info!("Running device task for {discovered:?}");

        let device = match Self::open(&discovered, token.clone()).await {
            Ok(device) => device,
            Err(err) => {
                log::error!("Had error during device init for {discovered:?}: {err}");
                Self::cleanup_by_id(&discovered.id, false).await;
                return;
            }
        };

        device.register().await;
        device.publish().await;
        device.rerender_images().await;
        device.run_loop(token).await;
        device.cleanup(false).await;
    }

    pub(crate) async fn open(discovered: &DiscoveredDevice, token: CancellationToken) -> Result<Arc<Self>> {
        let plugin_config = config::load_or_default().await;
        let transport = Arc::new(DeviceTransport::open(&discovered.dev, discovered.kind.clone(), token).await?);

        let inputs = Inputs::new(transport.clone());
        let display = Display::new(transport.clone());

        // Init I found to work reliably, may not be necessary for all devices.
        transport.wake_screen().await?;
        transport.set_brightness(plugin_config.default_brightness).await?;
        display.clear_all_keys().await?;
        if let Some(fw) = transport.firmware_version() {
            log::info!("M3 firmware version: {fw}");
        }
        transport.flush().await?;

        let keepalive_interval = plugin_config.keepalive_interval();
        let device = Arc::new(Self { id: discovered.id.clone(), transport, keepalive_interval, inputs, display });
        Ok(device)
    }

    pub(crate) async fn apply_image_event(&self, event: crate::opendeck::SetImageEvent) -> Result<()> {
        self.display.apply_image_event(event).await
    }

    pub(crate) async fn apply_background(&self, image: String) -> Result<()> {
        self.display.apply_background(image).await
    }

    pub(crate) async fn clear_background(&self) -> Result<()> {
        self.display.clear_background().await
    }

    pub(crate) fn kind(&self) -> &Kind {
        &self.transport.kind
    }

    pub(crate) async fn set_brightness(&self, percent: u8) -> Result<()> {
        self.transport.set_brightness(percent).await
    }

    pub(crate) async fn heartbeat(&self) -> Result<()> {
        log::info!("Sending pulse to {}", self.id);
        self.transport.heartbeat().await
    }

    async fn rerender_images(self: &Arc<Self>) {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if let Err(e) = api::rerender_images(self.id.clone()).await {
            log::error!("Failed to rerender images for {}: {e}", self.id);
        }
    }

    async fn register(self: &Arc<Self>) {
        log::info!("Registering device {}", self.id);
        let name = self.kind().human_name();
        let rows = u8::try_from(ROW_COUNT).expect("ROW_COUNT fits in u8");
        let cols = u8::try_from(COL_COUNT).expect("COL_COUNT fits in u8");
        let encoders = u8::try_from(ENCODER_COUNT).expect("ENCODER_COUNT fits in u8");
        if let Err(e) = api::register_device(self.id.clone(), name, rows, cols, encoders, DEVICE_TYPE).await {
            log::error!("Failed to register device {}: {e}", self.id);
        }
    }

    async fn publish(self: &Arc<Self>) {
        DEVICES.write().await.insert(self.id.clone(), self.clone());
        profile::sync_profile_background(self, &self.id).await;
    }

    async fn run_loop(self: &Arc<Self>, token: CancellationToken) {
        let events = async {
            self.inputs.forward_events(&self.id).await.ok();
        };
        let keep = async {
            self.heartbeat_loop().await.ok();
        };
        let () = events.or(keep).or(token.cancelled()).await;
    }

    async fn heartbeat_loop(self: &Arc<Self>) -> Result<()> {
        let mut interval = interval(self.keepalive_interval);

        loop {
            interval.tick().await;

            if let Err(e) = self.heartbeat().await {
                log::error!("Pulse failed for {}: {}", self.id, e);
                self.cleanup(false).await;
                break;
            }
        }

        Ok(())
    }

    pub(crate) async fn cleanup(self: &Arc<Self>, deregister: bool) {
        log::info!("Shutting down device {}", self.id);

        if let Err(e) = self.transport.shutdown().await {
            log::error!("Failed to shut down device {}: {e}", self.id);
        }

        Self::cleanup_by_id(&self.id, deregister).await;
    }

    pub(crate) async fn cleanup_by_id(id: &str, deregister: bool) {
        if deregister
            && let Err(e) = api::unregister_device(id.to_string()).await
        {
            log::error!("Failed to unregister device {id}: {e}");
        }

        TOKENS.write().await.remove(id);
        DEVICES.write().await.remove(id);
    }
}
