use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sdk_path = std::env::var("STREAMDOCK_SDK_PATH").map(PathBuf::from).unwrap_or_else(|_| root.join("vendor").join("StreamDock-Device-SDK"));
    let include = sdk_path.join("include");
    let header = include.join("transport_c.h");

    if !header.exists() {
        panic!(
            "StreamDock SDK header not found at {}. \
             Set STREAMDOCK_SDK_PATH or place the SDK at vendor/StreamDock-Device-SDK. \
             See README.md for instructions.",
            header.display()
        );
    }

    // Required libtransport methods for bindings.
    // Additional methods that may be needed later:
    // get_last_error_info, set_led_color/set_single_led_color, transport_sleep.
    let used_transport_bindings = [
        "transport_create",
        "transport_destroy",
        "transport_get_firmware_version",
        "transport_wakeup_screen",
        "transport_set_key_brightness",
        "transport_clear_key",
        "transport_set_key_image_stream",
        "transport_set_background_frame_stream",
        "transport_clear_background_frame_stream",
        "transport_refresh",
        "transport_heartbeat",
        "transport_read",
        "transport_set_reportSize",
    ];

    let mut builder = bindgen::Builder::default()
        .header(header.to_str().unwrap())
        .clang_arg(format!("-I{}", include.display()))
        .dynamic_library_name("LibTransport")
        .dynamic_link_require_all(true)
        .wrap_unsafe_ops(true)
        .allowlist_type("hid_device_info")
        .allowlist_type("TransportResult")
        .allowlist_type("TransportHandle")
        .size_t_is_usize(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    for name in &used_transport_bindings {
        builder = builder.allowlist_function(format!("^{name}$"));
    }

    let bindings = builder.generate().expect("failed to generate libtransport bindings");

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("libtransport_bindings.rs");
    let generated = bindings.to_string();
    let code = format!("#[allow(nonstandard_style, dead_code, clippy::too_many_arguments, clippy::doc_markdown)]\npub mod raw {{\n{generated}\n}}\n");
    std::fs::write(out, code).expect("failed to write bindings");
}
