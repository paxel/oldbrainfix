# Domain Docs

How AI coding agents use this repo's domain documentation. The repo has one context.

## Where it lives

- **`CONTEXT.md`** at the repo root holds the terms: each one defined, with the words to avoid for it.
- **`docs/adr/`** holds the decisions, one file each, numbered `NNNN-title.md` (see `docs/adr/README.md`). A later change to a decision is added to its file as an amendment section; a decision that replaces another says so at its top.

## How to use them

- Before working on an area, read `CONTEXT.md` and the ADRs that touch it.
- Name domain concepts (in issues, code, tests, commit messages) with the terms of `CONTEXT.md`, not with the words it lists to avoid. A concept that is missing there is either a word the project doesn't use or a gap to name in the result.
- If a change contradicts an ADR, say so and name the ADR instead of overriding it silently.
