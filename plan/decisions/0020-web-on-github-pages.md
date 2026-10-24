---
status: accepted
date: 2026-10-24
decided_by: Mikey, 2026-10-24
supersedes:
superseded_by:
---
# 0020: The browser build is hosted on GitHub Pages

## Decision

Every push that changes `game/` builds the web version and publishes it
to GitHub Pages. Mikey installs Android builds himself, from his phone
or the laptop.

## Context

Mikey prefers Cloudflare Pages, but Cloudflare Pages caps each file at
25 MiB, and Godot's standard web engine file is about 39.5 MB. Getting
under the cap means building a slimmed 2D-only engine. Mikey: "I don't
want to create a slim engine only for the purpose of using Cloudflare
Pages... Just want the best free option where we can have multiple
deploys per day."

## Why

- Free for a public repo, no per-file cap that the engine breaks.
- Builds run in GitHub Actions on each push; each deploy replaces the
  last.

## Alternatives rejected

- **Cloudflare Pages with a slim engine:** extra work only to fit the
  host. Worth revisiting if a slimmer download becomes a goal on its own.
- **Hosting at home:** possible, but not reachable from anywhere.

## Consequences

- Mikey switches Pages on once (Settings → Pages → Source: GitHub
  Actions).
- The game lives at https://nowmichaelclark.github.io/brokenfriends/.

## Sources

- Cloudflare Pages limits: "The maximum file size for a single
  Cloudflare Pages site asset is 25 MiB."
  https://developers.cloudflare.com/pages/platform/limits (checked
  2026-10-24)
