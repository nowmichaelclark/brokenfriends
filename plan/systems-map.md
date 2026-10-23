---
status: idea
---
# Systems map

To be written in Phase 2. One row per connection between two systems,
matching the "Talks to" tables in the system docs exactly.

| From | To | What flows | When |
|---|---|---|---|

Rules:

- Every piece of data has one owner. Other systems read it or ask the
  owner to change it.
- Prefer events ("creature got wet") over systems reaching into each
  other's data. It keeps them separable, and lets `audio-music` listen
  without the others knowing it exists.

A diagram may follow the table once it's stable. The table is the
source of truth.
