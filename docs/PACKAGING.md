# Packaging Guide

This project uses nFPM to generate:

- Debian package (`.deb`)
- Rocky Linux compatible RPM package (`.rpm`)

## Build packages

```bash
chmod +x scripts/package-artifacts.sh
scripts/package-artifacts.sh
# Or package only one arch
scripts/package-artifacts.sh amd64
scripts/package-artifacts.sh arm64
```

Output files are placed in `dist/`.

Standard output names:

- `bluetty-<version>-linux-<arch>` (copied raw binary)
- `bluetty-<version>-linux-<arch>.deb`
- `bluetty-<version>-linux-<arch>.rpm`

Supported package arches:

- `amd64`
- `arm64`

## Requirements

- `dist/bluetty-x86_64-unknown-linux-gnu`, or
- `dist/bluetty-amd64`, or
- `dist/bluetty-aarch64-unknown-linux-gnu`, or
- `dist/bluetty-arm64`, or
- `dist/bluetty-aarch64`, or
- `target/release/bluetty`

For arm64 packaging from local target dir, ensure `target/aarch64-unknown-linux-gnu/release/bluetty` exists.

The script will bootstrap nFPM automatically into `.tools/`.
