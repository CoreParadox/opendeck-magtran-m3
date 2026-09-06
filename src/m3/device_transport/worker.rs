use std::{
    ops::ControlFlow,
    ptr,
    sync::{Arc, mpsc},
    time::Duration,
};
use crate::m3::{
    device_transport::device_commands::Command, inputs::{DeviceState, DeviceStateUpdate, handle_input_event}, transport::{LibTransport, TransportHandle, hid_device_info, raw},
};
use tokio::sync::mpsc::UnboundedSender;


pub(crate) fn handle_command(transport: &LibTransport, handle: &mut TransportHandle, command: Command) -> ControlFlow<()> {
    match command {
        Command::Create { path, vid, pid, usage_page, usage, reply } => {
            let info = hid_device_info {
                path: path.as_ptr().cast_mut(),
                vendor_id: vid,
                product_id: pid,
                serial_number: ptr::null_mut(),
                release_number: 0,
                manufacturer_string: ptr::null_mut(),
                product_string: ptr::null_mut(),
                usage_page,
                usage,
                interface_number: 0,
                next: ptr::null_mut(),
                bus_type: raw::hid_bus_type_HID_API_BUS_UNKNOWN,
            };

            match transport.create_device(&info) {
                Ok(h) => {
                    *handle = h;
                    let _ = reply.send(Ok(()));
                }
                Err(e) => {
                    let _ = reply.send(Err(e));
                }
            }
        }
        Command::Destroy => {
            transport.destroy_device(*handle);
            return ControlFlow::Break(());
        }
        Command::SetBrightness(percent, reply) => {
            let _ = reply.send(transport.set_key_brightness(*handle, percent));
        }
        Command::WakeScreen(reply) => {
            let _ = reply.send(transport.wake_screen(*handle));
        }
        Command::ClearKeyImage(key, reply) => {
            let _ = reply.send(transport.clear_key(*handle, key));
        }
        Command::SetKeyImage(key, data, reply) => {
            let _ = reply.send(transport.set_key_image(*handle, key, &data));
        }
        Command::SetBackgroundRegion { data, width, height, x, y, fb_layer, reply } => {
            let _ = reply.send(transport.set_background_region(*handle, &data, width, height, x, y, fb_layer));
        }
        Command::ClearBackgroundRegion(fb_layer, reply) => {
            let _ = reply.send(transport.clear_background_region(*handle, fb_layer));
        }
        Command::Flush(reply) => {
            let _ = reply.send(transport.flush(*handle));
        }
        Command::Heartbeat(reply) => {
            let _ = reply.send(transport.heartbeat(*handle));
        }
        Command::GetFirmwareVersion(reply) => {
            let _ = reply.send(transport.get_firmware_version(*handle));
        }
    }

    ControlFlow::Continue(())
}

pub(crate) fn transport_worker(transport: Arc<LibTransport>, rx: mpsc::Receiver<Command>, event_tx: UnboundedSender<DeviceStateUpdate>) {
    let mut handle: TransportHandle = ptr::null_mut();
    let mut input_state = DeviceState::new();
    let mut buf = vec![0u8; 1024];

    loop {
        match rx.recv_timeout(Duration::from_millis(5)) {
            Ok(command) => {
                if handle_command(&transport, &mut handle, command).is_break() {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                destroy_if_present(&transport, handle);
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        poll_input(
            &transport,
            handle,
            &mut input_state,
            &mut buf,
            &event_tx,
        );
    }
}

fn poll_input(transport: &LibTransport, handle: TransportHandle, input_state: &mut DeviceState, buf: &mut [u8], event_tx: &UnboundedSender<DeviceStateUpdate>) {
    if handle.is_null() {
        return;
    }

    let mut len = buf.len();
    let res = transport.read_input(handle, buf, &mut len);
    if res == 0
        && let Some(data) = buf.get(..len)
        && !data.is_empty()
        && let Some(update) = decode_input_report(input_state, data)
    {
        let _ = event_tx.send(update);
    }
}

fn decode_input_report(state: &mut DeviceState, buf: &[u8]) -> Option<DeviceStateUpdate> {
    if buf.is_empty() {
        return None;
    }

    let (input, raw_state) = if buf.starts_with(b"ACK") {
        if buf.len() < 11 {
            return None;
        }
        (*buf.get(9)?, *buf.get(10)?)
    } else if buf.starts_with(&[0x00]) && buf.get(1..).is_some_and(|s| s.starts_with(b"ACK")) {
        if buf.len() < 12 {
            return None;
        }
        (*buf.get(10)?, *buf.get(11)?)
    } else {
        return None;
    };

    if input == 0xFF {
        return None;
    }

    handle_input_event(state, input, raw_state)
}

fn destroy_if_present(transport: &LibTransport, handle: TransportHandle) {
    if !handle.is_null() {
        transport.destroy_device(handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUTTON_1: u8 = 1;
    const ENC_TOP: u8 = 53;
    const ENC_TOP_CCW: u8 = 80;
    const ENC_TOP_CW: u8 = 81;
    const PRESSED: u8 = 0x01;
    const RELEASED: u8 = 0x00;

    fn ack_report(input: u8, raw_state: u8) -> Vec<u8> {
        let mut buf = vec![0u8; 11];
        buf[0..3].copy_from_slice(b"ACK");
        buf[9] = input;
        buf[10] = raw_state;
        buf
    }

    fn zero_ack_report(input: u8, raw_state: u8) -> Vec<u8> {
        let mut buf = vec![0u8; 12];
        buf[1..4].copy_from_slice(b"ACK");
        buf[10] = input;
        buf[11] = raw_state;
        buf
    }

    #[test]
    fn decodes_supported_report_layouts() {
        let mut state = DeviceState::new();
        let update = decode_input_report(&mut state, &ack_report(BUTTON_1, PRESSED));
        assert!(matches!(update, Some(DeviceStateUpdate::ButtonDown(0))));

        let mut state = DeviceState::new();
        let update = decode_input_report(&mut state, &zero_ack_report(BUTTON_1, PRESSED));
        assert!(matches!(update, Some(DeviceStateUpdate::ButtonDown(0))));
    }

    #[test]
    fn rejects_short_or_unknown_reports() {
        let mut state = DeviceState::new();
        assert!(decode_input_report(&mut state, &[]).is_none());
        assert!(decode_input_report(&mut state, b"ACK").is_none());
        assert!(decode_input_report(&mut state, b"not-a-report").is_none());
    }

    #[test]
    fn emits_only_when_button_state_changes() {
        let mut state = DeviceState::new();
        let first = decode_input_report(&mut state, &ack_report(BUTTON_1, PRESSED));
        assert!(matches!(first, Some(DeviceStateUpdate::ButtonDown(0))));

        let second = decode_input_report(&mut state, &ack_report(BUTTON_1, PRESSED));
        assert!(second.is_none());

        let third = decode_input_report(&mut state, &ack_report(BUTTON_1, RELEASED));
        assert!(matches!(third, Some(DeviceStateUpdate::ButtonUp(0))));
    }

    #[test]
    fn ignores_unknown_button_codes() {
        let mut state = DeviceState::new();
        let update = decode_input_report(&mut state, &ack_report(0xAB, PRESSED));
        assert!(update.is_none());
    }

    #[test]
    fn emits_encoder_press_and_release_transitions() {
        let mut state = DeviceState::new();
        let down = decode_input_report(&mut state, &ack_report(ENC_TOP, PRESSED));
        assert!(matches!(down, Some(DeviceStateUpdate::EncoderDown(0))));

        let repeat = decode_input_report(&mut state, &ack_report(ENC_TOP, PRESSED));
        assert!(repeat.is_none());

        let up = decode_input_report(&mut state, &ack_report(ENC_TOP, RELEASED));
        assert!(matches!(up, Some(DeviceStateUpdate::EncoderUp(0))));
    }

    #[test]
    fn maps_rotation_direction_to_encoder_delta() {
        let mut state = DeviceState::new();
        let ccw = decode_input_report(&mut state, &ack_report(ENC_TOP_CCW, 0));
        assert!(matches!(ccw, Some(DeviceStateUpdate::EncoderTwist(0, -1))));

        let cw = decode_input_report(&mut state, &ack_report(ENC_TOP_CW, 0));
        assert!(matches!(cw, Some(DeviceStateUpdate::EncoderTwist(0, 1))));
    }
}