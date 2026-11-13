# How the game is built

Status: draft, 2026-11-13. This doc comes from the research on world
generation. Items marked **proposed** need a "yes" from Mikey (see
`decisions.md`).

## The structure (Option A, chosen by Mikey)

- **Surface.** Rust makes a heightmap. A heightmap is a grid of
  ground heights. A clipmap draws it. A clipmap is a set of rings
  around the camera. The rings near the camera have the most detail.
  This is the least expensive smooth terrain, and it makes the
  mountains.
- **Caves (after demo 1).** Caves are a separate layer of smooth
  voxels. Rust makes a graph of caves and changes it into a signed
  distance field (SDF). An SDF gives the distance to the nearest
  surface at each point. Rust makes meshes from the SDF near the
  player (with Surface Nets) and gives them to Godot. Each cave mouth
  is a hole in the surface.
- **Zones.** The world is not endless. It is a set of finite zones.
  Each zone is 2–8 km across, with its centre at the origin. The world
  seed and the zone number fully set the contents of a zone. Erosion
  runs on the full zone at one time, so the chunks have no seams.
- **Biome first.** The biome selects the height function. The height
  does not select the biome. Demo 1 has one biome, so blending between
  biomes comes later.

## Folders

```
game/                 Godot 4.7.1 project (Mobile renderer)
  addons/terrain_3d/  Terrain3D v1.1.0 (proposed). Not in git:
                      tools/install-terrain3d.sh downloads it.
  bf.gdextension      loads the Rust library, reloadable = true
rust/                 one Cargo workspace with one target/ folder
  worldgen/           bf-worldgen: Rust only, no Godot. Noise, biome
                      height functions, erosion. Unit tests and a
                      preview tool that writes PNG images.
  game/               bf-game: the GDExtension (a cdylib). Player,
                      camera, terrain bridge, scatter, debug overlay.
```

`bf-worldgen` does not use Godot. To test the generation without
Godot, run `cargo run --release -p bf-worldgen --example preview --
--seed 7` in `rust/`. In about 5 seconds, it writes a heightmap image,
a shaded-relief image and two views into `rust/preview/`.

## Fast changes in Rust

Mikey wants the game logic in Rust from the start. These items keep
the changes fast.

**The loop:**
1. In a terminal, run `bacon` (or `cargo watch -x build`).
2. Save a `.rs` file. The build starts automatically.
3. Click the Godot editor. The editor loads the new library
   (`reloadable = true`, Godot 4.2 and later).
4. Push F5. The game starts with the new code.

**Most changes need no compile.** Each tuning value is an `#[export]`
field. Examples: walk speed, jump height, climb stamina, fog density,
noise octaves. Change the value in the Inspector. While the game runs,
use the Remote scene tree.

**Build settings** (in `rust/Cargo.toml`):

```toml
[profile.dev]
opt-level = 1
debug = "line-tables-only"   # faster links, still readable panics

[profile.dev.package."*"]
opt-level = 3                # godot, noise etc.: optimized once, cached

[profile.dev.package.bf-worldgen]
opt-level = 3                # generation must be fast even in dev

[profile.release]
lto = "thin"
codegen-units = 1
strip = true
```

**Notes:**
- There are two crates. Thus, a change to the gameplay compiles only
  `bf-game`, not the generator.
- On x86-64 Linux, Rust uses the fast `lld` linker by default (since
  version 1.90). The `mold` linker is optional. Measure before you
  change the linker.
- The first build compiles all of `godot` and takes some minutes.
  Small changes after that will probably take a few seconds
  [unverified: S-101 measures this on Mikey's laptop].
- Hot reload has limits. If you change the fields or the base class of
  a class, restart the editor (Project → Reload Current Project).

## Calls from Rust to Terrain3D

godot-rust has typed bindings for Godot's own classes only. Terrain3D
is a different extension. Thus, Rust calls Terrain3D methods by name
(`Object::call`).

Rust makes the heightmap as a Godot `Image` with 32-bit float values
(`FORMAT_RF`). Then Rust calls `Terrain3D.data.import_images([height,
null, null], corner, 0.0, 1.0)`. The corner is the world position of
the first sample. S-101 tested this with Terrain3D v1.1.0. A region is
one square tile of Terrain3D data. A zone of 2048 samples uses 2 × 2
regions of size 1024. All of these calls are in
`rust/game/src/terrain.rs`.

If this is too difficult, use the fallback: our own clipmap. It is a
few ring meshes and a shader that reads a height texture. Rust owns
all of it. This is more work, but it removes the C++ dependency.

## Rules for speed

- **Renderer.** Use Mobile (proposed). On integrated graphics, it is
  lighter than Forward+. If the Intel Vulkan driver fails, use
  Compatibility. Do not use SDFGI, volumetric fog or SSR. Mobile does
  not have them.
- **Render scale.** Set the 3D render scale to 0.75 with FSR 1. The UI
  stays sharp at full resolution.
- **Time for each frame.** The limit is 16.6 ms in total. Terrain gets
  4 ms at most. Scatter gets 4 ms at most. All other work gets 5 ms at
  most.
- **Draw calls.** Use a few hundred at most. A draw call is one
  request to the GPU to draw an object. Draw each repeated rock and
  shrub with MultiMesh (one draw call for many copies). Use visibility
  ranges to remove far props.
- **Shadows.** Use one sun with 2 cascades. Set the shadow distance to
  about 100 m. Turn off soft shadows.
- **Threads.** Run generation and meshing on Rust worker threads
  (`rayon`). The main thread only receives finished data.
- **The haze is the draw distance.** The smog hides all objects more
  than a few hundred metres away. Thus, the look and the speed limit
  are the same decision.
- **Debug overlay (F3).** It shows fps, frame time in ms, draw calls,
  primitives and video memory. The data comes from the `Performance`
  singleton of Godot.

## Builds on GitHub

Mikey plays builds that GitHub makes (decision R10). The workflow
`.github/workflows/game-build.yml` does these steps after each push to
`main`:

1. Build the Rust library in release mode on Linux and on Windows.
2. Install the pinned Terrain3D addon.
3. Export the game for Linux and Windows with Godot 4.7.1 and
   `game/export_presets.cfg`.
4. Start the Linux build headless. Make sure that it makes a zone.
5. Put the two zips on the `game-latest` release.

The editor loads the library from the debug path. Thus, the workflow
copies the release library to `rust/target/debug/` before the import.
If the editor cannot load the library, the export loses the Rust
classes.

## Tests in Claude Code

Cloud containers usually have no GPU, and some have no Vulkan.
- To check that the game loads, use `--headless`.
- To make screenshots, use `xvfb-run` with
  `--rendering-method gl_compatibility`. This renderer looks different
  from Mobile. Write that in the summary.
- Measure speed only on Mikey's laptops.

## Crates

| Need | Crate |
|---|---|
| Godot bindings | `godot` (pin the version; the 2D build used 0.5.5) |
| Noise | our own gradient noise in `bf-worldgen` (`noise.rs`). It has derivatives and gives the same bits on all computers. |
| Erosion look | our port of the erosion filter by Rune Skovbo Johansen (`erosion_filter.rs`, MPL-2.0, from the CPU code in `bevy_erosion_filter`) |
| Threads | `rayon` |
| Maths | `glam` (not used yet). `detmath.rs` has `sin`, `cos` and `exp` that give the same bits on all computers. |
| Cave meshes (later) | `fast-surface-nets` |
| Preview images | `image` (dev dependency only) |

## More to read

- Erosion filter: https://blog.runevision.com/2026/03/fast-and-gorgeous-erosion-filter.html
- Terrain3D: https://github.com/TokisanGames/Terrain3D
- The godot-rust book: https://godot-rust.github.io/book/
- Sebastian Lague, hydraulic erosion: https://www.youtube.com/watch?v=eaXk97ujbPQ
- World generation in Veloren (Rust, GPL licence; read it, do not copy it): https://blog.jsbarretto.com/post/veloren
