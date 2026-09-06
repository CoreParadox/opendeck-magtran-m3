// Bindings are generated in build.rs using the StreamDock Device SDK headers.
// This module is only the low-level libtransport wrapper: loading and raw FFI calls.

include!(concat!(env!("OUT_DIR"), "/libtransport_bindings.rs"));
pub(crate) use raw::{LibTransport, TransportHandle, TransportResult, hid_device_info};

// LibTransport is mainly immutable function pointers, so it SHOULD be safe to share
// between the async facade and the worker thread.
// though I am not sure what may happen with multiple devices
// TODO: maybe it would be better to have a single transport service that manages all device handles
unsafe impl Sync for LibTransport {}

use std::{
    ffi::c_char,
    path::{Path, PathBuf},
    ptr,
    sync::{Arc, OnceLock},
};

use anyhow::{Result, anyhow, bail};
use libloading::Library;

use crate::opendeck::config;

static TRANSPORT: OnceLock<Result<Arc<LibTransport>, String>> = OnceLock::new();

pub(crate) fn find_libtransport() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("LIBTRANSPORT_PATH") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Ok(p);
        }
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let p = dir.join("libtransport.so");
        if p.exists() {
            return Ok(p);
        }
    }

    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("libtransport.so");
    if p.exists() {
        return Ok(p);
    }

    let p = config::plugin_dir().join("libtransport.so");
    if p.exists() {
        return Ok(p);
    }

    bail!("libtransport.so not found; set LIBTRANSPORT_PATH or place it next to the executable")
}

impl LibTransport {
    pub(crate) fn global() -> Result<Arc<Self>> {
        let loaded = TRANSPORT.get_or_init(|| -> Result<Arc<LibTransport>, String> {
            let path = find_libtransport().map_err(|e| e.to_string())?;
            log::info!("Loading libtransport from {}", path.display());
            let lib = unsafe { Library::new(&path).map_err(|e| e.to_string())? };
            let libtransport = unsafe { LibTransport::from_library(lib).map_err(|e| e.to_string())? };
            Ok(Arc::new(libtransport))
        });
        match loaded {
            Ok(t) => Ok(t.clone()),
            Err(e) => Err(anyhow!("{e}")),
        }
    }

    pub(crate) fn create_device(&self, info: &hid_device_info) -> Result<TransportHandle> {
        let mut handle: TransportHandle = ptr::null_mut();

        let res = unsafe { self.transport_create(info, &raw mut handle) };
        if res != 0 {
            bail!("transport_create failed: 0x{res:08x}");
        }

        let res = unsafe { self.transport_set_reportSize(handle, 513, 1025, 0) };
        if res != 0 {
            unsafe { self.transport_destroy(handle) };
            bail!("transport_set_reportSize failed: 0x{res:08x}");
        }

        Ok(handle)
    }

    pub(crate) fn destroy_device(&self, handle: TransportHandle) {
        if !handle.is_null() {
            let _ = unsafe { self.transport_destroy(handle) };
        }
    }

    pub(crate) fn wake_screen(&self, handle: TransportHandle) -> Result<()> {
        to_result(unsafe { self.transport_wakeup_screen(handle) }, "transport_wakeup_screen")
    }

    pub(crate) fn set_key_brightness(&self, handle: TransportHandle, percent: u8) -> Result<()> {
        to_result(unsafe { self.transport_set_key_brightness(handle, percent) }, "transport_set_key_brightness")
    }

    pub(crate) fn clear_key(&self, handle: TransportHandle, key: u8) -> Result<()> {
        to_result(unsafe { self.transport_clear_key(handle, key + 1) }, "transport_clear_key")
    }

    pub(crate) fn set_key_image(&self, handle: TransportHandle, key: u8, data: &[u8]) -> Result<()> {
        let res = unsafe { self.transport_set_key_image_stream(handle, data.as_ptr().cast::<c_char>(), data.len(), key + 1) };
        to_result(res, "transport_set_key_image_stream")
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn set_background_region(&self, handle: TransportHandle, data: &[u8], width: u16, height: u16, x: u16, y: u16, fb_layer: u8) -> Result<()> {
        let res = unsafe { self.transport_set_background_frame_stream(handle, data.as_ptr().cast::<c_char>(), data.len(), width, height, x, y, fb_layer) };
        to_result(res, "transport_set_background_frame_stream")
    }

    pub(crate) fn clear_background_region(&self, handle: TransportHandle, fb_layer: u8) -> Result<()> {
        let res = unsafe { self.transport_clear_background_frame_stream(handle, fb_layer) };
        to_result(res, "transport_clear_background_frame_stream")
    }

    pub(crate) fn flush(&self, handle: TransportHandle) -> Result<()> {
        to_result(unsafe { self.transport_refresh(handle) }, "transport_refresh")
    }

    pub(crate) fn heartbeat(&self, handle: TransportHandle) -> Result<()> {
        to_result(unsafe { self.transport_heartbeat(handle) }, "transport_heartbeat")
    }

    pub(crate) fn get_firmware_version(&self, handle: TransportHandle) -> Result<String> {
        let mut ver = vec![0u8; 64];
        let res = unsafe { self.transport_get_firmware_version(handle, ver.as_mut_ptr().cast::<c_char>(), ver.len()) };
        if res != 0 {
            bail!("transport_get_firmware_version failed: 0x{res:08x}");
        }
        let s = String::from_utf8_lossy(&ver).split('\0').next().unwrap_or("").to_string();
        Ok(s)
    }

    pub(crate) fn read_input(&self, handle: TransportHandle, buf: &mut [u8], len: &mut usize) -> TransportResult {
        unsafe { self.transport_read(handle, buf.as_mut_ptr(), &raw mut *len, 0) }
    }
}

fn to_result(res: TransportResult, name: &str) -> Result<()> {
    if res == 0 { Ok(()) } else { Err(anyhow!("{name} failed: 0x{res:08x}")) }
}
