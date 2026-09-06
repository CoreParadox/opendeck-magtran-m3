use crate::{
    core::state::DEVICES,
    opendeck::{SetBrightnessEvent, SetImageEvent, profile},
};

pub(crate) async fn handle_set_image(event: SetImageEvent) {
    let id = event.device.clone();
    let is_profile_switch = event.position.is_none() && event.image.is_none();

    let device = DEVICES.read().await.get(&id).cloned();
    if let Some(device) = device {
        if let Err(err) = device.apply_image_event(event).await {
            log::error!("Device {id} error: {err}");
        }
    } else {
        log::error!("Received setImage for unknown device: {id}");
    }

    if is_profile_switch {
        profile::watch_for_profile_change(id);
    }
}

pub(crate) async fn handle_set_brightness(event: SetBrightnessEvent) {
    let id = event.device;
    let device = DEVICES.read().await.get(&id).cloned();
    if let Some(device) = device {
        if let Err(err) = device.set_brightness(event.brightness).await {
            log::error!("Device {id} error: {err}");
        }
    } else {
        log::error!("Received setBrightness for unknown device: {id}");
    }
}
