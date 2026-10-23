# To verify

Technical claims waiting for a Verifier session (`kit/sessions/verifier.md`).
One line each: number, claim, where it's used, how to confirm. Ticked
items are cleared by Cleanup. Next number: V-012.

## Open

- [ ] V-001: Godot has no built-in 2D soft body, so squish must be faked with springs, joints or a shader. (systems/creature-body) Confirm in: Godot class reference, 2D physics nodes.
- [ ] V-002: Godot custom Resources can hold a species' data as a file, editable in the editor without code. (systems/species-habitats) Confirm in: Godot docs, "Resources".
- [ ] V-003: Godot 4 web exports must use the Compatibility renderer. (systems/platforms-tech, decisions/0004) Confirm in: Godot docs, "Exporting for the Web".
- [ ] V-004: Exporting a Godot game for iOS requires macOS with Xcode. (systems/platforms-tech, decisions/0003) Confirm in: Godot docs, "Exporting for iOS".
- [ ] V-005: Defold can bundle iOS builds from Linux without a Mac, given Apple certificates. (decisions/0004) Confirm in: Defold manual, iOS bundling.
- [ ] V-006: Godot 2D has skeletons with deformable polygon skins, joints, shaders and particles built in. (decisions/0004) Confirm in: Godot docs, 2D skeletons and cutout animation.
- [ ] V-007: Defold's 2D skeletal deformation usually means the Spine extension, which needs a paid Spine licence. (decisions/0004) Confirm in: Defold docs and Esoteric Software's Spine licence page.
- [ ] V-008: A Rust GDExtension (e.g. godot-rust) can run in Godot's web export, and what that requires. (systems/platforms-tech, decisions/0004) Confirm in: Godot web export docs, godot-rust docs.
- [ ] V-009: Changing Godot's 2D physics engine is a project setting that needs no game-code changes. (decisions/0005) Confirm in: Godot docs, Rapier plugin docs.
- [ ] V-010: Godot 2D joints don't break on their own; breaking must be scripted. (first chat; will feed systems/creature-body) Confirm in: Godot class reference, Joint2D.
- [ ] V-011: The current stable Godot is 4.7.1. (first chat) Confirm on: godotengine.org.

## Done

None yet.
