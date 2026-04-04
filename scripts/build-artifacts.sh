#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-x86_64-unknown-linux-gnu}"
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
ARCH_ALIAS="${2:-$TARGET}"
BIN_NAME="bluetty"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/dist"

if [[ "$TARGET_DIR" = /* ]]; then
  TARGET_BASE_DIR="$TARGET_DIR"
else
  TARGET_BASE_DIR="$ROOT_DIR/$TARGET_DIR"
fi

mkdir -p "$OUT_DIR"

echo "==> Building $BIN_NAME for target: $TARGET"
rustup target add "$TARGET"

if [[ "$TARGET" == "x86_64-unknown-linux-gnu" ]]; then
  cargo build --release --locked
  BIN_PATH="$TARGET_BASE_DIR/release/$BIN_NAME"
else
  if [[ "$TARGET" == "aarch64-unknown-linux-gnu" ]]; then
    if command -v aarch64-linux-gnu-gcc >/dev/null 2>&1; then
      export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
    else
      echo "Missing cross linker: aarch64-linux-gnu-gcc" >&2
      echo "Use the GitHub Actions build workflow, or install the cross linker locally." >&2
      exit 2
    fi
  fi

  cargo build --release --locked --target "$TARGET"
  BIN_PATH="$TARGET_BASE_DIR/$TARGET/release/$BIN_NAME"
fi

if [[ ! -f "$BIN_PATH" ]]; then
  echo "Build artifact not found: $BIN_PATH" >&2
  exit 1
fi

ARTIFACT_BASENAME="${BIN_NAME}-${ARCH_ALIAS}"
cp "$BIN_PATH" "$OUT_DIR/$ARTIFACT_BASENAME"

tar -C "$OUT_DIR" -czf "$OUT_DIR/${ARTIFACT_BASENAME}.tar.gz" "$ARTIFACT_BASENAME"
sha256sum "$OUT_DIR/${ARTIFACT_BASENAME}.tar.gz" > "$OUT_DIR/${ARTIFACT_BASENAME}.tar.gz.sha256"

ls -lh "$OUT_DIR"
echo "==> Done: $OUT_DIR/${ARTIFACT_BASENAME}.tar.gz"
