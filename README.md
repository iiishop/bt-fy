# Butterfly Effect

Butterfly Effect is a paired mechanical butterfly system with companion software. Each butterfly can sense nearby interaction and trigger motion in its paired butterfly. The relationship is two-way: device A can trigger device B, and device B can trigger device A.

At the hardware level, the project combines an ESP32-C3, a time-of-flight distance sensor, and servo-driven motion. At the software level, it combines embedded Rust firmware with a Flutter Android app for provisioning, pairing, and control.

Kickstarter video: https://youtu.be/o-Wq5kcca9Q

<p align="center">
  <img src="Butterfly device system layout.png" width="900" />
</p>

This overview shows the intended interaction: one butterfly senses nearby presence and a paired butterfly responds remotely through flapping and rotation.

## What the project does

The project builds a networked butterfly device and the software needed to use it.

- A butterfly detects nearby presence with a distance sensor.
- The wings flap in response to that presence.
- A paired butterfly can reproduce that activity remotely.
- The mobile app provisions devices onto Wi-Fi, monitors their status, and manages pairing.

In practical terms, Butterfly Effect is a lightweight physical notification and connection system. One interaction in one place can create a visible response somewhere else.

<p align="center">
  <img src="System workflow.png" width="900" />
</p>

The system combines a sensing layer, embedded processing on the ESP32-C3, mobile app support, and remote actuation through paired butterfly motion.

## Why the project is useful

Many notification systems rely on light, sound, or phone alerts. Butterfly Effect explores a softer and more expressive alternative. A butterfly is already a meaningful visual object, so motion carries more emotional weight than a generic signal.

The idea is also tied to the metaphor of the butterfly effect itself: a small movement in one place produces a response somewhere else. That makes the project useful not only as a technical prototype, but also as a way to think about remote care, ambient notification, and emotional connection.

Possible use cases include:

- a doorbell-like reminder, where one butterfly is near an entrance and the other is in another room
- remote care, where an elderly person or family member can trigger a light-touch physical signal at a distance
- emotional connection, where two people in different places can send each other a small physical sign of presence

## Repository contents

```text
.
├── src/                  Rust firmware for the ESP32-C3 device
├── flutter_app/          Flutter Android app for setup, pairing, and control
├── Cargo.toml            Firmware project manifest
└── other reference images and development notes
```

## Hardware needed

The current prototype is based on the following parts:

- 1 x Seeed Studio XIAO ESP32C3
- 1 x VL53L0X time-of-flight sensor
- 1 x SG92R servo
- 1 x SG90-HV continuous servo
- 1 x 800 mAh LiPo battery
- 1 x 3D-printed butterfly model
- jumper wires as needed

The current firmware configuration in `src/system/config.rs` uses:

- sensor SDA: `GPIO5`
- sensor SCL: `GPIO2`
- servo 1 signal: `GPIO3`
- servo 2 signal: `GPIO4`

The main circuit layout is shown below.

<p align="center">
  <img src="Circuit connection layout.png" width="700" />
</p>

## How users can get started with the project

There are two parts to getting started: hardware and software.

### 1. Build the hardware

Purchase the required components and wire them according to the diagrams in this repository.

Make sure your wiring matches the current firmware pin definitions in `src/system/config.rs` before flashing the board.

### 2. Clone the repository

```bash
git clone <your-repo-url>
cd bt-fy
```

### 3. Set up the ESP32 firmware environment

The firmware is a Rust project targeting `riscv32imc-esp-espidf`.

This repository already includes:

- `.cargo/config.toml` for target and runner configuration
- `rust-toolchain.toml` with a pinned nightly toolchain
- `espflash` as the configured runner

Set up a working ESP-IDF 5.x Rust environment first. The repo is currently configured with `ESP_IDF_VERSION = "v5.3.3"` in `.cargo/config.toml`.

Then build and flash from the repository root:

```bash
cargo build --release
cargo run --release
```

If you need to specify the serial port explicitly, use:

```bash
cargo espflash flash --release --monitor --port COM6
```

### 4. Set up and run the mobile app

The Android app lives in `flutter_app/`.

The current working development flow is:

```bash
cd flutter_app
flutter run
```

You will need:

- a working Flutter installation
- Android Studio / Android SDK
- an Android phone connected for development, or an Android test device

If you prefer to distribute an APK later, build and install it from the same Flutter project.

### 5. Basic setup flow

Once both parts are running, the normal workflow is:

1. Power the butterfly device.
2. Open the Android app.
3. Scan for a hotspot with the `BF_` prefix.
4. Connect to the device hotspot.
5. Send home Wi-Fi credentials from the app.
6. Let the device switch from Soft-AP mode to STA mode.
7. Rediscover and bind the device on the local network.
8. Pair two devices in the app.

The app flow is shown below: add the device, configure Wi-Fi, then bind and pair it for use.

<p align="center">
  <img src="Add device.png" width="30%" />
  <img src="Configure Wifi.png" width="30%" />
  <img src="Control and Pair.png" width="30%" />
</p>

These screens correspond to the three practical stages of onboarding. The first is device discovery through the temporary `BF_` hotspot. The second is Wi-Fi provisioning, where the phone sends home network credentials to the ESP32 device. The third is device management after reconnection, where the user can confirm that the device is online, bind it in the app, and pair it with another butterfly.

## Reproducibility notes

If you want to reproduce the project, use the current code and pin configuration in the repository rather than relying on older presentation material.

Important current constraints:

- pairing currently depends on local-network discovery and direct TCP communication
- Android does not always reconnect to the intended Wi-Fi automatically after provisioning
- the ESP32-C3 handles UDP state broadcast and TCP control at the same time, so responsiveness is limited by hardware resources

## Where users can get help with the project

For technical help, start with the repository itself:

- firmware source: `src/`
- hardware configuration: `src/system/config.rs`
- Android app source: `flutter_app/`
- app setup notes: `flutter_app/README.md`

If you are using GitHub, the best place to ask questions or report issues is the repository Issues section.

## Team

This project is maintained by the course team:

- Matilda Nelson
- Yitong Wu
- Yuqian Lin

Contribution summary:

- Matilda Nelson: concept development, physical form, mechanism and enclosure design, and overall product direction
- Yitong Wu: hardware and circuit design, integration, wiring, assembly, and hardware debugging
- Yuqian Lin: software and code-related implementation, including the mobile app, ESP32 communication logic, provisioning, pairing, control, and hardware control logic for sensing and actuation
