use std::{sync::Arc, time::Duration};

use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use super::{DeviceState, DeviceStateUpdate, handle_input_event};
use crate::m3::device_transport::MirajazzBackend;

/// Spawns the input reader task: polls raw HID reports from the transport
/// backend, decodes them into state updates, and forwards them until the
/// receiving end is dropped or `token` is cancelled.
pub(crate) fn spawn_poller(backend: Arc<MirajazzBackend>, event_tx: UnboundedSender<DeviceStateUpdate>, token: CancellationToken) {
    tokio::spawn(async move {
        let mut state = DeviceState::new();

        loop {
            tokio::select! {
                () = token.cancelled() => break,
                result = backend.poll_input(Duration::from_millis(50)) => {
                    match result {
                        Ok(Some(buf)) => {
                            if let Some(update) = decode_input_report(&mut state, &buf)
                                && event_tx.send(update).is_err()
                            {
                                break;
                            }
                        }
                        Ok(None) => {}
                        Err(e) => {
                            log::error!("mirajazz input read failed: {e}");
                            break;
                        }
                    }
                }
            }
        }
    });
}

/// Decodes raw HID inputs into a state update.
// The M3 (or at least my variation of it) prefixes inputs with `ACK`.
// A leading 0x00 report-id byte may or may not be before it, which would shift the byte offsets, so we handle both cases.
pub(crate) fn decode_input_report(state: &mut DeviceState, buf: &[u8]) -> Option<DeviceStateUpdate> {
    if buf.is_empty() {
        return None;
    }

    let (input, raw_state) = if buf.starts_with(b"ACK") {
        if buf.len() < 11 {
            return None;
        }
        (*buf.get(9)?, *buf.get(10)?)
    } 
    else if buf.starts_with(&[0x00]) && buf.get(1..).is_some_and(|s| s.starts_with(b"ACK")) {
        if buf.len() < 12 {
            return None;
        }
        (*buf.get(10)?, *buf.get(11)?)
    } 
    else {
        return None;
    };

    if input == 0xFF {
        return None;
    }

    handle_input_event(state, input, raw_state)
}
