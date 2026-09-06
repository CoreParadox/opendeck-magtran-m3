use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tokio::sync::{Mutex, RwLock};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use crate::m3::device::Device;

pub(crate) static DEVICES: LazyLock<RwLock<HashMap<String, Arc<Device>>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

pub(crate) static TOKENS: LazyLock<RwLock<HashMap<String, CancellationToken>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

pub(crate) static TRACKER: LazyLock<Mutex<TaskTracker>> = LazyLock::new(|| Mutex::new(TaskTracker::new()));

pub(crate) async fn shutdown() {
    for token in TOKENS.write().await.values() {
        token.cancel();
    }
    let tracker = TRACKER.lock().await.clone();
    tracker.close();
    tracker.wait().await;
}
