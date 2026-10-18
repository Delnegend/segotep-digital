# Development Guide

## Prerequisites

- **Rust 1.85 or newer** — the crate is edition 2024.
- **Linux build dependencies:**

  ```bash
  # Debian / Ubuntu
  sudo apt-get install -y pkg-config libusb-1.0-0-dev libudev-dev
  # Fedora
  sudo dnf install -y pkgconfig libusb1-devel systemd-devel
  # Arch
  sudo pacman -S --needed pkgconf libusb systemd
  ```

- **Optional:** a devcontainer is included — *Reopen in Container* installs the
  toolchain and the libraries above.

## Build and check

```bash
cargo build --release        # target/release/segotep-digital[.exe]
just check                   # cargo fmt --check + clippy -D warnings + tests
just lint                    # clippy alone
just test                    # tests alone
```

## The Windows half does not compile on Linux

Everything Windows-specific is behind `#[cfg(target_os = "windows")]`, so a
Linux `cargo check` never sees it. Two things have bitten this repository:

- **FFI types move between crate releases.** `windows-sys` 0.61 changed PDH
  handles from `isize` to `*mut c_void`, and the monitor silently stopped
  compiling for `x86_64-pc-windows-msvc` while Linux stayed green.
- **WiX only accepts a real version.** The installer refuses anything that is not
  a four-part or semantic version, so a test build cannot pass a commit SHA.

Both are caught by building on a Windows runner, which is what the release
workflow's `commit` target is for:

```bash
gh workflow run release.yml -f target=commit   # builds every platform, publishes nothing
```

Run it before merging anything that touches `src/monitor/windows_native.rs`,
`src/windows_service.rs`, or `installer/`.

## Releases

Releases are cut by `.github/workflows/release.yml`, which takes an optional `ref`:

| Ref | What it does |
| --- | --- |
| *(empty — default, and what the weekly cron uses)* | Gates on `just check`, derives the next version, bumps `Cargo.toml` via `just bump`, pushes the commit and tag together, then builds and publishes |
| a version tag | Rebuilds an existing tag and publishes its release — the repair path |
| a commit SHA | Test-builds a commit: artifacts land in the Actions run, nothing is published |

The version is never typed by hand: it comes from the Conventional Commits since
the last tag (`feat:` → minor, `fix:` → patch, and so on). A week whose commits
are all `docs:`/`ci:`/`chore:` releases nothing, and the run ends green.

`prepare` pushes the tag **before** the build starts, so a build failure leaves a
tag with no release. Repair it by dispatching with that tag as `ref`.

Each release ships:

- `segotep-digital-<version>-x86_64-unknown-linux-musl.tar.xz` and `…-aarch64-unknown-linux-musl.tar.xz` — binary, udev rule, systemd unit, README
- `segotep-digital-v<version>-windows-x64.msi` — installer, registers the service
- `segotep-digital-<version>-x86_64-pc-windows-msvc.zip` — portable build

A weekly run fires every Sunday at 00:00 UTC; the repository is in maintenance
mode, so bug fixes and dependency updates are the only expected changes.

## CI

`.github/workflows/ci.yml` runs one gate — `Check (just check)` — plus four
scans in parallel: Gitleaks (secrets, full history), Semgrep (SAST), Trivy
(vulnerabilities and Dockerfile misconfigurations) and OSV-Scanner
(dependencies). Only the gate is required to merge; the scans report findings
without blocking.
