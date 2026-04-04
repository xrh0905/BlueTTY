# CI/CD Guide (GitHub Actions)

This repository uses GitHub Actions workflows under .github/workflows.

## Workflows

1. ci.yml
- Trigger: pull_request, push to main
- Jobs: cargo check, multi-arch build, deb/rpm package
- Output: workflow artifacts from dist

2. release.yml
- Trigger: tag push matching v*, plus manual workflow_dispatch
- Jobs: build + package + GitHub Release asset upload

## Required permissions

- ci.yml: read-only repository access
- release.yml: contents: write

Release notes are generated directly from git history using git-cliff during release workflow execution.

## Local parity with CI

Local commands mirror CI behavior:

```bash
cargo check --locked
scripts/build-artifacts.sh x86_64-unknown-linux-gnu
scripts/build-artifacts.sh aarch64-unknown-linux-gnu arm64
scripts/package-artifacts.sh
```

## Artifact naming

Release workflows publish these standard names:

- bluetty-<version>-linux-amd64
- bluetty-<version>-linux-amd64.deb
- bluetty-<version>-linux-amd64.rpm
- bluetty-<version>-linux-arm64
- bluetty-<version>-linux-arm64.deb
- bluetty-<version>-linux-arm64.rpm
