use crate::m3::{inputs::{DeviceState, DeviceStateUpdate}, layout};

pub(crate) struct Buttons;

impl Buttons {
    pub(crate) fn update(state: &mut DeviceState, input: u8, raw_state: u8) -> Option<DeviceStateUpdate> {
        let opendeck = layout::hardware_button_to_opendeck(input)? as usize;
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
