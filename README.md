# Segotep Digital

Cross-platform driver and background service for Segotep Ice Moon / Digital series AIO CPU coolers with integrated 7-segment digital pump block displays.

![License](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue)
![Platform](https://img.shields.io/badge/platform-Linux%20(x86__64%20%7C%20aarch64)%20%7C%20Windows%20(x64)-orange)

---

## Features

- **Cross-Platform Telemetry**:
  - **Linux**: Direct zero-overhead hardware monitoring via `sysfs` (`/sys/class/hwmon`), Intel/AMD RAPL energy counters (`/sys/class/powercap`), `/proc/stat`, and `cpufreq`.
  - **Windows**: Multi-tier telemetry engine supporting Segotep LDGT helper, official standalone HWiNFO64 shared memory (`Global\HWiNFO_SENS_SM2`), AIDA64 / LibreHardwareMonitor XML shared memory (`AIDA64_SensorValues`), and native fallback via Win32 / `sysinfo`.
- **Reverse-Engineered 34-Byte HID Protocol**: Sends exact packet structures matching official Chinese vendor hardware revisions (Model 1 Standard, Model 3 Ice Moon 360).
- **Robust USB HID Reconnection**: Automatically detects cooler disconnections/sleep cycles and seamlessly reconnects without crashing.
- **Lightweight Background Daemon**: Zero bloat, consuming negligible CPU and < 10MB of RAM.
- **Customizable**: Configurable refresh intervals, Fahrenheit display mode, screen power toggle, and custom VID/PID.

---

## Windows Requirements & Sensor Sources

On Windows, reading ring-0 hardware sensors (like CPU core temperatures and package power in Watts) requires one of the following sensor providers:

### Option 1: HWiNFO64 (Recommended)
1. Install and launch [HWiNFO64](https://www.hwinfo.com/).
2. Open **Settings** -> **General / User Interface**.
3. Enable **"Shared Memory Support"**.
4. `segotep-digital` will automatically attach to `Global\HWiNFO_SENS_SM2` with mutex synchronization to fetch real-time CPU `Tctl/Tdie` temperature, package power, and clock speeds.

### Option 2: Segotep Official Sensor Engine (`LDGT.exe`)
- If the official Segotep Digital software is installed in `C:\Program Files\Segotep DigitalCAP`, `segotep-digital` will automatically start and communicate with the 2MB `shareMemory_LDGTInfo` sensor bank.

### Option 3: AIDA64 / LibreHardwareMonitor
- In AIDA64: **File** -> **Preferences** -> **Hardware Monitoring** -> **External Applications** -> Enable **"Enable shared memory"**.
- In LibreHardwareMonitor: Enable the AIDA64 shared memory plugin.

### Option 4: Pure Native User-Space (Driverless)
- If none of the above are running, `segotep-digital` falls back to user-space Win32 APIs and `sysinfo`.
- *Note:* User-space Windows APIs cannot query raw thermal diode temperatures without an installed kernel driver.

---

## Installation

### Method 1: Homebrew (Linux)

You can install `segotep-digital` on Linux using [Homebrew](https://brew.sh/) through the official tap:

```bash
# Add the tap and install
brew tap Delnegend/tap
brew install segotep-digital

# Install udev rule and start background systemd service
sudo cp $(brew --prefix segotep-digital)/share/segotep-digital/99-segotep.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger

sudo cp $(brew --prefix segotep-digital)/share/segotep-digital/segotep-digital.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now segotep-digital.service
```

---

### Method 2: Pre-built Binary Releases

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
2. Extract the archive.
3. Run `segotep-digital.exe` with administrator privileges:
   ```powershell
   sudo .\segotep-digital.exe -v
   ```
4. To run automatically on boot, add a shortcut to Windows Startup (`shell:startup`) or create a Task Scheduler task with "Run with highest privileges".

---

### Method 3: Build from Source

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
| `-m, --model-id <ID>` | `3` | Device model ID (`3` for Ice Moon 360 / `1` for Standard Digital) |
| `-f, --fahrenheit` | `false` | Display temperature in Fahrenheit instead of Celsius |
| `--screen-off` | `false` | Turn off the 7-segment display screen |
| `-s, --source <SOURCE>` | `auto` | *(Windows only)* Sensor backend: `auto`, `ldgt`, `hwinfo`, `aida64`, `sysinfo` |
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
