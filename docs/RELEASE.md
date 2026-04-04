# Release Process

## Tag-driven release (recommended)

1. Ensure working tree is clean and CI is green on main.
2. Create and push a version tag:

```bash
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0
```

3. GitHub Actions release workflow will:
- Build amd64 and arm64 artifacts
- Package deb and rpm outputs
- Upload assets to GitHub Release

## Manual release (workflow_dispatch)

Use Actions -> release.yml -> Run workflow to publish without pushing a new tag.

## Local verification before tagging

```bash
cargo check --locked
scripts/build-artifacts.sh x86_64-unknown-linux-gnu
scripts/build-artifacts.sh aarch64-unknown-linux-gnu arm64
scripts/package-artifacts.sh
cd dist
sha256sum -c *.sha256
```

## Expected release assets

- bluetty-x86_64-unknown-linux-gnu.tar.gz
- bluetty-x86_64-unknown-linux-gnu.tar.gz.sha256
- bluetty-arm64.tar.gz
- bluetty-arm64.tar.gz.sha256
- bluetty-<version>-linux-amd64
- bluetty-<version>-linux-amd64.deb
- bluetty-<version>-linux-amd64.rpm
- bluetty-<version>-linux-arm64
- bluetty-<version>-linux-arm64.deb
- bluetty-<version>-linux-arm64.rpm
