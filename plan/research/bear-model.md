# Research: the bear model in code (R17, S-119)

From the Build session S-119, 2026-11-14. Check each licence again
before you copy code.

## Method that the build uses

- **SDF shapes.** Each part is a smooth union of ellipsoids and round
  cones. The formulas come from Inigo Quilez. His round cone is exact.
  His ellipsoid is a bound: it is correct near the surface only, and
  that is enough for meshing.
- **Meshing.** `fast-surface-nets` makes a closed mesh for each group
  of shapes on a small grid. The grid cell is smaller for small shapes
  (eyes, nose, pads). The crate gives counter-clockwise triangles.
  Godot wants clockwise front faces, so the code swaps two corners.
- **Hidden triangles.** A pad or an eye is a separate mesh piece that
  sinks into its part. The code removes the triangles that are deep
  inside another piece of the same part.
- **Thread and seams.** Thin shapes need very small cells and many
  triangles. Thus, the code makes the mouth and the seam rings as
  simple tubes with 5 sides.
- **Fur without shells.** One shader: triplanar mapping of a 128 px
  fur texture in object space, Lambert wrap diffuse, the built-in
  `RIM`, and a little vertex noise on the outline. Shell fur and hair
  cards cost too much (`bear.md`).

## Sources

| Source | Use | Licence |
|---|---|---|
| Inigo Quilez, distance functions, https://iquilezles.org/articles/distfunctions/ | Round cone, ellipsoid bound, smooth union. | The page gives no licence. We use the formulas, not code. |
| fast-surface-nets-rs, https://github.com/bonsairobo/fast-surface-nets-rs | The mesher. About 20 million triangles per second on one core. | MIT OR Apache-2.0 |
| saft (Embark Studios), https://docs.rs/saft | A Rust SDF compiler and mesher. We do not use it: it is larger than we need. | MIT OR Apache-2.0 |
| Godot BaseMaterial3D docs, https://docs.godotengine.org/en/stable/classes/class_basematerial3d.html | `RIM`, `RIM_TINT`, Lambert wrap and backlight. | MIT |
| Filament cloth model, https://google.github.io/filament/Filament.md.html | Sheen and wrapped diffuse for cloth. An idea for later if `RIM` is not soft enough. | Apache-2.0 [unverified] |
| Blender Studio, cartoon shading, https://studio.blender.org/blog/cartoon-character-shading-with-geometry-nodes | Noise on the rim and the outline. | Article only. |
