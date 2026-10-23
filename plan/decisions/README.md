# Decisions

One short file per decision: `NNNN-short-name.md`, from
`kit/templates/decision.md`. Numbers never get reused.

- `proposed`: options set out; waiting on Mikey.
- `accepted` / `rejected`: Mikey decided; signed with his name and date.
- `superseded`: replaced by a later decision (`superseded_by` says which).

To change your mind, write a new decision that supersedes the old one.
Don't rewrite an accepted decision.

`python3 kit/tools/check.py` lists them all by status.
