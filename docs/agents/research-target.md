# The research reference target

Read before researching or revising a command, before changing `tests/research_target.rs` or its
pin, and before taking an item from the research loop.

## What "researched" means for one command

A command is at the target when its `[[command]]` entry carries every artefact the procedure in
AGENTS.md ("Researching a new command") leaves behind. `tests/research_target.rs` checks each one
structurally, per command name (`tests/research_target/criteria.rs`):

| criterion | met when |
|---|---|
| `entry` | the command has a `[[command]]` entry. A command known only to Rust handler code (by the first name of its `--list-commands` heading) owes one |
| `description` | at least 160 characters, says where the network stands (network, remote, offline, HTTP, …) and how fast upstream moves (release, stable, unchanged, …), each as the start of a word, and never names our classification in any case (`SafeRead`, `SafeWrite`, `Inert`, allowlist, safe-chains) |
| `version` | `researched_version` is set, and is not "unknown" |
| `examples` | at least one `examples_safe` and one `examples_denied`; a command wholly above the line (`candidate = true` on the command) owes only the denial |
| `profiled` | no node in the tree is `candidate = true` without a `profile` archetype: `profile` is the facet-derived successor to `candidate` |
| `no-tolerance` | no node sets `tolerate_unknown_short` or `tolerate_unknown_long`: every admitted flag was researched |
| `no-overlap` | no scope of the command (or an alias) has a row in `tests/fixtures/standalone_valued_overlap.tsv`: no flag is listed in both `standalone` and `valued` of one scope |

These are proxies. They prove a step was taken, never that it was taken well: the review of each
change is what judges the research. A criterion is added here, with its check and tests, when a
new kind of research artefact becomes part of the procedure; the pin is then regenerated once
with the new criterion owed everywhere it is unmet.

"Custom code" is reported, not a criterion: an entry whose tree names a `handler`. Such a command
is held to the same target, and its handler to the standard in AGENTS.md ("When to use a Rust
handler"): logic only, data in TOML.

## The ratchet

`tests/fixtures/research_target_owed.tsv` lists each command below the target and the criteria it
misses. It may only shrink:

- a command not listed must meet every criterion, so a new command lands at the target;
- a command that now meets a listed criterion fails the test until the pin drops it
  (`UPDATE_RESEARCH_TARGET=1 cargo test --test research_target`, which refuses to add anything);
- a listed command that no longer exists fails the test too;
- a command pinned as owing `entry` may, once it has an entry, owe any of the other criteria:
  it had nothing to judge before, so the entry can land before the research is complete.

`CREATE_RESEARCH_TARGET=1` writes the pin only when it is missing, and is for creating it once.
Deleting the pin to recreate it is a rebaseline, which review must refuse.

The pin was created on 2026-10-14 with 30 of 1,622 commands at the target (1.8%).
The `no-overlap` criterion was added on 2026-10-15 and owed by every command with a pinned pair.

A command a handler dispatches but `--list-commands` does not document is invisible to this
test; handler docs are the only list of handler-only commands there is.

## Reading the gap

```bash
cargo test --test research_target -- --nocapture          # the summary: N of M at target, per criterion
RESEARCH_TARGET_REPORT=/tmp/rt.tsv cargo test --test research_target   # every command, as TSV
```

The report's columns: `name`, `at_target`, `missing` (comma-separated), `custom`, `aliases`,
`files`.

## Working one command up to the target

One command per change, as AGENTS.md's commit rules say.

1. Research the latest upstream release, the whole surface, along every facet (AGENTS.md). Run the
   tool where it can be run: `--help`, `--version`, the subcommand list.
2. Rewrite `description` as the facet profile, set `researched_version`, and add examples that pin
   the research: something approved, something refused.
3. Replace each `candidate = true` with the archetype the research supports (`archetypes.toml`),
   with `fact` and `source`. When no archetype fits, that is a mechanism gap, not a reason to
   guess: record it in TODO.md and leave the candidate in place.
4. Replace blanket tolerance with the flags the research found, valued and standalone. Sort each
   of the command's pinned standalone/valued overlap pairs from the tool's own option table into
   `optional_valued`, `valued` alone or `standalone` alone, and delete its rows from
   `tests/fixtures/standalone_valued_overlap.tsv` (the criterion `no-overlap`). A move to `valued`
   approves `--flag VALUE` and `-fVALUE` forms that deny today, so any move that approves something
   new gets the Opus review below, with the value's path role checked first.
5. For custom code, move any data the handler holds into TOML first.
6. Run the gates in AGENTS.md, regenerate the pin, and look at the verdict snapshot's regressions.

Anything that widens what is approved is security-relevant and gets a review before it ships.
Registry data and the generated command lists are not copy; a new `doc_body`, a change to the
docs' prose, the README or any message the binary prints is, and waits for the owner's approval
of the exact before and after.
