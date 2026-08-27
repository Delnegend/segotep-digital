# segotep-digital-rs

An open-source, ultra-lightweight Linux driver and daemon for **Segotep Ice Moon** (and other Segotep Digital series) AIO liquid coolers with integrated 7-segment pump-block displays.

Written in Rust with zero unnecessary overhead.

---

## Features

- **Accurate Telemetry**:
  - **CPU Temperature (°C / °F)**: Reads direct `/sys/class/hwmon` interfaces (`k10temp`, `coretemp`, `zenpower`, etc.) with fallback to `sysinfo`.
  - **CPU Utilization (%)**: Accurate delta sampling via `/proc/stat`.
  - **CPU Package Power (Watts)**: Native Intel RAPL / AMD Energy counter calculations via `/sys/class/powercap` and `hwmon`.
  - **CPU Frequency (MHz)**: Direct `cpufreq` / `sysinfo`.
- **Auto-reconnection**: Automatically reconnects if the USB cable is disconnected or power cycled across sleep/hibernate.
- **Full Customizability**:
  - Switch between Celsius (°C) and Fahrenheit (°F).
  - Configurable update rates (default: `1000ms`).
  - Screen power toggle (`--screen-off`).
  - Custom VID/PID options.
- **Non-root Operation**: Includes `udev` rules so you don't have to run it as `root`.
- **Systemd Integration**: Runs seamlessly in the background as a systemd service.

---

## Hardware Protocol Details

Reverse-engineered from the official Windows `LEDDisplay.dll`:

| Field | Value | Notes |
| :--- | :--- | :--- |
| **Vendor ID (VID)** | `0x1A86` | WCH / QinHeng Electronics |
| **Product ID (PID)** | `0xA001` | Segotep Digital Series Display |
| **Transport** | USB HID | Output report length: 34 bytes |
| **Header** | `0x00, 0xDC, 0xDD` | Report ID + Magic Header |
| **Model ID** | `3` | Segotep Ice Moon |
| **Display Modes** | Temp / Power / Load | Auto-cycled on the pump block |

---

## Installation & Setup on Linux

### 1. Prerequisites
On Ubuntu / Debian / Fedora / Arch:
```bash
# Ubuntu / Debian
sudo apt update && sudo apt install libusb-1.0-0-dev libudev-dev pkg-config build-essential

# Fedora
sudo dnf install libusb1-devel systemd-devel pkgconf-pkg-config gcc

# Arch Linux
sudo pacman -S libusb systemd pkgconf base-devel
```

### 2. Install Rust
If you don't have Rust installed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. Build & Install Binary
```bash
git clone https://github.com/your-username/segotep-digital-rs.git
cd segotep-digital-rs

# Compile release binary
cargo build --release

# Install binary to system path
sudo cp target/release/segotep-digital-rs /usr/local/bin/
```

### 4. Setup udev Rules (Non-root USB Access)
```bash
sudo cp udev/99-segotep.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### 5. Test the Binary
```bash
# Run with verbose output
segotep-digital-rs -v
```

---

## Running as a Background Service (systemd)

To ensure the display updates automatically whenever your computer is on:

```bash
# Copy systemd service file
sudo cp systemd/segotep-digital.service /etc/systemd/system/

# Reload systemd, enable and start the service
sudo systemctl daemon-reload
sudo systemctl enable --now segotep-digital.service

# Check status
sudo systemctl status segotep-digital.service
```

---

## CLI Options

```text
segotep-digital-rs [OPTIONS]

Options:
  -i, --interval-ms <INTERVAL_MS>  Update interval in milliseconds [default: 1000]
  -m, --model-id <MODEL_ID>        Device model ID [default: 3]
  -f, --fahrenheit                 Display temperature in Fahrenheit instead of Celsius
      --screen-off                 Turn off the 7-segment display screen
      --vid <VID>                  Custom USB Vendor ID in hex (e.g. 1a86)
      --pid <PID>                  Custom USB Product ID in hex (e.g. a001)
  -v, --verbose                    Print telemetry metrics to stdout on each tick
  -h, --help                       Print help
  -V, --version                    Print version
```

---

## License

MIT OR Apache-2.0
