# Research: the smoke edge (S-115, R14)

From the build of S-115. The cloud container has no GPU. It rendered
the pictures with the Compatibility renderer and a software GL driver.
Nobody has seen the Mobile renderer yet [unverified].

## What the build does

- **Shape.** A sum of 5 waves around the circle for each of 4 fields:
  the front radius, the height, the band width and the density. Each
  wave has a whole number of lobes (2, 3, 5, 8, 13), a phase from the
  seed, and a speed. The wall closes with no seam. `bf-worldgen`,
  `smoke.rs`.
- **Speed.** Each wave moves the front at 0.35 m/s at most. The waves
  add up. A test over 30 seeds shows a mean of less than 0.5 m/s and a
  top speed of less than 1.4 m/s (a walk). The first version used a
  wave period of 120 s, and the front moved at up to 10 m/s. A bear
  cannot run from that.
- **Rule.** A timer counts the seconds in the smoke. At 12 s, or at a
  depth of 45 m, the bear dies. Outside, the timer goes down twice as
  fast. A bear higher than the top of the wall is not in the smoke.
  `bf-game`, `smoke_dose.rs`.
- **Look.** Two vertical sheets (the front and the back of the band)
  and 96 billboard cards between them. 3 draw calls. The shape goes to
  the GPU as a 256 x 2 float texture, 20 times each second. The vertex
  shader moves the sheets and the cards. Rust and the GPU use the same
  numbers.
- **Fog.** In the smoke, the environment fog gets darker and thicker
  (colour and density). The change follows the danger, so the screen
  shows that death is near.

## Cost (to measure on Mikey's laptop)

- The wall is overdraw: transparent layers that cover the screen. On
  integrated graphics, this costs more than the 3 draw calls. The
  worst case is the bear in the band, with cards close to the camera.
  Cards fade out near the camera.
- The texture write is 4 KB each 50 ms (about 0.1 ms on the CPU).
- The `card_count` and `card_size_m` values on `SmokeEdge` are the
  first things to cut. `show_wall` turns off all drawing.

## Sources

| Source | Use | Licence |
|---|---|---|
| `smog-and-budget.md` (Research run, 2026-11-14) | The recipe: sheet ribbon, cards, own height smog. | Docs. |
| Godot docs, spatial shader `POSITION` and `INSTANCE_CUSTOM` | The cards write the clip position and read their data from the multimesh. | Docs. |

No outside code was copied.
