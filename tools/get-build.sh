#!/usr/bin/env bash
# Downloads the newest game build from the game-latest release and
# unzips it. Mikey plays this build (R10). Needs the GitHub CLI (gh).
#
#   tools/get-build.sh            Linux build into ~/Games
#   tools/get-build.sh DIR        Linux build into DIR
#
# The playtests stay: unzip replaces the game files only.
set -euo pipefail

dest="${1:-${HOME}/Games}"
mkdir -p "${dest}"
tmp="$(mktemp -d)"
gh release download game-latest --repo nowmichaelclark/brokenfriends \
  --pattern broken-friends-linux.zip --dir "${tmp}"
unzip -oq "${tmp}/broken-friends-linux.zip" -d "${dest}"
rm -rf "${tmp}"
echo "Build $(cat "${dest}/broken-friends-linux/BUILD.txt") is in ${dest}/broken-friends-linux"
echo "Start it: ${dest}/broken-friends-linux/BrokenFriends.x86_64"
echo "Playtests: ${dest}/broken-friends-linux/playtests/"
