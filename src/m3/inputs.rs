use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::opendeck::api;

use crate::m3::{
    device_transport::DeviceTransport,
    layout::{ENCODER_COUNT, KEY_COUNT},
};

pub(crate) mod buttons;
pub(crate) mod encoders;
pub(crate) mod input_reader;
pub(crate) use buttons::Buttons;
pub(crate) use encoders::Encoders;

pub(crate) struct DeviceState {
    pub buttons: [bool; KEY_COUNT],
    pub encoders: [bool; ENCODER_COUNT],
}

impl DeviceState {
    pub(crate) fn new() -> Self {
        Self { buttons: [false; KEY_COUNT], encoders: [false; ENCODER_COUNT] }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum DeviceStateUpdate {
    ButtonDown(u8),
    ButtonUp(u8),
    EncoderDown(u8),
    EncoderUp(u8),
    EncoderTwist(u8, i8),
}

pub(crate) struct Inputs {
    transport: Arc<DeviceTransport>,
}

impl Inputs {
    pub(crate) fn new(transport: Arc<DeviceTransport>) -> Self {
        Self { transport }
    }

    pub(crate) async fn forward_events(&self, device_id: &str) -> Result<()> {
        log::info!("Connecting to {device_id} for incoming events");

        let Some(mut events) = self.open_stream().await else {
            return Ok(());
        };
        log::info!("Connected to {device_id} for incoming events");

        while let Some(update) = events.recv().await {
            forward_event(device_id, update).await;
        }

        Ok(())
    }

    async fn open_stream(&self) -> Option<UnboundedReceiver<DeviceStateUpdate>> {
        self.transport.events.lock().await.take()
    }
}

pub(crate) fn handle_input_event(state: &mut DeviceState, input: u8, raw_state: u8) -> Option<DeviceStateUpdate> {
    log::info!("Processing input: key={input}, state={raw_state}");

    match input {
        1..=15 => Buttons::update(state, input, raw_state),
        _ => Encoders::update(state, input, raw_state),
    }
}

async fn forward_event(id: &str, update: DeviceStateUpdate) {
    match update {
        DeviceStateUpdate::ButtonDown(key) => {
            let _ = api::key_down(id.to_string(), key).await;
        }
        DeviceStateUpdate::ButtonUp(key) => {
            let _ = api::key_up(id.to_string(), key).await;
        }
        DeviceStateUpdate::EncoderDown(encoder) => {
            let _ = api::encoder_down(id.to_string(), encoder).await;
        }
        DeviceStateUpdate::EncoderUp(encoder) => {
            let _ = api::encoder_up(id.to_string(), encoder).await;
        }
        DeviceStateUpdate::EncoderTwist(encoder, val) => {
            let _ = api::encoder_change(id.to_string(), encoder, i16::from(val)).await;
        }
    }
}
