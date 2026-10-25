# State

Updated: 2026-10-25 (first playable built; mini games and physics next)

Rewrite this file, don't add to it. Keep it under 100 lines.

## Phase

**Gate A passed** (`reviews/2026-10-25-gate-a.md`). The first playable
(`prototypes/03-first-playable.md`, B1 to B11) is built and live, as a
sketch (0021). Mikey has playtest notes to give later (B12). He wants
a better-built slice of the first version, with real mini games
(proposed 0043) that any friend can play, its body changing how
(proposed 0044). Those stand on physics, so the **physics test**
(`prototypes/04-physics-test.md`) comes next.

## How sessions run

One job per chat (decision 0001); this file, `game/README.md` and the
prototype docs carry everything between chats. Building and gates on a
strong model; Brain, cleanup and verify sessions on any model. The repo
stays lean by `kit/RULES.md` section 6: `check.py` warnings are the
next Cleanup session's jobs (plans) or Build session's (code).

## Next steps

1. **Build: the physics test** (`prototypes/04-physics-test.md`), then
   Mikey plays it on his phone. First split `friend_view.gd` and
   `tent.gd` (over 600 lines each) and trim `game/README.md` to what
   the build is now.
   - Alongside, any time, a **Cleanup session** (cheap model) digests
     `inbox/` and prunes `log/`.
2. Mikey signs or changes proposed 0043 and 0044, and answers Q-032.
3. If the physics test feels good: the skateboard grind course as the
   first real mini game.
4. Mikey's B12 playtest notes, when he's ready; tune what he flags.
5. Later: a Brain session on mini games (which ones, rewards, where
   they happen), and one on friend looks (fur, head shapes; see
   `inbox/2026-10-25-friend-looks-and-minigames.md`).
6. Mikey, whenever: Q-026 to Q-030.

## Blockers

None.

## Decided so far

- 0001 Docs first, cheap models, strong-model gates.
- 0002 2D only. 0003 Mobile first, browser too, maybe desktop.
- 0004 Godot. 0005 Built-in physics first.
- 0006 Called Broken Friends. 0007 One repo: `plan/` and `game/`.
- 0008 "A mobile pet game about stitched creatures that fall apart."
- 0009 The player is a benevolent doctor, not a god.
- 0010 Nothing gated, no microtransactions, both play patterns.
- 0011 Time passes; change happens; loss is possible; no revive.
- 0012 Pillars test the tone; non-goals fence it.
- 0013 Collection size up to the player; camps have limits; many camps.
- 0014 Free. 0015 No iPhone for now.
- 0016 Driftloom's most important proof game. 0017 Play, not train.
- 0018 Open source, free, no ads.
- 0019 Start the Godot game now. 0020 Web build on GitHub Pages.
- 0021 The build is a sketch; the plans lead.
- 0022 They're called friends. 0023 MIT licence. 0024 No corporate hooks.
- 0025 Any part can come off; parts wear out. 0026 The head is the
  friend; a dead friend's parts can be reused. 0027 Things are scarce.
- 0028 How friends die; time away. 0029 Coming back, you look.
- 0030 Gross is playful; the tone fence stays. 0031 Front and back
  only; surgery on its own table. 0032 Almost no text.
- 0033 Skills grow by repetition, quietly. 0034 Quests: main promise,
  risk on a map; new friends arrive. 0035 Many camps by biome, built
  from materials; one ending. 0036 Friend maker (dev tool first).
- 0037 Friends made by maths. 0038 Music changes at moments.
- 0039 Single player, offline. 0040 No weapons. 0041 Chores.
- 0042 Without the supplies, some things can't be treated.
- Proposed, awaiting Mikey: 0043 Many mini games. 0044 Any friend can
  play; its body changes how.

## Open counts

See `open-questions.md` and `verify.md`. `check.py` prints totals.
