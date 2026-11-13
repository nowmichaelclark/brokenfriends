# Research: smog, the smoke edge and the detail budget (R7, R14, R15)

From the Research run of 2026-11-14. Check each licence again before
you copy code.

**Note:** the Research run assumed a PS1 look. Mikey says the game is
**not** going for a PS1 look (R15). The PS1 level of detail is only a
budget. Do not add vertex snapping, affine warping or dithering as a
style. The cheap settings below are budget tools, not a look.

## Cheap rendering (budget tools)

- `render_mode vertex_lighting` and simple lighting where they look
  good enough.
- Small textures (32 to 128 px), made in Rust from seeded noise and
  uploaded as `ImageTexture`.
- One shared shader for many objects, so that Godot can batch them.
- On integrated graphics, transparent layers that cover the screen
  (overdraw) cost more than vertices. Keep them few.

## Smog and the smoke edge (Mobile renderer)

The Mobile renderer has no volumetric fog. Use these parts:

1. Environment depth fog with a warm brown colour.
2. Height smog: calculate it in our own shader from world Y. Godot 4
   has one fog colour only, so coloured height smog needs a shader.
3. The smoke edge: a large curved ribbon mesh along the zone edge.
   Use an unshaded, alpha-blended shader with 2 or 3 octaves of
   moving world-space noise and a soft depth fade.
4. Some dozen billboard fog cards with proximity fade near the base of
   the wall. The Godot docs give this recipe.
5. Dust particles near the bear.

## Sources

| Source | Use | Licence |
|---|---|---|
| Godot docs, Volumetric fog, https://docs.godotengine.org/en/stable/tutorials/3d/volumetric_fog.html | Not on Mobile; the fake-fog recipe with quads. | Docs. |
| godot-proposals #5625, https://github.com/godotengine/godot-proposals/issues/5625 | One fog colour only in Godot 4. | Discussion. |
| godotshaders "Smoke or fog", https://godotshaders.com/shader/smoke-or-fog/ | fbm noise smoke. Use 2 to 4 octaves. | CC0 |
| MenacingMecha godot-psx-style-demo, https://menacingmecha.itch.io/godot-psx-style-demo | Simple lighting and vertex colours. Only if Mikey wants it. | MIT |
