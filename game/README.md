# Broken Friends: the game

The Godot project. Godot **4.7.1**, GDScript, the Compatibility
renderer (so it runs in browsers and on cheap phones), portrait 720×1280.

**This is a sketch** (Mikey, 2026-10-24; decision 0021). Any of it may
be reworked or thrown out as planning settles the game. The plans lead;
the build is evidence about feel, never a reason. It follows
`plan/prototypes/03-first-playable.md` (the first playable) and
`plan/prototypes/04-physics-test.md` (friends as soft ragdolls).

## What's in it now

**The entrance** (the starting screen; Mikey's playtest notes): the
camp's friends stand outside the tent. A newcomer sparkles until
chosen; parts and heads that came back from quests wait in a crate,
sparkling when new. Tap a friend to choose them, then a picture: the
grooming screen (sponge), the surgery table (needle), the play room,
the map. What the chosen friend needs most glows. Start over (tap
twice) and the dev menu (gear) are here too.

**The tent**, in two rooms (`TentRooms`). The first friend starts
passed out, muddy, with a rotting leg and an eye hanging by a thread.
**Poke** to wake them (head top: no; tickle spot: a giggle; elsewhere:
a boop); **stroke** to pet; **double-tap** to zoom.
- **The grooming screen:** the friend in front, the others (up to four)
  further back; tap one to bring them forward. Sponge (mud off, gets
  them wet; scrubs the floor), towel, stuffing, oil, mold spray.
- **The surgery table** (0031): the friend upright against a padded
  board, closer. **Parts come off** here (0025, 0026): the third hard
  pull takes an arm or leg (a rotting one at once); keep it in the
  pocket for another friend, or bin it. Three hard pulls up take the
  head: the friend lives on in it, for a body without one. **Surgery**
  comes closer still: pull away the loose fluff, connect the ligament
  tube, soak up the spill, sew along the glowing points; the dangling
  eye the same way. **Sewn shut:** drag thread to a hole where a part
  is missing, clear the fluff, sew across; the friend lives on without
  it. A hole left open leaks, rots (a green-brown stain) and feeds rot.
- Stuffing, oil, thread and spray run out (0027, 0042): the empty thing
  shakes and the treatment can't be done. Water is free.
- **No words** (0032): what could be done next glows (a ring, a dotted
  line to drag along, the shelf, pocket, bin, or the other room's
  button). `#hints` brings the words back.

**Change over time** (0028, 0029). Rot needs a source (a tear, mold,
mud, soaking, an open wound) and grows faster in a dirty camp and a
hungry friend. Mold spreads. When the body falls far enough apart the
chip brain fails and the friend dies, for good. Rot draws bugs (tap
them); muddy friends track footprints (scrub them). Time passes while
the game is closed; a healthy, clean friend never rots. Newcomers turn
up now and then with something wrong (0034); a camp holds five (0013).

**The play room** (Q-036): three screens wide; every friend at camp
wanders about it together, walking toward and away from the screen
(hop on one leg, crawl with none), napping if left alone; poke and pet
any of them. **Pinch** (or the mouse wheel) zooms from the whole room
to close up; drag the floor to look around; double-tap the floor to
come in or go back out. Flick the ball: at a head they try to catch,
at a foot they kick, off the body it bounces. Catching and kicking are
learned slowly by doing (0033).

**The dev menu** (0045; words are fine there): the physics test, a
test friend of each body plan or trouble (no legs, one arm, soaked, a
rotting leg, an open hole, a loose seam), the play room's stress test.

**Quests** (0034, 0035). **Map**: the meadow, the woods, the river and
the old toy shop, two routes each, dangers drawn. A pie shows the
chance of coming back well, a clock the time. The friend walks it in
real time and comes home changed (soaked, muddy, torn, missing a limb,
or fitter) with supplies, parts, sometimes a new friend, or dies.

**Generated friends** (0037): made by maths from a seed (`data/looks.gd`
holds the weights: the developer's sliders). Six body plans (bear,
bunny, cat, monster, alien, a plush ghost-like one), each with its own
head, ears, muzzle, eyes, build and colours, and its own voice. Any
head fits any body. `#gallery` shows twenty side by side.

**Music** (0038; when the Driftloom extension is built): a piece
repeats until something happens (the tent's state, the play room, the
map, late night, a friend coming home or dying). Each change prints
its reason. See `rust/README.md`. The game saves on its own.

## The physics test (`#physics`, or the dev menu)

Prototype 4. A friend as a soft ragdoll: body, head, arms and legs are
each a body in Godot's built-in 2D physics (0005), pinned at the seams,
drawn from the friend's generated look. **Drag** a part and let go
mid-swipe to **fling**. A ramp, a skateboard, a step. Debug buttons:
**next friend** (a copy of the friend in front, then tiny, huge, no
legs, one arm, soaked, a loose seam), **three at once**, **board**,
**drop**. `&three` starts with three; `&perf` shows speed.

Stuffing sets weight and bounce; wetness adds weight, cuts bounce. A
limb hit hard (landing, or something hitting it) is a hard pull on its
seam (0025): a loose seam tears, a healthy one after a few. The neck
takes only very hard hits, and more; then the head comes off and the
friend lives on in it (0026). A collar on the head fills the notch
under the chin where the board once caught (V-020).

## Play it

- **Phone browser:** https://nowmichaelclark.github.io/brokenfriends/
  (rebuilt on every push to `main` that changes `game/`).
- **Laptop:** open this folder in Godot 4.7.1 and press F5. The mouse
  acts as a finger (in the browser too, V-021). `-- --physics` for the
  physics test.
- **Android app:** every push to `main` builds it: on the phone, open
  https://github.com/nowmichaelclark/brokenfriends/releases/tag/android-latest
  and tap `broken-friends.apk`; it installs over the last. Signed with
  a test key (`.github/android-test.keystore`); before sharing, add a
  private key as the secrets `ANDROID_KEYSTORE_BASE64`,
  `ANDROID_KEYSTORE_USER`, `ANDROID_KEYSTORE_PASSWORD`.
- **Android, from the laptop:** the Android SDK once (Godot docs,
  "Exporting for Android"), then one-click deploy (preset ready).

## How the code is laid out

| Folder | What |
|---|---|
| `autoload/` | Always loaded: `Events` (the event bus), `Voice` (sounds, synthesized), `Save`, `Music` (Driftloom) |
| `data/` | `CampData`, `FriendData`, `PartData`, `Looks`, quests: the camp as plain data, saved as JSON |
| `scenes/entrance/` | `Entrance` (the starting screen) and `DevMenu` |
| `friend/` | `FriendRig` (how a friend is drawn), `FriendView` (how it reacts), `FriendArt` and `PlushArt` (drawing helpers) |
| `fx/` | `FxLayer` (fluff, fluid, bubbles, sparkles), `Baked` (a drawing made once into a picture), icons |
| `scenes/tent/` | `Tent`, with `TentRooms` (grooming screen, surgery table, camera), `TentTools` (the shelf), `TentSurgery`, `TentParts` (parts and heads in the hand) and `TentCues` (what to do next) |
| `scenes/play/` | `PlayRoom`, `Wanderer` (one friend walking about) and the ball |
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
`FLOW OK`. Run it before pushing game changes. It goes from the
entrance through both rooms, the play room (printing frame time with
0, 10, 20 and 40 more friends), the dev menu and holes, to the physics
test: drops, a loose seam tearing, the neck (nothing wedges; very hard
hits), a board dropped on a still friend, finger and mouse flings, the
board rolling, three at once.

Speed: `#perf` on the web address (or `-- --perf`) shows frame time,
script, physics and music cost and draw calls, printed as `PERF` lines
too. `&nobg`, `&nofriend`, `&nohud`, `&nomusic` or `&empty` show what
each part costs. Numbers: `plan/systems/platforms-tech/README.md` and
`plan/prototypes/04-physics-test.md`. `#crowd=N` opens the play room
with N more friends: the stress test for how many a phone holds
(`#crowd=20&perf`). `#place=table` (or `grooming`, `play_room`, `map`)
opens straight there.

Time: `#days=3` pretends three more days passed; `#fast=60` runs the
clock 60 times faster; `#friends=3` starts a new game with three
friends; `#hour=N` pretends it is that hour; `#supplies=N` sets the
supplies on a new game.

## Assumed in this build, waiting on Mikey

Each is easy to change.

- Stuffing is weight; oil and hydraulic fluid keep joints loose.
- Grooming has washing, drying, feeding, mold spray, petting; the
  table has pulling parts, the pocket, the bin, sewing. Feeding and
  spray could go either way.
- The play room is its own room at camp, from the entrance, three
  screens wide (Q-036). Crowd friends are made up, never saved; dev
  menu test friends join the camp past its limit.
- A spare leg appears in the pocket once the rotten one is binned.
- A friend naps after about 40 seconds alone in the play room.
- Sewing a hole shut takes a thread and costs only the part (no leak,
  no rot; a part can still go on later, stitches cut). An open hole
  rots fully in about four days (faster hungry or in a dirty camp).
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
- **The physics test** (guesses, to tune by playing): a friend is 0.42
  of its tent size (tiny 0.6 of that, huge 1.5); weights body 1, head
  0.6, arm 0.12, leg 0.18, times size², stuffing 0.6× to 1.3×, soaked
  doubles; bounce 0.02 to 0.4, cut by wetness; gravity twice Godot's.
  Seams are pin joints; parts spring back to their pose and swing
  freely up to arms 2.4, legs 1.1, head 0.55 radians. A limb hit at
  2500 pixels a second or more is a hard pull (a drop from the top of
  the screen lands at about 1700; rotting parts tear at the first);
  the neck counts hits of 3000 or more and gives at the sixth (a hand
  takes the head off in the tent at the third). Never saved.
