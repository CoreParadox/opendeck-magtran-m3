use futures_lite::{FutureExt, StreamExt};
use mirajazz::{
    device::{DeviceWatcher, list_devices},
    types::{DeviceLifecycleEvent, HidDeviceInfo},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use crate::{
    m3::{
        device::{Device, DiscoveredDevice},
        layout::{DEVICE_NAMESPACE, Kind, QUERIES},
    },
    state::{TOKENS, TRACKER},
};

fn device_id_from_info(dev: &HidDeviceInfo) -> Option<String> {
    Kind::from_vid_pid(dev.vendor_id, dev.product_id)?;
    Some(format!("{}-{}", DEVICE_NAMESPACE, dev.serial_number.clone()?))
}

fn discovered_from_info(dev: HidDeviceInfo) -> Option<DiscoveredDevice> {
    let id = device_id_from_info(&dev)?;
    let kind = Kind::from_vid_pid(dev.vendor_id, dev.product_id)?;
    Some(DiscoveredDevice { id, dev, kind })
}

async fn scan_devices() -> anyhow::Result<Vec<DiscoveredDevice>> {
    log::info!("Scanning for M3 devices");

    let mut discovered = Vec::new();
    for dev in list_devices(&QUERIES).await? {
        if let Some(device) = discovered_from_info(dev.to_device_info()) {
            discovered.push(device);
        }
    }

    Ok(discovered)
}

pub(crate) async fn run(token: CancellationToken) -> anyhow::Result<()> {
    let tracker = TRACKER.lock().await.clone();

    let discovered = scan_devices().await?;
    log::info!("Looking for connected devices");

    for device in discovered {
        spawn_device(device, &tracker).await;
    }

    let mut watcher = DeviceWatcher::new();
    let mut watcher_stream = watcher.watch(&QUERIES).await?;
    log::info!("Device lifecycle manager is ready");

    loop {
        let ev = watcher_stream
            .next()
            .or(async {
                token.cancelled().await;
                None
            })
            .await;

        let Some(ev) = ev else {
            log::info!("Device lifecycle manager is shutting down");
            break Ok(());
        };

        log::info!("New device event: {ev:?}");

        match ev {
            DeviceLifecycleEvent::Connected(info) => on_connected(info, &tracker).await,
            DeviceLifecycleEvent::Disconnected(info) => on_disconnected(info).await,
        }
    }
}

async fn spawn_device(device: DiscoveredDevice, tracker: &TaskTracker) {
    let dev_token = CancellationToken::new();
    let mut tokens = TOKENS.write().await;
    if tokens.contains_key(&device.id) {
        return;
    }

    tokens.insert(device.id.clone(), dev_token.clone());
    drop(tokens);

    log::info!("Spawning task for new device: {device:?}");
    tracker.spawn(Device::run(device, dev_token));
}

async fn on_connected(info: HidDeviceInfo, tracker: &TaskTracker) {
    if let Some(device) = discovered_from_info(info) {
        spawn_device(device, tracker).await;
    }
}

async fn on_disconnected(info: HidDeviceInfo) {
    let Some(id) = device_id_from_info(&info) else {
        log::warn!("Disconnected M3 device had no usable device identifier: {info:?}");
        return;
    };

    Device::cleanup_by_id(&id, true).await;
    log::info!("Disconnected device {id}");
}
