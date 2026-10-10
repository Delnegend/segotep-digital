# Configuration Reference

`segotep-digital` is configured entirely from the command line — there is no
config file and no environment variable.

## Options

| Flag | Default | Description |
| --- | --- | --- |
| `-i, --interval-ms <MS>` | `1000` | Telemetry refresh interval in milliseconds. Anything below `100` is raised to `100` |
| `-m, --model-id <ID>` | `3` | Device model ID — see below |
| `-f, --fahrenheit` | off | Show temperature in Fahrenheit instead of Celsius |
| `--screen-off` | off | Turn the 7-segment display off and exit |
| `--vid <HEX>` | `1a86` | Override the USB Vendor ID |
| `--pid <HEX>` | `a001` | Override the USB Product ID |
| `-v, --verbose` | off | Print the telemetry of every tick to stdout |
| `--install-service` | — | Windows only: install and start the background service |
| `--uninstall-service` | — | Windows only: stop and remove the background service |
| `-h, --help` | — | Print help |
| `-V, --version` | — | Print the version |

## Model IDs

| ID | Device |
| --- | --- |
| `1` | Digital standard |
| `3` | Ice Moon 360 (default) |

The ID selects which packet layout the display expects. A cooler that accepts
the connection but shows nothing is usually the other ID.

## USB identity

The defaults match the cooler's bridge chip (WCH `0x1a86:0xa001`). A device that
enumerates differently — a different revision, or a second unit on the same
machine — can be addressed with `--vid` / `--pid`. Both take hexadecimal.

## Examples

```bash
# Ice Moon 360, 1 s refresh (the defaults)
segotep-digital

# Digital standard, 500 ms refresh, Fahrenheit, with live telemetry
segotep-digital -m 1 -i 500 -f -v

# A unit that enumerates under a different product ID
segotep-digital --pid a002

# Blank the display and stop
segotep-digital --screen-off
```

## On Windows, run it as a service

The installer registers the service for you. To do it by hand, from an elevated
PowerShell:

```powershell
sudo .\segotep-digital.exe --install-service -m 3 -i 1000
sudo .\segotep-digital.exe --uninstall-service
```

`--install-service` accepts the same `-m` and `-i` flags, which become the
service's arguments on every boot.
