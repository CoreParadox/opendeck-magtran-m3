use anyhow::Result;
use std::ffi::CString;
use tokio::sync::oneshot;

pub(crate) enum Command {
    Create { path: CString, vid: u16, pid: u16, usage_page: u16, usage: u16, reply: oneshot::Sender<Result<(), anyhow::Error>> },
    Destroy,
    SetBrightness(u8, oneshot::Sender<Result<(), anyhow::Error>>),
    WakeScreen(oneshot::Sender<Result<(), anyhow::Error>>),
    ClearKeyImage(u8, oneshot::Sender<Result<(), anyhow::Error>>),
    SetKeyImage(u8, Vec<u8>, oneshot::Sender<Result<(), anyhow::Error>>),
    SetBackgroundRegion { data: Vec<u8>, width: u16, height: u16, x: u16, y: u16, fb_layer: u8, reply: oneshot::Sender<Result<(), anyhow::Error>> },
    ClearBackgroundRegion(u8, oneshot::Sender<Result<(), anyhow::Error>>),
    Flush(oneshot::Sender<Result<(), anyhow::Error>>),
    Heartbeat(oneshot::Sender<Result<(), anyhow::Error>>),
    GetFirmwareVersion(oneshot::Sender<Result<String, anyhow::Error>>),
}
