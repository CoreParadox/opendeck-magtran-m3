![Plugin Icon](assets/icon.png)

# OpenDeck MagTran M3 Plugin

An unofficial plugin for MagTran M3 family devices.

**This is a fork of [opendeck-m18](https://github.com/ibanks42/opendeck-m18) by Isaiah Banks, adapted for the M3 device.**

## OpenDeck version

Requires OpenDeck 2.5.0 or newer

## Supported devices

- VSD Inside M3 (5548:1020)
- ActionRing MagTran M3 (5548:1038)

## Device Layout

The M3 has:
- 15 keys (5 columns x 3 rows)
- 3 rotary encoders
- a full 854x480 LCD display with support for (animated) backgrounds

> [!NOTE]
> Static background images are supported. Animated backgrounds are not yet supported.

## Platform support

- Linux: Primary development platform
- Mac: Best effort, may need testing (currently excluded to reduce package times)
- Windows: Not tested, contributions welcome (currently excluded to reduce package times)

## Installation

1. Download an archive from [releases](https://github.com/sakloui/opendeck-M3/releases)
2. In OpenDeck: Plugins -> Install from file
3. Linux: Download [udev rules](./40-opendeck-magtran-m3.rules) and install them by copying into `/etc/udev/rules.d/` and running `sudo udevadm control --reload-rules`
4. Unplug and plug again the device, restart OpenDeck

## Building

### Prerequisites

You'll need:

- A Linux OS of some sort
- Rust 1.87 and up with `x86_64-unknown-linux-gnu` and `x86_64-pc-windows-gnu` targets installed
- Docker
- [just](https://just.systems)
- StreamDock Device SDK headers and `libtransport.so` (see below)

### OpenAction

The plugin implements the OpenAction WebSocket protocol directly in `src/opendeck/protocol.rs`. This is a device plugin with `Actions: []` and `DeviceNamespace: "M3"` in `manifest.json`, so it can receive device events and the OpenDeck-specific `showSettingsInterface` event.

### StreamDock SDK

The plugin communicates with M3 devices through the vendor's `libtransport` library. The build also uses the StreamDock Device SDK headers to generate the Rust FFI bindings.

At build time, `build.rs` looks for `transport_c.h` and `hidapi.h` in `vendor/StreamDock-Device-SDK/include` by default. If you keep the SDK elsewhere, point to it with:

```sh
STREAMDOCK_SDK_PATH=/path/to/StreamDock-Device-SDK cargo build
```

At runtime, the plugin needs `libtransport.so` (or the platform equivalent) to be available. It is searched in this order:

1. The path in the `LIBTRANSPORT_PATH` environment variable.
2. The same directory as the running executable.
3. The project root (useful for development builds).
4. The OpenDeck plugin directory (`com.coreparadox.opendeck.magtran-m3.sdPlugin`).

You can also explicitly set the library path when launching OpenDeck:

```sh
LIBTRANSPORT_PATH=/path/to/libtransport.so opendeck
```

### Building a release package

```sh
$ just package
```

## Acknowledgments

This plugin is a fork of [opendeck-m18](https://github.com/ibanks42/opendeck-m18) by Isaiah Banks.

It is also heavily based on work by contributors of [elgato-streamdeck](https://github.com/streamduck-org/elgato-streamdeck) crate.

## License

GPL-3.0 (same as the original plugin)
