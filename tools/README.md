# The toolbox: tools that Claude sessions use

A tool is a thing that a session runs to see, test, measure or make
something without Mikey. We make tools during the whole project (R29).
A session uses a tool, writes down where the tool was wrong or weak,
and makes it better.

## Rules

1. **Look here first.** Before you check a thing by hand, look for a
   tool in this list.
2. **Make a tool when you do a thing the second time**, or when a
   check by hand is slow or easy to get wrong. If the tool needs less
   than about one hour, make it in your row. If it needs more, add a
   row (Builder table with "–" in the "Play" column, or Fix table).
3. **Add the tool to this list** in the same commit: what it is for,
   how to run it, and its limits.
4. **Write each fault of a tool in its "Limits" cell** when you find
   it: a wrong answer, a thing it does not see, a slow run. Fix it
   now if it is small. If not, add a row.
5. **Write a wish** in "Wishes" when you needed a tool that does not
   exist. One line. The Hygiene lead makes rows from the wishes.
6. A tool for the game is in Rust (CLAUDE.md, "Rust first"). A script
   in `tools/` only starts things and reads their results.

## Tools that exist

| Tool | Use it to | Run | Limits |
|---|---|---|---|
| Full check | Check a row before the commit | `tools/check.sh` (`--quick`: no Godot) | About 5 minutes. One seed in the tour. |
| Plan check | Check `STATE.md` against `rows.md` | `tools/check-plan.py` | Reads only the table format. |
| Workflow cost | See, for each week, the docs-only commits, the words a lead reads before work, and the size of `STATE.md` and `rows.md` | `tools/workflow-cost.py` (`--weeks N`, default 8) | Counts commits, not time. A file in `plan/` or `archive/`, or a `.md` file, is a doc. The newest week is not complete. Needs the full history (`git fetch --unshallow` in a shallow clone). |
| Get build | Get the build that Mikey plays | `tools/get-build.sh` | Needs `gh`. |
| Preview images | See a zone as maps, with no Godot | `cargo run --release -p bf-worldgen --example preview -- --seed 1` | Maps from above only. Only CI builds it on a clean clone. |
| Playtest recorder | Read what happened in a play | Always on; the zip is in `playtests/` (`plan/diagnostics.md`) | Checks the bear and the speed, not the picture. Small pictures with the overlay. |
| Marks | See the moment that Mikey names | Back button or F9 | One picture and 10 s of samples. |
| Benchmark, Draw distance | Compare speed on a fixed path | `-- --bf-test=benchmark`, `--bf-test=draw-distance`, with `--gpu-index 0` | Noise of about 0.5 ms. Valid only on the laptop with no build running. |
| Water view | Measure and see water and falls | `-- --bf-test=water-view`, options `--bf-water=off`, `--bf-water-<name>=<value>`, `--bf-waterfalls=off` | One view of each kind. Can select a small fall. |
| Cave tour | Measure and see the caves | `-- --bf-test=cave-tour`, option `--bf-cave-cut=off` | With `--bf-caves=off` it finds no path (S-205). |
| Autoplay tour | Let the bot play fixed tasks | `-- --bf-autoplay=tour --bf-autoplay-seeds=1,7,42` | Does not swim or go into caves yet (S-171). |
| Nightly wander | Find stuck places and falls on more seeds | GitHub, each night; `autoplay-nightly.md` on the `game-latest` release | Headless: no picture, no GPU numbers. |
| Dev menu | Go to things, turn parts off | Hold Back, or F10; F2 fly camera, F4 new zone, F6 next obstacle | Needs a hand on the pad or the keys. |

`game/README.md` has the details of each test and option.

## Tools that are planned

| Tool | Row |
|---|---|
| Camera probes (`camera_clip`) | S-213 |
| World audit (shape rules on many seeds) | S-214 |
| Look tour (clean pictures of each kind of thing) | S-215 |
| Look review (faults in pictures, reference pictures) | S-216 |
| Water sweep (one table of GPU ms for many settings) | S-182 |
| Zone fingerprint (a test fails when the zone of a seed changes) | S-217 |
| Recipe audit (ranges, promises and corners on many seeds) | S-227 |
| Corner recipes and cost numbers (the speed of the worst recipe) | S-228 |
| Playtest replay | S-124 |

## Wishes

- A way to go to a place and make one picture from the command line
  (`seed, x, z, look direction`), for a bug that Mikey names.
- A "GPU parts" test: the GPU time with each part off, one process for
  each part (`inbox.md`).
