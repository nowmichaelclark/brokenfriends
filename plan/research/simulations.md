# Research: dynamic world simulations (later)

From the Research run of 2026-11-14. For sand, avalanches, water,
lava, goo and weather. Not in demo 1. Check each licence again before
you copy code.

## Method to use

- **One framework in Rust:** a grid of 32 x 32 or 64 x 64 cell chunks
  in a window of about 128 to 256 m around the bear. Each chunk has an
  "active" flag and a dirty rectangle. Only active cells update.
- **Threads:** `rayon` with a 4-colour checkerboard of chunks, so that
  two threads never write into the same area (the Noita method).
- **Layers for each cell:** rock, loose material (sand, scree), water,
  goo or lava, temperature, wind.
  - Avalanches and thermal erosion: move loose material to the lowest
    neighbour when the slope is above the talus angle.
  - Water, waterfalls, lava, goo: the virtual-pipes model (Mei et al.
    2007), with a viscosity for each material. Lava cools into rock.
  - Blowing sand: move loose material with a coarse wind field.
  - Tumbling rocks: a few Jolt rigid bodies, removed when they stop.
  - Weather: a very coarse field (for example 16 m cells) of
    temperature, humidity and smog, moved by the wind.
- **Rate:** 10 to 20 updates each second, not each frame.
- **Back to the terrain:** if a height changes by more than about
  2 cm, call Terrain3D `set_height`. Batch the edits, then call
  `update_maps(TYPE_HEIGHT, false)` one time for each update. Dynamic
  collision near the camera is the default and the fastest mode.
- Draw water, lava and goo as our own mesh layer, not in Terrain3D.

## Sources

| Source | Use | Licence |
|---|---|---|
| Petri Purho, "Exploring the Tech and Design of Noita", GDC 2019 | 64 x 64 chunks, dirty rectangles, checkerboard threads. | Talk. |
| TanTanDev/3d_celluar_automata, https://github.com/TanTanDev/3d_celluar_automata | 3D cellular automata in Rust, threading. | MIT OR Apache-2.0 |
| Mei, Decaudin and Hu 2007, "Fast Hydraulic Erosion Simulation and Visualization on GPU" | The virtual-pipes model. | Paper. |
| bshishov/UnityTerrainErosionGPU, https://github.com/bshishov/UnityTerrainErosionGPU | All formulas for water, hydraulic and thermal erosion in the README. | Not confirmed. Use the maths only. |
| Terrain3D docs: Terrain3DData and Collision, https://terrain3d.readthedocs.io/ | Runtime edits and dynamic collision. | Terrain3D. |
