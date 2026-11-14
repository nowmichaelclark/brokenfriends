#!/usr/bin/env bash
# The full end-of-row check for a Claude Code session (CLAUDE.md, "End
# of every agenda row", steps 1 and 2). It installs what is missing:
# Godot 4.7.1 (into tools/.cache/) and the Terrain3D addon.
#
#   tools/check.sh          all checks
#   tools/check.sh --quick  only cargo fmt, clippy and test
#
# It exits with an error if a check fails or if the game prints an
# error when it loads.
set -euo pipefail

GODOT_VERSION="4.7.1"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cache="${root}/tools/.cache"

step() { echo; echo "== $* =="; }

step "cargo fmt, clippy and test"
cd "${root}/rust"
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

if [[ "${1:-}" == "--quick" ]]; then
  echo; echo "Quick check passed."
  exit 0
fi

step "cargo build (debug library for the editor)"
cargo build -p bf-game

step "Godot ${GODOT_VERSION}"
godot_bin="${GODOT:-}"
if [[ -z "${godot_bin}" ]] && command -v godot >/dev/null \
   && godot --version 2>/dev/null | grep -q "^${GODOT_VERSION}\.stable"; then
  godot_bin="$(command -v godot)"
fi
if [[ -z "${godot_bin}" ]]; then
  godot_bin="${cache}/godot-${GODOT_VERSION}"
  if [[ ! -x "${godot_bin}" ]]; then
    echo "Downloading Godot ${GODOT_VERSION} into tools/.cache/ ..."
    mkdir -p "${cache}"
    tmp="$(mktemp -d)"
    curl -fsSL -o "${tmp}/godot.zip" \
      "https://github.com/godotengine/godot/releases/download/${GODOT_VERSION}-stable/Godot_v${GODOT_VERSION}-stable_linux.x86_64.zip"
    unzip -q "${tmp}/godot.zip" -d "${tmp}"
    mv "${tmp}/Godot_v${GODOT_VERSION}-stable_linux.x86_64" "${godot_bin}"
    chmod +x "${godot_bin}"
    rm -rf "${tmp}"
  fi
fi
"${godot_bin}" --version

step "Terrain3D addon"
if [[ ! -f "${root}/game/addons/terrain_3d/terrain.gdextension" ]]; then
  "${root}/tools/install-terrain3d.sh"
else
  echo "Installed."
fi

step "Import (the first run after a clone)"
if [[ ! -d "${root}/game/.godot/imported" ]]; then
  # Godot 4.7.1 can crash when it quits after an import. The import is
  # still complete, so the exit code is not checked.
  timeout 300 "${godot_bin}" --headless --path "${root}/game" --import >/dev/null 2>&1 || true
fi
echo "Done."

step "Headless load (300 frames)"
log="$(mktemp)"
set +e
timeout 180 "${godot_bin}" --headless --path "${root}/game" --quit-after 300 >"${log}" 2>&1
code=$?
set -e
tail -n 5 "${log}"
if [[ ${code} -ne 0 ]]; then
  echo "The game did not quit normally (exit code ${code}). Full log: ${log}" >&2
  exit 1
fi
if grep -E "^(ERROR|SCRIPT ERROR)" "${log}"; then
  echo "The game printed errors when it loaded. Full log: ${log}" >&2
  exit 1
fi
rm -f "${log}"

echo; echo "All checks passed."
