use crate::m3::inputs::{DeviceState, DeviceStateUpdate};

pub(crate) struct Buttons;

impl Buttons {
    pub(crate) fn update(state: &mut DeviceState, input: u8, raw_state: u8) -> Option<DeviceStateUpdate> {
        let opendeck = hardware_to_opendeck(input)?;
        let opendeck = opendeck as usize;
        let pressed = raw_state == 0x01;

        let slot = state.buttons.get_mut(opendeck)?;
        if *slot == pressed {
            return None;
        }

        *slot = pressed;

        let key = u8::try_from(opendeck).expect("button index fits in u8");
        if pressed { Some(DeviceStateUpdate::ButtonDown(key)) } else { Some(DeviceStateUpdate::ButtonUp(key)) }
    }
}

fn hardware_to_opendeck(hw: u8) -> Option<u8> {
    if (1..=15).contains(&hw) { Some(hw - 1) } else { None }
}
