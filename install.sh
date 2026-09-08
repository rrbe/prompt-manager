#!/usr/bin/env bash
set -euo pipefail

BASE="https://github.com/rrbe/prompt-manager/releases"
INSTALL_DIR="${PM_INSTALL_DIR:-/usr/local/bin}"

case "$(uname -s)" in
  Darwin) os=apple ;;
  Linux) os=linux ;;
  *) echo "Unsupported OS; pm releases support macOS and Linux." >&2; exit 1 ;;
esac

case "$(uname -m)" in
  arm64|aarch64) arch=arm ;;
  x86_64|amd64) arch=intel ;;
  *) echo "Unsupported architecture; pm releases support ARM64 and x86_64." >&2; exit 1 ;;
esac

if command -v shasum >/dev/null 2>&1; then
  checksum=(shasum -a 256 -c)
elif command -v sha256sum >/dev/null 2>&1; then
  checksum=(sha256sum -c)
else
  echo "Install shasum or sha256sum to verify the download." >&2
  exit 1
fi

release_url="$(curl -fsSL -o /dev/null -w '%{url_effective}' "${BASE}/latest")"
version="${release_url##*/}"
binary="pm-${version}-${os}-${arch}"
download="${BASE}/download/${version}"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

echo "Downloading ${binary}..." >&2
curl -fsSL -o "${tmp_dir}/${binary}" "${download}/${binary}"
curl -fsSL -o "${tmp_dir}/${binary}.sha256" "${download}/${binary}.sha256"
(cd "$tmp_dir" && "${checksum[@]}" "${binary}.sha256") >&2

if ! mkdir -p "$INSTALL_DIR" || ! install -m 0755 "${tmp_dir}/${binary}" "${INSTALL_DIR}/pm"; then
  echo "Cannot install to ${INSTALL_DIR}. Use PM_INSTALL_DIR=\"\$HOME/.local/bin\" for a user installation." >&2
  exit 1
fi

echo "Installed pm to ${INSTALL_DIR}/pm" >&2
case ":${PATH}:" in
  *:"${INSTALL_DIR}":*) ;;
  *) echo "Add ${INSTALL_DIR} to your PATH to run pm." >&2 ;;
esac
"${INSTALL_DIR}/pm" --version
