# To verify

Technical claims waiting for a Verifier session (`kit/sessions/verifier.md`).
One line each: number, claim, where it's used, how to confirm. Ticked
items are cleared by Cleanup. Next number: V-019.

## Open

- [ ] V-001: Godot has no built-in 2D soft body, so squish must be faked with springs, joints or a shader. (systems/friend-body) Confirm in: Godot class reference, 2D physics nodes.
- [ ] V-002: Godot custom Resources can hold a species' data as a file, editable in the editor without code. (systems/species-habitats) Confirm in: Godot docs, "Resources".
- [x] V-003: **True.** Source: Godot docs, Exporting for the Web (4.7): web exports use the Compatibility renderer; confirmed by exporting this project with 4.7.1, checked 2026-10-24. Godot 4 web exports must use the Compatibility renderer. (systems/platforms-tech, decisions/0004) Confirm in: Godot docs, "Exporting for the Web".
- [x] V-004: **Moot:** iOS is out of scope (decision 0015). Exporting a Godot game for iOS requires macOS with Xcode. (systems/platforms-tech, decisions/0003) Confirm in: Godot docs, "Exporting for iOS".
- [x] V-005: **Moot:** iOS is out of scope (decision 0015). Defold can bundle iOS builds from Linux without a Mac, given Apple certificates. (decisions/0004) Confirm in: Defold manual, iOS bundling.
- [ ] V-006: Godot 2D has skeletons with deformable polygon skins, joints, shaders and particles built in. (decisions/0004) Confirm in: Godot docs, 2D skeletons and cutout animation.
- [ ] V-007: Defold's 2D skeletal deformation usually means the Spine extension, which needs a paid Spine licence. (decisions/0004) Confirm in: Defold docs and Esoteric Software's Spine licence page.
- [x] V-008: **True, with conditions.** A Rust GDExtension runs in Godot 4.7.1's web build when exported with Extensions Support (the `web_dlink` engine) and built with Emscripten 4.0.20, nightly Rust `-Zbuild-std`, and panic=abort. Tested in Chromium 2026-10-24: Driftloom's golden loops bit-identical. Source: `prototypes/02-driftloom-spike.md`, `rust/README.md`.
- [ ] V-009: Changing Godot's 2D physics engine is a project setting that needs no game-code changes. (decisions/0005) Confirm in: Godot docs, Rapier plugin docs.
- [ ] V-010: Godot 2D joints don't break on their own; breaking must be scripted. (first chat; will feed systems/friend-body) Confirm in: Godot class reference, Joint2D.
- [x] V-011: **True.** Godot 4.7.1-stable downloaded and run, 2026-10-24. The current stable Godot is 4.7.1. (first chat) Confirm on: godotengine.org.
- [ ] V-012: In Trauma Center (DS), the player does the cutting and stitching directly with the stylus. (00-vision/pitch.md) Confirm in: game reviews or a playthrough.
- [ ] V-013: The Nightmare Before Christmas is stop-motion; 9 is computer-animated, not stop-motion; check how Tim Burton was involved in each. (00-vision/pitch.md) Confirm in: the films' production notes.
- [ ] V-014: Files an AI session must read in full should stay short (the hot set capped near 450 lines in all; docs at their RULES section 6 budgets), because models attend less well to the middle of long contexts and instruction files that grow long get followed less reliably. (kit/RULES.md section 6) Confirm in: Anthropic's guidance on CLAUDE.md and context, and research on long-context recall ("lost in the middle").
- [ ] V-015: Code files around 600 lines or less are easier for AI coding agents to read and edit correctly than files over 1,000 lines. (kit/RULES.md section 6) Confirm in: Anthropic's Claude Code best practices and other agent-tool guidance; say if no good evidence exists and the number is a judgment call.
- [ ] V-016: Deleting digested raw notes and relying on git history loses nothing that can't be recovered, and keeping a few files per folder (inbox 4, log 6, reviews 3, research 6) is a reasonable cap. (kit/RULES.md section 6) Confirm in: git documentation; for the caps, say whether any evidence exists or they are judgment calls.
- [ ] V-017: In Godot 4.7.1's built-in 2D physics, PinJoint2D's angular limits pulled a ragdoll's seams far apart on landing (up to 18 part widths; under 1 without them). Engine bug, or our misuse? (prototypes/04-physics-test.md) Confirm in: Godot's PinJoint2D docs and issue tracker.
- [ ] V-018: In Godot 4.7.1, continuous collision detection (cast ray) on jointed RigidBody2D parts made a ragdoll bounce back faster than it landed (about 1.1 to 1.4 times); without it about 0.1. (prototypes/04-physics-test.md) Confirm in: Godot's issue tracker.

## Done

None yet.
