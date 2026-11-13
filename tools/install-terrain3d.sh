#!/usr/bin/env bash
# Installs the pinned version of the Terrain3D addon into game/addons/.
# Run this one time after you clone the repo: tools/install-terrain3d.sh
set -euo pipefail

VERSION="v1.1.0-stable"
SHA256="51b78bc758f5759cbc8eaaab14e5cf4e8661717e265fdd760f5abf945f15b243"
URL="https://github.com/TokisanGames/Terrain3D/releases/download/${VERSION}/Terrain3D_${VERSION}.zip"

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dest="${root}/game/addons/terrain_3d"
tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT

echo "Downloading Terrain3D ${VERSION} ..."
curl -fsSL -o "${tmp}/terrain3d.zip" "${URL}"
echo "${SHA256}  ${tmp}/terrain3d.zip" | sha256sum -c - >/dev/null \
  || { echo "The checksum of the download is wrong. Nothing was installed." >&2; exit 1; }

unzip -q "${tmp}/terrain3d.zip" 'addons/terrain_3d/*' -d "${tmp}"
rm -rf "${dest}"
mkdir -p "$(dirname "${dest}")"
mv "${tmp}/addons/terrain_3d" "${dest}"
echo "${VERSION}" > "${dest}/BF_PINNED_VERSION"
echo "Terrain3D ${VERSION} is installed in game/addons/terrain_3d."
