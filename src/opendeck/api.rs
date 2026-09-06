/// Facade over the OpenAction device-plugin outbound API. I initially implemented this because the OpenAction crate
/// didn't expose the settings API directly, but instead I just forked it to resolve that problem, so this is probably unnecessary.
pub async fn register_device(
    id: String,
    name: String,
    rows: u8,
    columns: u8,
    encoders: u8,
    r#type: u8,
) -> anyhow::Result<()> {
    openaction::device_plugin::register_device(id, name, rows, columns, encoders, r#type)
        .await
        .map_err(anyhow::Error::from)
}

pub async fn unregister_device(id: String) -> anyhow::Result<()> {
    openaction::device_plugin::unregister_device(id).await.map_err(anyhow::Error::from)
}

pub async fn rerender_images(id: String) -> anyhow::Result<()> {
    openaction::device_plugin::rerender_images(id).await.map_err(anyhow::Error::from)
}

pub async fn key_down(device: String, position: u8) -> anyhow::Result<()> {
    openaction::device_plugin::key_down(device, position).await.map_err(anyhow::Error::from)
}

pub async fn key_up(device: String, position: u8) -> anyhow::Result<()> {
    openaction::device_plugin::key_up(device, position).await.map_err(anyhow::Error::from)
}

pub async fn encoder_down(device: String, position: u8) -> anyhow::Result<()> {
    openaction::device_plugin::encoder_down(device, position).await.map_err(anyhow::Error::from)
}

pub async fn encoder_up(device: String, position: u8) -> anyhow::Result<()> {
    openaction::device_plugin::encoder_up(device, position).await.map_err(anyhow::Error::from)
}

pub async fn encoder_change(device: String, position: u8, ticks: i16) -> anyhow::Result<()> {
    openaction::device_plugin::encoder_change(device, position, ticks).await.map_err(anyhow::Error::from)
}
