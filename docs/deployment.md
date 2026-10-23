# Deployment

`segotep-digital` runs as a background service on both platforms: `systemd` on
Linux, the Service Control Manager on Windows. It can also be run interactively,
which is the quickest way to watch telemetry while something is wrong.

## Windows

### 1. PawnIO (CPU temperature only)

Windows motherboard firmware does not expose the CPU thermal diode through ACPI,
so reading AMD Zen (SMN) or Intel (MSR) sensors needs Ring 0. `segotep-digital`
gets there with [PawnIO](https://pawnio.eu/) — a signed, open-source driver that
runs sandboxed bytecode in kernel space and stays compatible with HVCI / Core
Isolation. Without it, everything except CPU temperature still works.

```powershell
winget install -e --id namazso.PawnIO
```

Legacy alternatives such as WinRing0 are blocked by Microsoft Defender; PawnIO
is the supported route.

### 2. Install

**Installer (MSI)** — download `segotep-digital-v<version>-windows-x64.msi` from
[Releases](https://github.com/Delnegend/segotep-digital/releases) and run it. It
registers the background service (and removes it on uninstall), and puts
`segotep-digital.exe` on `PATH`.

**Portable ZIP** — extract `segotep-digital-<version>-x86_64-pc-windows-msvc.zip`, then
from an **elevated** PowerShell:

```powershell
# On Windows 11 this prompt can use `sudo`; otherwise run PowerShell as Administrator.
sudo .\segotep-digital.exe --install-service -m 3 -i 1000
```

The service starts immediately and comes back on every boot.

### 3. Uninstall

```powershell
sudo .\segotep-digital.exe --uninstall-service
```

## Linux

### Homebrew

```bash
brew install Delnegend/tap/segotep-digital

# Non-root USB access for the cooler
sudo cp $(brew --prefix segotep-digital)/share/segotep-digital/99-segotep.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger

# Background service, managed by Homebrew
brew services start segotep-digital
```

The formula declares the service, so `brew services` generates the unit and
carries it across upgrades — no shipped unit, no `systemctl enable`.

Prefer a root-owned system service that starts before login? Copy the shipped
unit instead:

```bash
sudo cp $(brew --prefix segotep-digital)/share/segotep-digital/segotep-digital.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now segotep-digital.service
```

Only one daemon may drive the display at a time. A second instance opens the
same HID device without complaining rather than refusing to start, so stop any
service you started the other way before starting this one.

### Pre-built tarball

```bash
# Pick your architecture: x86_64-unknown-linux-musl on x64,
# aarch64-unknown-linux-musl on arm64.
tar -xJf segotep-digital-<version>-x86_64-unknown-linux-musl.tar.xz
cd segotep-digital-<version>-x86_64-unknown-linux-musl

sudo cp segotep-digital /usr/local/bin/ && sudo chmod +x /usr/local/bin/segotep-digital

sudo cp 99-segotep.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger

sudo cp segotep-digital.service /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now segotep-digital.service
```

### Uninstall

If you started it with `brew services`:

```bash
brew services stop segotep-digital
brew uninstall segotep-digital
sudo rm /etc/udev/rules.d/99-segotep.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```

If you copied the shipped unit to `/etc/systemd/system/` instead:

```bash
sudo systemctl disable --now segotep-digital.service
sudo rm /etc/systemd/system/segotep-digital.service /etc/udev/rules.d/99-segotep.rules /usr/local/bin/segotep-digital
sudo systemctl daemon-reload && sudo udevadm control --reload-rules
```

## The shipped unit and udev rule

Both live in the repository and are installed verbatim:

- [`systemd/segotep-digital.service`](../systemd/segotep-digital.service) — `Restart=always`, `RestartSec=3`, logs to the journal. Used when you copy it to `/etc/systemd/system/`; the Homebrew formula declares an equivalent service, so `brew services` does not need this file.
- [`udev/99-segotep.rules`](../udev/99-segotep.rules) — `MODE="0666"` plus the `uaccess` tag for `1a86:a001`, which is what lets a logged-in user talk to the cooler without root.

## Checking it is alive

```bash
brew services list                        # Linux, when started via Homebrew
tail -f /tmp/segotep-digital.log          # Linux, follow the Homebrew service log
systemctl status segotep-digital          # Linux, when using the shipped unit
journalctl -u segotep-digital -f          # Linux, follow the shipped unit's log
segotep-digital -v                        # either platform: watch the telemetry (elevated on Windows)
```
