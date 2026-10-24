# Broken Friends: the game

The Godot project. Godot **4.7.1**, GDScript, the Compatibility
renderer (so it runs in browsers and on cheap phones), portrait 720×1280.

## What's in it now: the first five minutes

Built from Mikey's recording of 2026-10-24
(`plan/inbox/2026-10-24-recording-1.md`).

**In the tent**
- A sick creature, passed out and shaking, front-on. Mud, a rotting
  leg torn open with fluff poking out, mold, a button eye hanging by
  a thread, a little thin, with stiff, squeaky joints.
- **Poke** to wake them. Tap the top of the head (they shake it, no),
  the tickle spot on the belly (they giggle), anywhere else (a soft
  boop). **Stroke** across them to pet: they close their eyes and purr.
- **Grab the rotting leg and pull.** It stretches, fluff starts
  escaping, and it tears off with a burst of fluff and hydraulic fluid.
  Healthy limbs stretch a little and protest, but don't come off
  (donating healthy parts is still open: Q-016).
- **Toss it in the bin.** A spare leg (different fabric: they're never
  the same) then waits in a pocket on the right.
- **Drag the spare onto the wound** to start surgery, which zooms in:
  1. pull away the loose fluff,
  2. connect the ligament tube to the port,
  3. soak up the spilled fluid with the cloth,
  4. sew along the glowing points.
  The stitches stay for good.
- **The dangling eye:** drag it back into place, then stitch it.
- **Tools on the shelf:** drag the sponge over mud (it gets them wet),
  the towel to dry, stuffing to their mouth (they eat stuffing; it
  fills them out), and oil for the joints.
- **Double-tap** anywhere to zoom in or out.

**In the play corner**
- They walk around (if both legs are on), nap if left alone, and you
  can poke and pet them.
- **Throw the ball** with a flick. At first they miss and laugh when it
  bonks their head; every throw teaches them, until they catch it and
  toss it back.

The game saves on its own. **Start over** (tap twice) begins again.

**Music** (when the Driftloom extension is built): Driftloom's own core
plays six of its golden loops in turn, quietly. See `rust/README.md`.

## Play it

- **Phone browser:** https://nowmichaelclark.github.io/brokenfriends/
  (rebuilt on every push that changes `game/`, once GitHub Pages is
  switched on; see below).
- **Laptop:** open this folder in Godot 4.7.1 and press F5. The mouse
  acts as a finger.
- **Android, from the laptop:** set up the Android SDK once (Godot docs,
  "Exporting for Android"), plug in the phone with USB debugging on,
  then press the Android button at the top right of the editor
  (one-click deploy). The **Android** export preset is ready.
- **Android, from the phone:** the Godot Android editor can open this
  project and export it.

### Switching on GitHub Pages (once)

On GitHub, as nowmichaelclark: the repo → **Settings** → **Pages** →
under **Build and deployment**, set **Source** to **GitHub Actions**.
Then **Actions** → **web** → **Run workflow** (or just push).

## How the code is laid out

| Folder | What |
|---|---|
| `autoload/` | Always-loaded: `Events` (the event bus), `Voice` (sounds, synthesized), `Save`, `Music` (Driftloom) |
| `data/` | `CreatureData` and `PartData`: creatures as plain data, saved as JSON |
| `creature/` | `CreatureView` (drawing and body reactions) and `PlushArt` (drawing helpers) |
| `fx/` | `FxLayer`: fluff, fluid, bubbles, drips, sparkles |
| `scenes/tent/` | The tent: care, the bin, the spare pocket, surgery, the tool shelf |
| `scenes/play/` | The play corner and the ball |
| `addons/driftloom/` | The Driftloom extension's config and its golden loops (`rust/` builds it) |
| `tests/` | `flow_test.gd` plays the first five minutes; `music_test.gd` checks Driftloom's hashes |

Rules the code follows:
- **Data and drawing are separate.** `CreatureData` knows nothing about
  screens; `CreatureView` draws whatever the data says.
- **Systems talk through `Events`.** Driftloom music will listen there
  (decision 0016) without the rest of the game knowing.
- **No image or sound files.** Everything is drawn and synthesized in
  code, a first step toward generated art (Q-006).
- **The body is the interface** (pillar 1). No health bars; health shows
  as shaking, droopy lids, a slow heart patch, a pale or wet coat. The
  text hints are a crutch for now.

## Testing

```bash
# Headless: does everything load?
godot --headless --path game --quit-after 120

# The full playthrough with screenshots (needs a display or xvfb-run)
godot --path game --resolution 720x1280 -- --flow-test=/tmp/shots
```

The playthrough prints `ok` or `FAIL` for each step and ends with
`FLOW OK`. Run it before pushing changes to the tent or the creature.

```bash
# Driftloom's golden loops, rendered in the game (needs rust/build.sh linux)
godot --headless --path game -- --music-test
```

## Assumed in this build, waiting on Mikey

These are guesses from the recording (see the numbered list in the
planning chat). Each is easy to change.

- Front view only; the creature never turns sideways.
- Stuffing is weight: eating stuffing fills them out, losing it makes
  them thin (assumption 8).
- Oil and hydraulic fluid keep joints loose; dry joints squeak and move
  stiffly (assumption 9).
- A spare leg appears in the pocket once the rotten one is in the bin.
- Each camp has a tent; surgery and play happen inside it (assumption 16).
- Only rotting parts tear off; healthy parts can't be removed yet (Q-016).
- The creature naps after about 40 seconds alone in the play corner.
