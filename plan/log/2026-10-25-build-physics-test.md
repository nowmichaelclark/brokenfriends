---
date: 2026-10-25
session: build
model: Opus 5.5 (Claude Code)
---
# Build: the physics test (S-001)

## Changed

- `game/friend/`: `friend_view.gd` split; drawing and pose now in
  `FriendRig`, body geometry in `FriendArt`.
- `game/scenes/tent/`: `tent.gd` split into `Tent`, `TentSurgery`,
  `TentParts`, `TentCues`. No behavior change.
- `game/tests/`: the playthrough split in three; it now ends with the
  physics test; `--physics-only` runs just that part.
- `game/scenes/physics/`: new. The physics test (`#physics`): friends
  as ragdolls, drag and fling, ramp, step, skateboard, debug buttons.
- `game/debug/perf_overlay.gd`: shows physics time.
- `game/README.md`: rewritten to what the build is now (203 lines).
- `prototypes/04-physics-test.md`: built; what building it showed,
  speed numbers.
- `glossary.md`: ragdoll, seam. `verify.md`: V-017, V-018.

## Decided by Mikey

- Nothing.

## Still open

- Every physics number is a guess (`game/README.md`, "Assumed").
- Should a hard landing count as a hard pull (0025)? Built so; ask.
- V-017, V-018 (S-010).

## Next

- S-005: Mikey plays `#physics` on his phone.
