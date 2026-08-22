# The every-command re-research campaign — moved

This plan now lives with the research project, which is maintained separately
from this repository. See `docs/design/research-project-moved.md`.

Kept here as a pointer because several places still cite it by name — `TODO.md`'s
campaign section, `src/registry/tests.rs` (the AWS credential-smell fixture), and
`commands/cloud/aws.toml`.

## What stayed in safe-chains

The per-command authoring conventions an author needs while editing TOML in this
repo are in `AGENTS.md` → "Researching a new command": classify behaviour along
the facets rather than "read-only vs the rest", record `researched_version`,
research the latest upstream rather than the local install, keep obsolete-but-safe
entries.

Hand-editing `commands/**/*.toml` remains normal and correct.

## What moved

The campaign itself — the batch order, the cadence, the per-batch findings log,
and the standard for what a complete research result contains.
