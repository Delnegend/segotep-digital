<div align="center">

# Segotep Digital

**Drives the 7-segment display on Segotep Ice Moon and Digital AIO CPU coolers from Linux and Windows: a small Rust service that replaces the vendor's closed-source utility.**

[![CI](https://badgers.space/github/checks/Delnegend/segotep-digital/main?corner_radius=0&label=build)](https://github.com/Delnegend/segotep-digital/actions)
[![Release](https://badgers.space/github/release/Delnegend/segotep-digital?corner_radius=0&color=ea7233)](https://github.com/Delnegend/segotep-digital/releases)
[![License](https://badgers.space/badge/license/MIT%20%7C%20Apache-2.0/007ec6?corner_radius=0)](LICENSE-MIT)

</div>

---

## Quick Start

**Linux**: Homebrew, then the udev rule and the background service:

```bash
brew tap Delnegend/tap && brew install segotep-digital

sudo cp $(brew --prefix segotep-digital)/share/segotep-digital/99-segotep.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger

brew services start segotep-digital
```

**Windows**: the MSI on [Releases](https://github.com/Delnegend/segotep-digital/releases) registers and starts the service. For CPU temperature, install the signed PawnIO driver once:

```powershell
winget install -e --id namazso.PawnIO
```

Either way the pump block shows CPU temperature within a second.

## Highlights

- **Replaces the vendor utility**: a native background service under 10 MB of RAM and around 0% CPU, with no GUI and no tray icon.
- **Real telemetry, not estimates**: Linux reads kernel sysfs, Intel/AMD RAPL energy counters, cpufreq and /proc/stat; Windows reads PDH performance counters and NVIDIA NVML for the GPU.
- **No kernel driver of its own**: Windows CPU temperature comes through the signed PawnIO driver, so HVCI and Core Integrity stay switched on.
- **Survives the hardware**: the cooler can sleep, unplug or power-cycle, and the service reconnects instead of crashing.
- **Configurable**: refresh interval, model ID, °C or °F, screen off, and custom USB IDs.

## Documentation

- **[Architecture](docs/architecture.md)**: Telemetry pipelines, the 34-byte HID protocol, and the polling loop.
- **[Development](docs/development.md)**: Building, testing, and how releases are cut.
- **[Deployment](docs/deployment.md)**: Installing, uninstalling, udev rules and the PawnIO driver; see [configuration](docs/configuration.md) for command-line options.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
