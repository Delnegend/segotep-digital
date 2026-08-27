# Segotep Digital

Linux driver and background service for Segotep Ice Moon / Digital series AIO CPU coolers with integrated 7-segment digital pump block displays.

![License](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue)
![Platform](https://img.shields.io/badge/platform-Linux%20(x86__64%20%7C%20aarch64)-orange)

---

## Features

- **Zero-Config Auto Detection**: Automatically queries the cooler MCU on startup to detect the Model ID, telemetry capability bitmask, and Fahrenheit display support.
- **Hardware Telemetry Support**:
  - **CPU Temperature**: Direct high-speed sysfs (`/sys/class/hwmon`) reader with fallback to `sysinfo`.
  - **CPU Package Power**: Real-time energy delta calculations via Intel/AMD RAPL (`/sys/class/powercap` and `hwmon` energy counters).
  - **CPU Utilization**: Accurate `/proc/stat` delta monitoring.
  - **CPU Frequency**: Linux `cpufreq` scaling clock reader.
- **Robust USB HID Reconnection**: Automatically detects cooler disconnections/sleep cycles and seamlessly reconnects.
- **Lightweight Systemd Service**: Runs in the background consuming negligible CPU and RAM (< 10MB).
- **Customizable**: Configurable refresh intervals, Fahrenheit display mode, screen power toggle, and custom VID/PID.

---

## Installation

### Method 1: Pre-built Binary Releases (Recommended)

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
