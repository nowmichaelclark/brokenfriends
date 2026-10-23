# Verifier session: check the facts

**Tier:** M, and it **must** have web search or the relevant docs in
the pack. A model checking facts from its own memory is not
verification. **With:** nobody.

## Purpose

Clear the open items in `verify.md`, so docs can be locked.

## Pack with

`verify.md`, the docs that contain the claims, and any documentation
pages Mikey has saved.

## How to run it

For each open item (take them in order; stop after about 15 to keep
the session small):

1. Find a primary source: official engine docs, the plugin's own
   repository or release notes, the platform's own docs. Forum posts
   only when nothing better exists, and say so.
2. Record the result in `verify.md`:
   - `- [x] V-012: ... **True.** Source: <title, URL>, checked YYYY-MM-DD`
   - `- [x] V-012: ... **False:** <what's actually true>. Source: ...`
   - `- [ ] V-012: ... **Unclear:** <what you found>. Needs: <next step>`
3. In the doc: remove the `[unverified]` tag from claims that are true
   and add the source to its **Sources** section. For a false claim,
   rewrite the sentence to what's true, mark it
   `(corrected: see V-012)`, and add the consequence to **Weak spots**.
   If the fix needs a design change, don't make it. Add an open
   question.
4. Note the engine or tool version the source applies to. Versions
   change; a fact about Godot 4.3 may not hold in 4.7.

Return `verify.md`, the changed docs, the log note and **Weak spots**.
