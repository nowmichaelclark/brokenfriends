# How the game is built

Status: draft, written 2026-11-13 from the world-generation research.
Anything marked **proposed** still needs Mikey's yes (`decisions.md`).

## The shape of it (Option A, Mikey's choice)

- **Surface:** a heightmap, made in Rust and drawn with a clipmap.
  The detail is highest near the camera and coarser further out.
  This is the cheapest smooth terrain there is, and it's where the
  mountains come from.
- **Caves (after demo 1):** a separate smooth-voxel layer. Rust builds
  a cave graph, turns it into a signed distance field (SDF), meshes it
  near the player (Surface Nets) and hands the meshes to Godot. Cave
  mouths are holes cut in the surface.
- **Zones, not an endless world:** each zone is a finite map, about
  2–8 km across and centred on the origin. A zone is a pure function
  of (world seed, zone id). Erosion runs over the whole zone at once,
  so chunks never have seams.
- **Biome first:** the biome picks the height function, not the other
  way round. Demo 1 has one biome, so blending between biomes comes
  later.

## Layout

```
game/                 Godot 4.7.1 project (Mobile renderer)
  addons/terrain3d/   Terrain3D (proposed), pinned version
  bf.gdextension      loads the Rust library, reloadable = true
rust/                 one Cargo workspace, one target/ folder
  worldgen/           bf-worldgen: pure Rust, no Godot. Noise, biome
                      height functions, erosion. Unit tests and a
                      preview tool that writes PNGs.
  game/               bf-game: the GDExtension (cdylib). Player,
                      camera, terrain bridge, scatter, debug overlay.
```

`bf-worldgen` never depends on Godot. You can test and tune
generation with `cargo run -p bf-worldgen --example preview -- --seed 7`,
which writes a heightmap and a shaded-relief PNG in about a second,
without opening Godot at all.

## Fast iteration with Rust (Mikey asked for Rust first)

**The loop.** Run `bacon` (or `cargo watch -x build`) in a terminal.
Save a `.rs` file and it builds. Click back into the Godot editor and
the extension reloads (`reloadable = true`, Godot 4.2+). Press F5 and
the game runs with the new code.

**Most tweaks need no compile at all.** Every tuning number (walk
speed, jump height, climb stamina, fog density, noise octaves) is an
`#[export]` field. You change it in the Inspector, or while the game
runs, through the Remote scene tree.

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
- Splitting the crates means a gameplay change rebuilds only
  `bf-game`, not the generator.
- On x86-64 Linux, Rust uses the fast `lld` linker by default
  (since 1.90). `mold` is optional, so measure first.
- The first build compiles all of `godot` and takes a few minutes.
  After that, small changes should rebuild in seconds
  [unverified: S-101 measures it on Mikey's laptop].
- Hot reload has limits. Changing a class's fields or base class can
  still need an editor restart (Project → Reload Current Project).

## Talking to Terrain3D from Rust

godot-rust has typed bindings only for Godot's own classes. Terrain3D
is another extension, so Rust calls it by method name (`Object::call`).

The plan is for Rust to build the height map as a Godot `Image`
(32-bit float) and hand it to Terrain3D's import function
[unverified: exact method names and region size, S-101 checks them
against the pinned version].

If this turns out painful, the fallback is our own clipmap: a few
ring meshes plus a height-texture shader, all owned by Rust. That's
more work, but it removes the C++ dependency.

## Speed rules

- **Renderer:** Mobile (proposed). It's lighter than Forward+ on
  integrated graphics. Compatibility is the fallback for broken Intel
  Vulkan drivers. No SDFGI, volumetric fog or SSR, since Mobile
  doesn't have them anyway.
- **3D render scale 0.75** with FSR 1. The UI stays sharp at full
  resolution.
- **Budget per frame:** 16.6 ms in total. Terrain should take at most
  4 ms, scatter at most 4 ms, and everything else at most 5 ms.
- **Draw calls:** a few hundred at most. Use MultiMesh for every
  repeated rock and shrub, and visibility ranges to cut props by
  distance.
- **Shadows:** one sun, 2 cascades, about 100 m shadow distance, soft
  shadows off.
- **Threads:** generation and meshing run on Rust worker threads
  (`rayon`). The main thread only receives finished data.
- **The haze is the draw distance.** The smog hides everything beyond
  a few hundred metres, so the look and the speed budget are the same
  decision.
- **Debug overlay (F3):** FPS, frame ms, draw calls, primitives and
  video memory, from Godot's `Performance` singleton.

## Testing in Claude Code

Cloud containers usually have no GPU and may have no Vulkan. Headless
runs (`--headless`) check that things load. For screenshots, use
`xvfb-run` with `--rendering-method gl_compatibility`. It looks
different from Mobile, so say so in the summary. Speed is measured
only on Mikey's laptops.

## Crates

| Need | Crate |
|---|---|
| Godot bindings | `godot` (pin it; the 2D build used 0.5.5) |
| Noise | `fastnoise-lite` or `noise` |
| Erosion look | port of Rune Skovbo Johansen's erosion filter (CPU code in `bevy_erosion_filter`) |
| Threads | `rayon` |
| Maths | `glam` |
| Cave meshing (later) | `fast-surface-nets` |
| Preview PNGs | `image` (dev-dependency only) |

## Further reading

- Erosion filter: https://blog.runevision.com/2026/03/fast-and-gorgeous-erosion-filter.html
- Terrain3D: https://github.com/TokisanGames/Terrain3D
- godot-rust book: https://godot-rust.github.io/book/
- Sebastian Lague, hydraulic erosion: https://www.youtube.com/watch?v=eaXk97ujbPQ
- Veloren's world generation (Rust, GPL; read it, don't copy it): https://blog.jsbarretto.com/post/veloren
