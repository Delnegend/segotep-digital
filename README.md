# Segotep Digital

Cross-platform driver and background service for Segotep Ice Moon / Digital series AIO CPU coolers with integrated 7-segment digital pump block displays.

![License](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue)
![Platform](https://img.shields.io/badge/platform-Linux%20(x86__64%20%7C%20aarch64)%20%7C%20Windows%20(x64)-orange)

---

## Features

- **Zero-Config Auto Detection**: Automatically queries the cooler MCU on startup to detect the Model ID, telemetry capability bitmask, and Fahrenheit display support.
- **Hardware Telemetry Support**:
  - **CPU Temperature**: Direct high-speed sysfs (`/sys/class/hwmon`) reader on Linux and native sensor sampling via `sysinfo` on Windows.
  - **CPU Package Power**: Real-time energy delta calculations via Intel/AMD RAPL (`/sys/class/powercap` and `hwmon` energy counters).
  - **CPU Utilization**: Accurate `/proc/stat` delta monitoring on Linux and high-precision `GetSystemTimes` Win32 API on Windows.
  - **CPU Frequency**: Linux `cpufreq` scaling clock reader and Windows clock speed monitoring.
- **Robust USB HID Reconnection**: Automatically detects cooler disconnections/sleep cycles and seamlessly reconnects.
- **Lightweight Background Daemon**: Runs in the background consuming negligible CPU and RAM (< 10MB).
- **Customizable**: Configurable refresh intervals, Fahrenheit display mode, screen power toggle, and custom VID/PID.

---

## Installation

### Method 1: Pre-built Binary Releases (Recommended)

#### Linux

1. Download the latest tarball for your architecture from [Releases](https://github.com/Delnegend/segotep-digital/releases):
   ```bash
   tar -xJf segotep-digital-v0.1.0-linux-amd64.tar.xz
   cd segotep-digital-v0.1.0-linux-amd64
   ```

2. Copy the binary to `/usr/local/bin`:
   ```bash
   sudo cp segotep-digital /usr/local/bin/
   sudo chmod +x /usr/local/bin/segotep-digital
   ```

3. Install the udev rule to allow non-root USB communication:
   ```bash
   sudo cp udev/99-segotep.rules /etc/udev/rules.d/
   sudo udevadm control --reload-rules && sudo udevadm trigger
   ```

4. Enable and start the systemd service:
   ```bash
   sudo cp systemd/segotep-digital.service /etc/systemd/system/
   sudo systemctl daemon-reload
   sudo systemctl enable --now segotep-digital.service
   ```

#### Windows

1. Download the latest zip archive from [Releases](https://github.com/Delnegend/segotep-digital/releases):
   - `segotep-digital-v0.1.0-windows-x64.zip`
2. Extract the archive and run `segotep-digital.exe` directly or add a shortcut to Windows Startup (`shell:startup`) / Task Scheduler.

---

### Method 2: Build from Source

#### Prerequisites

- **Rust toolchain** (Rust 2024 edition / `rustc >= 1.85.0`)
- **Linux dependencies**: `pkg-config`, `libusb-1.0-0-dev`, `libudev-dev`
  ```bash
  # Debian / Ubuntu
  sudo apt-get install -y pkg-config libusb-1.0-0-dev libudev-dev
  # Fedora
  sudo dnf install -y pkgconfig libusb1-devel systemd-devel
  # Arch Linux
  sudo pacman -S --needed pkgconf libusb systemd
  ```

#### Build Instructions

1. Clone the repository:
   ```bash
   git clone https://github.com/Delnegend/segotep-digital.git
   cd segotep-digital
   ```

2. Build the optimized release binary:
   ```bash
   cargo build --release
   ```

3. The compiled binary will be available at:
   - **Linux**: `target/release/segotep-digital`
   - **Windows**: `target/release/segotep-digital.exe`

---

## Usage & CLI Options

```bash
segotep-digital [OPTIONS]
```

### Options

| Flag / Option | Default | Description |
| :--- | :--- | :--- |
| `-i, --interval-ms <MS>` | `1000` | Update telemetry interval in milliseconds (min: 100ms) |
| `-m, --model-id <ID>` | *Auto* | Manual override for model ID (detected automatically by default) |
| `-f, --fahrenheit` | `false` | Display temperature in Fahrenheit instead of Celsius |
| `--screen-off` | `false` | Turn off the 7-segment display screen |
| `--vid <HEX>` | `1a86` | Custom USB Vendor ID in hexadecimal |
| `--pid <HEX>` | `a001` | Custom USB Product ID in hexadecimal |
| `-v, --verbose` | `false` | Print telemetry metrics and sensor stats to stdout |
| `-h, --help` | | Print help |
| `-V, --version` | | Print version |

---

## License

Dual-licensed under either:
- **MIT License** ([LICENSE-MIT](LICENSE-MIT))
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))
