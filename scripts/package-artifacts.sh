#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist"
TOOLS_DIR="$ROOT_DIR/.tools"
NFPM_VERSION="2.41.3"
WORK_BIN="$DIST_DIR/bluetty"

mkdir -p "$DIST_DIR" "$TOOLS_DIR"

chmod +x "$ROOT_DIR/packaging/postinstall.sh" "$ROOT_DIR/packaging/postremove.sh"

NFPM_BIN="$TOOLS_DIR/nfpm"
if [[ ! -x "$NFPM_BIN" ]]; then
  echo "==> Installing nfpm v$NFPM_VERSION"
  ARCH=$(uname -m)
  case "$ARCH" in
    x86_64) NFPM_ARCH="x86_64" ;;
    aarch64) NFPM_ARCH="arm64" ;;
    *) echo "Unsupported host arch for nfpm bootstrap: $ARCH" >&2; exit 1 ;;
  esac

  URL="https://ghfast.top/https://github.com/goreleaser/nfpm/releases/download/v${NFPM_VERSION}/nfpm_${NFPM_VERSION}_Linux_${NFPM_ARCH}.tar.gz"
  TMP_TAR="$TOOLS_DIR/nfpm.tar.gz"
  curl -fsSL "$URL" -o "$TMP_TAR"
  tar -C "$TOOLS_DIR" -xzf "$TMP_TAR" nfpm
  rm -f "$TMP_TAR"
  chmod +x "$NFPM_BIN"
fi

VERSION="${GITHUB_REF_NAME:-$(grep '^version = ' "$ROOT_DIR/Cargo.toml" | head -n1 | sed -E 's/version = "([^"]+)"/\1/' | tr -d '[:space:]')}"
VERSION="${VERSION#v}"
export VERSION

if [[ "$#" -gt 0 ]]; then
  TARGET_ARCHES=("$@")
else
  TARGET_ARCHES=(amd64 arm64)
fi

echo "==> Packaging version $VERSION"

pick_binary() {
  local arch="$1"
  case "$arch" in
    amd64)
      for p in \
        "$DIST_DIR/bluetty-x86_64-unknown-linux-gnu" \
        "$DIST_DIR/bluetty-amd64" \
        "$ROOT_DIR/target/release/bluetty"; do
        if [[ -f "$p" ]]; then
          echo "$p"
          return 0
        fi
      done
      ;;
    arm64)
      for p in \
        "$DIST_DIR/bluetty-aarch64" \
        "$DIST_DIR/bluetty-arm64" \
        "$DIST_DIR/bluetty-aarch64-unknown-linux-gnu" \
        "$ROOT_DIR/target/aarch64-unknown-linux-gnu/release/bluetty"; do
        if [[ -f "$p" ]]; then
          echo "$p"
          return 0
        fi
      done
      ;;
  esac

  return 1
}

for arch in "${TARGET_ARCHES[@]}"; do
  case "$arch" in
    amd64|arm64) ;;
    *)
      echo "Unsupported package arch: $arch (supported: amd64 arm64)" >&2
      exit 3
      ;;
  esac

  BIN_SRC="$(pick_binary "$arch" || true)"
  if [[ -z "$BIN_SRC" ]]; then
    echo "No binary found for arch=$arch. Expected one of prebuilt binaries in dist/ or target/." >&2
    exit 2
  fi

  cp "$BIN_SRC" "$WORK_BIN"
  chmod +x "$WORK_BIN"

  STD_BIN_NAME="bluetty-${VERSION}-linux-${arch}"
  cp "$BIN_SRC" "$DIST_DIR/$STD_BIN_NAME"
  chmod +x "$DIST_DIR/$STD_BIN_NAME"

  export PKG_ARCH="$arch"
  echo "==> Packaging arch=$arch from $BIN_SRC"

  PKG_OUT_DIR="$(mktemp -d "$DIST_DIR/.nfpm-${arch}-XXXXXX")"
  trap 'rm -rf "$PKG_OUT_DIR"' EXIT

  "$NFPM_BIN" package --config "$ROOT_DIR/packaging/nfpm.yaml" --packager deb --target "$PKG_OUT_DIR/"
  "$NFPM_BIN" package --config "$ROOT_DIR/packaging/nfpm.yaml" --packager rpm --target "$PKG_OUT_DIR/"

  mapfile -t GENERATED_DEBS < <(find "$PKG_OUT_DIR" -maxdepth 1 -type f -name '*.deb' | sort)
  mapfile -t GENERATED_RPMS < <(find "$PKG_OUT_DIR" -maxdepth 1 -type f -name '*.rpm' | sort)

  if [[ "${#GENERATED_DEBS[@]}" -ne 1 || "${#GENERATED_RPMS[@]}" -ne 1 ]]; then
    echo "Failed to detect generated package files for arch=$arch in $PKG_OUT_DIR" >&2
    echo "deb count=${#GENERATED_DEBS[@]}, rpm count=${#GENERATED_RPMS[@]}" >&2
    exit 4
  fi

  cp "${GENERATED_DEBS[0]}" "$DIST_DIR/bluetty-${VERSION}-linux-${arch}.deb"
  cp "${GENERATED_RPMS[0]}" "$DIST_DIR/bluetty-${VERSION}-linux-${arch}.rpm"

  rm -rf "$PKG_OUT_DIR"
  trap - EXIT
done

ls -lh "$DIST_DIR"/*.deb "$DIST_DIR"/*.rpm
