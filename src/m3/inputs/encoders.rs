use crate::m3::inputs::{DeviceState, DeviceStateUpdate};

const ENC_TOP: u8 = 53;
const ENC_TOP_CCW: u8 = 80;
const ENC_TOP_CW: u8 = 81;
const ENC_MID: u8 = 51;
const ENC_MID_CCW: u8 = 144;
const ENC_MID_CW: u8 = 145;
const ENC_BOT: u8 = 55;
const ENC_BOT_CCW: u8 = 160;
const ENC_BOT_CW: u8 = 161;

pub(crate) struct Encoders;

impl Encoders {
    pub(crate) fn update(state: &mut DeviceState, input: u8, raw_state: u8) -> Option<DeviceStateUpdate> {
        match input {
            ENC_TOP | ENC_MID | ENC_BOT => Self::press(state, input, raw_state),
            ENC_TOP_CCW | ENC_TOP_CW | ENC_MID_CCW | ENC_MID_CW | ENC_BOT_CCW | ENC_BOT_CW => Self::twist(input),
            _ => {
                log::warn!("unknown input code {input}");
                None
            }
        }
    }

    fn press(state: &mut DeviceState, input: u8, raw_state: u8) -> Option<DeviceStateUpdate> {
        let encoder: u8 = match input {
            ENC_TOP => 0,
            ENC_MID => 1,
            ENC_BOT => 2,
            _ => return None,
        };

        let pressed = raw_state == 0x01;
        let idx = encoder as usize;

        let slot = state.encoders.get_mut(idx)?;
        if *slot == pressed {
            return None;
        }

        *slot = pressed;

        if pressed { Some(DeviceStateUpdate::EncoderDown(encoder)) } else { Some(DeviceStateUpdate::EncoderUp(encoder)) }
    }

    fn twist(input: u8) -> Option<DeviceStateUpdate> {
        let (encoder, value): (u8, i8) = match input {
            ENC_TOP_CCW => (0, -1),
            ENC_TOP_CW => (0, 1),
            ENC_MID_CCW => (1, -1),
            ENC_MID_CW => (1, 1),
            ENC_BOT_CCW => (2, -1),
            ENC_BOT_CW => (2, 1),
            _ => return None,
        };

        Some(DeviceStateUpdate::EncoderTwist(encoder, value))
    }
}
