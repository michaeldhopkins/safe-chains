# The research work has moved out of this repo

The command research — the campaign to characterise every command, subcommand,
flag and flag interplay along the behavioural facet model — is no longer done
in this repository. It has moved to a separate project.

safe-chains keeps the classifier. Its `commands/**/*.toml` will increasingly be
GENERATED from that project's data rather than hand-written, and generated TOML
stays checked in so this repo remains fast, local and offline, and so the TOML
diff remains the review artifact.

## What that changes here, and what it does not

**Unchanged:** how the engine is written, how the TOML schema works, the guard
suite, and the per-command authoring conventions in `AGENTS.md` under
"Researching a new command". Hand-editing `commands/**/*.toml` remains normal
and correct until the generator is actually wired up.

**Changed:** the facet vocabulary — every `behavioral-taxonomy-*.md` in this
directory — is now owned by the research project. The copies here are
downstream and may lag; where they disagree, the research project wins. The
version implemented here is **1.4**.

## Proposing vocabulary

Ownership moving does not close the channel that has actually produced
vocabulary. Every term added in August 2026 was discovered *here*, by
implementing and hitting a gap: the `device` locus rung, the `raw-device`
region role, the `RawDevice` refusal reason, the `read_tree` path role,
`per_database`. None came from a research pass.

So when the engine needs a term that does not exist:

1. Implement what is needed to fix the bug — do not wait on anything.
2. Record the term as a proposal, with the finding that produced it.
3. Expect it to be adopted, renamed, or generalised, and follow if it is.
