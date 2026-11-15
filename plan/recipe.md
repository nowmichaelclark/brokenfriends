# The recipe: how a seed chooses a zone

Status: spec from S-210 (Fable 5.2, 2026-11-15). It is the foundation
for "procedural almost everything" (`vision.md`). The structure is a
technical choice and is fixed here. The four questions in "Questions
for Mikey" are taste and scope. Until Mikey answers, the rows build
the recommended option and write it in the "Assumed" list of
`game/README.md`.

This spec comes from the code (`rust/`), not only from the plans. The
line numbers are from commit da01f88.

## The idea in short

A **recipe** is the list of all values that a seed chooses for one
zone: sizes, amounts, looks and timings. Rust makes the recipe first,
from the seed only. Then each system makes its part of the zone from
its part of the recipe.

1. The seed chooses a few **dials**. A dial is one property of the
   full zone that a player can feel: wet or dry, rugged or soft.
2. One or two dials are **heroes**. A hero dial goes far from the
   middle. The other dials stay near the middle.
3. Each system has **knobs**. A knob is one value with a hard range.
   A dial moves the knobs that belong to it, all in the same
   direction.
4. Later, a zone can have a **trait**: one big named thing with a
   promise, for example "a fall of 80 m or more".
5. **Limits** keep the result good: hard ranges, rules between
   values, a check of the finished zone, and the speed budget.
6. The **world audit** makes many recipes and many zones, and checks
   all of this with numbers.

The **plain recipe** has all dials in the middle. It gives the same
zone as the game gives today, bit for bit. Tests, speed tests and
tours use the plain recipe, so their numbers stay valid.

## What the code does today

The numbers are counts of fields and constants in `rust/`.

| System | Seeded today | Tuned (`#[export]`) | Fixed in code |
|---|---|---|---|
| Terrain (`worldgen/highlands.rs`, `game/terrain.rs`) | Only the noise. All 30 values of `HighlandsParams` are the same for each seed. | 12 of the 30 values, on `BfTerrain` | 6 ground colours (`highlands.rs:661`), the octaves and gains of the noise |
| Lakes and ponds (`worldgen/water.rs`) | Fill part, pond choice, look kind (7 weights for each of 6 cases), colour ±8%, clarity, density, temperature | None of the 27 values of `WaterParams` | The weight tables (`pick_look`), the look tables (colour, clarity, density, viscosity) |
| Rivers and falls (`rivers.rs`, `waterfalls.rs`) | Look kind, fill part | None | `RIVER_LOOK_WEIGHTS`, 5 constants |
| Water and fall drawing (`game/water.rs`, `waterfall.rs`) | Nothing | 27 + 13 | The shaders |
| Caves (`worldgen/caves/`, `game/caves.rs`) | The graph: number of systems, depth, sizes, in the ranges of `CaveParams` (about 50 values) | 30 on `BfCaves` (drawing, light, minerals). `CaveParams` is not reachable from Godot. | Ranges as numbers in `build.rs` (tunnel length 60 to 180 m at line 463, 30 to 90 m at line 585) |
| Obstacles (`worldgen/obstacles.rs`, `game/obstacles.rs`) | Places, sizes, period, phase, difficulty | 60 on `BfObstacles` (tells, forces), 3 on `BfTerrain` | `max_count`, `same_gap_m`, the period formulas (lines 793 to 830) |
| Scatter (`worldgen/scatter.rs`, `props.rs`) | Places, sizes, turns | 18 on `BfScatter` (density and size as multipliers) | The meshes: 2 rocks, 1 pebble and 2 shrubs with fixed seeds, the same in each zone |
| Smog and sky (`game/atmosphere.rs`) | Nothing | 29 | – |
| Smoke edge (`worldgen/smoke.rs`, `game/smoke_edge.rs`) | The wave phases | 23 | `LOBES`, `WAVES` |
| The bear (`bear.rs`, `bear_model.rs`, `anim.rs`) | Nothing | 152 | – |

Three facts follow.

- The systems already take their values as one struct each
  (`HighlandsParams`, `WaterParams`, `CaveParams`, `ObstacleParams`,
  `ScatterParams`, `SmokeParams`). This is the good base. A recipe is
  these structs, filled by a seed.
- A seed changes the *places* of things today. It does not change the
  *character* of a zone. Each zone has the same relief, the same
  amount of water, the same sky and the same two rocks.
- The water looks already use weights. They are the model for the
  other systems, but the weights are in the code, not in a table.

## The research in short

One subagent made a summary on 2026-11-15. It read search summaries,
not the primary sources. The wiki article comes with S-209.

- **Choose from the top down.** No Man's Sky selects a biome from a
  weight table that depends on the star type. The parent sets the
  weights of the child. We use: zone dials, then knobs, then things.
- **Push a few values far.** Kate Compton's "10,000 bowls of oatmeal":
  a player sees only a few differences. If each value is random and
  independent, all zones feel average. We use hero dials.
- **Derive, do not roll again.** The fall height comes from the
  relief. The haze colour comes from the palette. We use dials that
  move many knobs together, and rules.
- **Build the must-have thing first, then check.** Spelunky makes the
  path before the rooms. Dwarf Fortress rejects a world that misses a
  minimum, and logs the reason. We use promises, a small number of
  tries, and a log of each failed promise.
- **A budget is a rule of the generator.** Risk of Rain 2 gives each
  thing a cost in credits. We use a speed target for each system and
  cost numbers from the zone data.
- **Hash a name for each random value.** One counter that all draws
  share moves each later draw when a session adds one draw. We use
  `hash(zone seed, name)`.
- **Look at many results.** Expressive range analysis (Smith and
  Whitehead, 2010) makes thousands of results and plots their
  numbers. We put this in the world audit.

Sources: <https://www.pcgworkshop.com/archive/smith2010analyzing.pdf>,
<https://galaxykate0.tumblr.com/post/139774965871/so-you-want-to-build-a-generator>,
<https://dwarffortresswiki.org/index.php/World_rejection>,
<https://tinysubversions.com/spelunkyGen/>,
<https://riskofrain2.wiki.gg/wiki/Difficulty>.

## Three kinds of value

Each value in the game is one of three kinds. A session that adds a
value says which kind it is.

| Kind | What it is | Where it lives | Who changes it |
|---|---|---|---|
| **Fixed** | A rule of scale, safety, fairness, the engine or speed | A constant, or a field that is not a knob | A session, with a reason in the commit |
| **Tuned** | One value for all zones: how a thing feels or how well it draws | An `#[export]` field (R10) | A session, from Mikey's feel notes |
| **Seeded** | A value that the seed chooses for each zone | A knob in the knob table of its system | The seed. A session changes the table from Mikey's notes. |

**How tuning (R10) and seeded ranges live together.** Mikey tunes the
table, not the zone. Each knob has a middle value and two hard ends.

- Mikey says "the caves are too narrow". This is true for all zones.
  A session moves the middle value.
- Mikey says "the zone with the giant chambers was great" or "was too
  much". A session moves an end of the range, or changes how often
  that dial is a hero.
- Mikey says "the bear jumps too low". This is a tuned value. It is
  not in the recipe.

For this, Mikey must know which zone he is in. The dev menu and the
playtest report show one line for the zone, for example
`Seed 7: wet +0.8, hollow -0.2`. Then he can say "the wet one".

## The parts

All of this is in `rust/worldgen/src/recipe.rs` (new, S-218). It does
not use Godot.

### The zone seed and the draws

```rust
pub const RECIPE_VERSION: u32 = 1;

/// Zone 0 keeps the world seed, so the test worlds do not change.
pub fn zone_seed(world_seed: u32, zone: u32) -> u32;

/// Seeded numbers that a caller reads by name.
pub struct Draw { seed: u32 }
impl Draw {
    pub fn new(zone_seed: u32, attempt: u32) -> Self;
    /// A number from 0 to 1 for a name, for example "water.pond_part".
    pub fn unit(&self, name: &str) -> f32;
    /// A number from -1 to 1. Most results are near 0.
    pub fn signed(&self, name: &str) -> f32;
    /// The index of one entry of a weight list.
    pub fn pick(&self, name: &str, weights: &[f32]) -> usize;
}
```

- `unit` is `hash01(fnv1a(name), 0, seed ^ 0x5EC1_9E00)`. Write our own
  FNV-1a (32 bits) in the file. Do not use the hasher of the standard
  library: it is not the same on each run.
- `signed` is `(2u - 1) * |2u - 1|`, with `u = unit(name)`.
- A new knob gets a new name. Thus a new knob never changes the value
  of an old knob.
- The old code keeps its salts (`seed ^ 0x1111_1111`) and its counter
  (`caves/build.rs:32`). If a row changed them, the plain zone would
  change.
- Store `RECIPE_VERSION` in each playtest. Add 1 when a seeded zone
  of an old build would be different in the new build.

### Dials

A dial is a number from -1 to 1. The middle, 0, is the world of today.

| Dial | -1 | +1 | Systems that it moves |
|---|---|---|---|
| `rugged` | low, round hills | high, sharp ranges with deep gullies | terrain; then falls and caves get more height to use |
| `wet` | almost dry: a few ponds | lake land: full lakes, many rivers | water, rivers, falls, damp ground |
| `hollow` | one small cave system | many systems, giant chambers | caves |
| `danger` | few, easy obstacles | many, hard obstacles | obstacles |
| `cover` | bare ground | many rocks and shrubs | scatter |
| `smog` | thinner, paler air | thick, dark air | smog, sky, sun, smoke edge |
| `strange` | – (0 is natural) | strange colours, looks and shapes | all looks. It goes from 0 to 1 only. |

**How the seed sets the dials** (the recommended option of question
1):

1. Select the heroes: one hero in 70% of the zones, two in 30%. Use
   `Draw::pick` with a weight for each dial. `strange` is never a
   hero.
2. A hero gets a size from 0.6 to 1.0. The sign comes from a weight
   for each dial (for example, `wet` is plus in 65% of the zones).
3. Each other dial gets `0.25 * signed(name)`.
4. `strange` comes from the zone number: 0 in zone 0, then more in
   each later zone. Until the game has more than one zone, it is 0.
5. Use the rules between dials (see "Limits").

All numbers of this list are data in one table (`DIAL_TABLE`), with
the weights.

### Knobs

A knob is one seeded value of one system.

```rust
pub struct Knob {
    pub name: &'static str,  // "caves.chamber_width_max_m"
    pub lo: f32,             // the hard low end
    pub hi: f32,             // the hard high end
    pub dial: Dial,          // the dial that moves it
    pub dial_part: f32,      // -1 to 1: a full dial moves it this far
    pub wobble: f32,         // 0 to 1: its own random part
}

/// `base` is the middle value: the tuned default of today.
pub fn value(k: &Knob, base: f32, dials: &Dials, draw: &Draw) -> f32 {
    let t = (k.dial_part * dials.get(k.dial)
        + k.wobble * dials.wobble * draw.signed(k.name)).clamp(-1.0, 1.0);
    let b = base.clamp(k.lo, k.hi);
    if t >= 0.0 { b + t * (k.hi - b) } else { b + t * (b - k.lo) }
}
```

- `base` comes from the `Default` of the params struct, or from the
  `#[export]` field if the node has one. Thus R10 stays: a session
  changes the default, and the seed moves around it.
- `dials.wobble` is 0 in the plain recipe and 1 in a seeded recipe.
  With all dials at 0 and no wobble, `value` gives `base` exactly.
- A negative `dial_part` turns the direction. Example: `wet` at +1
  makes `water.wet_min_area_m2` smaller, so more rivers have water.
- A whole number is rounded. A minimum and a maximum that are a pair
  (`chamber_width_min_m`, `chamber_width_max_m`) are two knobs with a
  rule (see "Limits").
- Each system keeps its table in its own file: `pub const KNOBS:
  &[Knob]` in `water.rs`, `caves/mod.rs` and so on. A value that is
  not in the table is tuned or fixed.

**Weight tables.** A weight list (water looks, obstacle types, rock
shapes) has a second list, the bias for one dial:
`weight = base * max(0, 1 + bias * dial)`. Example: with `strange` at
1, tar and milky water get a bias of +2 and clear water gets -0.5.

### The recipe

```rust
pub enum Mode { Plain, Seeded }

pub struct ZoneRecipe {
    pub version: u32,
    pub world_seed: u32,
    pub zone: u32,
    pub attempt: u32,
    pub mode: Mode,
    pub dials: Dials,
    pub highlands: HighlandsParams,   // has the WaterParams in it
    pub caves: CaveParams,
    pub obstacles: ObstacleParams,
    pub scatter: ScatterParams,
    pub smoke: SmokeParams,
    pub looks: LookParams,            // new: sky, smog, ground, rock
}

pub struct Base { /* the same structs, from the defaults and exports */ }

impl ZoneRecipe {
    pub fn plain(world_seed: u32, base: &Base) -> Self;
    pub fn seeded(world_seed: u32, zone: u32, attempt: u32, base: &Base) -> Self;
    /// One line for the dev menu and the report.
    pub fn describe(&self) -> String;
}
```

- Each system has one function `seeded(base, dials, draw) -> XParams`
  in its own file. S-218 makes all of them as functions that give
  `base` back. Each system row then fills its own function. Thus two
  system rows never change the same file.
- The game makes the recipe one time for each zone, in
  `BfTerrain::regenerate`. Each other node gets its values from that
  recipe. No node calls `XParams::default()` for a zone (see risk 3).
- The game mode comes from `--bf-recipe=plain` or `--bf-recipe=seeded`
  and from one line of the dev menu. The default is `plain` until
  Mikey agrees (question 4).

### Traits (later, S-226)

A trait is one big named thing: the giant fall, the great lake, a
geyser field, a chamber with a strange shape. A dial changes amounts.
A trait is a promise.

- A trait is data: a name, a weight, the dials that it needs (for
  example `rugged` above 0.3), the knobs that it sets, a cost, and a
  **promise**: a rule on the finished zone ("one lake of 40,000 m² or
  more").
- A seeded zone has zero or one trait at first.
- A trait builds its thing on purpose where it can. Example: the
  feature fall selects its river and cuts its cliff (S-202). A
  promise that only hopes for luck fails too often.

## Limits: how the parts stay good together

Use the limits in this order. The first ones are the least expensive.

1. **Hard ends.** `value` can never leave `lo` to `hi`. The ends are
   the safety of each knob. A session can make a range wider only
   after the audit and the speed test pass at the new end.
2. **Dials move knobs together.** The values of one system do not
   fight, because one dial moves them. This is the main tool.
3. **Rules between values.** A rule repairs; it does not reject. Each
   system has one function `fix(&mut params)` that runs after
   `seeded`, with a fixed order of rules. Examples: a maximum is not
   below its minimum; the hall is not wider than the widest chamber
   allows; the tall fall is not higher than 60% of the relief; the
   smoke front moves slower than the bear walks.
4. **Rules between dials.** A small table of pairs that are bad
   together, with the repair. Example: `smog` below -0.5 with `cover`
   above +0.5 costs too much (far view and many props), so `cover`
   moves to +0.5. Keep this table short.
5. **Promises and tries.** After the zone is made, check the promise
   of the trait and the rules of the system (the lists that the tests
   have now, for example "each system has a hall"). If a check fails,
   make the recipe again with `attempt + 1`, 3 tries at most. Then use
   the recipe without the trait. The number of tries depends only on
   the seed, so the same seed still gives the same zone.
6. **Log each failure** with the seed and the reason. The audit
   counts them. Targets: fewer than 10% of the seeds need a second
   try, and fewer than 1% lose their trait. A rule that fails more
   often is wrong, or its range is too wide.

A zone takes 2 to 5 s to make [unverified on Mikey's laptop]. Three
tries can be 15 s. Thus a promise must be easy to keep by design, and
limit 6 is a hard target.

## The speed budget (R5)

Speed is valid only on Mikey's laptop. The generator cannot measure
it. Thus the budget has three parts.

1. **Each system keeps its own target.** `tech.md` and `demo-2.md`
   give a GPU target to each system (water 1.0 ms, falls 0.8 ms,
   caves 1.5 ms, and so on). A knob raises only the cost of its own
   system. If each system is inside its target at the worst end of
   its own knobs, each mix of knobs is inside the frame.
2. **The view distance is the one shared cost.** Thin smog shows more
   terrain, more props and more water at the same time ("the haze is
   the draw distance", `tech.md`). The `lo` end of the smog density
   and the maximum view distance are fixed by the Speed lead, not by
   taste.
3. **Cost numbers from the zone data.** For each system, the audit
   computes a number that predicts the cost, with no GPU. Examples:
   the water triangles inside the view distance at the worst place;
   the area of the largest fall; the most props in one chunk ring;
   the triangles of the largest chamber; the number of transparent
   layers on one line of sight. Each number has a limit as data. The
   Speed lead finds the limit: it measures the worst view of a few
   zones on the laptop and compares (S-228).

**A zone over a limit loses detail, not shape.** The game uses the
cuts of `demo-2.md` in their order (fewer mist cards, one ripple
layer, a far fall as one card). The shape of the world and the play
stay the same on each computer.

**The corner test.** For each system, a recipe with all knobs of
that system at their most expensive end, and the other systems plain.
The Speed lead runs the Benchmark and the tours with these recipes
(`--bf-recipe=corner:water`). A range is valid only if its corner is
inside the target.

## How the world audit checks a recipe (S-214, S-227)

The audit of S-214 checks the shape rules of a zone. S-227 adds three
levels.

1. **Recipes only: 10,000 seeds, less than 1 s** [unverified]. Each
   value is inside its range. Each `fix` rule holds. The hero and
   trait counts agree with the weights. `plain` gives the base. The
   report has a table for each dial (how many zones in each fifth of
   its range), so a session sees that the zones are different.
2. **Zones: 3 seeds in `cargo test`, 50 seeds each night.** Each
   seed is made with its seeded recipe. All shape rules of S-214
   hold. The promises hold, with the count of tries. The cost numbers
   are below their limits. The time to make the zone is below its
   limit.
3. **Corners: each night, a part of the list.** For each knob, one
   zone with that knob at `lo` and one at `hi`, and all other values
   plain. The shape rules must hold. This finds a range that is too
   wide before a player finds it.

The report `audit.md` gets a "Recipes" section: the dial tables, the
failed promises with their seeds, and the worst cost numbers. For
taste, the look tour (S-215) makes one contact sheet of wide views of
12 seeded zones. Mikey looks at it and says which zones are good.

## What stays fixed

- **Scale** (R6): the bear is 0.40 m tall, and things have their real
  size.
- **The bear**: its body, its controls, its movement and its tuning
  (question 2). The Feel lead needs one bear in all zones.
- **Fairness**: the shortest warning time of an obstacle, the start
  point on safe ground, no obstacle and no cave mouth near the start,
  a path into each cave that the bear can walk.
- **Safety of shape**: the thinnest cave roof, the bank margin of a
  river, the distance of caves from the smoke front.
- **The grid**: 1 m vertex spacing, 2048 samples, 32 m cave chunks
  with 1 m voxels. The zone size is fixed too for now: many distances
  in the code are in metres of a 2 km zone (risk 9).
- **The speed budget** and the renderer rules of `tech.md`.
- **The exact maths** (`detmath.rs`), so a seed gives the same bits on
  each computer.
- **The plain recipe.** It is the reference for tests and speed.

## Questions for Mikey (taste and scope)

1. **How different are two zones?**
   - a. *One strange thing in a normal place.* Each zone has one or
     two hero dials. The other dials stay near the middle.
   - b. *All wild.* Each dial can go far in each zone. Each zone is
     very different, but more zones are odd in a bad way, and more
     cost speed.
   - c. *A ramp.* The first zone is as in a. Each later zone is
     stranger (the `strange` dial grows).
   - **Recommended: c.** It starts as a, so the first zone is easy to
     read, and it gives the dream feeling of `vision.md` later.
2. **Does the seed change how the bear plays?**
   - a. *No.* The bear and its rules are the same in each zone.
   - b. *The world does, the bear does not.* Water can be thick, the
     ground can have more or less grip, the wind can be strong. The
     bear's own values stay.
   - c. *All.* Also gravity, jump height and stamina.
   - **Recommended: b.** The water already does this (viscosity). The
     tuning of the bear (S-137) needs a bear that does not change.
3. **Can a zone have none of a thing?**
   - a. *No.* Each zone has some water, one cave system or more, and
     some obstacles.
   - b. *Yes.* A dry zone or a zone with no caves is possible. The
     zones are more different, but a player can get a dull zone.
   - **Recommended: a for the first zone, b later**, with one rule:
     each zone has one hero or more.
4. **When does the work start?**
   - a. *Now, all systems*, beside demo 2.
   - b. *After the demo 2 playtest* (S-173).
   - c. *The base now, the systems later.* The base is the fingerprint
     test, the recipe core and the one-source fix (S-217 to S-219).
     It does not change the build that Mikey plays. The system rows
     wait for the first Feel lead.
   - **Recommended: c.** The base is small, and it stops new code
     from adding more fixed numbers. Demo 2 stays first.

## Risks in the present code, the worst first

1. **No test fixes the world of a seed.** The tests compare a seed
   with itself only (`same_seed_same_zone`, `highlands.rs:731`). A
   row can change the zones of seeds 1, 7 and 42 and no check fails.
   The Benchmark, the tours and the numbers in `play-notes.md` use
   these zones. Row: **S-217**.
2. **Seeded ranges are numbers in the code.** `caves/build.rs:462`
   to `593` (lengths, angles, depths), `obstacles.rs:793` to `830`
   (periods, sizes), `water.rs:660` to `678` (the look weights),
   `water.rs:187` to `234` (the look tables), `rivers.rs:29`,
   `highlands.rs:661` (ground colours), `smoke.rs:18` to `23`. Rows:
   S-220 to S-225, one system each.
3. **Two sources for one value.** `game/terrain.rs:377` makes the
   caves with `SmokeParams::default()` and `CaveParams::default()`.
   But `game/smoke_edge.rs:457` makes the smoke edge from its own
   exports. If a session tunes the smoke front, a cave can be under
   the smoke. `BfTerrain` copies only 12 of the 30 terrain values,
   and `WaterParams` has no way in from Godot. Row: **S-219**.
4. **Seeds have no structure.** `terrain.rs:355` cuts the 64-bit seed
   to 32 bits, and `smoke.rs:103` folds it in a different way. Each
   file has its own salts, with no list of them. No code knows a zone
   number, but `tech.md` says that the world seed and the zone number
   set a zone. Row: **S-218** (`zone_seed`, `Draw`).
5. **Counters for random numbers.** `caves/build.rs:32` and
   `props.rs:127` read random numbers in order. One new draw changes
   each later one. A row that adds a seeded value to the caves this
   way changes all caves. Rows: S-218 (`Draw`), S-222.
6. **Looks are in Godot nodes with no seed.** The sky, the sun and
   the smog (29 exports in `atmosphere.rs`), the cave rock and the
   minerals (`game/caves.rs`), and the prop meshes (`props.rs:262`)
   are the same in each zone. A zone cannot have its own sky. Rows:
   S-222, S-224, S-225.
7. **Speed is measured on three plain zones only.** No number
   predicts the cost of a zone from its data
   (`game/src/playtest/bench.rs` has fixed paths). A value at the end
   of a range was never measured. Row: **S-228**.
8. **Rules are tested with one set of values.** The rule tests
   (`water.rs:1142`, `caves/tests.rs`, `obstacles.rs:1191`) use the
   defaults and seeds 1, 7 and 42. A rule can fail with other values
   and no test shows it. Row: **S-227** (corners).
9. **Distances that assume a 2 km zone.** `erosion_length_m: 2048`,
   the obstacle distances (`hard_at_m: 800`), `min_front_m: 320`.
   A recipe cannot change the zone size before these are parts of
   the zone size. No row: the zone size stays fixed. The line is in
   `inbox.md`.
10. **Timing has two owners.** The obstacle periods are formulas in
    `worldgen/obstacles.rs`, and the tells are 60 exports in
    `game/obstacles.rs`. Row: S-223.

## The rows, in order

| Order | Row | What | Changes the build that Mikey plays |
|---|---|---|---|
| 1 | S-217 | The zone fingerprint test | no |
| 2 | S-218 | The recipe core (`recipe.rs`) | no |
| 3 | S-219 | The game uses one recipe; one source for each value | no |
| 4 | S-227 | The audit checks recipes (after S-214) | no |
| 5 | S-220 | Water | only in `seeded` mode |
| 6 | S-221 | Terrain | only in `seeded` mode |
| 7 | S-222 | Caves | only in `seeded` mode |
| 8 | S-223 | Obstacles | only in `seeded` mode |
| 9 | S-224 | Scatter and prop shapes | only in `seeded` mode |
| 10 | S-225 | Smog, sky and smoke edge | only in `seeded` mode |
| 11 | S-228 | Cost numbers and the corner test (Speed) | no |
| 12 | S-229 | Mikey looks at seeded zones | – |
| 13 | S-226 | Traits and promises, with the first trait | only in `seeded` mode |

Rows 5 to 10 can run in any order after S-219, and beside each other:
each one changes the files of its own system only.

**What each system row does** (S-220 to S-225):

1. Read this spec and the files of the system.
2. Move each seeded number that is in the code into the params
   struct of the system. The plain zone must not change (S-217).
3. Write the knob table `KNOBS` and the weight tables with their
   bias. Give each knob a dial, a range and a `dial_part`. Set the
   range from the code and the tests, not from a guess: both ends
   must pass the rules of the system.
4. Fill `seeded(base, dials, draw)` and `fix`.
5. Add tests: `seeded` with dials at 0 and no wobble gives `base`;
   each knob at `lo` and at `hi` makes a zone that passes the rules
   of the system on seeds 1, 7 and 42.
6. Make preview images of four seeded zones (hero at -1 and at +1,
   two seeds) and send them to Mikey with one line each.
7. Write each assumed range in the "Assumed" list of
   `game/README.md`.
