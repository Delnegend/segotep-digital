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

**Portable ZIP** — extract `segotep-digital-v<version>-windows-x64.zip`, then
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

### Run it in the console instead

```powershell
sudo .\segotep-digital.exe -v -m 3
```

## Linux

### Homebrew

```bash
brew tap Delnegend/tap
brew install segotep-digital

# Non-root USB access for the cooler
sudo cp $(brew --prefix segotep-digital)/share/segotep-digital/99-segotep.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger

# Background service
sudo cp $(brew --prefix segotep-digital)/share/segotep-digital/segotep-digital.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now segotep-digital.service
```

`brew info segotep-digital` prints the same steps. The formula also documents a
user-level service, which needs no `sudo`:

```bash
sed -e 's|/usr/local/bin|'"$(brew --prefix)"'/bin|' -e 's|multi-user.target|default.target|' \
  "$(brew --prefix)/share/segotep-digital/segotep-digital.service" > ~/.config/systemd/user/segotep-digital.service
systemctl --user enable --now segotep-digital.service
```

### Pre-built tarball

```bash
tar -xJf segotep-digital-v<version>-linux-amd64.tar.xz
cd segotep-digital-v<version>-linux-amd64

sudo cp segotep-digital /usr/local/bin/ && sudo chmod +x /usr/local/bin/segotep-digital

sudo cp udev/99-segotep.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger

sudo cp systemd/segotep-digital.service /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now segotep-digital.service
```

### Uninstall

```bash
sudo systemctl disable --now segotep-digital.service
sudo rm /etc/systemd/system/segotep-digital.service /etc/udev/rules.d/99-segotep.rules /usr/local/bin/segotep-digital
sudo systemctl daemon-reload && sudo udevadm control --reload-rules
```

## The shipped unit and udev rule

Both live in the repository and are installed verbatim:

- [`systemd/segotep-digital.service`](../systemd/segotep-digital.service) — `Restart=always`, `RestartSec=3`, logs to the journal.
- [`udev/99-segotep.rules`](../udev/99-segotep.rules) — `MODE="0666"` plus the `uaccess` tag for `1a86:a001`, which is what lets a logged-in user talk to the cooler without root.

## Checking it is alive

```bash
systemctl status segotep-digital          # Linux
journalctl -u segotep-digital -f          # Linux, follow the log
segotep-digital -v                        # either platform: watch the telemetry
```
