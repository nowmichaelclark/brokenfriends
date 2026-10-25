# To verify

Technical claims waiting for a Verifier session (`kit/sessions/verifier.md`).
One line each: number, claim, where it's used, how to confirm. Ticked
items are cleared by Cleanup. Next number: V-020.

## Open

- [x] V-004: **Moot:** iOS is out of scope (decision 0015). Exporting a Godot game for iOS requires macOS with Xcode. (systems/platforms-tech, decisions/0003) Confirm in: Godot docs, "Exporting for iOS".
- [x] V-005: **Moot:** iOS is out of scope (decision 0015). Defold can bundle iOS builds from Linux without a Mac, given Apple certificates. (decisions/0004) Confirm in: Defold manual, iOS bundling.
- [ ] V-007: Defold's 2D skeletal deformation usually means the Spine extension, which needs a paid Spine licence. (decisions/0004) **Unclear, partly true:** Defold's Spine support is an extension (defold/extension-spine README, Spine runtime 4.2.xx, checked 2026-10-25), and Esoteric says licensing the Spine editor grants the runtime rights (forum reply seen via search; their licence pages were unreachable). Not checked: whether Defold has a free built-in alternative. Needs: Defold manual and Esoteric licence page (docs sites blocked here).
- [x] V-008: **True, with conditions.** A Rust GDExtension runs in Godot 4.7.1's web build when exported with Extensions Support (the `web_dlink` engine) and built with Emscripten 4.0.20, nightly Rust `-Zbuild-std`, and panic=abort. Tested in Chromium 2026-10-24: Driftloom's golden loops bit-identical. Source: `prototypes/02-driftloom-spike.md`, `rust/README.md`.
- [ ] V-013: The Nightmare Before Christmas is stop-motion; 9 is computer-animated, not stop-motion; check how Tim Burton was involved in each. (00-vision/pitch.md) **Partly checked:** 9 (2009) is computer-animated, directed by Shane Acker, produced by Tim Burton and Timur Bekmambetov (Wikipedia via search results, checked 2026-10-25). So "Tim Burton's stop-motion films" is wrong for 9 (see Q-033). Nightmare Before Christmas not checked. Needs: its production notes (Wikipedia unreachable here).
- [ ] V-017: The games, films and talks in `00-vision/references.md` are named and described correctly (written from memory in Phase 0; Gate A finding 6). **Still open:** only Trauma Center (V-012) and 9 (V-013) were checked this session; most sites are unreachable from this container. Needs: a session with web access, one source per entry.
- [ ] V-018: In Godot 4.7.1's built-in 2D physics, PinJoint2D's angular limits pulled a ragdoll's seams far apart on landing (up to 18 part widths; under 1 without them). Engine bug, or our misuse? (prototypes/04-physics-test.md) **Unclear:** the 4.7.1 class reference documents the limits as plain min/max rotation with nothing about a caveat. The open issue godotengine/godot#91691 (4.2.2; labels bug, topic:2d, topic:physics) reports the limits having no effect, and forum posts through 4.5.1 report them unreliable: a different symptom from ours, but evidence the feature is shaky. No issue found for seams pulling apart. Needs: a minimal repro on 4.7.1, then an issue report if it holds.
- [ ] V-019: In Godot 4.7.1, continuous collision detection (cast ray) on jointed RigidBody2D parts made a ragdoll bounce back faster than it landed (about 1.1 to 1.4 times); without it about 0.1. (prototypes/04-physics-test.md) **Unclear:** the class reference only says CCD predicts the collision instead of correcting after it; no issue or doc about jointed bodies or extra bounce was found (searched, checked 2026-10-25). Needs: a minimal repro on 4.7.1, then an issue report if it holds.

- [ ] V-020: The skateboard caught in a friend's neck because it slipped into the gap between head and body at the seam's joint, which the parts' collision shapes leave open. (prototypes/04-physics-test.md) Confirm in: a repro in the `#physics` scene, checking the collision shapes at the neck.

## Done

None yet.
