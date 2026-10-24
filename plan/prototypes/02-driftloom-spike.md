---
status: played
date: 2026-10-24
tests: [audio-music, platforms-tech]
---
# Spike 2: Driftloom's core inside the game

## The question

Can Driftloom's Rust core run inside Broken Friends, natively and in a
phone's browser, sounding exactly as it does in the Driftloom app
(decision 0016, V-008)?

## Smallest build

A Rust GDExtension (`rust/driftloom-godot/`, godot-rust 0.5.5) linking
the core at Driftloom commit f654fdb; a `Music` autoload streaming it
through an `AudioStreamGenerator`; six golden note lists with their
reference hashes; a test that renders them in the game and compares.

## Out of scope

Writing new music in Rust (Driftloom's generator is still JavaScript
until its phase 2), creature themes, music that follows the camp,
Android and Windows libraries.

## How it was judged

Every golden loop must render with exactly Driftloom's hashes (its
promise 3), and the music must stream with no underruns while the game
runs. Mikey's ears on a real phone come next.

## Result

**Yes, on Linux and in Chromium.**
- All six loops bit-identical to Driftloom's golden hashes, natively
  and in the browser.
- Linux 22-32x real time; browser about 19x. No stream underruns in
  4 seconds of play on either.
- The web build needs Godot's `web_dlink` engine (Extensions Support),
  Emscripten 4.0.20 to match it, nightly Rust with `-Zbuild-std`, and
  panics set to abort: Rust's Emscripten target now unwinds with
  WebAssembly exceptions, which Godot's web engine lacks.
- Cost: the extension-capable web engine is about 44 MB (was 39.5 MB);
  the extension itself 0.67 MB.
- Not yet tried on a phone, or with real speakers.

## What changed in the docs

V-008 answered. `rust/README.md` and `game/README.md`. Driftloom's own
docs now name Broken Friends as the one proof game (Driftloom PR #258).
