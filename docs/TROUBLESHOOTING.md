# Troubleshooting

This page covers common runtime and packaging issues and practical checks.

## Session closes immediately after connect

Symptoms:

- bluetooth session is accepted then quickly closed
- logs show peer closed soon after startup

Checks:

- confirm PTY slave descriptor is held during session lifetime by current runtime
- verify target process started successfully in getty or exec mode
- check journal logs for session child exit status

Commands:

```bash
sudo journalctl -u bluetty.service -n 200 --no-pager
```

## agetty or login not found

Symptoms:

- session child fails on spawn

Checks:

- verify command binaries referenced by session.SubcommandTemplate
- verify binaries exist and are executable

Commands:

```bash
ls -l /sbin/agetty /bin/login
```

## Cross build fails for arm64

Symptoms:

- build script exits with missing linker message

Checks:

- install aarch64-linux-gnu-gcc for local cross builds
- otherwise use GitHub Actions build and release workflows

## Packaging fails to find generated files

Symptoms:

- package step exits after nfpm run

Checks:

- current script packages into per-arch temporary output and expects exactly one deb and one rpm
- inspect nfpm output and packaging config
- verify packaging/nfpm.yaml is valid for target arch

Commands:

```bash
bash -n scripts/package-artifacts.sh
scripts/package-artifacts.sh amd64
```

## Service starts but no bluetooth handling

Symptoms:

- service active but no expected profile behavior

Checks:

- verify profile path and uuid values in config
- verify bluez is running and system dbus reachable
- verify logs for ProfileManager1 registration success

Commands:

```bash
sudo systemctl status bluetooth
sudo systemctl status bluetty
sudo journalctl -u bluetty.service -f
```

## Dist directory confusion with old artifacts

Symptoms:

- mixed versions in dist make release upload error-prone

Checks:

- keep standardized files for intended version only during release cut
- verify version in Cargo.toml and generated filenames match

Commands:

```bash
grep '^version = ' Cargo.toml
ls -lh dist
```
