# Research: obstacles (R23)

From the Build session S-127, 2026-11-14. This session did not search
the web [unverified against other games]. The design sources are
`plan/obstacles.md` (Mikey's Fall Guys article) and the methods below.

## Method that the build uses

- **Places from the terrain.** A coarse grid (4 m cells) of the
  finished heightmap. Flat ground: 16 height samples around a point.
  Gullies: the flow of rain (`drainage.rs`) with an area of 600 to
  20 000 m², followed downhill for 12 cells. Passes: higher ground on
  two opposite sides at 12 m, level ground along the other axis.
  Ledges: a ray of height samples from flat ground to a face of 48
  degrees or more with a flat top.
- **Selection.** A hash of the cell and the seed gives a random order.
  A distance rule keeps obstacles apart. The cost is about 0.1 s for
  each zone.
- **Motion without physics bodies.** The gusts and the vents are
  functions of the time. A stone is a point that follows the slope of
  the heightmap. A hit is a distance test. Thus the same steps always
  give the same result, and unit tests cover the logic.
- **Wind as a velocity.** The wind is added to the speed that the bear
  goes to. A force does not work: the ground control of the bear
  (24 m/s² to stop) removes it in one step.
- **One MultiMesh for all puffs.** Soft cards that look at the camera,
  moved in Rust. The cost is 1 draw call.

## Open points

- A pit needs the heightmap. Smaller holes need a different method
  (a mesh layer, or the SDF layer of the caves in demo 2).
- Jolt bodies for stones cost more, but they bounce and stack. Look
  at this again when S-114 has the ragdoll.
