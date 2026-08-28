# Segotep Digital

Cross-platform driver and background service for Segotep Ice Moon / Digital series AIO CPU coolers with integrated 7-segment digital pump block displays.

![License](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue)
![Platform](https://img.shields.io/badge/platform-Linux%20(x86__64%20%7C%20aarch64)%20%7C%20Windows%20(x64)-orange)

---

## Features

- **100% Open-Source & Bloat-Free**: Completely replaces closed-source first-party vendor utilities with a lightweight, secure Rust daemon consuming negligible CPU and < 10MB of RAM.
- **Cross-Platform Telemetry Engine**:
  - **Linux**: Zero-overhead hardware monitoring via direct kernel `sysfs` (`/sys/class/hwmon`), Intel/AMD RAPL energy counters (`/sys/class/powercap`), `/proc/stat`, and `cpufreq`.
  - **Windows**: High-precision native telemetry via Windows Performance Counters (PDH) for live CPU Package Power (Watts), dynamic boost clock frequency (MHz), and CPU % load; native NVIDIA NVML for GPU stats; and direct kernel CPU temperature monitoring via the signed [PawnIO](https://github.com/namazso/PawnIO) driver.
- **Reverse-Engineered 34-Byte HID Protocol**: Sends exact packet structures matching official hardware revisions (Model 1 Standard, Model 3 Ice Moon 360).
- **Native Background Service**: Native system service support on both platforms (`systemd` on Linux, native Windows Service Control Manager on Windows).
- **Robust USB HID Reconnection**: Automatically detects cooler disconnections, sleep cycles, and power states, seamlessly reconnecting without crashing.
- **Customizable**: Configurable refresh intervals, Fahrenheit display mode, screen power toggle, and custom VID/PID overrides.

---

## Why Use This Over Official Vendor Software?

| Feature | Segotep Official Software | `segotep-digital` + PawnIO |
| :--- | :--- | :--- |
| **Open Source** | ❌ Proprietary / Closed Source | ✅ **100% Open Source** (Rust + PawnIO) |
| **Cross-Platform** | ❌ Windows-only GUI app | ✅ **Linux (`systemd`) & Windows (Service)** |
| **Resource Usage** | ⚠️ GUI process (~15–25MB RAM) | ⚡ **< 10MB RAM, ~0% CPU usage** |
| **Background Mode** | ❌ Requires tray app / open console | ✅ **Runs silently as a native OS service** |

---

## Windows Requirements: CPU Temperature & PawnIO

On Windows, desktop motherboard BIOSes (ASUS, MSI, ASRock, Gigabyte) do not route CPU thermal diode temperatures through ACPI. Querying CPU Digital Thermal Sensors (DTS) on AMD Ryzen (SMN bus) and Intel Core (MSRs) requires Ring 0 kernel execution.

To maintain a **fully open-source stack** while remaining compatible with Windows HVCI / Core Isolation (Memory Integrity), `segotep-digital` utilizes **[PawnIO](https://pawnio.eu/)**—a modern, signed, open-source driver ecosystem.

### One-Time Driver Setup (Recommended via `winget`)

Install the official signed PawnIO driver via Windows Package Manager:

```powershell
winget install -e --id namazso.PawnIO
```

> **Why PawnIO?**
> Unlike vulnerable legacy kernel drivers (e.g. WinRing0) blocked by Microsoft Defender, PawnIO executes sandboxed bytecode in kernel space with strict access controls, providing safe, WHQL-compatible hardware access for open-source tools.

---

## Installation & Setup

### Windows

#### 1. Install as a Background Service (Recommended)

**Option A — Installer (MSI):** Download `segotep-digital-v<version>-windows-x64.msi` from [Releases](https://github.com/Delnegend/segotep-digital/releases) (replace `<version>` with the current release tag, e.g. `0.1.0`) and run it. The installer registers the Segotep Digital background service automatically on install (removing it on uninstall) and adds `segotep-digital.exe` to your PATH so it can be invoked from any terminal.

**Option B — Portable ZIP:** Download `segotep-digital-v<version>-windows-x64.zip` from [Releases](https://github.com/Delnegend/segotep-digital/releases) and extract it (replace `<version>` with the current release tag, e.g. `0.1.0`).

1. Open an **elevated** PowerShell (Run as Administrator) and install the background service:
    ```powershell
    # On Windows 11 use `sudo`; on older Windows run this prompt as Administrator directly.
    sudo .\segotep-digital.exe --install-service -m 3 -i 1000
    ```
    *The service will start immediately and automatically boot with Windows.*

2. To stop and uninstall the service at any time:
    ```powershell
    sudo .\segotep-digital.exe --uninstall-service
    ```

#### 2. Run Interactively in Console

```powershell
# Verbose live telemetry monitoring
sudo .\segotep-digital.exe -v -m 3
```

---

### Linux

#### Method 1: Homebrew (Linux)

```bash
# Add tap and install
brew tap Delnegend/tap
brew install segotep-digital

# Install udev rules for non-root USB access
sudo cp $(brew --prefix segotep-digital)/share/segotep-digital/99-segotep.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger

# Enable and start background systemd service
sudo cp $(brew --prefix segotep-digital)/share/segotep-digital/segotep-digital.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now segotep-digital.service
```

#### Method 2: Pre-built Binary Releases

1. Download the latest tarball from [Releases](https://github.com/Delnegend/segotep-digital/releases) (replace `<version>` with the current release tag, e.g. `0.1.0`):
    ```bash
    tar -xJf segotep-digital-v<version>-linux-amd64.tar.xz
    cd segotep-digital-v<version>-linux-amd64
    ```

2. Copy the binary and set permissions:
   ```bash
   sudo cp segotep-digital /usr/local/bin/
   sudo chmod +x /usr/local/bin/segotep-digital
   ```

3. Install udev rule and start the systemd service:
   ```bash
   sudo cp udev/99-segotep.rules /etc/udev/rules.d/
   sudo udevadm control --reload-rules && sudo udevadm trigger

   sudo cp systemd/segotep-digital.service /etc/systemd/system/
   sudo systemctl daemon-reload
   sudo systemctl enable --now segotep-digital.service
   ```

---

### Build from Source

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

```bash
git clone https://github.com/Delnegend/segotep-digital.git
cd segotep-digital
cargo build --release
```

The compiled binary will be located at:
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
| `-i, --interval-ms <MS>` | `1000` | Telemetry refresh interval in milliseconds (min: 100ms) |
| `-m, --model-id <ID>` | `3` | Device model ID (`3` for Ice Moon 360 / `1` for Standard Digital) |
| `-f, --fahrenheit` | `false` | Display temperature in Fahrenheit instead of Celsius |
| `--screen-off` | `false` | Turn off the 7-segment display screen and exit |
| `--vid <HEX>` | `1a86` | Custom USB Vendor ID in hexadecimal |
| `--pid <HEX>` | `a001` | Custom USB Product ID in hexadecimal |
| `-v, --verbose` | `false` | Print telemetry metrics and sensor stats to stdout |
| `--install-service` | | *(Windows only)* Install as a background Windows Service (auto-starts on boot) |
| `--uninstall-service` | | *(Windows only)* Stop and remove the Windows Service |
| `-h, --help` | | Print help |
| `-V, --version` | | Print version |

---

## License

Dual-licensed under either:
- **MIT License** ([LICENSE-MIT](LICENSE-MIT))
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))
