# Build Guide

## Native build

```bash
cargo check --locked
cargo build --release --locked
```

## Generate release artifacts

```bash
chmod +x scripts/build-artifacts.sh
scripts/build-artifacts.sh x86_64-unknown-linux-gnu
scripts/build-artifacts.sh aarch64-unknown-linux-gnu arm64
```

Artifacts are generated in dist:

- bluetty-<arch>
- bluetty-<arch>.tar.gz
- bluetty-<arch>.tar.gz.sha256

## Cross compile notes

- Local arm64 cross-build requires aarch64-linux-gnu-gcc.
- If local toolchain is unavailable, use the GitHub Actions build workflow.
- CI produces amd64 and arm64 release-ready artifacts.
