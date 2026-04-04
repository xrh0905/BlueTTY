# bluetty

bluetty is a production-oriented bridge between Bluetooth SPP connections and PTY-backed Linux sessions on BlueZ.

BlueTTY is a modern implementation direction for RFCOMM serial workflows, with built-in session orchestration, PTY lifecycle management, and release automation for current Linux environments.

## Why now

Legacy BlueZ RFCOMM-era tooling patterns are increasingly outdated for production operations:

- `rfcomm` style ad-hoc bindings are fragile in service-managed systems.
- `sdptool` style manual SDP workflows are deprecated in modern BlueZ deployments.
- Pairing and trust policy are better handled by native BlueZ tooling (`bluetoothctl`) and policy stack integration.

BlueTTY focuses on stable PTY session delivery, while Bluetooth pairing/authorization is delegated back to the native BlueZ stack.

## Architecture at a glance

1. BlueTTY registers an SPP profile on BlueZ over system D-Bus.
2. BlueZ delivers RFCOMM file descriptors through `Profile1.NewConnection`.
3. BlueTTY creates a PTY pair and runs `session.SubcommandTemplate`.
4. BlueTTY forwards bytes bidirectionally between Bluetooth socket and PTY master.

## v1.0.1 highlights

- Stable 1.0 release line with amd64 and arm64 release assets.
- Tag-driven GitHub Release automation.
- Reworked CI/CD on GitHub Actions (PR checks, build, package, release).
- Release-ready documentation set for build, packaging, deployment, and troubleshooting.

## What it does

- Registers SPP profile through BlueZ ProfileManager1.RegisterProfile.
- Accepts RFCOMM file descriptors from Profile1.NewConnection.
- Creates one PTY per connection and starts one session process per PTY.
- Forwards data bidirectionally between Bluetooth socket and PTY master.
- Supports per-device disconnect and concurrent sessions.
- Uses BlueZ profile registration options for authentication and authorization.
- Supports session modes none, getty, and exec with template-rendered subcommands.

## Quick start

```bash
cargo check --locked
cargo build --release --locked
RUST_LOG=info ./target/release/bluetty
```

Recommended first-run flow:

1. Start from `bluetty.conf.minimal`.
2. Keep `bluez.RequireAuthentication` and `bluez.RequireAuthorization` aligned with your BlueZ trust policy.
3. Choose `session.Mode`:
- `getty` for login shell on PTY
- `exec` for managed custom program
- `none` for notify-only integration hook
4. Validate with `journalctl -u bluetty.service -f` while connecting from a known device.

Configuration lookup order:

1. BLUETTY_CONFIG environment variable path
2. ./bluetty.conf
3. /etc/bluetty/bluetty.conf
4. Built-in defaults

Example configs:

- bluetty.conf.example
- bluetty.conf.minimal

## Build and package artifacts

```bash
chmod +x scripts/build-artifacts.sh
scripts/build-artifacts.sh x86_64-unknown-linux-gnu
scripts/build-artifacts.sh aarch64-unknown-linux-gnu arm64

chmod +x scripts/package-artifacts.sh
scripts/package-artifacts.sh
```

Standard output names in dist:

- bluetty-<version>-linux-amd64
- bluetty-<version>-linux-amd64.deb
- bluetty-<version>-linux-amd64.rpm
- bluetty-<version>-linux-arm64
- bluetty-<version>-linux-arm64.deb
- bluetty-<version>-linux-arm64.rpm

## Run as a service

Installed paths:

- Binary: /usr/local/bin/bluetty
- Config: /etc/bluetty/bluetty.conf
- Unit file: systemd/bluetty.service

Service management:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now bluetty.service
sudo journalctl -u bluetty.service -f
```

## Dependencies

- BlueZ and system D-Bus
- agetty and login binaries for getty mode
- aarch64-linux-gnu-gcc for local arm64 cross builds

## Security and policy
- Pairing, trust, and authorization are enforced by BlueZ.
- Use `bluez.RequireAuthentication=true` and/or `bluez.RequireAuthorization=true` when deployment policy requires device-level control.

## Documentation

- Index: docs/INDEX.md
- Build: docs/BUILD.md
- CI/CD: docs/CI.md
- Release: docs/RELEASE.md
- Docker: docs/DOCKER.md
- Packaging: docs/PACKAGING.md
- Configuration reference: docs/CONFIGURATION.md
- Troubleshooting: docs/TROUBLESHOOTING.md

## Repository layout

- src: application source code
- scripts: build and packaging scripts
- docs: operational and release docs
- packaging: nfpm manifest and lifecycle hooks
- systemd: systemd unit
- .github/workflows: CI/CD automation

## License

This project is licensed under the MIT License. See LICENSE.
