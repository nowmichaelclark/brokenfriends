# To verify

Technical claims waiting for a Verifier session (`kit/sessions/verifier.md`).
One line each: number, claim, where it's used, how to confirm. Ticked
items are cleared by Cleanup. Next number: V-014.

## Open

- [ ] V-001: Godot has no built-in 2D soft body, so squish must be faked with springs, joints or a shader. (systems/creature-body) Confirm in: Godot class reference, 2D physics nodes.
- [ ] V-002: Godot custom Resources can hold a species' data as a file, editable in the editor without code. (systems/species-habitats) Confirm in: Godot docs, "Resources".
- [x] V-003: **True.** Source: Godot docs, Exporting for the Web (4.7): web exports use the Compatibility renderer; confirmed by exporting this project with 4.7.1, checked 2026-10-24. Godot 4 web exports must use the Compatibility renderer. (systems/platforms-tech, decisions/0004) Confirm in: Godot docs, "Exporting for the Web".
- [x] V-004: **Moot:** iOS is out of scope (decision 0015). Exporting a Godot game for iOS requires macOS with Xcode. (systems/platforms-tech, decisions/0003) Confirm in: Godot docs, "Exporting for iOS".
- [x] V-005: **Moot:** iOS is out of scope (decision 0015). Defold can bundle iOS builds from Linux without a Mac, given Apple certificates. (decisions/0004) Confirm in: Defold manual, iOS bundling.
- [ ] V-006: Godot 2D has skeletons with deformable polygon skins, joints, shaders and particles built in. (decisions/0004) Confirm in: Godot docs, 2D skeletons and cutout animation.
- [ ] V-007: Defold's 2D skeletal deformation usually means the Spine extension, which needs a paid Spine licence. (decisions/0004) Confirm in: Defold docs and Esoteric Software's Spine licence page.
- [x] V-008: **True, with conditions.** A Rust GDExtension runs in Godot 4.7.1's web build when exported with Extensions Support (the `web_dlink` engine) and built with Emscripten 4.0.20, nightly Rust `-Zbuild-std`, and panic=abort. Tested in Chromium 2026-10-24: Driftloom's golden loops bit-identical. Source: `prototypes/02-driftloom-spike.md`, `rust/README.md`.
- [ ] V-009: Changing Godot's 2D physics engine is a project setting that needs no game-code changes. (decisions/0005) Confirm in: Godot docs, Rapier plugin docs.
- [ ] V-010: Godot 2D joints don't break on their own; breaking must be scripted. (first chat; will feed systems/creature-body) Confirm in: Godot class reference, Joint2D.
- [x] V-011: **True.** Godot 4.7.1-stable downloaded and run, 2026-10-24. The current stable Godot is 4.7.1. (first chat) Confirm on: godotengine.org.
- [ ] V-012: In Trauma Center (DS), the player does the cutting and stitching directly with the stylus. (00-vision/pitch.md) Confirm in: game reviews or a playthrough.
- [ ] V-013: The Nightmare Before Christmas is stop-motion; 9 is computer-animated, not stop-motion; check how Tim Burton was involved in each. (00-vision/pitch.md) Confirm in: the films' production notes.

## Done

None yet.
