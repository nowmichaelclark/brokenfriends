# Broken Friends: the game

The Godot project. Godot **4.7.1**, GDScript, the Compatibility
renderer (so it runs in browsers and on cheap phones), portrait 720×1280.

**This is a sketch** (Mikey, 2026-10-24; decision 0021). Any of it may
be reworked or thrown out as planning settles the game. The plans lead;
the build is evidence about feel, never a reason. It follows
`plan/prototypes/03-first-playable.md` (the first playable) and
`plan/prototypes/04-physics-test.md` (friends as soft ragdolls).

## What's in it now

**The tent.** The friend being looked after stands in front; the
camp's other friends (up to four) sit further back, and tapping one
brings them forward. The first friend starts passed out, muddy, with a
rotting leg and an eye hanging by a thread.
- **Poke** to wake them: the top of the head (they shake it, no), the
  tickle spot (they giggle), anywhere else (a boop). **Stroke** to pet:
  they close their eyes and purr. **Double-tap** to zoom.
- **Parts come off** (0025, 0026): each hard pull, held, loosens an arm
  or leg, and the third takes it off (a rotting one tears at once). Keep
  a part in the pocket and sew it onto another friend, or bin it. Pull
  the head up hard three times and it comes off: the friend lives on in
  the head, which can go on a body without one.
- **Surgery** on its own table (0031), zoomed in: pull away the loose
  fluff, connect the ligament tube, soak up the spill, sew along the
  glowing points. Stitches stay. The dangling eye is dragged back and
  stitched the same way.
- **The shelf:** sponge (mud off, gets them wet; also scrubs the
  floor), towel, stuffing (fills them out), oil (loosens squeaky
  joints), mold spray. Stuffing, oil, thread and spray run out (0027,
  0042); with none left the thing shakes and the treatment can't be
  done. Water is free.
- **No words** (0032): what could be done next glows (a ring where to
  touch, a dotted line where to drag, a glow on the shelf, pocket or
  bin). Buttons are pictures. `#hints` brings the words back.

**Change over time** (0028, 0029). Rot needs a source (a tear, mold,
mud, soaking, an open wound) and grows faster in a dirty camp and a
hungry friend. Mold spreads. When the body falls far enough apart the
chip brain fails and the friend dies, for good. Rot draws bugs (tap
them); muddy friends track footprints (scrub them). Time passes while
the game is closed; a healthy, clean friend never rots. Newcomers turn
up now and then with something wrong (0034); a camp holds five (0013).

**The play corner.** They walk around (hop on one leg, crawl with
none), nap if left alone, and can be poked and petted. Flick the ball:
at the head they try to catch, at a foot they kick, off the body it
bounces. Catching and kicking are learned slowly by doing (0033).

**Quests** (0034, 0035). **Map** shows the meadow, the woods, the river
and the old toy shop, each with two routes and their dangers. Choose a
route; a pie shows the chance of coming back well and a clock the
time. The friend walks it in real time and comes home changed (soaked,
muddy, torn, missing a limb, or fitter) with supplies, parts, sometimes
a new friend. A friend can die out there.

**Generated friends** (0037): made by maths from a seed (`data/looks.gd`
holds the weights: the developer's sliders). Six body plans (bear,
bunny, cat, monster, alien, a plush ghost-like one), each with its own
head, ears, muzzle, eyes, build and colours, and its own voice. Any
head fits any body. `#gallery` shows twenty side by side.

**Music** (0038; when the Driftloom extension is built): a piece
repeats until something happens (the tent's state, the play corner,
the map, late night, a friend coming home or dying). Each change prints
its reason. See `rust/README.md`.

The game saves on its own. **Start over** (tap twice) begins again.

## The physics test (`#physics`)

Prototype 4, on its own, not reached from the tent. A friend as a soft
ragdoll: body, head, arms and legs are each a body in Godot's built-in
2D physics (decision 0005), pinned together at the seams, drawn from
the friend's real generated look. **Drag** any part with a finger and
let go mid-swipe to **fling**. A ramp with a skateboard at the top, and
a step at the bottom. Debug buttons: **next friend** (a copy of the
friend in front at camp, then tiny, huge, no legs, one arm, soaked, a
loose seam), **three at once**, **board** (back to the top), **drop**
(the same friends again). `&three` starts with three; `&perf` shows
speed, physics included.

The friend's body sets the physics: stuffing its weight and bounce,
wetness more weight and less bounce. A limb that hits something hard
counts as a hard pull on its seam (0025): a loose seam tears off, a
healthy one after a few. A torn part stays torn in that friend's data.

## Play it

- **Phone browser:** https://nowmichaelclark.github.io/brokenfriends/
  (rebuilt on every push to `main` that changes `game/`). The physics
  test: add `#physics` to the address.
- **Laptop:** open this folder in Godot 4.7.1 and press F5. The mouse
  acts as a finger. `-- --physics` for the physics test.
- **Android app:** every push to `main` builds it. On the phone, open
  https://github.com/nowmichaelclark/brokenfriends/releases/tag/android-latest
  and tap `broken-friends.apk`. Each build installs over the last. It
  is signed with a test key (`.github/android-test.keystore`); before
  the game is shared, add a private key as the repository secrets
  `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_USER` and
  `ANDROID_KEYSTORE_PASSWORD` (players would then reinstall once).
- **Android, from the laptop:** set up the Android SDK once (Godot docs,
  "Exporting for Android"), then the editor's one-click deploy. The
  **Android** export preset is ready.

GitHub Pages must be switched on once: the repo → **Settings** →
**Pages** → **Source**: **GitHub Actions**.

## How the code is laid out

| Folder | What |
|---|---|
| `autoload/` | Always loaded: `Events` (the event bus), `Voice` (sounds, synthesized), `Save`, `Music` (Driftloom) |
| `data/` | `CampData`, `FriendData`, `PartData`, `Looks`, quests: the camp as plain data, saved as JSON |
| `friend/` | `FriendRig` (how a friend is drawn), `FriendView` (how it reacts), `FriendArt` and `PlushArt` (drawing helpers) |
| `fx/` | `FxLayer` (fluff, fluid, bubbles, sparkles), `Baked` (a drawing made once into a picture), icons |
| `scenes/tent/` | `Tent`, with `TentSurgery`, `TentParts` (parts and heads in the hand) and `TentCues` (what to do next); the shelf, the bench, the table |
| `scenes/play/` | The play corner and the ball |
| `scenes/map/` | The map and quests |
| `scenes/physics/` | The physics test: `PhysicsPlay`, `Ragdoll`, `Skateboard` |
| `addons/driftloom/` | The Driftloom extension's config and golden loops (`rust/` builds it) |
| `debug/` | `Flags` (debug switches) and the `#perf` overlay |
| `tests/` | The playthrough (`flow_test.gd`, `flow_world.gd`, `flow_helpers.gd`), the gallery, `music_test.gd` |

Rules the code follows:
- **Data and drawing are separate.** `FriendData` knows nothing about
  screens; the views draw whatever the data says.
- **Systems talk through `Events`.** Music listens there (0016)
  without the rest of the game knowing.
- **No image or sound files.** Everything is drawn and synthesized in
  code (0037).
- **Draw once, then move** (Driftloom's habit). Detailed drawings are
  baked into a picture with `Baked` and re-baked only when what they
  show changes; motion is moving and turning them. Sounds are made at
  start-up. A cheap phone (Mikey's Moto G Power 2022) is the yardstick.
- **The body is the interface** (pillar 1). No health bars.

## Testing

```bash
# Headless: does everything load?
godot --headless --path game --quit-after 120

# The full playthrough with screenshots (needs a display or xvfb-run)
godot --path game --resolution 720x1280 -- --flow-test=/tmp/shots

# Only its physics part (prototype 4)
godot --path game --resolution 720x1280 -- --flow-test=/tmp/shots --physics-only

# Driftloom's golden loops, rendered in the game (needs rust/build.sh linux)
godot --headless --path game -- --music-test
```

The playthrough prints `ok` or `FAIL` for each step and ends with
`FLOW OK`. Run it before pushing game changes. It ends with the physics
test: every test friend dropped from a height lands, settles and stays
whole; a loose seam dropped hard tears; a finger flings; the board
rolls; three at once hold together.

Speed: `#perf` on the web address (or `-- --perf`) shows frame time,
script, physics and music cost and draw calls, printed as `PERF` lines
too. `&nobg`, `&nofriend`, `&nohud`, `&nomusic` or `&empty` show what
each part costs. Numbers: `plan/systems/platforms-tech/README.md` and
`plan/prototypes/04-physics-test.md`.

Time: `#days=3` pretends three more days passed; `#fast=60` runs the
clock 60 times faster; `#friends=3` starts a new game with three
friends; `#hour=N` pretends it is that hour; `#supplies=N` sets the
supplies on a new game.

## Assumed in this build, waiting on Mikey

Each is easy to change.

- Stuffing is weight; oil and hydraulic fluid keep joints loose.
- Surgery and play happen inside the tent; surgery on its own table.
- A spare leg appears in the pocket once the rotten one is binned.
- The friend naps after about 40 seconds alone in the play corner.
- Each day a friend loses about a tenth of its stuffing and some oil;
  open wounds leak stuffing fast; wet friends stay wet until dried.
  Left alone, the first friend dies in about two and a half weeks.
- Body plans: bear 30%, bunny 18%, cat 18%, monster 14%, alien 10%,
  ghost-like 10%; cheeks one in four, a head patch one in seven.
- Healthy parts take 3 hard pulls (4 for good parts, 5 for the best);
  a head takes 3. A head without a body dies after a day.
- Quests take 3 minutes to 4 hours; only routes with wolves or a cliff
  can kill; the last friend at camp can't be sent.
- A newcomer about every day and a half while there's room.
- A new camp starts with 6 stuffing, 3 oil, 4 thread, 2 spray.
- **The physics test** (all guesses, to tune by playing):
  - A friend is 0.42 of its tent size; tiny is 0.6 of that, huge 1.5.
  - Weights: body 1, head 0.6, an arm 0.12, a leg 0.18; times size²;
    stuffing from 0.6× (thin) to 1.3× (plump); soaked doubles it.
  - Bounce 0.02 to 0.4 by stuffing, cut by wetness; gravity twice
    Godot's default.
  - Seams are Godot pin joints; parts spring back to their pose
    (arms loosely, legs and head more firmly) and swing freely up to
    a limit (arms 2.4, legs 1.1, head 0.55 radians).
  - A limb hitting something at 2500 pixels a second or more is a hard
    pull (a drop from the top of the screen lands at about 1700). A
    rotting part tears at the first. The head never comes off here.
  - Its friends are copies or made up, and never saved.
