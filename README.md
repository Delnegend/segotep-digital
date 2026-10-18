<div align="center">

# Segotep Digital

**Drives the 7-segment display on Segotep Ice Moon and Digital AIO CPU coolers from Linux and Windows — a small Rust service that replaces the vendor's closed-source utility.**

[![CI](https://img.shields.io/github/actions/workflow/status/Delnegend/segotep-digital/ci.yml?branch=main&style=flat-square)](https://github.com/Delnegend/segotep-digital/actions)
[![Release](https://img.shields.io/github/v/release/Delnegend/segotep-digital?style=flat-square)](https://github.com/Delnegend/segotep-digital/releases)
[![License](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue?style=flat-square)](LICENSE-MIT)

</div>

> **Maintenance mode.** Bug fixes and dependency updates only — no new features are planned.

## Quick Start

**Linux** — Homebrew, then the udev rule and the background service:

```bash
brew tap Delnegend/tap && brew install segotep-digital

sudo cp $(brew --prefix segotep-digital)/share/segotep-digital/99-segotep.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger

brew services start segotep-digital
```

**Windows** — the MSI on [Releases](https://github.com/Delnegend/segotep-digital/releases) registers and starts the service. For CPU temperature, install the signed PawnIO driver once:

```powershell
winget install -e --id namazso.PawnIO
```

Either way the pump block shows CPU temperature within a second.

## Highlights

- **Replaces the vendor utility** — a native background service under 10 MB of RAM and around 0% CPU, with no GUI and no tray icon.
- **Real telemetry, not estimates** — Linux reads kernel `sysfs`, Intel/AMD RAPL energy counters, `cpufreq` and `/proc/stat`; Windows reads PDH performance counters and NVIDIA NVML for the GPU.
- **No kernel driver of its own** — Windows CPU temperature comes through the signed PawnIO driver, so HVCI and Core Integrity stay switched on.
- **Survives the hardware** — the cooler can sleep, unplug or power-cycle, and the service reconnects instead of crashing.
- **Configurable** — refresh interval, model ID, °C or °F, screen off, and custom USB IDs.

## Documentation

- **[Configuration](docs/configuration.md)** — every flag, its default, and the model IDs.
- **[Architecture](docs/architecture.md)** — telemetry pipelines, the 34-byte HID protocol, and the polling loop.
- **[Deployment](docs/deployment.md)** — installing, uninstalling, udev rules and the PawnIO driver.
- **[Development](docs/development.md)** — building, testing, and how releases are cut.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
