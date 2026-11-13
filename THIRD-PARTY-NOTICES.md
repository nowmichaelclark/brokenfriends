# Third-party notices

Broken Friends has the MIT licence (see `LICENSE`). The items below
have a different licence or come from a different author.

## Erosion filter (one source file, MPL-2.0)

`rust/worldgen/src/erosion_filter.rs` has the Mozilla Public License
2.0 (https://mozilla.org/MPL/2.0/). The licence applies to this file
only. If you change this file and distribute the game, you must make
the source of this file available under the same licence.

The file is a port of:

- **Rune Skovbo Johansen**, "Advanced Terrain Erosion Filter" (2025).
  Shadertoy: https://www.shadertoy.com/view/wXcfWn. Blog:
  https://blog.runevision.com/2026/03/fast-and-gorgeous-erosion-filter.html
- through the CPU code of the **`bevy_erosion_filter`** crate, version
  0.3.0 (MPL-2.0).
- Earlier work: **Fewes**, "Terrain Erosion Noise" (2023), and
  **Clay John**, "Eroded Terrain Noise" (2020).

## Terrain3D (addon, MIT)

The install scripts in `tools/` download Terrain3D v1.1.0 by Cory
Petkovsek and Roope Palmroos (https://github.com/TokisanGames/Terrain3D).
The addon is not in this repo.

## godot-rust (crate, MPL-2.0)

`bf-game` uses the `godot` crate (https://github.com/godot-rust/gdext)
without changes.
