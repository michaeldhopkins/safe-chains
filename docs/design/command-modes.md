# Command modes

Status: **DESIGN, not built.** Written 2026-08-07 after a fourth workaround for the same missing
concept was proposed and declined.

## The problem, stated once

A command's behaviour often depends on WHICH FLAGS are present, and the schema can only describe one
behaviour per command. Every awkward corner of the registry is that sentence.

The tree already selects behaviour by flag in three places, each invented separately and each able to
express only a sliver of it:

| existing mechanism | what it selects on | what it can say |
|---|---|---|
| `[[command.sub]]` | a **positional** | anything — its own level, flags, nested subs |
| `[[command.flag]] classifies` | a **flag** | only "this invocation is archetype X" |
| `[command.output] invalidated_by` | a **flag** | only "the output claim is OFF" |
| (proposed) `[command.output] requires` | a **flag** | only "the output claim is ON" |

The last two are the same idea pointing opposite directions. That is what makes adding the fourth
feel wrong rather than merely tedious, and it is why this document exists instead of that field.

`[[command.sub]]` is the proof the model works. It IS mode selection — just restricted to the
positional axis, where it is allowed to carry a full description of the variant.

## Six customers, all measured

These are not hypothetical. Each was hit while doing other work, and each currently has no correct
expression.

**1. `git diff` / `jj diff` — output shape depends on a flag.**
`git diff --name-only` emits paths; bare `git diff` emits a PATCH. So `grep -rn x $(git diff
--name-only)` should be admissible and today is not, because the only way to say it would be an
unconditional `[command.output]` claim on `git diff` — which would then assert that patch text
denotes paths at the cwd locus. That is a fail-open, not merely a wrong classification: an absolute
path appearing in a diff body would be claimed worktree-bounded while the consumer reads it for real.
`invalidated_by` cannot help, because here the SAFE case is the exception.

Note `git ls-files` and `jj file list` always emit paths and need nothing new. The split between
those and `git diff` is exactly a mode boundary.

**2. `php -l file.php` — a lint mode with a file operand.**
`php`'s `[command.fallback]` is `max_positional = 0` with standalone diagnostic flags. `-l` parses a
file and reports syntax errors, executing nothing, so it is a clean `SafeRead` — but it takes an
operand. Adding `-l` plus a positional to the shared fallback would also admit `php --version
somefile`. The lint mode is a different command wearing the same name.

**3. `ruby -S CMD ARGS` — a delegation mode.**
`-S` searches `PATH` for CMD and runs it, so it is structurally `mise exec --`: the correct model is
recursion into the inner command, with an `exec`-locus gate on the value (a bare name resolves via
`PATH` and is trusted; `/tmp/evil` is not). `ruby` has no way to say "under this flag I am a
wrapper".

**4. `fourmolu --mode inplace` — the mode selector is a flag VALUE, not a flag.**
Added 2026-08-08, and it is the customer that shows how narrow a flag-presence mechanism is. The
in-place formatters and the autofix linters are one family, but they were gated two different ways —
`positional = "write"` for the formatters, the new `write_when` for the linters — which produces
opposite verdicts on the same shape:

    gofmt .git/config          DENIES   (a read-only invocation: false deny)
    ansible-lint .git/config   allows

The principled fix is `positional = "read"` + `write_when` on both. It cannot be applied, because
`fourmolu` and `ormolu` also accept `--mode inplace`: the mode is selected by a flag's VALUE, and
`write_when` only sees flag PRESENCE. Converting them would trade a narrow false deny for a real
hole, so the family stays inconsistent until predicates can say "this flag with this value".

That is the open question in this document's own predicate section, arriving with a concrete
customer attached rather than as a hypothetical. It also shows the cost of shipping the narrow
version first: `write_when` closed eight commands cheaply and then could not close the two beside
them, leaving a split that has to be explained rather than derived.

**5. `gomodifytags -w -file X` — a FLAG VALUE whose role depends on another flag.**
Added 2026-08-08. `-file` names the Go source to operate on; `-w` decides whether the run rewrites
it or prints to stdout. So the value's role is `write` with `-w` and `read` without — and that is a
DIFFERENT shape from every customer above, because `write_when` promotes POSITIONALS. There is no
declarative way to say "this flag's value is a read or a write depending on that flag", so the gate
was authored `write` unconditionally as the fail-closed choice, buying a narrow over-deny: a
read-only `gomodifytags -file ~/other-project/x.go -add-tags json` now denies.

Worth noting what this adds to the design rather than just lengthening the list: mode selection has
to apply to FLAG ROLES, not only to positional roles. A mode that could only re-role positionals
would still not serve this customer. Customer 4 needs the PREDICATE to read a flag's value; this one
needs the PAYLOAD to re-role a flag's value. Those are separate requirements, and a v1 that covers
one is not close to covering both.

`dart format` is the case that needs both at once, which is why it had to be written as a Rust
handler rather than declared: `-o write|show|json|none` selects the mode by value (predicate side),
and the selected mode decides whether the POSITIONALS are read or written (payload side). It is the
smallest invocation in the tree that defeats every declarative mechanism currently proposed, so it
is the right acceptance test for any mode v1 — if the design cannot express `dart format`, it has
not cleared the bar the existing handlers already clear.

**6. `base64 -i/-o` — the grammar itself differs by platform.**
GNU's `-i` is `--ignore-garbage`, a boolean. macOS/BSD's `-i INPUT` takes a value, and `-o OUTPUT`
WRITES a file. The same spelling is a boolean on one implementation and valued on the other, so any
single flag list is wrong on one platform. This is the `standalone`+`valued` overlap (234 scopes)
in its sharpest form — the overlap is not sloppiness, it is two grammars in one entry.

## The concept

A **mode** is a variant of a command, selected by a predicate over its flags, carrying its own
description of behaviour: level or archetype, flag lists, path roles, output claim, delegation.

An entry with no modes IS a single implicit mode — which is every entry today, unchanged.

Sketch only; the field names are not the decision:

    [[command.mode]]
    name = "name-only"
    when = ["--name-only", "--name-status"]     # selection predicate
    [command.mode.output]
    locus_from = "operands"

What it subsumes:

  - `invalidated_by` → a mode whose predicate matches those flags and declares no output claim.
  - the proposed `requires` → the ordinary case: a mode's claim holds when its predicate matches.
  - `[[command.flag]] classifies` → a mode that names an archetype.
  - platform grammars (`base64`) → two modes with different flag lists.

## Most of this is already decided by existing commitments

An earlier draft listed four "open questions". Three are not choices — they are forced by properties
this repo already holds (fail closed; authored-data mistakes are build errors; no silent partial
implementations). Recording them as open invited someone to re-litigate settled ground.

**Does a mode subsume `[[command.sub]]`? No — and the nesting requirement settles it.**
`git diff --name-only` is a mode INSIDE a sub. So modes must be able to live within a sub, which
means they cannot be the same construct: a sub selects on a positional and may contain modes; a mode
selects on flags and contains a behaviour payload. Sub and mode share the PAYLOAD type and differ
only in the selector. Unifying them into "a variant selected by any predicate" is superficially
tidier but makes selection order (positionals are ordered, flags are not) an implementation detail of
something that must stay obvious.

**Where does the level live? On the mode, completely, with NO inheritance.**
Partial inheritance is drift: a mode that specifies some fields and silently borrows the rest is the
same defect class as an entry that under-records why a subcommand sits above the line. A selected
mode fully describes that variant. The single-implicit-mode case IS the command's own fields today,
so nothing changes for the ~1600 entries that have one behaviour.

**What can a predicate say? Flag PRESENCE, and the no-match case must be the most restrictive.**
Presence covers all four customers. Values (`--format=%f`) are not expressible and stay that way in
v1 — under-reaching fails closed. The load-bearing half is the second clause: if no mode matches,
the invocation must land on the most restrictive description, never a permissive default. A
permissive fallback under a flag-selected model is a fail-open generator.

**Two modes match? Build error.** This repo already refuses ~63 authored-data conditions at load.
Ambiguous mode selection is an author mistake of exactly that kind, and "first match wins" would make
mode ORDER load-bearing and invisible.

## What is actually open

These are the ones with real content, and they are about ENFORCEMENT rather than shape:

1. **How is mode exhaustiveness proved?** If no-match must be most-restrictive, something has to
   check that a command's modes plus its fallback actually cover the flag space, or that the fallback
   is genuinely the floor. Without this the model is only as good as each author's care — which is
   the failure this whole document exists to end. Candidate: a build-time check that every mode's
   payload is <= the fallback's permissiveness, so an unmatched invocation can never be MORE
   permissive than a matched one.
2. **When are modes evaluated relative to flag parsing?** A predicate names a flag; the flag lists
   decide what is a flag at all. If a mode predicate can name a token the flag grammar does not know,
   the two disagree and the mode silently never fires. Needs a rule and a guard.
3. **How do modes compose with `[command.wrapper]` and delegation?** `ruby -S` is a delegation mode,
   but delegation is currently its own mechanism. Either a mode can carry a delegation payload, or
   this customer stays unserved.

## Where the pressure to FINISH comes from — and the evidence that a ratchet alone supplies none

A ratchet prevents GROWTH. It does not cause shrinkage, and this repo has the measurements to prove
it: `no_new_unresearched_first_arg_family` has moved 250 -> 237 families; the
`tolerate_unknown_short/long` counts (2328 / 1630) have not moved at all; the `standalone`/`valued`
overlap has sat at 234. Every one of those is a live ratchet that has been open for weeks. Assuming
this one behaves differently would be assuming a fact not in evidence.

So the honest answer to "from whence the pressure" is: **from nowhere, unless the design supplies
it.** Three things can, in descending order of reliability:

1. **Make the residue small enough to finish in one sitting, and then make the old shape a build
   error.** A ratchet is only necessary if the leftover is large. It probably is not — see the
   denominator below. This is the only option that ENDS.
2. **Make modes the only way to express new work.** Every re-researched command already has to touch
   its entry, and the re-research campaign is already running; if the new shape is required at that
   moment, migration rides on work that is happening anyway rather than competing with it.
3. **Couple it to a gate that already exists.** The pre-1.0 hardening list is the natural one.

**MEASURE THE DENOMINATOR BEFORE CHOOSING.** The migration is NOT ~1600 entries. An entry with one
behaviour is already correct as the implicit single mode and needs no change at all. The real backlog
is only entries that genuinely have MORE than one mode: the four customers, whichever of the 234
overlaps are two grammars rather than typos, and whichever glob families are actually multi-mode.
That number is unknown and is the first thing to compute, because it decides the whole strategy — a
few hundred mechanical conversions is a single pass plus an immediate build error, which is
strictly better than a ratchet that never completes.

Do not adopt the ratchet by default. Adopt it only if the measured residue is too large for one pass,
and if it is, pair it with (2) so it drains rather than merely stops growing.

## Migration: a ratchet, not a rewrite

The expensive part is not the code — it is that ~1600 entries encode assumptions a mode model would
make explicit. A big-bang migration is the wrong trade.

Modes should be ADDITIVE: no `[[command.mode]]` block means one implicit mode, exactly as today. New
and re-researched commands adopt it; the existing backlogs migrate INTO it rather than being separate
campaigns — the 234-scope `standalone`/`valued` overlap and the 237-family glob migration are both
"this entry is really several commands" problems. That converts a rewrite into a ratchet, which is
how `first_arg_standalone` already landed here.

## What not to do meanwhile

**Do not add `requires` to `TomlOutput`.** It is the fifth workaround for the concept named above,
it needs a precedence rule against `invalidated_by` that nobody has written, and `git diff` would
need BOTH fields anyway (`requires = --name-only`, `invalidated_by = --stat/-z/--relative`) — so the
very first customer exercises the confusing interaction. It would have to be unpicked.

The cheap thing that needs no new mechanism: `git ls-files` and `jj file list` always emit paths and
can take an ordinary `[command.output]` claim today. That closes part of customer 1 without
prejudging any of this.
