# Why the Intel GPU gave 12 fps, and the cuts (S-139)

Status: 2026-11-15. All numbers come from Mikey's playtest laptop: Intel
UHD Graphics (CML GT2, 1050 MHz, on mains power), started with
`--gpu-index 0`, window 1920 x 1023, Mobile renderer, Vulkan. This GPU
is of the class of the speed target (R5).

## The result in short

| | GPU time for each frame |
|---|---|
| Before (build 13829a4, the start view of seed 1) | 75 to 80 ms (12 fps) |
| After, the same view | 12.0 ms |
| After, the Benchmark (seeds 1, 7 and 42) | 12.3 ms average, 70 fps |
| After, the Draw distance test (300, 800, 3000 m, smog) | 11.0, 12.6, 13.1, 12.5 ms |
| After, the NVIDIA GTX 1650 of the same laptop | 1.2 ms |
| After, with the ground scatter of S-108, the Benchmark | 13.4, 12.5, 12.1 ms (seeds 1, 7, 42), 63 to 71 fps |

The scatter came in from S-108 during this row. On seed 1 (the first
zone of the run), the Benchmark with the scatter had hitches: worst
frame 45.8 ms, 1% low 29 fps. Seeds 7 and 42 had none. The cause is
not known yet: maybe the first scatter chunks, or the first compile of
the scatter shaders [unverified]. S-141 looks at it.

The screen of this laptop runs at 144 Hz (`session.json`). With vsync, a frame that needs more
than 13.9 ms waits until 20.8 ms. Thus the 1% low of the Benchmark is
about 49 fps. On a 60 Hz screen, a GPU time of 12 ms gives 60 fps.

## Where the 80 ms went

These numbers are the savings of each cut, measured one after the
other. The order changes the size of each saving.

| Part | Cost | Cut |
|---|---|---|
| The default shader of Terrain3D (fragment) | about 25 ms | The light terrain shader |
| The sun shadow map of 4096 pixels | about 5 to 12 ms | 2048 pixels |
| The sky light map, drawn again each 0.5 s | about 3 ms | Set sky and sun values only when they change |
| Ambient light and reflections from the sky map | about 2.6 ms | One flat ambient colour, no reflections |
| Soft shadow filter | about 0.9 ms | Hard shadows |

What is left (12 ms): the terrain itself (about 6 ms: vertex work,
shadows that it casts, light and fog for each pixel), the sun shadows
(about 4.6 ms in total), the fog (0.9 ms), and the sky, the bear, the
obstacles, the dust, the smoke and the overlay (each 0.5 to 1.3 ms).
With no terrain and no smoke, the frame needs about 5.6 ms.

## Cuts that did not help

- **A smaller `mesh_size` of Terrain3D** (32 or 24 instead of 48).
  The rings then reach past the smog distance (1024 m instead of
  768 m). The extra far ground costs more than the saved triangles.
- **Vertex lighting** (`render_mode vertex_lighting`) on the terrain
  saved less than 0.3 ms.
- **One shadow cascade at 50 m** saved nothing measurable.
- **The terrain casts no shadows** saves about 1 to 2.7 ms. It stays
  on, because cliffs then give no shadow on the ground near the bear.

## How to measure (the method)

1. Start the game on the Intel GPU: `--gpu-index 0`.
2. Godot's `--gpu-profile` prints the GPU time of each render pass.
   On this GPU, the names of the passes were not always correct. For
   example, "Setup Sky" showed 21 ms, but the cut that removed the sky
   saved only 3 ms.
3. To find a part, turn it off and compare the GPU time
   (`RenderingServer.viewport_get_measured_render_time_gpu`). Average
   at least 4 s. Wait 30 s after the start, so that the shader
   pipelines are ready.
4. **Start a new process for each setting.** A change while the game
   runs (for example a new shadow map size) can leave the renderer
   slower for a long time, and the result then depends on the order
   of the changes. Godot 4.4 and later draw with a general shader
   until a special pipeline is compiled [unverified: this is a
   probable cause, not a checked one].
5. Check the result with the Benchmark and the Draw distance test
   (`-- --bf-test=benchmark`).

The probe scripts of this session were short GDScript tests outside
the repo. They are not kept (CLAUDE.md, "Rust first").

## For the next sessions

- **Scatter (S-108, S-141)** costs about 0 to 1.5 ms of GPU time on
  this GPU in the Benchmark. Keep the full frame under about 14 ms.
- **The Vivobook test (S-137)** shows if a newer Intel GPU has the
  same costs. `session.json` now has a `look` object with the shader,
  the shadow and the ambient settings of the run.
- **If more time is necessary,** the next cuts are: the terrain casts
  no shadows (about 2 ms), a shadow map of 1024 pixels (about 1.3 ms),
  and no fog (0.9 ms). Each changes the look, so ask Mikey first.
