# TODO

safe-chains is the IMPLEMENTER of a behavioural-facet model: a bash command line
in, a verdict out, with no execution and no filesystem probing. Everything tracked
here is MECHANISM — the parser, the TOML schema, the facet engine, the region and
level models, the path gates, the guard suite, and the harness envelopes.

**What is not tracked here.** Findings about a particular command — what a flag
does, whether a subcommand reaches the network, which upstream version was read —
are not tasks. They belong in that command's `description` and its TOML fields,
authored per AGENTS.md → "Researching a new command". Likewise a row on one of the
worklist fixtures is DATA, not a todo: the guard that keeps the fixture honest is
the mechanism, and the guard is what this file owns.

The distinction is load-bearing, because the two fail differently. A wrong finding
about one command is one wrong answer. A gap in the mechanism is every command that
would have used it — which is why the backlog below is ordered by how much of the
model a gap makes inexpressible, not by how many commands are waiting on it.

## The open mechanism backlog, in order

**Tier 1 — the schema cannot express what the model already knows.** These come
first because each one forces a correct finding to be recorded as a falsehood, a
contradiction, or a dropped spelling. Nothing downstream can be trusted more than
the schema that carries it.

| # | item | section |
|---|---|---|
| ~~1~~ | ~~Optional-value flags have no representation~~ — **DONE**, `optional_valued` | "Support OPTIONAL-VALUE flags by design" |
| 2 | A valued flag mismodelled as `standalone`. **Enumerated** for long flags (118 rows, ratcheted); the short-flag half is not enumerable this way | "A valued flag mismodelled as `standalone`…" |
| 3 | The `standalone`+`valued` overlap audit — unblocked by #1. **Measured: 233 scopes, 1527 (scope,flag) pairs, 869 distinct flags** — and the bulk is short flags. Sort them, then make a raw overlap a build error | "The `standalone` + `valued` overlap audit" |
| 4 | Command MODES: the schema says one behaviour per command, but behaviour varies by flag. Four mechanisms each express a sliver | "Command MODES — design written, not built" |
| ~~5~~ | ~~`[command.output]` cannot describe a command that prints a path to somewhere else~~ — **CLOSED as not buildable.** A locus-only claim can never clear a NAME-based shield, so both candidate forms (`which`/`bundle show`, `git rev-parse --show-toplevel`) are fail-opens. Needs a which-FILES claim or harness ground truth, neither of which is a `locus_from` variant | "RESEARCHED, not doing: binding `$(which X)`…" |

#3 is next, and #2 wants doing alongside it: the audit visits exactly the scopes
where a mismodelled arity would live, so the two sweeps read the same data. Do #3
first — it removes the population #2 would otherwise have to filter.

**The short glued form (`-r0`) is still not expressible** and was left that way
deliberately: the closed value sets that need it are better enumerated
(7z declares `-r`, `-r-`, `-r0`, which is exact where a mechanism would be
approximate). Revisit only if a tool turns up with an open-ended short-glued value.

**Tier 2 — the engine resolves less than the model declares.**

| item | section |
|---|---|
| Eleven facet axes carry a declared `hazard` that no authored level constrains. Part is deliberate (the supply-chain group); the rest is unverified, and a mis-declaration there is invisible | "Eleven facet axes have no authored level constraint" |
| ~~The `user` locus rung is never emitted~~ — **DONE**, `regions::home_role`; verified 2026-09-01 | "The `user` locus rung is constructed ONLY in tests" |
| ~~Reading anywhere in `~` separable from writing~~ — **DONE**; the op×locus matrix now behaves as that section specifies | "Reading anywhere in `~` should be separable…" |
| ~~Loopback destinations~~ — the MECHANISM is shipped and generic (`is_loopback`, `loopback_valued`, `loopback_localizes`). What remains is per-service write-surface research, not engine work | "Loopback destinations" |
| ~~Atom confinement: the `$SCRATCH` half~~ — closed as CORRECT, not a bug: `$SCRATCH` is not a harness convention, so it names anywhere and must deny. The section's own sub-heading says do not reopen | "Atom confinement" |
| ~~`$(( ))` containing a substitution~~ — **DONE and guarded**; no hang, sub-second, `arithmetic_with_a_substitution_is_judged_by_its_inner_command` | "Three reported prompts", A |
| A recursive searcher cannot tell a file from a tree, so it refuses both above the workspace (accepted false deny; revisit if the shape generalises) | "A recursive searcher cannot tell…" |
| `cpio -o` reads its file list from stdin, and the list is unknowable | "`cpio -o` archives a file list…" |

**Tier 3 — dispatch and gate correctness.**

| item | section |
|---|---|
| ~~`dispatch_executor` skips the flag policy when a positional is present~~ — **DONE** via `passes_argv` (default false); karma's compensating gate removed | "`dispatch_executor` skips the flag policy…" |
| `cargo fuzz` needs a pathgate handler and a positional shape | "cargo fuzz: REVERTED" |
| Retire blanket flag tolerance (`tolerate_unknown_short/long`); glob-family migration in progress | "Retire blanket flag tolerance", "Glob-family flag migration" |
| One real structural-invariant gap remains | "Structural invariants: three probed clean" |
| Command-tree duplicates: one intent question | "Command-tree duplicates" |
| Known limitations of the gate guards — recorded, not fixed | "Known limitations of the new guards" |

**Tier 4 — surface: CLI, harness targets, refusal copy.**

| item | section |
|---|---|
| `--suggest` writes the file its name implies it only proposes — **a CLI-design decision, see below**. The other two (unparseable config, ancestor walk) are DONE and guarded | "`--suggest` writes the file…", "…appends to a `.safe-chains.toml`…", "…can write OUTSIDE the worktree" |
| ~~A denied compound construct records no reason at all~~ — **DONE**: culprit and facets now come from the command INSIDE the construct | "A denied compound construct records no reason" |
| Refusal copy — spec written, not implemented | "Refusal copy — SPEC WRITTEN" |
| A grant should cover what it names — spec written | "A grant should cover what it names" |
| ~~`--setup` silently rewrites a wrong-typed key on three targets~~ — **DONE**; two were already fixed, antigravity guards in place, and the shared helper no longer discards a non-object ROOT | "`--setup` silently rewrites…" |
| Two targets cannot self-filter on the tool — verify their envelopes | "Two targets cannot self-filter" |
| Re-tokenize split words instead of refusing them | "Re-tokenize split words" |

**Tier 5 — guards and fuzzing.**

| item | section |
|---|---|
| The registry validators are largely untested | "The registry validators are largely UNTESTED" |
| Two more fuzz targets specified but not built; the parse target finds availability bugs only | "Fuzz suite", "Fuzzing finds availability bugs only" |
| Verify the `config_load` nightly actually goes green | "Original note: verify the `config_load` nightly" |

**Decisions needed before the work can be done.**

| question | section |
|---|---|
| Three sources disagree about what a grant NAMING a credential store does (currently fails closed) | "DECISION NEEDED: three sources disagree…" |
| `permissions.allow` out-ranks every level, including `paranoid` | "DECISION NEEDED: `permissions.allow` out-ranks…" |

**Known and accepted, not scheduled.** `time ! (cmd)` does not parse while
`time ! cmd` does; the fix means editing the shared `pipeline()` path for a form
nobody writes, and the failure is a prompt. Recorded in the doc comment on
`opt_time_keyword_before_compound`, with a test pinning the ordering so a change
there is deliberate.

## When the engine needs a facet term that does not exist

Every term added in August 2026 was discovered by implementing and hitting a gap,
not by a survey: the `device` locus rung, the `raw-device` region role, the
`RawDevice` refusal reason, the `read_tree` path role, `per_database`. So:

1. Implement what is needed to fix the bug — do not wait on anything.
2. Record the term, and the finding that produced it, in the section below.
3. Expect it to be renamed or generalised later, and follow if it is.

The facet model implemented here is **v1.4** (`docs/design/behavioral-taxonomy-v1.4.md`).

---

Everything below is the detailed record: the reasoning behind each item above,
plus the history of what was already closed and why. Sections are kept after they
are done when the reasoning still explains a decision.

## PARTLY DONE: Rails 8 multi-database task variants — `db:migrate:<db>` is the one still denied

Found while researching `db:verify`. Two separate answers, and the second is the real one.

**`db:verify` does not exist.** Enumerated against a live Rails 8.1.3.1 app (`bin/rails -T`): zero
matches. Nothing to research or add.

**`dev:verify` is APP-DEFINED and must stay denied.** In the app it was seen in, `dev:` holds three
locally-written tasks — `dev:verify`, `dev:states`, and `dev:reset`, the last of which DROPS THE DEV
DATABASE. Rails does ship a `dev:` namespace (`dev:cache`), so the namespace is shared and a name
cannot tell a built-in from an app's own. Adding `dev:verify` to the built-in allowlist would let
ANY checkout define a `dev:verify` that does anything at all, which is precisely the shape
safe-chains refuses. Custom tasks belong in the project's `.safe-chains.toml`, hash-pinned — that
mechanism exists for exactly this.

**The real gap: per-database task variants.** Rails 8 ships solid_cache, solid_queue and solid_cable
by default, so a NEW Rails 8 app has four databases and rake generates a variant per database:

    db:create        db:drop         db:migrate       db:migrate:down
    db:migrate:redo  db:migrate:reset  db:migrate:status  db:migrate:up
    db:reset         db:rollback     db:schema:dump   db:schema:load
    db:setup         db:version

14 base tasks x 4 databases = **56 tasks, every one denied today** (`db:migrate:primary`,
`db:create:cache`, `db:drop:queue`, `db:migrate:down:cable`, …) while the bare `db:migrate` allows.
Also missing regardless of multi-db: `db:environment:set`, `db:migrate:up`, `db:migrate:down`, and
the genuine built-in `dev:cache`.

Enumeration cannot close this. The trailing segment is a database NAME out of `config/database.yml`,
so `primary/cache/queue/cable` are only this app's; another app has `db:migrate:analytics`. The
honest rule is behavioural: **`db:<task>:<dbname>` does to one configured database exactly what
`db:<task>` does to all of them, so it inherits the base task's classification** — which means a
suffix-pattern primitive in the TOML rather than 56 more lines.

BUILT: `per_database = true` on a `[[command.sub]]` makes it also match one trailing plain
identifier, inheriting the base classification. Eleven db: tasks carry it, so `db:create:cache`,
`db:schema:dump:queue`, `db:version:primary` and the rest now answer as their base does, while
`db:drop:cache` and `db:rollback:primary` stay denied with theirs.

STILL DENIED, and this is the interesting part: **`db:migrate:<db>`**, the most common variant of
all. `db:migrate` prefixes three other declared tasks — `db:migrate:redo`, `:reset`, `:status` —
and `db:migrate:primary` (a database) is the same SHAPE as `db:migrate:redo` (a task). Nothing in
the string distinguishes them, so marking `db:migrate` read `redo` as a database name and admitted
a destructive task that is deliberately `candidate`. The repo caught it: `examples_denied` already
held `rails db:migrate:redo VERSION=1`.

`filter_candidates` now refuses the marking on any sub that prefixes another declared task, so it
cannot be re-added by hand. `db:migrate`s four variants are instead ENUMERATED — primary, cache,
queue, cable, which is exactly a default Rails 8 app.

RESIDUE: a CUSTOM database name under db:migrate (`db:migrate:analytics`) still prompts, because
enumeration cannot cover it and the rule cannot be used here. Closing that needs a way to say
"these suffixes are subtasks, anything else is a database" — an exclusion list on the marking.
Small, and worth doing only if a real app hits it.

Net: 28 of the 56 variants newly allowed. The other 28 belong to db:drop / db:rollback / db:reset
and correctly stay denied with their bases.

## RESEARCHED, not doing: binding `$(which X)` / `$(command -v X)` as a path

`head -1 $(which bundle)` denies — the substitution is unpinnable. Reported by the decision log
2026-08-18. The obvious fix is a `[command.output]` claim on `which`, on the reasoning that it
prints an executable path on `$PATH` and `$PATH` resolution is already safe-chains' trust model for
bare command names (AGENTS.md §0.2). Measured, that reasoning does not survive:

    which bundle        →  /Users/michaelhopkins/.local/share/mise/installs/ruby/3.4.2/bin/bundle
    which -a python3    →  three paths, the first also under ~/.local/share/mise
    command -v cd       →  cd                      (a bare word, not a path)
    command -v f        →  f                       (a shell function)
    command -v ll       →  alias ll='ls -l'        (arbitrary text with quotes and spaces)

Two separate problems:

1. **`command -v` does not print paths.** For a builtin, function or alias it prints a word or a
   whole alias definition. Declaring an output locus on it would be a straight fail-open of exactly
   the kind SAMPLE.toml warns about — the field is TRANSITIVE, so it widens every consumer.
2. **`which` prints a real path, but not to a predictable place.** A version manager (mise, asdf,
   rbenv, nvm) puts the binary under `~`, so "a standard bin directory" is false on the first
   machine tested. There is no existing `OutputLocus` variant that fits, and the honest bound for
   "wherever PATH happens to point" is `machine` — which is what an unpinnable path already gets.

So the only version worth building is a new variant meaning "a PATH-resolved executable, locus
machine", declared on `which` alone and never on `command -v`. Under the read policy that would be
enough to make `head -1 $(which bundle)` allow while `rm $(which bundle)` still denies.

Deferred rather than done because it is a schema addition with a transitive fail-open surface, for
one convenience form, and the `command -v` half must be excluded on evidence rather than by
omission. If it is built, the guard is: `$(command -v <alias>)` must NOT become a bounded path.

### That variant CANNOT be built — checked 2026-08-29, and the reason generalises

The paragraph above is wrong, and the engine already contains the refutation. `resolve.rs`, in the
`OutputLocus::Operands` arm:

    // A bounded claim is only meaningful BELOW `user`. […] At `user` or above it is not: the
    // claim carries a LOCUS and says nothing about WHICH file, and which file is exactly what
    // the shield needs to see.
    if worst >= LocalLocus::User { return None; }

A "locus machine" variant is `>= user` by construction, so it is precisely the shape that rule
exists to reject. The bug it was added for — `cat $(fd pat ~/.ssh)` allowed while
`cat ~/.ssh/id_rsa` denied — is the same bug this variant would reintroduce, and
`no_abstraction_is_more_permissive_than_a_path_it_could_denote` would catch it as a property
violation rather than as one example.

The generalisation is worth stating plainly, because it applies to every future proposal of this
kind: **the credential shield is NAME-based, so a claim that carries only a LOCATION can never
clear it.** Below `user` that does not matter — nothing under the worktree is a credential store,
so the rung is the whole truth about the value. At `user` and above the two are independent, and no
amount of precision about WHERE substitutes for knowing WHICH.

`git rev-parse --show-toplevel` looked like the sound sub-case and is not, for a second reason
worth keeping separate: it would claim the locus of `.`, but it NAMES an ancestor of `.`. Where the
workspace root is itself a subdirectory of a larger repository — a monorepo subdirectory opened as
the project — the toplevel is ABOVE the workspace, so the claim would report `worktree` for a path
that is `adjacent` or `user`. That is an under-report, which is the fail-open direction, and it is
unreachable statically because the toplevel is exactly what we do not know.

So this is not "a schema addition nobody got to". Both candidate forms are refused by the same
structural fact, and a variant that ignored it would be a fail-open with a guard already written to
catch it. What would actually be needed is a claim that bounds WHICH FILES rather than which
location, or ground truth from the harness about where the substitution resolved — neither of which
is a `locus_from` variant. Until one of those exists, `$( )` in a path stays unpinnable, and the
cost is a prompt on `sed -n … $(bundle show doorkeeper)/…`, which is the correct trade.

## DECISION NEEDED: three sources disagree about what a grant NAMING a credential store does

Measured, with `[[grant]] path = "~/.ssh"  read = true  write = true`:

    touch ~/.ssh/x            ALLOW    the grant widened the WRITE face
    cat ~/.ssh/known_hosts    DENY     the READ face did not move
    cat ~/.ssh/id_rsa         DENY

Identical in 0.226.0 — pre-existing, not introduced by the read-policy release. Three places in
the tree describe this differently and they cannot all be right:

1. `regions/default.toml` header — "a user grant can NEVER widen it (see `apply_grant`), so
   `grant ~/` still can't hand over an SSH key." Reads as absolute.
2. `docs/src/how-it-works.md` — "A grant on a parent directory never reaches `~/.ssh` … A grant
   that names one does, and so does a grant on a path inside one." Promises naming works.
3. The `ReachReason::Credential` nudge — "If you do want to allow it, name that path in
   ~/.config/safe-chains.toml." Tells the user to do the thing that does not work for a read.
4. `a_grant_reaches_a_credential_store_only_when_it_names_it` asserts naming DOES reach it — but
   at the region layer (`classify_region(path).read_locus`), which is not where the read is
   finally decided.

The mechanism: a grant lowers the region role's locus, and the write face is decided from that.
The read runs additionally through `reads_path` → `unclearable_read` → `names_credential_store`,
which is a test on the path alone and never consults grants. So the write face honours the grant
and the read face cannot see it.

Which way to resolve is a real policy choice, not a bug fix:

- **Make the shield absolute for reads.** Fix (1) as the truth; correct the doc and the nudge, and
  the nudge needs a different remedy (there is none — that is the point). Simple, and the current
  behaviour already matches.
- **Make naming work for reads.** Fix (3) as the truth; `names_credential_store` becomes
  grant-aware. This is the intent the region-layer guard encodes, and it makes the documented
  remedy real — but it puts grant logic inside the shield, which is the one component whose value
  is that it cannot be widened.

Worth noting the asymmetry is currently backwards from the release's own philosophy: writes to a
named store are permitted and reads are not, when reads are the direction this release opened.

NOT fixed in 0.227.0 deliberately: it is pre-existing, it fails CLOSED (over-denying), and either
resolution touches the credential shield, which wants its own change and its own review. The docs
were left as they are rather than edited to match the current behaviour, because editing them
would presuppose the first option.

## `cpio -o` archives a file list it reads from stdin, and the list is unknowable

`cpio -o < ./list` reads pathnames from stdin and writes those files to stdout as an archive. The
list is data, so nothing in the command line names what gets read:

    printf '%s\n' ~/.ssh/id_rsa > list && cpio -o < list > out

`cpio` is in the `write` pathgate group, which gates its operands — but here there are none. The
same shape as `xargs`, and unlike xargs the stdin side is not modelled: there is no pipe whose
left-hand side can be classified, because the list arrives from a file redirect.

Pre-existing — open in 0.226.0, not introduced by the read-policy change. Not fixed because the
honest fix is to model the stdin-list commands (`cpio -o`, `tar -T`, `xargs -a`, `rsync
--files-from`) as a class, binding their item to the same unknowable sentinel a traversal gets,
and that wants doing once rather than per-tool.

Found by the review probe alongside `pax -w ~` and `ditto ~/.ssh ./s`, which ARE fixed.

## A recursive searcher cannot tell a file from a tree, so it refuses both above the workspace

`rg`, `ag`, `ack`, `ugrep`, `sift` and `pt` are gated `read_tree_after_first`: their path operand
is the root of a recursive search, and a root cannot be cleared by a shield that tests names —
`rg foo ~` names `~`, which is not a credential store, then reads `~/.ssh/id_rsa` out of it.

The cost is that `rg foo ~/notes.txt` also refuses, because nothing static distinguishes a file
operand from a directory operand. Two ways out, neither taken yet:

- Read the flags that bound the search (`--max-depth 0`, `-g`, `--files-with-matches` on a single
  named file) and downgrade to a single read when the search provably cannot descend. Real, but it
  is per-tool grammar work and each tool spells it differently.
- Model these tools in the engine the way `grep` is modelled, where recursion is known rather than
  assumed. Bigger, and the right end state.

Not urgent: this is exactly what these tools did before local reads opened up, so it is unchanged
behaviour rather than a regression, and `grep` reads the same file fine.

## DECIDED (2026-08-16): dotfiles are allowed; the shield keeps naming the credential ones

**Option 1 below — keep enumerating.** `~/.zshrc`, `~/.gitconfig`, `~/.config/nvim/init.lua` and
every other home dotfile read without a prompt; the shield names the credential-bearing ones and
those refuse. No allowlist for dotfiles, no grant requirement.

This accepts the trailing-denylist risk stated below with eyes open: the next tool that invents a
credential dotfile is readable until someone adds it. That is the cost of the thing being useful,
and the alternative — a curated allowlist of readable dotfiles — buys less than it costs, because
the common case is an agent reading ordinary config and the uncommon case is the one we can name.

What to do about it, instead of inverting: **treat a new credential store as a bug to fix, not a
gap to tolerate.** When one is found, declare it. The enumeration is at 24 (23 dotfiles plus
safe-chains' own decision log, which is a store we KNOW the contents of). The two probe rounds that
found the last ten are worth repeating whenever this area is touched.

The analysis that led here is kept below, because it is the argument for revisiting if the
enumeration starts losing visibly rather than theoretically.

---

Admitting all of `~` rests on the shield naming every credential-bearing dotfile. Two rounds in, that
is not looking like a race the enumeration wins.

Round one declared thirteen from research. An adversarial probe immediately found **ten more
readable** — `.boto`, `.vault-token`, `.databrickscfg`, `.authinfo`, `.hgrc`, `~/.pip/pip.conf`,
`~/.kaggle/`, `~/.subversion/auth/`, `~/.oci/`, `~/.snowflake/`. `.boto` had been WRITTEN DOWN in
this file as missing and then not added, which is the failure mode in miniature: even the person
holding the list drops entries from it.

All twenty-three are now declared and `~/.zshrc` still reads. But the structural point stands: a home
dotfile is config, config for anything that authenticates holds a credential, and the set of things
that authenticate is unbounded and grows. `~/.vault-token` is a bare live token in a file named after
itself; nobody would have guessed `.authinfo` without knowing Emacs. Each new tool ships a new one.

Three ways out, and this wants a decision rather than another pass:

1. **Keep enumerating.** Honest about what it is — a denylist, permanently trailing. Cheap per entry,
   never finished, and every gap is a plaintext credential in the model's context.
2. **Invert for dotfiles only.** Non-dotfile home content reads freely; a home DOTFILE reads only if
   allowlisted (`.zshrc`, `.gitconfig`, `.vimrc`, `.tmux.conf`, …) or covered by a user grant. This
   is allowlist-shaped, matches the project's stated posture, and fails closed on the next tool's
   invention. Cost: a curated list, and a prompt the first time someone reads an unlisted dotfile.
   Note this is NOT the region admit-map that was deleted — that map admitted whole system roots to
   get reads working at all; this is one narrow list at one rung.
3. **Gate dotfiles behind the grant mechanism.** `[[grant]] path = "~/.config/foo"` already exists and
   is the user's own trust statement. Zero curation, more friction.

Nothing blocks the bound lift on this — it is a policy choice about how much of `~` opens — but it
should be made deliberately rather than settled by which files someone happened to think of.

## DECIDED (2026-08-16): sibling DELETE stays refused at developer — revisit on feel

A sibling checkout (`../branchdiff`, the `adjacent` rung) is readable and writable at developer, but
not deletable:

    cat ../branchdiff/x            allow
    touch ../branchdiff/x          allow
    cp ./a ../branchdiff/b         allow
    mv ../branchdiff/a ./b         allow
    rm ../branchdiff/x             DENY
    rm -rf ../branchdiff/build     DENY

The asymmetry is deliberate, not an oversight. Reaching into a peer project to read it, or to write
a file into it, is the thing agents legitimately do across a multi-repo checkout. Deleting out of
one is a different proposition: the blast radius is a project the user did not point the agent at,
and `rm -rf ../<sibling>/build` is one typo away from `rm -rf ../<sibling>`. The reversibility spine
already treats destroy as the operation that earns the most caution, and `adjacent` is the rung
where "the agent was invited here" stops being true.

**Marked for revisit based on how it feels in use.** If cleaning a sibling's build output turns out
to be a routine prompt, the answer is probably a `destroy` clause scoped to `adjacent` at developer
rather than a blanket lift — the sibling equivalent of how worktree destroy is already admitted.
Until then it stays refused, and a user who wants it can grant the path.

Covered by the `write-sibling-destroy` rows in `tests/fixtures/path_policy_corpus.tsv`, so a change
here shows up as a corpus diff rather than a surprise.

**One edge this decision does not settle**, flagged rather than decided quietly: `mv
../branchdiff/a ./b` is ALLOWED, and it does make `a` disappear from the sibling.

It is NOT a hole in the rule, and reading it as one is the trap — that argues from consequence (the
peer project is missing a directory) to classification (so call it a destroy). `mv` is a relocate:
the bytes are intact and the act reverses. What it exposes is a vocabulary gap — the model cannot
currently say "these bytes moved from one locus to another", so the sibling rule had to attach to
`destroy`, which `mv` correctly does not carry.

Diagnosis and the proposed fix (an `Operation::Relocate` term, plus an `origin` on `Locus`) are in
**`docs/design/behavioral-taxonomy-relocation.md`**. Deferred out of 0.227.0 on purpose: it is a
schema change to the level files and wants its own release.

Note the design note does not presuppose the verdict changes. Once the crossing is expressible,
allowing `mv` out of a sibling may well be the right answer.

## Permissive reads: what is left, and the one finding that changes its shape

Landed so far (all zero-behavioural-delta, so they are safe on their own):

- the credential shield reaches the level algebra at all (`secret · reads`), instead of resting
  entirely on the locus cap that is about to be lifted;
- a read the shield cannot CHECK (`$VAR`, an undeclared `$(…)`, an xargs item) claims secret too;
- shields are no longer OS-gated — `/etc/shadow` and `/root/` bite on macOS;
- another user's home (`~root`, `~alice`) is shielded rather than merely far away;
- `LocalLocus::User` is reachable: `~/notes.txt` resolves to `user`, siblings still to `adjacent`.

Remaining, in order:

**1. Home DOTFILES are the hard part, and were not in the plan.** `home_role` deliberately excludes
any hidden component, so `~/.zshrc` and `~/.cargo/registry/…` still resolve to `machine`. Lifting
that is not a one-line follow-on, because home dotfiles are simultaneously the most ordinary read on
the disk and the most credential-dense. The shield does NOT currently name:

    ~/.git-credentials     plaintext usernames and passwords
    ~/.npmrc               `_authToken=` registry credentials
    ~/.pypirc              PyPI upload passwords
    ~/.pgpass              Postgres passwords
    ~/.boto                AWS credentials (legacy)
    ~/.dockercfg           the pre-`.docker/` registry auth file

`a_grant_does_not_widen_hidden_files_or_system_secrets` caught the first version of this, where
dotfiles were admitted and `~/.git-credentials` classified `user`.

**But excluding them does not protect them, and the first draft of this note claimed it did.**
Adversarial review measured it: an excluded dotfile falls through to `unknown` → `machine`, and
`machine` reads are exactly what the bound lift admits. With the bound lifted,
`cat ~/.git-credentials` ALLOWS — indistinguishable from `~/.zshrc`. The exclusion only bites while
the cap is down, which is precisely when nothing needed protecting.

So declaring the credential dotfiles in the shield is a HARD PREREQUISITE for lifting the bound, not
a follow-up to it. Nothing else is standing in front of those files.

**2. Lift the reader level's observe bound** from `<= worktree-trusted` to `<= machine`, and delete
the seventeen `package-content` nodes it exists to compensate for.

**3. ~~Restate the property guards.~~ DONE** — `UNREADABLE` split out of `OUT_OF_WORKSPACE`, the
read guards draw from it, and they pass under BOTH policies. That was expected to be the blocker and
was not: with the bound lifted experimentally the restated guards stayed green, and so did
`no_abstraction_is_more_permissive_than_a_path_it_could_denote` and
`substitution_is_never_more_permissive_than_a_path_it_could_produce`.

**4. xargs — 19 laundered compositions down to 9, two distinct causes left.**

The first cause is FIXED and shipped (see `unpinnable_after_resolution`): the unshieldable test ran
`tagged_substitution` before checking, and that function replaces a substitution sentinel with a
benign residue, handing the locus back separately. So the check saw the residue and reported
"pinnable" for the very sentinel that means unknowable — and `xargs cat`, whose items resolve to
exactly that sentinel, slipped the shield. Checking before the rewrite closed 10 of the 19.

The nine that remain, measured with the bound lifted:

**4a. Transfer sources (4 cases).** `xargs -I{} cp {} /tmp/dest`, same for `ln -sf`. `cp`'s source
capability comes from `transfer_profile`/`per_source`, not `reads_to_model`, so it never reaches
`reads_path` and never claims secret. The fix is the same conversion already done for the readers —
route the transfer SOURCE through the path-aware builder. Note the guard's comparison is partly an
artifact here (bare `cp /tmp/dest` denies for having one operand, not for a locus), but the
underlying read of an unknowable source is real.

**4b. Locus-bound stdin representatives (5 cases).** `find / | xargs -I{} cat {}`. The pipeline
walker binds the item to a representative carrying the SOURCE's output locus, so `find /` yields a
synthetic path at `machine` rather than the unpinnable sentinel. It is therefore "pinnable", the
shield is consulted on a name that is not the real file, `names_credential_store` says no, and with
machine reads admitted the whole thing auto-approves — including `~/.ssh/id_rsa` if the find turned
it up.

This is the deeper one, and the rule that fixes it is: **an item representative is shield-clearable
only when its locus is strictly below `user`.** At worktree/adjacent/temp the item cannot be a
credential store, so the bound is meaningful and `find ./src | xargs cat` should keep working; at
`user` or above the item could be anything and the shield was never really consulted. Cleanest
implementation is at the source — have the walker emit the UNPINNABLE sentinel instead of a
locus-bound repr when the source locus is `>= user`, so everything downstream treats it as unknowable
without needing to know about provenance.

Until 4a and 4b land the bound stays where it is. The corpus reaches 70/70 with it lifted, so these
are the only things between here and done; the bound-lift diff is preserved at
`scratchpad/bound-lift.patch`. The other ~58 failures under the lift are policy snapshots naming the
old behaviour (`macos_system_and_home_are_not_auto_read`,
`reads_the_workspace_denies_everything_outside`) plus `loop_over_*_sub` and `resolution::*` locus
assertions — mechanical, but they should be re-derived rather than bulk-edited.

`tests/fixtures/path_policy_corpus.tsv` is the acceptance test — 60/70 today, with all ten
mismatches in `read-home` and `read-machine`.

## DONE 2026-09-01 — the `user` rung, and the read/write split that depended on it

Both sections below are resolved, and were resolved before this check: `regions::home_role` is the
production construction site the first one says does not exist, and its doc comment narrates the
same defect. Re-measured rather than assumed, with cwd/root at a project:

    ~/notes.txt            user        (was machine)
    ~/Downloads/x.pdf      user        (was machine)
    ~/.grok/config.toml    user        (was machine)
    ~/.cargo/registry/…    user        (was adjacent — a hand-placed rung)
    /etc/hosts             machine
    /usr/lib/x.so          machine     (was adjacent — the same collapse pointing permissive)

And the operation × locus matrix the second section asks for:

    observe · user      ALLOW    cat ~/notes.txt
    create · user       DENY     touch ~/x
    mutate · user       DENY     tee ~/x
    destroy · user      DENY     rm -rf ~
    mutate · machine    DENY     tee /etc/hosts
    secret axis         DENY     cat ~/.ssh/id_rsa   (orthogonal, as specified)

One deliberate difference from what that section proposed: it wanted `anything · machine` refused,
but `observe · machine` ALLOWS (`cat /etc/hosts`). That is the permissive read policy as decided —
everything readable except the credential shields — not a regression against this design.

Guarded end to end, so the "green tests documenting a rung production never reaches" problem is
closed too: `locus.rs` asserts `read_locus("~/notes") == User` and `regions.rs` asserts
`classify_region("~/notes.txt").read_locus == User`, both against real path strings rather than
hand-built enum values.

**Kept below for the reasoning, which is still the clearest statement of why the rung matters.**

## The `user` locus rung is constructed ONLY in tests — the resolver never emits it

The precise defect, which is sharper than "the rung is unusable": `LocalLocus::User` has **zero
production construction sites**. Every reference outside the enum itself is a test
(`authoring.rs`, `bridge.rs`, `level.rs`) building the value by hand, or a bound comparing against
it. Nothing in the path→locus resolver ever produces it.

Meanwhile the rung is fully SPECIFIED. `facet.rs` defines it as `User => "user", // ~, keychain`,
and the level-authoring tests assert real behaviour against it:

    authoring.rs:407  assert!(!read_local.admits(&observe_at(LocalLocus::User)), "cat ~/.ssh/id_rsa");
    authoring.rs:500  assert!(!developer.admits(&destroy_at(LocalLocus::User)), "rm -rf ~");
    authoring.rs:552  assert!(!developer.admits(&exec_at(LocalLocus::User)), "~/x.sh waits for a higher level");

So the ladder was authored against `~` being `user`, those assertions pass, and **none of them
describes what happens to a real `~` path** — which resolves to `machine`, on reads and writes
alike. Green tests documenting a rung production never reaches.

## Reading anywhere in `~` should be separable from writing anywhere in `~`

Measured 2026-08-14 (0.225.0), reading with cwd/root at a project:

    ~/notes.txt                          locus.local = machine
    ~/Downloads/x.pdf                    locus.local = machine
    ~/.grok/config.toml                  locus.local = machine
    ~/.local/state/safe-chains/log.jsonl locus.local = machine
    ~/.claude/CLAUDE.md                  locus.local = worktree-trusted
    ~/.cargo/registry/src/x/lib.rs       locus.local = adjacent
    /etc/hosts                           locus.local = machine
    /usr/lib/x.so                        locus.local = adjacent

**Nothing lands on `user`.** An ordinary file in the user's own home gets the SAME rung as
`/etc/hosts` — the rung that is supposed to mean machine-wide. The ladder has a rung between
`worktree-trusted` and `machine` whose entire population is "things under `$HOME`", and that
population resolves past it.

Three consequences, and the third is why this is not cosmetic:

1. **`machine` is not true.** `~/notes.txt` is not machine-wide state. A facet term that says
   something false is worse than a coarse one, because levels are authored against the term.
2. **A level cannot express "your own files, yes; the rest of the machine, no."** Admitting `user`
   admits nothing (dead rung); admitting `machine` also admits `/etc` and `/usr`. The distinction
   the ladder was drawn to make is unavailable.
3. **So the admit map is doing the level's job.** `~/.claude` → worktree-trusted and
   `~/.cargo/registry` → adjacent are hand-placed exceptions, each one a person noticing a false
   deny and hardcoding a rung. That is the leak `project_path_retreat` already decided to walk back,
   seen from the other end: the map exists BECAUSE the rung below it is unusable.

Note `/usr/lib` → `adjacent` too, which is the same defect pointing the permissive way: a system
library directory reading as "sibling workspace" is how `/usr/local/lib/foo.rb` ends up auto-approved
while `/usr/bin/env` denies.

**Make the rung reachable, then say the policy in facet terms.** Reading anywhere under `~` while
writing nowhere under `~` is a reasonable stance and is EXACTLY what the ladder was drawn to express.
The facets already carry the two axes separately — `operation` (observe vs create/mutate/destroy)
against `locus.local` — so the asymmetry needs no new vocabulary, only a resolver that emits `user`:

    observe · user                  admitted   (cat ~/notes.txt, cat ~/.local/state/…/log.jsonl)
    create/mutate/destroy · user    refused    (touch ~/x, rm -rf ~)
    anything · machine              refused    (/etc, /usr, another user's home)

Today both lines collapse into `machine`, so the read cannot be allowed without also allowing the
write, and neither without also allowing `/etc`. That collapse — not the policy — is the bug. Note
the credential shield is orthogonal and stays: `~/.ssh` denies on the secret axis, not the locus one,
which is why "read all of `~`" does not mean "read your keys".

Doing this should also let the admit map shrink toward the credential shield, which is what
`project_path_retreat` wanted: `~/.claude` → worktree-trusted and `~/.cargo/registry` → adjacent are
hand-placed rungs that exist because the rung beneath them never fires.

Watch `/usr/lib` → `adjacent` while fixing this; it is the same collapse pointing the permissive way.

## A denied compound construct records no reason at all

The decision log leaves `culprit: null` and `facets: null` for a `for`/`while`/`case` construct —
`explain` sees one segment, and the engine will not resolve a command whose first word is `for`, so
the entry says "denied" and stops. Measured on real entries; roughly a third of the denials in the
author's own log read "no reason recorded".

That is the same hole the per-segment facets closed for `&&`/`;` chains, one level further in: the
interesting command is INSIDE the loop body and never gets classified on its own for reporting. The
fix is presumably to walk the construct's body the way `explain` walks a chain, and attach a reason
per inner command.

DONE 2026-09-01. Confirmed first on `(…)`, `{…}`, `if`, `for`, `while` and `case`: every one denied
with no profile and no refusal line. Two separate causes, which is why the first change alone did
nothing visible:

1. `command_label` returned `None` for every non-`Simple` command, so no culprit was found. It now
   descends the body — including branches and arms that may not run, since the classifier already
   treats such a command as only as safe as its worst body, and reporting has to look in the same
   places.
2. The culprit was then suppressed anyway by `commands.len() <= 1`. That rule is right for a lone
   SIMPLE command, where the segment text already IS the command and labelling it says nothing —
   but a compound is one command too, and there the inner name is the only actionable information
   there is. Narrowed to `[Cmd::Simple(_)]`.

And a third for the facet block specifically: `facet_breakdown` tokenises the raw string with
`shell_words`, which cannot see into a construct — `(cat ~/.ssh/id_rsa)` splits to `["(cat",
"~/.ssh/id_rsa)"]` and no resolver claims `(cat`. It now asks the CST for the inner denied command's
words and labels the output with which command it is describing.

A construct whose inner command is LEGACY-classified still shows no facets (`case … tee /etc/hosts`),
which is the documented behaviour of that block — no resolver claims it, so there is nothing to
render. That is a property of the inner command, not of the construct.

## `sed -i ''` on macOS: the empty suffix is eaten as the script, and a `$` then denies

Found 2026-08-14 by the decision log, on the first real chain it recorded from another session —
which is the feature working as intended, so it is worth saying that is how this arrived.

    cwd = <project>/web
    sed -i '' '1257,$d' app/x.css      DENIED    locus.local = machine
    sed -i '' '1257,900d' app/x.css    allowed   (worktree)
    sed -n  '1257,$p'    app/x.css     allowed   (worktree)
    sed -i.bak '1,$d'    app/x.css     allowed   (worktree)

So it is the COMBINATION of `-i ''` and a `$` in the script, and the mechanism follows from the two
sed dialects. GNU takes the in-place suffix ATTACHED (`-i.bak`) and never as a separate word; BSD
(macOS) requires it SEPARATE, and empty for "no backup". Modelled the GNU way, `-i` consumes
nothing, so `''` lands in the first positional — which for sed is the SCRIPT. Everything shifts one:
the real script `1257,$d` becomes a FILE operand, its `$` reads as an unpinnable variable, and the
operand list worst-cases to `machine`.

Fail-CLOSED, so not a hole — but `sed -i ''` is the standard macOS spelling and `$` is everywhere in
sed (`$d`, `$p`, `1,$s/…`), so the false-deny is common on this platform. Two things to get right in
the fix, neither obvious from the symptom:

- The dialects genuinely conflict on the same flag, exactly as `base64 -i` does (see
  `commands/data/base64.toml`, which documents the same GNU/BSD split and concludes either modelling
  is wrong somewhere). Whatever is decided here should probably decide that too.
- Accepting a separate `-i ''` must not accept a separate `-i /etc/passwd`: under BSD the word after
  `-i` is a SUFFIX, not a path, so admitting it as a value has to be narrow (empty string only) or
  the flag starts swallowing operands.

Guard shape: a `$`-bearing single-quoted sed script must classify the same with `-i ''`, `-i.bak`
and `-n` — one operation, three spellings, one answer.

## `--suggest` writes the file its name implies it only proposes — refine the CLI

`safe-chains --suggest "<cmd>"` analyses the command, generates a `.safe-chains.toml` entry, and
WRITES it to the project root (`std::fs::write`, src/main.rs). It then prints a `[[trusted]]` pin for
the user to paste into `~/.config/safe-chains.toml`. The name says "suggest"; the success message
says "Added this to …". Those disagree, and the flag name is the one people read.

The write itself cannot escalate trust — the pin is `path + sha256 OF THE FILE CONTENTS`
(`registry::custom::repo_is_trusted`), so an unpinned file is ignored and any later edit to a pinned
one breaks the hash and drops the whole file back to ignored. A second `--suggest` therefore cannot
silently widen an already-pinned project; it invalidates the pin.

So the open question is CLI design, not a hole:

- Should the default be dry-run — print the block and the pin, write nothing, with `--write`
  (or `--apply`) to commit it? That matches the name and matches the runner-script convention.
- If it keeps writing by default, does the flag want renaming?
- `safe_chains_knows_its_own_cli_flags` lists `suggest` and `generate-book` in `NOT_AUTO_APPROVED`.
  Once the write behaviour is settled, revisit whether a dry-run `--suggest` should auto-approve
  (it would be a pure read) and whether `--generate-book`, a plain worktree docs write, belongs in
  that list at all. `--setup`/`--tool`/`--auto-detect` stay out regardless: they write ANOTHER
  tool's config outside the worktree and reconfigure how it behaves afterwards.

Raised 2026-08-13 while reviewing the OmniFocus inbox item "This is safe safe-chains --suggest …".

## HIGH: `dispatch_wrapper` skips valued flags without looking at their values

`dispatch_wrapper` consumes a valued flag with `i += 2` and never inspects the value. That is the
same shape as the `env` handler bug (which walked past `NAME=VALUE` to reach the command) and the
`-Cincremental` bug (which matched a flag name and ignored its path). Confirmed live, all
auto-approved before this was written:

    restic --password-command /tmp/evil snapshots     runs an arbitrary program        [FIXED]
    helmfile --helm-binary /tmp/evil list             runs an arbitrary binary         [FIXED]
    vite -c /tmp/evil.js build                        loads an arbitrary JS config     [FIXED]
    sandbox-exec -f /tmp/evil.sb ls                   caller-chosen sandbox profile    [FIXED]
    dotenv -f /tmp/evil.env ls                        arbitrary env injection          [FIXED]
    borg --rsh /tmp/evil check repo                   runs it to reach the repo        [FIXED]
    borg --remote-path /tmp/evil list repo            borg executable on the far side  [FIXED]

RE-VERIFIED after v0.221.0 changed `execute_file_verdict` (it now requires every whitespace-separated
token to look like a path, to stop a command LINE being judged as one path). All seven still refuse
`/tmp/evil`, and the in-workspace spellings — `vite -c ./vite.config.js`, `borg --rsh ./bin/myssh`,
`restic --password-command ./bin/pass` — still work. Worth re-running whenever that function is
touched, since these seven are exactly its consumers.

All seven now declare a `[command.path_gate]` with role `exec`, which withholds `/tmp` and home
where `read`/`write` would admit them, while keeping the in-workspace spelling
(`vite -c ./vite.config.js`, `borg --rsh ./bin/myssh`) working. The mechanism needed no extension —
`Role::Exec` on a `[command.path_gate]` was already the right tool.

CORRECTION (2026-07-29): this section previously recorded "`borg --rsh /tmp/evil` denies, so this is
not universal". That was WRONG, and instructively so — the probe behind it omitted borg's required
repository positional, so the refusal came from the missing argument, not from any gate. With the
positional supplied, `borg --rsh /tmp/evil check repo` auto-approved and ran `/tmp/evil`. A deny
observed without checking that the command otherwise WORKS proves nothing; the guards added for this
class assert both directions for that reason.

The same section cited `nix-env -f /tmp/evil.nix` as a second example of "already denies". It does
deny — but so does a bare `nix-env -q`, because nix-env is not auto-approved in any form, so the
refusal says nothing about whether `-f` is gated. Both halves of that sentence were evidence of
nothing. Whether nix-env's `-f` needs a gate is still OPEN and becomes a real question the moment
any nix-env invocation is allowed.

### The RELOCATION sub-class — FIXED 2026-08-04 (all nine)

All nine below now carry a `[command.path_gate]`, verified in BOTH directions: every foreign path
denies and every bare + in-workspace spelling still auto-approves (`composer -d ./packages/api
install`, `jj -R ./sub new`, `hatch --project ./pkg build`, `mc --config-dir ./.mc ls x`,
`helmfile -f ./helmfile.yaml list`, `alembic -c ./alembic.ini current`,
`i18n-tasks -c ./config/i18n-tasks.yml health`).

ROLE CHOICE was the substantive decision, and `write` is NOT always enough. `pathgate::Role::Exec`
denies a `/tmp` executor where `write` ADMITS it, so any flag whose value selects code takes `exec`:

    composer -d/--working-dir     exec   picks whose composer.json scripts run (a `write` gate would
                                        have left `-d /tmp/evil install` running a planted project)
    hatch --project/-p, --config  exec   selects which project is built, i.e. whose build hooks run
    alembic -c/--config           exec   the ini's script_location names the Python alembic imports
    i18n-tasks -c/--config        exec   its YAML is run through ERB, so loading it evaluates Ruby
    helmfile -f/--file/--helmfile exec   a helmfile can declare hooks that run
    jj -R/--repository            write  jj MUTATES that repo's store; it does not run code from it
    hatch --data-dir/--cache-dir  write  relocates hatch's own storage only
    mc --config-dir               write  holds aliases and access keys; mc reads and rewrites it

SCHEMA NOTE, because it cost time: this file previously said these gates use "role `exec`" and that
`registry::types::PathRole` spells it — `registry::types::PathRole` has only `Read`/`Write`. The enum
that `[command.path_gate]` actually deserializes into is `src/pathgate.rs::Role`, which has all three.
`envvars.rs::PathRole` also has `Exec` and its doc comment still claims the registry enum spells it
the same way; those two have diverged and the comment is wrong.

All seven commands also gained `examples_safe`/`examples_denied` pinning both directions. Six of them
had NO examples at all, so they were fuzz-coverage holes as well (AGENTS.md: un-exampled commands are
never reached by the registry-derived seeds).

### Original report — confirmed live 2026-08-03

The seven above are *executor* values: the flag names a program to run. There is a second sub-class
in the same `valued` lists that is still open. The value is not a program but a WORKING DIRECTORY,
project root, repo, or config file — it does not run anything itself, it moves where everything else
lands. Confirmed auto-approving, each with a control run so the result means something:

    composer -d /etc install          writes vendor/ + runs post-install scripts, in /etc
    composer -d ~/.ssh install        same, in a credential store
    jj -R /etc new                    creates a commit in a repo outside the workspace
    hatch --project /etc build        builds against a project outside the workspace
    hatch --data-dir /etc build       relocates hatch's own data directory
    mc --config-dir /etc ls x         relocates mc's config directory
    helmfile -f /etc/h.yaml list      reads a helmfile from outside the workspace
    i18n-tasks -c /etc/i.yml health   reads a config from outside the workspace
    alembic -c /etc/a.ini current     alembic's ini names the migration script location (code)

What makes these a GAP rather than a stance is that the same shape is handled elsewhere, three
different ways — so the fix has templates rather than needing a design:

    git -C /etc add .                 DENIES — fails closed on `-C` with any write sub. Cheapest
                                      correct answer; does not model the directory, just refuses.
    cargo run --manifest-path /etc/…  DENIES — modelled properly via the executor redirect flag.
    vite --config /etc/v.js build     DENIES — the value is gated.

Two cautions for whoever picks this up, both learned the hard way on this pass:

  - RUN THE CONTROL. `env -C /etc cat master.passwd`, `xargs -a /etc/master.passwd echo`,
    `watch … `, `sudo … ` and `script … ` all deny — by OMISSION, because those flags/commands are
    not allowlisted at all. They are evidence of nothing, exactly as the CORRECTION above records
    for `borg --rsh` and `nix-env -f`. Check the safe twin (`env -C /tmp ls`) before concluding.
  - `git add /etc/x` is APPROVED (git's positionals are not locus-gated). Probably harmless — git
    stages into its own index and refuses paths outside the repo — but it is unexamined, and it is
    a different question from `-C`.

**Shape of the work**, mirroring the env-prefix project: go through every `[command.wrapper]` (57 of
them) and classify each entry in `valued` as inert, a path (with a role), or a command. Inert stays
listed; a path gets a locus gate; a command recurses through `command_verdict`; anything unresearched
comes off the list and falls to approval. `restic --password-command` and `borg --rsh` are the
`RUSTC_WRAPPER` shape and should recurse. Needs a mechanism on the wrapper spec, since `valued` is
today just a list of names.

SECOND PASS (2026-07-29): the guard written for the first pass walked only `[command.wrapper]` —
307 flags — while `valued` also appears at top level (8,491), on each sub (5,825) and on a fallback
(45). Seven more executor flags were live outside it, all confirmed auto-approving with a working
baseline invocation:

    rsync --rsh /tmp/evil ./src/ ./dst/          the remote shell rsync executes
    rsync -e /tmp/evil ./src/ ./dst/             same flag, short spelling
    gotestsum --raw-command /tmp/evil            replaces the test command
    gotestsum --post-run-command /tmp/evil       run after the test run
    mypy --python-executable /tmp/evil ./src     mypy runs it to inspect the env
    pip-sync --python-executable /tmp/evil       invoked to perform the install
    kustomize build --helm-command /tmp/evil ./k the helm executable it shells out to
    steep check --steep-command /tmp/evil        the steep executable re-invoked

All gated with role `exec`; `rsync -e ssh` (a bare name on $PATH) still approves, which is the form
that actually matters. The guard now walks all four locations.

SIXTH PASS (2026-07-29) — measured the surface; NO new defects. Recorded so nobody re-runs it.

Scale of the gate surface: 1,604 commands, 14,669 valued-flag slots. 126 commands (7.9%) declare a
path_gate; 343 flag slots (2.34%) are gated. So 14,326 flag values are ungated — but that number is
NOT the exposure, and reading it as such would send the campaign in the wrong direction.

WHY it is not the exposure. An unresearched command is capped at SafeWrite: local, no execution, no
remote. A tool that merely READS an arbitrary path stays inside that cap, whatever the path — which
is why `detekt ~/.ssh/id_rsa` and `journalctl --file ~/.ssh/id_rsa` approve and are not bugs. Note
this is NOT a flag-vs-positional gap: the positional spelling approves too. Those commands simply
have no path model, and do not need one.

The bugs found in passes 1-5 were all ESCAPES from that cap — a flag value that gets EXECUTED. That
is the predicate worth sweeping, not "ungated path".

Swept clean this pass, both negative results worth keeping:
  - Reader flags. 134 flags named --input/--file/--cert/--identity/... that take an ungated path.
    Triaged: almost all are format names (`numfmt --from`), booleans (`terraform --input`), device
    specs (`findmnt --source`) or intended use (`age --identity` reading an SSH key IS its purpose).
    Reading a file into a parser is not disclosure; only content flowing OUT is.
  - Content-to-model disclosure. jq -f, xargs -a, base64, xxd, column, expand, fold, nl, rev, tac,
    paste, pr against ~/.ssh/id_rsa — every one denies, as does the `cat` control.
    **Superseded in part — see "Parsers that echo on error" below. That probe population was all
    DUMPERS, every one of which is modelled and gated. It never asked what a PARSER prints when it
    fails, and that is a different population with a different answer.**
  - Exfil (local secret to a remote). curl -T / -d @ / -F / --upload-file, scp, rsync remote,
    aws s3 cp, gh release upload, http POST @, wget --post-file — all eleven deny.

### Parsers that echo on error — a SECOND disclosure predicate (found 2026-08-24)

The claim above that the only predicate worth sweeping is "reaches an EXECUTION sink" is too narrow.
There is a second one, and `tsc` was a live instance of it in shipped 0.228.0:

```
tsc --project ~/.aws/credentials --noEmit --pretty
```

`--noEmit` stops tsc WRITING; it does not stop it READING, and a parser reports errors by quoting
the line it choked on. In pretty mode (the default when stdout is a terminal) that quoting prints
the file's content. Measured with a canary AWS key: it came back seven times. Now gated —
`commands/tools/tsc.toml` carries a `path_gate` on `--project`/`-p` and its positionals.

So the predicate is: **an ungated path argument on a tool whose DIAGNOSTICS quote source text.**
It is not covered by the dumper probe above, because the population is different — linters,
compilers and formatters, whose job is reading files someone else wrote and which therefore mostly
have no path model at all.

Unverified candidates, all of which currently auto-approve against `~/.ssh/id_rsa` and all of which
are believed to print the offending line: `shellcheck`, `ruff check`, `eslint`, `mypy`, `yamllint`,
`swiftlint lint`, `stylelint`. NONE of these was confirmed — none is installed on the machine where
this was found, so the echo behaviour is an assumption in every case and the list is a place to
start, not a finding. `jq`, `rustc`, `gcc -fsyntax-only` and `prettier` already deny.

The sweep wants the same shape as the output-flag one: enumerate commands whose positionals or path
flags are ungated, keep those documented to render source context in diagnostics (`--pretty`,
`--show-source`, caret output), gate them `read`, and put the verified non-echoers on a worklist
fixture so the guard stays green for a stated reason rather than by omission.

#### And a THIRD predicate underneath it: a prefix the TOOL strips

`tsc @FILE` is a response file — tsc opens FILE and splices its contents in as arguments, naming
each token it cannot resolve. The gate judged the literal token `@/path`, but the path the tool
opens is `/path`, so the two disagreed and every shield anchored to a LOCATION was slipped. Shields
matched by NAME segment (`.ssh`, `.npmrc`) bit straight through the prefix — which is exactly what
made this hard to see, because the paths anyone would reach for first still denied. Measured at
**40** region paths once a guard enumerated them, against 7 found by hand.

Fixed for tsc (`tsc_response_file` in `src/pathgate.rs`), and the guard
`a_response_file_argument_is_gated_as_the_path_it_names` now holds `@X` and `X` to the same answer
for every path in `regions/default.toml`.

The general problem is bigger than `@`, and is NOT swept: any argument whose literal spelling is not
the path the tool opens. The obvious siblings were checked and are NOT exposed — `clang`, `gcc` and
`ld` share the `@file` convention but already refuse the form for other reasons, and `javac` is not
in the corpus. So tsc was the live one, not the first of many.

What remains open is the shape, not a backlog: any tool taking a `prefix:path` or `sigil+path` form
the resolver does not decompose. The invariant to sweep for is the one the new guard states: **the
string the gate judges must be the string the tool opens.** Where a command's grammar rewrites its
argument, that rewriting has to happen before the gate, not after.

So the remaining campaign is bounded: find flag values that reach an EXECUTION sink. The two tags
(`twin_flag`/`twin_base`, `CONFIG_IS_CODE`) cover the members already known. What neither does is
DISCOVER new members — that is still the open mechanism, and the highest-value one left.

FIFTH PASS (2026-07-29) — a TAG for the config-is-code class, plus one more finding.

`marp --config-file` / `-c` (marp.config.js is JavaScript Node executes) and `--engine` (a JS module
marp loads) were ungated; both now carry `exec`. Found not by name but by a registry-internal
differential: flags declared `standalone` in one command while `valued` in many others.

NEW TAG — `CONFIG_IS_CODE` in `a_config_flag_on_a_code_config_tool`. The fact that a TOOL executes
its config is declared once, per tool, and the guard derives the obligation for every
config-selecting flag on it (`-c`, `--config`, `--config-file`, `--noxfile`, `--conf-dir`,
`--engine`, `--format`, `--formatter`). Adding a config flag to a listed tool now FAILS until it is
gated, instead of waiting for someone to remember that tool. Currently covers webpack, vite, eslint,
stylelint, marp, nox, sphinx-build, mkdocs. Extend the list as tools are researched — jest, vitest
and cmake are gated but not yet listed; rollup, babel, gulp, grunt, playwright, cypress, storybook,
tailwind, commitlint and prettier are config-is-code but not currently auto-approved at all, so they
carry no exposure until one of them is allowed.

OPEN QUESTION (not a defect count) — 234 scopes declare the SAME flag in both `standalone` and
`valued`. SAMPLE.toml defines `standalone` as "flags that take no value", so the two declarations
contradict each other and only one can be honoured; marp was one of them. But many entries look like
deliberate modelling of an OPTIONAL value (`zstd --long` vs `--long=27`, `7z -r` vs `-r-`), which the
schema has no way to express. Before treating any of these as bugs, decide what the schema means
here: either support optional-value flags explicitly, or make the overlap a build error. A guard
written against the current ambiguity would encode a convention nobody has chosen.

FOURTH PASS (2026-07-29) — the gates were bypassable by RESPELLING. Every executor flag gated in
the passes above had an environment twin that was not gated, so the flag gate read as closed while
the same operation sailed through:

    BORG_RSH=/tmp/evil borg check repo                    (--rsh was gated)
    BORG_REMOTE_PATH=/tmp/evil borg list repo             (--remote-path was gated)
    RESTIC_PASSWORD_COMMAND=/tmp/evil restic snapshots     (--password-command was gated)
    RSYNC_RSH=/tmp/evil rsync ./src/ ./dst/               (--rsh/-e was gated)
    KUBECONFIG=/tmp/evil.yaml kubectl get pods            (a kubeconfig can carry users[].user.exec)

All now classified in envvars.toml with `shape = "exec-path"` — the LOCUS-based shape, not
`command`, so the two spellings agree: a bare `ssh` stays trusted, `/tmp/evil` does not. The classic
env-exec vectors (NODE_OPTIONS, PYTHONSTARTUP, PERL5OPT, BASH_ENV, LESSOPEN, PAGER, GIT_SSH_COMMAND)
were already covered; the gap was only the twins of tool-specific flags.

NEW MECHANISM — `twin_flag` / `twin_base` on an envvars entry, naming the flag spelling of the same
thing. `a_tagged_env_var_classifies_the_same_as_its_flag_twin` holds both spellings to the same
verdict across a foreign path, a workspace path and a bare name, and refuses a pair that never
discriminates. It fails in BOTH directions, which is how it caught kubectl over-denying relative to
its twin. Tag every new executor flag that has an env form.

REMAINING in this class: the tag only checks pairs someone DECLARED. Nothing discovers an undeclared
pair — the four above were found by hand, by asking "what is the env spelling of this flag?" for
each flag gated. A generator that proposes candidate env names per gated flag (TOOL_FLAG, FLAG) and
reports unlisted ones would turn that into a sweep. Also unmodelled: `kubectl --kubeconfig` is not a
known flag at all, so it denies as unknown rather than by gate — an over-deny to fix when kubectl's
flag surface is next researched.

THIRD PASS (2026-07-29) — the part names cannot find, partly closed. The predicate that works for
this half is the TOOL, not the flag: a build/task runner whose config file is CODE. Eight more were
live, all confirmed with a working baseline:

    webpack -c /tmp/evil.js            webpack.config.js is JavaScript webpack evaluates
    webpack --config /tmp/evil.js
    eslint -c /tmp/evil.js ./src       eslint.config.js is JavaScript
    eslint --config /tmp/evil.js ./src
    stylelint --config /tmp/evil.js    stylelint.config.js is JavaScript
    nox -f /tmp/evil.py                a noxfile is Python nox imports and runs
    sphinx-build -c /tmp/evil ./d ./o  the directory holding conf.py, executed as Python
    mkdocs build -f /tmp/evil.yml      mkdocs.yml can declare `hooks:` Python modules

All gated `exec`; the in-workspace and bare-name spellings still approve (`eslint -f json`,
`stylelint -f string`). jest, vitest and cmake were already gated. make/just/ninja/rake/rollup/
prettier/cypress/storybook/invoke/fab are not auto-approved at all, so they carry no exposure today
— but each becomes a live question the moment any invocation of it is allowed.

DELIBERATELY NOT GATED, having checked: `swc --config-file` (.swcrc is JSON swc never executes),
`esbuild --tsconfig` (JSON, not executed), `mysqldump --init-command` (SQL run by the server).

A SEPARATE BUG CLASS surfaced here and is NOT swept: webpack's `-c` was listed in `standalone`
though it takes a value, so `webpack -c /tmp/evil.js` parsed the path as a POSITIONAL and escaped
flag gating entirely. A valued flag mismodelled as a boolean defeats every flag-level gate we have.
Nothing enumerates that mismatch today; it needs a pass of its own.

STILL OPEN — the part names cannot find. Every flag above advertised itself (`-rsh`, `-command`,
`-executable`). A flag whose name hides what it does is invisible to the ratchet: `vite --config`
evaluates JavaScript and `sandbox-exec -f` picks the sandbox profile, and both were found only
because they were already written down here. Candidates seen but NOT researched: `alembic --config`
(the ini selects an `env.py` that runs), `i18n-tasks --config` (ERB-evaluated YAML), `helm/flux
--kubeconfig` (a kubeconfig can carry a `users[].user.exec` credential plugin), `workon --config`.
Each needs the per-flag research this section describes — the ~14,600 valued flags cannot be swept
by name alone.

Found 2026-07-27 while reviewing the env-assignment work.

## DECISION NEEDED: `permissions.allow` out-ranks every level, including `paranoid`

Surfaced while unifying the CLI with the hook (2026-08-03). The coverage fallback marks a segment
covered by the user's own `permissions.allow` as `Allowed(Inert)`, and `Inert` clears every
threshold. So a `Bash(rm:*)` rule in `~/.claude/settings.json` auto-approves `rm -rf /` even at
`--level paranoid`.

This is long-standing hook behaviour, not new; what changed is that the CLI now reports it instead
of computing a stricter answer no harness would give. Two defensible readings and no obvious winner:

  - The rule is the user's own explicit statement, so it should win — the same principle as "a grant
    covers what it names", and safe-chains does not second-guess a deliberate act.
  - `level` is the ceiling, and a ceiling that a per-command rule can lift is not a ceiling. Someone
    setting `paranoid` is asking for a read-only plan and probably does not mean "except for the 300
    Bash() rules I accumulated for a different purpose."

Worth noting the two are separable: the coverage bridge could keep granting while the LEVEL clamps
the result (`min(covered, threshold)` rather than `Inert`), which honours the rule without letting
it exceed the stated ceiling. Not implemented; recording the option so the choice is informed.

## Support OPTIONAL-VALUE flags by design — BUILT 2026-08-29 (`optional_valued`)

DONE. Declaring a flag in `optional_valued` admits `--gitignore` and `--gitignore=false` and
nothing else; a guard fails the build if the same flag is repeated in `standalone` or `valued`.
Migrated: the seven `cargo mutants` flags that were omitted waiting on this, and ghostty's
`+list-fonts --bold`/`--italic`. Documented in SAMPLE.toml.

**One premise below was wrong, and it is the interesting part of this entry.** The original text
said a flag in both lists is "a contradiction… where only one can be honoured". It is not. Measured
(`policy::tests::a_flag_declared_in_both_lists_already_takes_an_optional_value`): `check_flags`
consults `standalone` first, so the BARE spelling matches there and cannot swallow the next token,
and its `=` branch consults `valued`, so the GLUED spelling matches too. Both are honoured, for
different spellings — which is already optional-value semantics, and already matches getopt, where
an optional argument must be glued.

So the behaviour existed and the schema simply had no word for it. That changed what got built:
`optional_valued` compiles down to membership in both lists and the walk was not touched at all.
The field buys INTENT, and intent was the whole blocker — the same pair of memberships meant either
"deliberate" or "mistake", nothing could tell them apart, and that ambiguity is what the overlap
audit was stuck behind.

Worth generalising from: the fix for an inexpressible grammar is not always new machinery. Check
what the walk already does before designing a third state for it — here the measurement removed the
engine change entirely, and with it the risk of touching a path every command goes through.

### Original entry, kept for the reasoning

DECIDED: add optional-value flags to the schema as a first-class thing, rather than making the
overlap a build error and forcing 234 scopes to drop a spelling. A flag that genuinely accepts both
`--long` and `--long=27` is a real grammar, and every tool that has one is currently modelled by a
contradiction (declared in `standalone` AND `valued`, where only one can be honoured) or by dropping
a form — and a dropped form is a FALSE DENY, which costs a user a prompt for a command that is fine.

Shape to design (not yet chosen):
  - a per-flag `value = "optional"`, or a third list alongside `standalone`/`valued`;
  - the parser must accept the glued/`=` form as carrying a value and the bare form as not, WITHOUT
    letting the bare form swallow the next token as its value — that swallow is exactly how a valued
    flag mismodelled as a boolean turns a path into a positional (see the section below);
  - once it exists, the `standalone`+`valued` overlap becomes a build error, and the 234 scopes are
    migrated to whichever of the three they actually are.

Known callers waiting on it, so the migration has a first batch: `zstd --long`, `7z -r`, and the
seven `cargo mutants` flags omitted for this reason (`--cap-lints`, `--jobserver`, `--copy-target`,
`--copy-vcs`, `--gitignore`, `--skip-calls-defaults`, `--test-workspace`).

Sequencing note: do this BEFORE the overlap audit below, not after. The audit's whole difficulty is
that it cannot tell a typo from an optional-value flag; with the third state expressible, that
distinction becomes mechanical.

## Command MODES — design written, not built (docs/design/command-modes.md)

The schema describes ONE behaviour per command, and behaviour often depends on which FLAGS are
present. Four mechanisms already select behaviour by flag, each expressing a sliver:
`[[command.sub]]` (positional axis, full payload), `[[command.flag]] classifies` (archetype only),
`output.invalidated_by` ("claim off"), and a proposed `output.requires` ("claim on") — the last two
being the same idea pointing opposite ways, which is what prompted a design instead of a fifth field.

FOUR MEASURED CUSTOMERS, each hit while doing other work:

    git diff --name-only   emits paths; bare `git diff` emits a PATCH. An unconditional output claim
                           would assert patch text denotes paths at the cwd locus — a FAIL-OPEN,
                           since an absolute path in a diff body would then be read for real.
    php -l file.php        lint mode: reads and parses, executes nothing (SafeRead) — but takes an
                           operand, and the shared fallback is max_positional = 0.
    ruby -S CMD ARGS       delegation mode, structurally `mise exec --`: recurse into CMD with an
                           exec-locus gate on the value (bare name via PATH trusted, /tmp/evil not).
    base64 -i/-o           GNU `-i` is a BOOLEAN; BSD `-i INPUT` is VALUED and `-o` WRITES. Two
                           grammars in one entry — the standalone/valued overlap at its sharpest.

DO NOT add `output.requires` meanwhile: it needs an unwritten precedence rule against
`invalidated_by`, and `git diff` would need BOTH anyway, so the very first customer exercises the
confusing interaction.

FREE MEANWHILE, needing no new mechanism: `git ls-files` and `jj file list` ALWAYS emit paths and can
take an ordinary `[command.output]` claim today, closing part of the command-substitution class.

**`git ls-files` DONE 2026-08-30.** `cat $(git ls-files)` and `grep -n foo $(git ls-files)` were
denied and now allow. `invalidated_by` is every flag that stops a line being a bare relative path,
confirmed against git-scm.com: `-t`/`-v`/`-f`, `-s`/`--stage`, `-u`/`--unmerged`, `--debug`,
`--eol`, `--resolve-undo`, `--format`, `--abbrev`, `-z`, and `--full-name`. The claim is safe under
`git -C DIR ls-files` even though the wrapper flag is stripped before the rule sees the arguments,
because every path git prints is RELATIVE and the CONSUMER resolves it against its own cwd —
redirecting git's directory changes which names appear, not where they land. `git ls-files ../` is
the form that can escape, and it escapes through an operand, which is what `locus_from = "operands"`
bounds. `every_output_claim_is_bounded_by_its_roots` picked the new claim up automatically.

**`jj file list` NOT done, and it is not the same "free".** It declares
`tolerate_unknown_short = true` and `tolerate_unknown_long = true`, so its flag surface is
deliberately unbounded — any flag nobody has enumerated is accepted as a positional. An output claim
asserts a property of the OUTPUT that any unlisted flag could break (`-T`/`--template` alone would),
and `invalidated_by` can only name spellings someone has thought of. So the prerequisite is
enumerating that entry's flags, which is a fact about jj rather than about the mechanism. Nested-sub
claims themselves work: `sub_output_locus` descends `[[command.sub.sub]]`, so there is no
mechanism blocker here, only an unenumerated grammar.

The overlap audit below and the glob-family migration are the same problem wearing different clothes
— "this entry is really several commands" — and should migrate INTO modes rather than run as separate
campaigns.

### The design contradicts itself on scope, and that has to be settled before building

Read 2026-08-30 with the intent to implement. The document decides two things that cannot both hold:

- §"What can a predicate say?" settles v1 as **flag PRESENCE only** — "Values (`--format=%f`) are not
  expressible and stay that way in v1 — under-reaching fails closed."
- §"Six customers" names **`dart format` the acceptance test** — "the smallest invocation in the tree
  that defeats every declarative mechanism currently proposed, so it is the right acceptance test for
  any mode v1 — if the design cannot express `dart format`, it has not cleared the bar the existing
  handlers already clear."

`dart format` selects its mode by a flag's VALUE (`-o write|show|json|none`) and the selected mode
re-roles the POSITIONALS. A presence-only v1 cannot express either half, so building v1 as specified
fails its own stated acceptance test on day one. Customers 4 (`fourmolu --mode inplace`) and 5
(`gomodifytags -w -file`) are the same shape: 4 needs the PREDICATE to read a value, 5 needs the
PAYLOAD to re-role a flag's value, and the document itself notes "a v1 that covers one is not close
to covering both".

This is a decision, not a defect — but it is the user's, because the two answers are very different
amounts of work:

  **(a) Lower the bar.** Ship presence-only. It serves customers 1, 2, 3 and 6 (`git diff`, `php -l`,
  `ruby -S`, `base64`), which is four of six and includes the whole `output.requires` question. Drop
  `dart format` as the acceptance test and say plainly that value-selected modes stay handlers.

  **(b) Raise the mechanism.** Value predicates plus flag-role payloads in v1. Covers all six and the
  in-place formatter family, at materially more design and a bigger blast radius, since the payload
  half touches path-role resolution rather than just flag lists.

Recommendation: **(a)**, and amend the design to match. The four customers it serves are the ones
that recur, `write_when` already demonstrates that shipping the narrow version buys real coverage,
and the document's own argument against narrowness — that `write_when` "closed eight commands cheaply
and then could not close the two beside them" — is an argument for choosing the boundary
deliberately, not for making v1 large. Whichever is chosen, the acceptance test in the document must
be changed to match the scope, or the first implementation will be measured against a bar it was
never designed to clear.

### DECIDED: (b). Stage 1 built 2026-08-31 — the value predicate and the path-role payload

`[[roles.X.when]]` in `pathgates.toml`: a clause names flag spellings and, optionally, the VALUES
they must carry, and declares the positional role while it holds. It REPLACES the declared role
rather than promoting it, which is what `write_when` could not do and what `dart format` needs — a
bare `dart format .` rewrites in place, so the clause has to make the invocation LESS restrictive.

**The acceptance test passes.** `dart format` is now declared, not coded: `dart_mode` is deleted,
and the entry expresses both halves the design said defeated every declarative mechanism — a
predicate over a flag's VALUE, and a payload that re-roles the positionals.

One deliberate behaviour change while converting: the old handler treated ANY non-`write` value as a
read, so `dart format -o bogus ~/x` was a read. The clause matches only values it names, so an
unrecognised one keeps the default (`write`) and denies. A value the entry has never seen can no
longer argue its way into a weaker role.

REMAINING for (b), in the order they should be taken:

  - ~~**Flag-role payloads.**~~ DONE 2026-08-31. A clause now carries `flags = { … }` beside
    `positional`, re-roling another flag's VALUE while it holds. `gomodifytags` is converted:
    `-file` is declared `read` and escalated to `write` by a clause on `-w`/`--w`, so the read-only
    run against a file outside the workspace stops denying.

    Note the direction. Here the clause ESCALATES (read by default), where `dart format`'s steps
    DOWN — so a clause that stopped matching would fall back to `read` and admit a rewrite. That is
    only sound because a gomodifytags run without `-w` genuinely does not write, and the guard pins
    it: red-demoed by deleting the clause, which makes the `-w` case fail OPEN and the test fail.

    The overlay resolves among clauses and then REPLACES the spec's entry rather than maxing
    against it — maxing would make a clause unable to lower a role, which is the direction
    `dart format` needs.
  - ~~**The formatter family.**~~ DONE 2026-08-31 for six: `gofmt`, `gofumpt`, `goimports`,
    `clang-format`, `fourmolu`, `ormolu` moved from a blanket `positional = "write"` to
    `positional = "read"` plus a write clause. `gofmt .git/config` allowed a read again while every
    write spelling still denies, closing the split against `ansible-lint`.

    Each write flag was enumerated from the tool's own documentation, because this direction is a
    LOOSENING and a missed spelling reads a real rewrite as a read. That caution paid: fourmolu's
    published docs give `--mode inplace`, but its option parser (`app/Main.hs`) also declares
    `short 'm'`, so `-m inplace` is real — declaring only the long form would have been exactly the
    hole the design predicted converting them would cause.

    NOT converted, each for a stated reason rather than by omission: `yapf`, `autoflake`,
    `autopep8` keep the blanket gate until their write-flag surface is enumerated the same way, and
    `cmake-format` additionally has `-o`/`--outfile-path`, which writes somewhere the positional
    gate does not look — so it needs the flag-role payload below, not just a clause.
  - **Modes proper** — the LEVEL and FLAG-LIST payload. **Re-measured 2026-09-01 before building,
    and the customer list had gone stale: four of the six are served.** `git diff --name-only` by
    `output.requires` (built after the design was written, and the design argues against adding it);
    `php -l` by the `interpreter` handler; `fourmolu` and `gomodifytags` by the two clause stages
    above. `base64` is served by deliberate degradation — the entry models GNU and says so, and
    BSD's `-o`, the only spelling that writes, is not allowlisted at all.

    That leaves `ruby -S CMD`, which denies. It is an OVER-DENY, not a hole, and what it needs is a
    DELEGATION payload — open question 3 in the design — rather than the level-and-flag-lists the
    document mostly describes.

    So the remaining case is one over-deny on one command, against a mechanism whose no-inheritance
    rule makes every mode repeat its grammar. Not obviously worth it; deliberately NOT built on that
    basis rather than left unfinished by accident. The design doc now carries the measured table, so
    the next person re-derives the list instead of trusting it.

## The `standalone` + `valued` overlap audit (blocked on the above)

Promoted 2026-08-04 out of the "FIFTH PASS" narrative above, where it was easy to miss.

**234 scopes declare the same flag in BOTH lists.** SAMPLE.toml defines `standalone` as "flags that
take no value", so the two declarations contradict each other and only one can be honoured. marp was
one of them, and that mattered: a valued flag mismodelled as a boolean defeats flag-level gating (see
the next section).

But most of the 234 are not typos. They look like deliberate attempts to model an OPTIONAL value —
`zstd --long` vs `--long=27`, `7z -r` vs `-r-` — which the schema has no way to express.

UNBLOCKED 2026-08-29: `optional_valued` exists, so the third state is expressible. What remains is
the MIGRATION — sort each scope into standalone / valued / optional_valued, and turn the remaining
raw overlap into a build error.

MEASURED while building the differential, because "234 scopes" understates the job: it is **233
scopes but 1527 (scope, flag) pairs across 869 distinct flags**. The count that matters for
planning is the pairs. It is also dominated by SHORT flags — `-r` overlaps in 34 scopes, `-c` 26,
`-d` 23, `-p` 23, `-h` 22 — and those are the ones no cross-tool comparison can adjudicate, so this
migration is per-tool work almost all the way down. The long-flag overlaps are comparatively few
and are where `optional_valued` will do most of its work.

Note while doing it: an overlap is NOT currently broken. It behaves as an optional-value flag
already (see the corrected section above), so this migration is about making intent legible and
letting the build reject the real typos — not about fixing live wrong answers. The exception is the
opposite error, a valued flag declared ONLY in `standalone`, which IS live and is the next section.

Still do NOT write a guard against the raw ambiguity until the sort is done: before then it cannot
distinguish a typo from an optional-value flag, which is exactly what the sort decides.

## A valued flag mismodelled as `standalone` defeats every flag gate — NOT swept

Promoted 2026-08-04 out of "THIRD PASS", where it was a closing paragraph. This is a live bug CLASS
with no enumeration, and it silently disables the gating mechanism the passes above spent themselves
building.

The shape: webpack's `-c` was listed in `standalone` though it TAKES a value, so
`webpack -c /tmp/evil.js` parsed the path as a POSITIONAL — it never reached the flag policy at all,
and the `exec` gate on `-c` could not fire because `-c` was never treated as a valued flag. Any
flag-level gate can be defeated the same way, which makes this a meta-bug: it does not add one hole,
it invalidates a defence everywhere it occurs.

Why it is not just the section above: an overlap (`standalone` AND `valued`) is at least VISIBLE. A
flag declared ONLY in `standalone` that actually takes a value looks completely normal, and nothing
compares a declaration against the tool's real grammar.

ENUMERATED 2026-08-29 for LONG flags, via the registry-internal differential below. The guard is
`a_long_flag_is_not_standalone_where_it_is_valued_elsewhere`, ratcheting
`tests/fixtures/flag_arity_worklist.tsv` (118 rows, 61 distinct flags). It fails on a candidate
that is not listed AND on a listed row that is no longer a candidate, so a new mismodelling cannot
land quietly and the file cannot rot.

The differential works: it independently re-flagged webpack (`--target`, `--mode`) — the command
whose `-c` produced this section — and seeding it turned up pre-commit's `-c`/`--config` on
install / uninstall / install-hooks / autoupdate (a config PATH, on subs that WRITE git hooks) and
`-j`/`--jobs` on autoupdate (a thread count). Both confirmed against pre-commit.com and fixed.

**The short-flag half is NOT enumerable this way, and that is a finding rather than a gap in the
implementation.** `-o` is an output path in one tool and a boolean in the next, so the cross-tool
comparison carries no information — and short flags are most of the population (`-r` overlaps in 34
scopes, `-c` in 26, `-d` and `-p` in 23). Whatever catches those has to come from the tools, not
from comparing them to each other. The remaining angle is the second one below.

DONE when: the short-flag half also has an enumeration. Two angles, one now built —
  - ~~**Registry-internal differential**~~ — BUILT, long flags only, for the reason above.
  - **Behavioural probe**: for each `standalone` flag on an auto-approving command, classify
    `<cmd> <flag> /etc/x` and see whether the path landed as a positional. Directly tests the
    property that matters rather than a proxy, and is the only one of the two that can speak to
    short flags. Note the probe needs care about what it concludes: a path landing as a positional
    is only a DEFECT where something would have gated it, which is why the gated subset already has
    its own guard (`a_gated_flag_is_never_declared_value_less`) and is not this population.

## The registry validators are largely UNTESTED — found by adversarial review

Mutation-testing during review of the refactor above: disabling `filter_candidates`'s check outright
left ALL 4504 TESTS PASSING. Its own comment records that the case was red-demoed when it was
written, but the demo was never left behind as a test.

This is NOT caused by the Result refactor — the check was equally untested as an `assert!`. What the
refactor changes is the stakes. Every test runs against VALID data (every built-in `commands/*.toml`
is well-formed), so no validator fires on the happy path, and a validator that stopped working would
be invisible to a green suite. 63 checks were just rewritten under exactly that blind spot.

Covered so far: 20 `should_panic` tests, plus `a_candidate_sub_may_not_declare_nested_subs` added
during this review (red-demoed). That leaves most of the 63 with no test asserting they reject
anything.

Those 20 are now FIXED (2026-08-07). They had become `should_panic` tests matching a Debug-ESCAPED
string, because `load_one` reached the validator through `.expect()`, whose panic message embeds the
`Err` via `{:?}` — so any `expected =` fragment containing a quote or newline would have silently
stopped matching, and the names claimed a panic that no longer happens.

All 20 now call `assert_rejected(toml, expected)`, which reads the `Err` directly. Three gains, in
order of importance:

  - it distinguishes ACCEPTED-when-it-should-not-be from REJECTED-FOR-THE-WRONG-REASON, which
    `should_panic` could not — red-demoed, and the failure now reads "rejected, but not for the
    stated reason — wanted X, got: <actual>" instead of a bare "did not panic as expected";
  - no Debug escaping, so the fragments mean what they say;
  - the names say `_is_rejected` rather than `_panics_at_build`.

### Pilot run DONE 2026-08-07 — `cargo mutants` over `registry/build.rs`

    133 mutants in 18m: 107 caught, 7 missed, 19 unviable, 0 timeouts
    mutation score 107/114 = 93.9%   (unviable excluded from the denominator)
    baseline: 18s build + 28s TEST  -> test-bound, so restricting the suite is the lever here
    per mutant: ~5s build + ~40s test    cargo-mutants 27.1.0, -j2, --no-shuffle

CALIBRATION, and the reason to trust the rest: `filter_candidates` is NOT in the missed list. It was
provably untested when this section was written, and is now caught by the test added during the
review. The tool independently reproduced the hand-mutation result and confirmed the fix.

THE 7 MISSES. Two are whole validators that can be replaced with `Ok(())` undetected — exactly the
class this section predicted:

    356   assert_loopback_localizes_is_coherent -> Ok(())      entire validator disabled
    838   assert_flat_or_structured -> Ok(())                  entire validator disabled
    200   first_arg_matches: == becomes !=                     see below
    897   assert_matrix_no_duplicate_parent_action: < becomes >
    1017  lower_behavior: delete arm (Transfer, None)          "transfer needs its block"
    1020  lower_behavior: match guard -> false                 "transfer block only with transfer"
    1092  build_command_archetype_flags: delete !              when_absent/value_prefix exclusivity

`first_arg_matches` is the one to fix first. It is build-time only (its sole caller is
`assert_no_candidate_shadowed_by_glob`), so inverting it is not a runtime bypass — but that guard is
the fail-closed detector for the candidate-under-glob footgun, whose own comment records that "the
AWS blob-readers hit exactly this: a `candidate` under `get-*` would have auto-approved". Inverting
`==` disables the detector for EXACT-token patterns, which suggests the existing coverage exercises
only the `get-*` glob arm.

ALL 7 KILLED, re-run verified: `133 mutants in 12m: 1 missed, 113 caught, 19 unviable` after the
first six tests, then the last one hand-verified red→green. `registry/build.rs` is at 114/114.

Two lessons from killing them, both portable:

  - **The obvious test for a boundary mutant can pass against the mutant.** `matrix.len() < 2`
    became `> 2`, and at exactly TWO matrices those are indistinguishable — both fall through to
    the duplicate check. Only a THIRD separates them. A test written to satisfy the report rather
    than to kill the specific mutant would have looked like a fix.
  - **Read the mutant's column, not just its line.** The last survivor was reported at `1092:67`
    and I tested the mutual-exclusion check at 1106, because both are `!` in the same function. The
    real target was the `!` inside the `judged` CLOSURE, whose inversion refuses a legitimate
    `judgment` and accepts a blank one. Nothing caught it because every other flag test omits
    `judgment` entirely, so `is_none_or` short-circuits on `None` and the predicate never runs —
    an optional field is unexercised in BOTH directions unless a test supplies it.

Also confirmed the skill's warning the hard way: two hand-applied mutations silently did not apply
(wrong line, then a pattern that occurs TWICE in the file), and each time the test "passed" against
unmutated code. Only printing an applied-count caught it. Never read a SURVIVED without proving the
edit landed.

NEXT: the same pilot over `engine/` and `pathgate.rs`, which carry the runtime decisions —
`registry/build.rs` only validates authored data, so its mutants are about our own data hygiene, not
about what gets auto-approved. Do NOT extrapolate the 12-18m/133-mutant rate: the skill's measured
lesson is that it transfers between neither scopes nor cold/warm build dirs. `--list` first; free.

## Original note: verify the `config_load` nightly actually goes green

The nightly failed four nights running on `config_load` — NOT a crash. The job carries
`timeout-minutes: 75` around 3600s of fuzzing, ran 80 minutes, and was killed; its siblings finish
in ~61. The one difference is that every malformed config printed two lines to stderr, the target
feeds mostly-invalid TOML, and it calls the loader twice per input. That is now silent under
`cfg(fuzzing)`.

The honest caveat: measured locally, silencing took it from 9,488 to 14,167 exec/s — 1.5x, which
does NOT by itself account for a twenty-minute overrun. Runner log ingestion is far slower than the
local pipe the measurement went through, so the gap is consistent with the flooding without being
proven by it. If the nightly still overruns after this ships, the cause is elsewhere and the next
step is to instrument the job rather than assume.

## Follow-up: remaining JVM code-supplying flags (deliberately denied)

`-cp`/`-classpath`/`--class-path` are DONE — gated at the executor locus on `JDK_JAVA_OPTIONS`, so
they agree with the `CLASSPATH` entry. Measured: they are launcher options, accepted only there;
`JAVA_TOOL_OPTIONS='-cp …'` is `Unrecognized option` and the JVM refuses to start.

Still denied, each on purpose rather than by omission:

- `-Xbootclasspath/a:<path>` — no environment twin admits it, so there is no inconsistency driving
  it, and the bootstrap loader is a higher-privilege position than the app classpath. Adding it
  would be widening the allowlist speculatively. It is also colon-joined, and `Sep` carries only
  `equals` and `space` today, so adding the flag means adding the variant (and a test) with it.
- `--module-path=<path>` / `--upgrade-module-path=<path>` — same: no env twin, so nothing to
  reconcile. `sep = "equals"` already exists if a real need turns up.
- `-XX:VMOptionsFile=<path>` — stays denied WHATEVER the path. It injects arbitrary VM options, so a
  worktree file could carry `-javaagent`; that is config injection, the way `GIT_CONFIG_*` is, not a
  path to gate.
- `-javaagent:` / `-agentpath:` — instrument every class before main. Arguably worktree-own code by
  the same argument as `-cp`, but the capability is broader and nothing forces the question yet.

## Eleven facet axes have no authored level constraint

Surfaced 2026-07-25 by `a_declared_hazard_is_the_term_authored_levels_reject`, which verifies each
axis's declared `hazard` against the levels that actually reject it. It can only check an axis some
authored clause constrains, and it reports the ones it cannot:

    isolation, persistence.trigger.escape, supply_chain.pinning, locus.binding,
    persistence.trigger.kind, disclosure.channel, disclosure.principal,
    secret.channel, secret.principal, supply_chain.source, supply_chain.exec_surface

Two consequences worth separating.

**The hazard declarations for these axes rest on their doc comments alone.** That includes both trust
ladders — `isolation` and `supply_chain.pinning` — whose direction is inverted (higher is safer, so
the hazard is the FLOOR). `Pinning`'s doc says a level "floors it (`>= version`)"; no level does,
and per the next paragraph none is expected to. Mis-declaring `Pinning::hazard = digest` (the SAFEST
term on that ladder) would go unnoticed by that test — verified by red demo; only
`the_sentinel_is_denied_even_with_any_one_axis_relaxed` would still hold, and only because the other
axes carry the denial.

**ADDRESSED 2026-09-01, for the ordinal half.** Re-confirmed the exposure first, and it was worse
than "would go unnoticed by that test": with `Pinning`'s hazard set to `Digest`, the whole suite
passed — 4601 tests, zero failures — while `Capability::worst()` claimed the most-pinned supply
chain was the worst case. Nothing anywhere was looking.

Two changes. `ordinal_term!` no longer accepts a hand-written `hazard =`; a trust ladder is marked
`inverted;` and the hazard is DERIVED (bottom when inverted, top otherwise). Direction has an
objectively right answer and the hazard is a consequence of it, so declaring the consequence was
what let the two disagree — the same shape the trait doc already blamed for the `TriggerKind::None`
drift. And `the_trust_ladders_take_their_hazard_from_the_bottom` pins both axes by name, which is
what catches the marker going MISSING; red-demoed, since that was the failure mode with no witness.

Categoricals still declare their hazard and must — there is no order to derive from. That half stays
resting on its doc comment for any axis no level constrains, which is the paragraph below.

**The whole supply-chain group is unconstrained BY DESIGN, and will likely stay that way.** The
developer install clause (`levels/default.toml`, the `npm ci --ignore-scripts` shape) has landed, and
it deliberately does NOT use the supply-chain facets: it models a pinned, scripts-off install as
`execution <= self` / `persistence = installing` / `network = fetches`, because a clause admitting
`execution = network-sourced` cannot be expressed cleanly — a `<=` ceiling loosens unguarded
`ambient-config` (Makefiles and hooks slip into developer) and an exact/floor bound breaks facet
monotonicity. The pinned/scripts-off distinction is enforced at the RESOLVER, which emits the safe
shape only for that exact form; anything less emits `supply-chain-build`, which has no home below
yolo.

So `supply_source`/`pinning`/`exec_surface` are not awaiting authoring — nothing is expected to
constrain them, and their hazard declarations rest on doc comments permanently unless the level model
changes. That makes them structurally unverifiable by this test rather than temporarily so, which is
the more useful thing to know.

Not a correctness bug today: `Capability::worst()` is denied by every level below yolo, and that is
now over-determined rather than resting on `locus.local` alone.

## Loopback destinations — remaining work

Shipped 2026-07-25: `netloc::is_loopback` recognizes a local destination, `loopback_valued` gates a
flag on naming one, and `loopback_localizes` clears the destination-determined facets (remote reach,
net direction, payload, metered cost) when it does — the facets describing the OPERATION are left
alone, so the level algebra composes rather than needing a local twin of every remote archetype.
Applied to `aws dynamodb`. Decision was **per-service research, not a uniform rollout** — the
mechanism is generic but each service has to earn it.

- **Services with a real local-emulator story, unresearched.** LocalStack fronts most of AWS on
  `http://localhost:4566`; `s3api`, `sqs`, `sns`, `lambda`, `logs`, `ssm` are the common ones. Each
  needs its write surface enumerated the way dynamodb's was — the gate alone does nothing for an
  operation that isn't in the registry.
- **Other tools with an endpoint flag.** `docker --host tcp://127.0.0.1:2375` and `kubectl --server
  https://127.0.0.1:6443` were identified as candidates; neither is researched.
- **Spellings deliberately unrecognized.** `http://2130706433`, `0x7f000001`, `0177.0.0.1`, `127.1`
  are loopback in fact and denied on principle (ambiguous parsing). Revisit only if a real workflow
  needs one.
- **The tunnel caveat is structural, not a bug.** `ssh -L 8000:<service>.amazonaws.com:443` makes
  `localhost:8000` production and no static classifier can see it. This is why destroy archetypes
  may not set `loopback_localizes`, enforced at build time against the archetype's
  `operation` facet rather than its name.

## Retire blanket flag tolerance (`tolerate_unknown_short/long`)

Decision (2026-07-25): eventually remove the "any flag is fine" escape hatch. A sub that declares it
accepts flags nobody researched, which is the same unresearched-assertion problem we just removed
from the per-sub lists — only bigger, and it silently defeats the per-sub flag enforcement (a sub
with `tolerate_unknown_long = true` accepts everything regardless of what it enumerated).

Current exposure: **2,328** `tolerate_unknown_short = true` and **1,630** `tolerate_unknown_long =
true`. Concentrated in the big cloud CLIs — az (700), gcloud (389), aws (247), oci (61) — plus jj
(53), claude (37), notion (18), codex (17).

Known live consequence — FIXED 2026-08-04. `systemctl status nginx -H remote.example.com` used to
auto-approve: `-H` retargets systemctl at a REMOTE host over SSH (and `-M` at a container), turning a
local read into an operation on another system, and `status` declared `tolerate_unknown_short = true`,
which accepts ANY short flag whatever the sub enumerated.

`tolerate_unknown_short` is gone from all six read subs (status, show, is-active, is-enabled,
is-failed, cat). The fix is allowlist-shaped, not a denylist on `-H`: the subs now stand on their
enumerated flags, so `-H`/`-M` fall to approval by omission along with anything else unresearched.
Verified both ways — `-H`/`-M` deny on every read sub, while `systemctl status nginx`, `status -a -l`,
`status -q`, `show -p Type`, `show --value -p Type`, `is-active`, `cat` and `list-units --type=service`
all still auto-approve. `--quiet`/`-q` was added to status and `--value`/`-P` to show, checked against
the systemd 257 man page rather than assumed, so removing the tolerance introduced no false denies.

This closes the ONE confirmed live consequence of the tolerance pile; the remaining ~2,300 entries are
still the deferred campaign below.

Two distinct shapes, both bypassing per-sub enforcement:
1. **A profiled sub that declares tolerance** — enumerates flags, then accepts anything anyway.
   Still open.
2. **A `first_arg` GLOB family** — `aws s3api` matches `get-*`/`head-*`/`list-*`, so those actions are
   never profiled subs at all and never reach the flag check. **Mechanism fixed 2026-07-25; migration
   in progress — see below.**

Why it is deferred, not skipped: retiring it means enumerating thousands of per-service flags, and
until a command is done its invocations start denying. Needs planning and batching like the
re-research campaign — not a cleanup. The build already REFUSES a profiled sub that declares neither
a flag list nor an explicit tolerance, so new subs cannot quietly join this pile.

### Glob-family flag migration (shape 2) — in progress

Decision (2026-07-25): keep the verb glob, gate its flags. The `describe-*`/`get-*`/`list-*` claim is
a real, durable statement about the CLI's verb convention — it keeps covering read APIs the provider
ships tomorrow, which a hand-enumerated operation list does not. What the glob lacked was a flag
list, so it decided on the first positional and never examined the rest of the line.

`first_arg_standalone` / `first_arg_valued` (see SAMPLE.toml) now gate an admitted verb's flags.
An UNDECLARED family stays permissive — ~250 service groups can't be researched at once and denying
them wholesale would break every cloud read — and `no_new_unresearched_first_arg_family` ratchets the
remaining set so it can only shrink.

**Remaining: 237** (was 250). Migrate high-blast-radius services first; the ratchet count in that
test is the running total.

Done: aws iam, s3api, logs, cloudtrail, ec2, rds, lambda, ecs, ssm, dynamodb, cloudformation, sns,
sqs. Converted to explicit sub-subs instead (more precise, worth it for credential stores): aws
secretsmanager, aws kms, gcloud secrets.

Next by blast radius: aws eks, ecr, apigateway, route53, organizations, sts-adjacent identity
services; then az (~700 tolerances) and gcloud (~389), which also still need the structural
subgroup-glob fix tracked in the cloud-CLI notes.

The per-service payoff is in the flags each service withholds, which is why this can't be done
generically. Real examples found so far: `logs --unmask` (returns data-protection-masked log content
in the clear), `ssm --with-decryption` (decrypts SecureString values), `s3api --sse-customer-*`
(supplies caller-held key material). Every family also withholds `--endpoint-url`, `--profile`,
`--ca-bundle`, `--no-verify-ssl`, `--no-sign-request`.

## Fuzz suite — four property targets live, two more specified

Live in the nightly `property-targets` matrix, each with its own corpus so they accumulate
independently: `equivalence`, `hook_envelope`, `explain_render`, `suggest_roundtrip`. Plus the
original `parse` (availability) on its own sharded pipeline.

What each ASSERTS beyond "did not crash", which is the whole point — `parse` discards the verdict,
so it can only ever find hangs and panics:

  equivalence        a semantics-preserving respelling cannot change the verdict
  hook_envelope      a target never emits a grant it was not asked for
  explain_render     the explanation describes the verdict that was ENFORCED; a command cannot
                     manufacture a marker line; no control character or bidi override survives
  suggest_roundtrip  every config we generate parses back, and merging never drops what was there

First runs: explain_render 395k clean, suggest_roundtrip 295k clean, equivalence 111k clean,
hook_envelope 899k clean. The two added earlier each found a real bug on day one.

SIX property targets live now: equivalence, hook_envelope, explain_render, suggest_roundtrip,
level_monotonic, config_load. First runs, all clean except the two that found real bugs on day one:
explain_render 395k, suggest_roundtrip 295k, level_monotonic 160k, config_load 505k.

SEVEN targets now, and the list is COMPLETE for the layers that exist: parse (availability),
equivalence, hook_envelope, explain_render, suggest_roundtrip, level_monotonic, config_load,
setup_merge. Each asserts a property the others cannot see; none was added for coverage's own sake.

setup_merge closes the last one: arbitrary bytes as an existing settings file, through each target's
real `install`. On refusal the file must be byte-identical; on success it must still parse as JSON.
Filesystem-driven, so it runs ~38k iterations where the pure targets run hundreds of thousands —
worth it because the property is about `install`, not about a parser. It carries an explicit
vacuity guard: discovery learns each target's config path by installing once into a clean tree, and
if that found nothing the target asserts rather than looping over an empty list.

DELIBERATELY NOT ADDED: docs.rs (no security property) and pathctx (already covered by proptests,
which shrink better than libFuzzer for a pure function). A target is only worth its nightly runtime
if it asserts something the others cannot.

## Fuzzing finds availability bugs only — two targets worth adding

State (2026-07-30): healthy and quiet. Nightly green five nights running (~5h, sharded), replay
green on every push, no crash/timeout/oom artifacts, corpus merged to 18,013 inputs.

But `fuzz_targets/parse.rs` ends in `let _ = is_safe_command(&command);` — the verdict is
DISCARDED by design, so the only contract under test is "does not panic or hang". Measured against
that: every one of the ~30 defects found in the 2026-07-29/30 review session was invisible to it.
The heredoc-body fail-open, the 24 executor-flag gaps and the env-twin bypasses are all
CLASSIFICATION bugs — the fuzzer runs them and sees no panic. The hook blank-command approval, the
`--setup` panic and the `--suggest` phishing vector live in `targets/*` and `suggest.rs`, which
`is_safe_command` never calls.

That is not a criticism of the target: it does its job, and the availability contract it guards is
real (a panicking PreToolUse hook fails OPEN). It just means the fuzzing budget currently buys
nothing against the bug class that actually dominates.

1. **A METAMORPHIC target — the highest-value one.** Assert a property OF the verdict instead of
   discarding it: a semantics-preserving respelling must not change the verdict. Every class found
   this session was exactly a respelling gap — `--flag=V` vs `--flag V` vs `-fV`, an env twin vs its
   flag, a heredoc body vs a herestring. Generate a command, apply a transform that provably does
   not change what the shell does, assert the two verdicts are equal. Unlike the current target this
   can find fail-opens, and it needs no oracle beyond self-consistency. Start from the transforms
   already hand-written as guards (`FormCase` flag-form equivalence, the twin tag, the heredoc
   herestring equivalence) — they are the seed set.

2. **A hook-envelope target** for `targets/*` (already noted in AGENTS.md §Fuzzing). Feed arbitrary
   bytes as an envelope to each target's `parse_input` + render path. The blank-command approval and
   the wrong-typed-key panic were both found by hand there; a target would have found both and keeps
   finding them as harnesses change. Note a subtlety: the interesting contract is not only
   "no panic" but "never emits an ALLOW decision it was not asked for", which is checkable.

Neither needs new machinery — `cargo fuzz` is already wired, sharded and merging corpora nightly.

## `--version`: research EVERY instance individually — DECIDED, do not sweep

Decision (2026-07-30, user): we will not add globally-accepted flags. 2,394 subs accept `--help`
without `--version`, and that count is NOT a work item to batch — each one is researched against
the tool it belongs to or it stays off the list. An omitted flag merely prompts; a wrongly-asserted
one lies about what the tool accepts.

Done under that rule: `cargo deny --version`, MEASURED against the installed cargo-deny 0.19.0
(prints `cargo-deny 0.19.0`, exit 0) rather than assumed from the clap convention.

## wasm-pack — CONVERTED to facets; one refinement left

Re-researched at 0.15.0 against the installed binary and converted from `candidate = true` to
archetype profiles, so the verdict is DERIVED rather than asserted. `--explain` now names the facet
that decides it: `persistence.level = installing (allowed: <= data)`.

  build, test, new  supply-chain-build   fetches an executable over the network and runs it
  publish           remote-create        creates a published version on the npm registry
  login             credential-mint      obtains a token and persists it — secret WRITING
  pack              (unlisted)           local create, no network; no archetype applies

REMAINING: `wasm-pack build --mode no-install` genuinely does not fetch or install — it is a local
build executing only workspace-authored code, exactly as `cargo build` does. Classifying that form
separately is defensible and needs the flag-conditional mechanism npm's `ci` entry already uses
(`when_absent` on a safety flag). It was NOT faked with a flat listing, because the default
invocation is the one an unqualified `wasm-pack build` performs and that is what the profile has to
describe. `--panic-unwind` stays off the flag list either way: it installs a nightly toolchain,
`rust-src` and the wasm32 target through rustup as a side effect of a build flag.

## Atom confinement — the `$SCRATCH` half is NOT fixed (found in adversarial review)

The reported command was `for i in $(seq 1 4); do … > "$SCRATCH/dx_$i.txt"; done`. The confinement
work fixed the `$i` half; the `$SCRATCH` half still denies, so the user's literal command STILL
does not approve. `echo hi > "$SCRATCH/plain.txt"` denies on its own — no loop, no atom — which
locates the residue in the PREFIX, not in anything the atom work touched.

This is correct, not a bug. `$SCRATCH` is NOT a harness convention — Claude Code does not set it
(the shell has `CLAUDE_JOB_DIR`, `CLAUDE_CODE_SESSION_ID`, `CLAUDE_EFFORT`; the scratchpad path is
deliberately not exposed at all). So it is a variable the user or the agent defined locally, naming
anywhere, and under abstraction soundness it must deny. The forms that DO work are the literal and relative prefixes: `./out/dx_$i.txt`,
`/tmp/dx_$i.txt`.

### Resolving a `$VAR` path prefix is UNSOUND — closed, do not reopen

Two candidates were considered and both are dead: `$CLAUDE_PROJECT_DIR` (the harness names the
project root, so why not use it) and `$TMPDIR` (POSIX standardizes what it means). The argument
kills any variable, so it is written once here.

1. Classifying `$VAR/rest` requires knowing VAR's value IN THE SHELL THAT RUNS THE COMMAND, and
   safe-chains cannot know it. Reading its own environment is not the same question: the hook's
   environment is not the agent's. MEASURED — the agent's Bash shell has `CLAUDE_JOB_DIR`,
   `CLAUDE_CODE_SESSION_ID`, `CLAUDE_EFFORT` and NOT `CLAUDE_PROJECT_DIR`, which the docs list as
   available to hooks. The variable the idea rested on is absent from the shell that would expand it.
2. The failure mode is severe and does not degrade gracefully. An unset variable does not make the
   path land somewhere else in the workspace — it makes it land at the ROOT:
   `"$CLAUDE_PROJECT_DIR/out/x"` expands to `/out/x` (measured, not reasoned). So a wrong guess
   turns a worktree write into a write at `/`.
3. `$TMPDIR` dies to the same argument. It is set on this machine, but "happens to be set here" is
   not a guarantee, and POSIX pinning its MEANING says nothing about its PRESENCE.
4. `envvars.toml` was never the mechanism anyway. It classifies `VAR=value cmd` ASSIGNMENTS, where
   the value is literally in the command string (`assignment_verdict(name, value)`). It does not
   resolve `$VAR` expansions and adding that would be a different feature with this problem.

Current behaviour — deny — is correct, and there is no sound relaxation available: the worst case of
`$VAR/out/x` is unbounded, and even its best-known case `/out/x` denies on its own.

What works instead: spell the prefix literally (`/tmp/dx_$i.txt`, `./out/dx_$i.txt`). The harness
exposing the scratchpad path would be the real fix and Claude Code declined it
(anthropics/claude-code#45745, "not planned"), which is why the session-id + path-shape recognition
in `pathctx` exists at all.

## Three reported prompts — two pieces of work (analysis done, implementation not)

### A. `$(( ))` containing a substitution — fix is KNOWN, blocked on a pre-existing hang

Symptom: `echo "days left: $(( (X - $(date -u +%s)) / 86400 ))"` denies. Every other segment of that
chain approves, including `date -u +%s` alone, plain `$(( ))`, and `$(date …)` in an ordinary string.

Cause: `arith_sub` (cst/parse.rs) backtracks whenever the body holds `$(` or a backtick. That hands
`$((` to `cmd_sub`, which reads it as `$(` plus a subshell — `--explain` renders `$( (1 + …))`, a
command the user never wrote — and then refuses it because `(1` is not a command. Fail-CLOSED, so no
security exposure; the cost is a false deny on an everyday idiom. The backtrack was deliberate and
defensible: treating the body as opaque text would hide the inner command, which is a fail-OPEN.

The fix that WORKS (built, full suite green, new guard passing):
  `Arith(String)` -> `Arith(Word)`; parse the body; `part_sub_verdict` recurses into it exactly as it
  already does for `DQuote`; `collect_part_subs` likewise; display renders the parts. Five sites.
  Arithmetic stays inert, the inner `$( )` is verdicted normally — `$(( 1 + $(rm -rf /) ))` still
  refuses. `$((cmd))` correctly becomes inert, which MATCHES bash: bash parses `$((` as arithmetic
  and errors on a non-arithmetic body, it does not execute it. `$( (cmd) )` with a space is
  untouched and still a real command substitution.

WHY IT IS NOT LANDED — a pre-existing landmine underneath it, measured this session:
  - Balanced nesting already hangs WITHOUT any change: `echo $((1+` x1000 `))` x1000 TIMEOUTs (>45s)
    on released 0.220.0 AND on current HEAD. `echo $( ` x50000 balanced TIMEOUTs too. So nested
    substitution blow-up is an EXISTING defect, not one the arithmetic fix introduces.
  - With `Arith(Word)` and no depth guard, that same x1000 case became exit=0 in 0.43s — the fix
    IMPROVED it — but `$((1+` x50000 then ABORTED (SIGABRT). Arithmetic recursion bypasses
    `MAX_PARSE_DEPTH`, which is enforced at `script()`, "the funnel every other recursion source goes
    through". An abort is the worst outcome: a hook crash fails OPEN and `catch_unwind` cannot
    recover a stack overflow.
  - Taking `DepthGuard::enter()` inside `arith_sub` fixed the abort and made x1000 take 68 SECONDS:
    bailing backtracks into `cmd_sub`, which re-parses the whole nest.
  - Parsing the body with a non-recursive part parser (no `arith_sub` inside) still timed out at
    x1000.

  MEASURED (release, profiling both halves separately — see numbers below); the guesswork in the
  paragraph after this is superseded by them.

  1. THE COST IS ENTIRELY IN PARSE. `parse` and full `is_safe_command` are equal to within noise
     (1.55s vs 1.57s at depth 400), so classification contributes nothing and only the parser is in
     question.
  2. IT IS NESTING, NOT LENGTH. At the SAME byte count (~2805):
         flat   `$((1)) ` x400  ->   56.6 us   (and linear: 17.8 / 31.0 / 56.6 us at 100/200/400)
         nested `$((1+`   x400  ->    1.55 s
     27000x apart on equal input. Real commands are flat, so a far tighter work budget would bite
     the pathological shape without touching them — that is the headroom the fix lives in.
  3. GROWTH is ~quadratic in DEPTH (~3.5x per doubling), not exponential.
  4. THE PARSE FAILS AND IS SLOW ANYWAY — `parsed_ok=false` throughout; 1.55s is spent arriving at a
     refusal. The expense starts exactly where the depth cap begins firing: `$( ` x25 parses in 17us,
     x50 fails in 19ms.
  5. WHY THE WORK BUDGET NEVER TRIPS: it counts `script()` ENTRIES and allows
     `16384 + 512 * len`. For a 2806-byte input that is 1.45 MILLION entries, far above the work
     actually done. `MAX_PARSE_WORK_PER_BYTE = 512` is the loose constant.
  6. TRIED AND REJECTED: making the depth bail a `Cut` instead of a backtrack, on the theory that
     `alt` was retrying other parsers over the same nest. It is principled and safe (the guard only
     fires where the parse already fails) but bought only ~8% — 1.68s -> 1.55s — so alt-retry is NOT
     the dominant cost. Reverted rather than left in the tree as an unvalidated change.

  NEXT: the remaining suspect is per-position balanced SCANNING — `arith_sub` and `cmd_sub` each
  scan forward for their closing delimiter before recursing, so at every nesting level the scan
  spans the whole remaining tail. Instrument that directly (count bytes scanned) before changing
  anything. If confirmed, the fix is to charge scan work to `PARSE_WORK` and lower PER_BYTE, which
  finding 2 says is safe for real commands.

  Superseded guesswork, kept only to show what was ruled out: suspect
  `MAX_PARSE_WORK_BASE + PER_BYTE * len` — for a 250 KB adversarial input that budget is enormous, so
  it never trips; the depth cap does not fire because bailing re-enters a different parser rather
  than failing the parse outright. A `cut`-style error (no alternative tried) at the depth bail is
  the first thing to try.

### B-CORRECTED. `find` has NO output claim — assignment propagation was never the problem

The original diagnosis here was WRONG and is corrected in place. Assignment propagation already
works: `D=$(pwd); cat "$D/README.md"` APPROVES, and `D=$(pwd); cat "$D/../../../etc/passwd"` denies,
so a claim survives a variable AND its descent is judged. `pwd` works because it declares
`[command.output] locus_from = "cwd"`.

`find` declares no `[command.output]` at all, so EVERY `$(find …)` is unpinnable regardless of root
— `cat "$(find ./sub -name x -type d)"` denies even inside the worktree. That is a missing
declaration, not a soundness barrier, and `fd` already has the shape to copy
(`locus_from = "operands"`, with `invalidated_by` for the flags that change what is printed —
`-printf`, `-exec`, `-ls`, `-fprint` at minimum).

The reported jjpr command STILL will not approve after that fix, and correctly: `"$D/src/"`
DESCENDS from the substitution, and `find` can match nothing, so `D=""` makes the path `/src/`,
which denies on its own. Adding the claim fixes non-descending uses; the descent-from-possibly-empty
case stays refused for the same reason as the `$VAR` prefix closure above.

### OLD (superseded) framing: a claim lost through assignment

Reported: in `jjpr`, `D=$(find ~/.cargo/registry/src -maxdepth 2 -name '…' -type d | head -1)` then
`grep -rn 'divergent' "$D/src/"` prompts. Measured: the `cd`, the assignment and the `echo` all
approve; grepping the LITERAL path approves (the package-content read admit works); only the
`"$D/src/"` segment denies.

This is NOT the `$SCRATCH` case and must not be filed with it. `$SCRATCH` is an unbound external
variable with no claim to propagate, and denying it is correct forever. Here the value IS bounded —
`find <root>` carries an output-locus claim that its stdout names paths under that root — and the
claim is simply dropped when the value passes through a variable.

The machinery already exists for the sibling case: `for i in $(seq 1 4)` propagates, because
`loop_reprs` binds the loop variable to the list's representative. Plain assignment has no
equivalent. So the work is to bind an assignment's RHS to the substitution's claim the same way, and
then `"$D/src/"` is a DESCENT from a tagged sentinel, which `tagged_substitution` already handles
(it is the `$(pwd)/.git/config` case).

Scope carefully: only an assignment whose RHS is a claim-carrying substitution IN THE SAME COMMAND.
An assignment from anything else stays unpinnable. This subsumes the third reported item ("handle D
assigned from command substitution") — they are one piece of work, not two.


### C. `cargo fuzz` is not an allowed cargo subcommand

`cargo fuzz --version` denies; `which cargo-fuzz`, `rustup toolchain list | head -5` and
`sed -n '/^\[dependencies\]/,/^\[/p' Cargo.toml | head -25` all approve, so it is the only blocker
in that chain. cargo-fuzz is a third-party cargo subcommand and is simply absent from cargo's sub
list. Research it as its own command surface — `init`, `add`, `build`, `run`, `fmt`, `cmin`, `tmin`,
`coverage`, `list` — and note that `run`/`cmin`/`tmin` EXECUTE the fuzz target (workspace-authored
code, but still execution) while `list`/`--version` only report.

## cargo fuzz: REVERTED — needs a pathgate handler and a positional shape, not TOML whittling

Found in adversarial review of the batch, in a change made earlier in the same session:
`cargo fuzz cmin parse ~/.ssh` was AUTO-APPROVED. `cmin` minifies a corpus by rewriting the
directory it is handed — removing files — and that directory is a plain POSITIONAL. `run` writes
new inputs into the same slot, and `tmin`/`fmt` read an arbitrary input FILE and print a derived
result.

`add` joins them for a different reason found in the SECOND review round: its positional is a target
NAME interpolated into `fuzz/fuzz_targets/<name>.rs`, so `cargo fuzz add ../../../etc/x` created a
file outside the workspace. "Scaffolds a file locally" sounds bounded until you notice the name is a
path component — the first round fixed the corpus positional and missed its sibling.

`add` is also the one that does NOT need the handler: its argument should be a bare identifier, so a
`positional_shape` admitting only a separator-free name would be enough, and adding a shape is a
one-line `PositionalShape` addition plus a match arm.

REVERTED ENTIRELY. Three review rounds each found another operand that becomes a path, and the
attempt to keep a minimal safe subset failed twice more: dropping `nested_bare` made the sub accept
arbitrary positionals (`cargo fuzz cmin parse ~/.ssh` admitted again), and restoring it with no
`[[command.sub.sub]]` declared did the same. The TOML shapes available cannot express "this sub
exists but takes no operands".

`init --target` is worth recording as an upstream footgun for whoever redoes this: `init` documents
`-t, --target <TARGET>` as "name of the first fuzz target to create", while `build` documents
`--target <TRIPLE>` as the target triple. One spelling, two meanings, one tool — so a flag list
copied between subs is wrong in a way that reads as correct.

Redo it with: a pathgate handler keyed on `tokens[1] == "fuzz"` gating the positional after the
target name (write for run/cmin, read for tmin/fmt/coverage), and a `positional_shape` admitting
only separator-free names for `add` and `init --target`. The corpus-free subs (`list`, `build`, `check`, `add`, `init`) are
allowed, which covers the reported `cargo fuzz --version` case.

Why the obvious fixes do not work:
  - A `positional = "write"` role in pathgates.toml keys on the COMMAND name, and cargo's
    positionals are meaningful everywhere else (`cargo test <filter>`, `cargo run <args>`), so the
    role cannot be scoped to `fuzz`.
  - `max_positional` was tried and reverted. It cannot distinguish the target NAME from a corpus
    DIRECTORY, and it refuses `cargo fuzz run t -- -max_total_time=60` — measured — because the
    libFuzzer arguments after `--` count as positionals. That is the ordinary invocation.

So this wants a pathgate handler keyed on cargo that inspects `tokens[1] == "fuzz"` and gates the
positional after the target name — `write` for `run`/`cmin`, `read` for `tmin`/`fmt`/`coverage` —
the same shape `ar_archive` uses to gate by operation.

## pulumi config: bounded to the bare form; every sub-form still needs research

`config` is now `nested_bare = true` + `max_positional = 0`, so the bare listing works
(secrets masked as `[secret]`; `--show-secrets` is an unknown flag and refuses) and every sub-form
denies by omission.

`get` was briefly listed at SafeRead in an earlier pass and has been REMOVED, which is the part
worth keeping: `pulumi config get <key>` returns the PLAINTEXT of a secret value where the bare list
masks it, and whether a key holds a secret is not knowable from the command string. The honest
classification is the existing `decrypt-read` archetype — "reveals plaintext secret material to the
caller", `secret = { level = "reads" }`, which sits at yolo — the same treatment `sops -d`,
`age -d` and `ansible-vault view` already get. Listing it at SafeRead was a secret-disclosure hole,
introduced and removed within this session.

MEASURED against pulumi v3.255.0 — an earlier pass recorded "pulumi is NOT installed locally" and
reasoned from documentation. That was wrong: `command -v pulumi` came back empty in this shell while
the binary sits at /opt/homebrew/bin/pulumi. A negative from one probe is not proof of absence, the
same lesson as classifying `seq` from macOS's BSD build.

What the real surface says:
  - Subcommands are `get`, `set`, `set-all`, `remove`, `remove-all`, `copy`, `refresh`, `env`. The
    doc-based guesses `rm` and `cp` were WRONG names, which only escaped notice because a wrong name
    denies by omission.
  - `get` CONFIRMED as `decrypt-read`, by structure rather than by running it: `--show-secrets`
    exists only on the listing form and is documented as "show secret values when listing config
    instead of displaying blinded values". Blinding belongs to LISTING; `get` has no such flag and
    no blinding concept, so it returns the value directly — plaintext for a secret key.
  - `--config-file` takes a filesystem PATH and would need a read gate before being listed.
  - `--open` defaults to TRUE and resolves ESC environments, so even a read form reaches remote
    providers unless it is disabled.

RESOLVED PREMISE (v3.255.0): the blocker recorded here — "the file backend writes
`Pulumi.<stack>.yaml` locally, the service backend writes REMOTE state, and the command string does
not say which" — rested on a mistake of mine. Pulumi stores stack CONFIG and stack STATE separately.
Config lives in a detected local file, which `--config-file` states outright: "use the configuration
values in the specified file rather than detecting the file name". The backend holds the
checkpoint/state, a different artifact. So `config set` does not write remote state under either
backend, and the file-vs-service distinction is not what decides its locus.

What that leaves, and it is a better-shaped question:

  - WRITE TARGET is `Pulumi.<stack>.yaml` in the project directory — a worktree path when run from
    the project, which is SafeWrite-shaped, and already locus-gated like any other file write.
  - `--secret` is the flag that changes the profile, and it is VISIBLE in the command string:
    "encrypt the value instead of storing it in plaintext". Encryption needs the stack's key — the
    Pulumi service's under the service backend, `PULUMI_CONFIG_PASSPHRASE` under a local one. That is
    exactly the `[[command.sub.flag]]` classifying-flag shape (`classifies = …`) this repo already
    uses for `sops`/`age`/`ansible-vault`, rather than a reason to withhold the whole subcommand.
  - `set`'s full flag surface is small and researched: `--path`, `--plaintext`, `--raw`, `--secret`,
    `--type`.

RESOLVED — `set` cannot be auto-approved, and the reason is not where it writes.

Measured: `pulumi whoami` on this machine returns a Pulumi SERVICE account, and `pulumi config` run
outside a project errors with "no Pulumi.yaml project file found". So config operations are
project-scoped, and the BACKEND is ambient — it comes from login state or `PULUMI_BACKEND_URL`, never
from the command string. A bare `pulumi config set` therefore may consult the service to resolve the
stack, and nothing in the command lets that be ruled out. SafeWrite is local-only, so the possibility
alone disqualifies it. The write TARGET being a local file was never the deciding factor; what the
command must consult to get there is.

That also closes the "measure it with a local backend" plan recorded above: measuring the local case
would prove only that ONE backend is local, which is not the question. The question is what the
abstraction can denote, and it can denote the service.

The one way this could change: if the backend were made visible in the command string — a
`PULUMI_BACKEND_URL=file://…` prefix classified through envvars.toml would do it, since that is the
existing mechanism for a value that changes what a command reaches. Then the file-backend spelling
could be admitted while the bare form stays out. That is a real design option, not a workaround, and
it is the same shape `GIT_DIR` and friends already use.

Also found while researching: `-C` / `--cwd` ("run pulumi as if it had been started in another
directory") is a GLOBAL flag, so it applies to every pulumi subcommand including the allowed bare
`config`. It takes a path and currently denies by omission — verified. Any future listing must not
add it without a path gate; it relocates the whole invocation.

Unchanged and still out for their own reasons: `copy` (writes a second NAMED stack), `refresh`
(remote read plus local write), `env` (a whole ESC surface), `remove`/`remove-all` (same shape as
`set`, so they resolve with it), and `get` (`decrypt-read`).

## Command-tree duplicates — ALL 35 FIXED. One intent question remains.

`the_command_tree_has_no_duplicate_names` found 35 duplicates on its first run; all are resolved and
the backlog fixture is empty, so the guard is now absolute. Every removal was behaviour-neutral,
verified against the released binary, because the loader's rule was established by MEASUREMENT
first: the FIRST declaration wins (proved on `aws sts`, where the later block's `first_arg` globs
and `tolerate_unknown_*` had no effect at all).

Three distinct origins, and the distinction is the useful part — a duplicate is not one kind of bug:

  1. GLOB-SWEEP OVERLAP — `aws sts`. An explicit block already existed when the glob-sweep batch
     appended a `first_arg` family. The dead block was the PERMISSIVE one, so nothing was
     mis-classified, but it was a hazard: had lowering order ever shifted, the globs would have gone
     live silently. Removing it lowered the pinned unresearched-glob-family count 237 -> 236.
  2. STRAIGHT DUPLICATION — `gcloud artifacts`, byte-identical at 3034 bytes, which alone accounted
     for 23 of the 35 rows; and `mc share`, two identical `candidate = true` markers.
  3. BULK-PASS LEFTOVERS — `dub describe`, `esptool flash_id`, `paket outdated`, `spack license`.
     A pass that marked unresearched subs `candidate = true` did not dedupe against entries research
     had already landed, so each has a live substantive block and a dead marker behind it.

STILL OPEN — a decision, not a defect: `mise` had a fourth shape. Its later blocks were a uniform
`bare = false` / `max_positional = 0` / `standalone = ["--help", "-h"]` — someone adding a
deliberate RESTRICTION to `install`, `use`, `upgrade`, `prune`, `uninstall`, `unset`. Because the
first declaration wins, that tightening NEVER TOOK EFFECT; those subs are live at their full
`SafeWrite` surfaces. The dead blocks were removed to preserve current behaviour rather than guess
at intent. Decide whether the restriction stands: if it does, it is a separate, deliberate
tightening with its own review, not a duplicate cleanup.

## Structural invariants: three probed clean, one REAL gap remains

Asked after the duplicate-tree guard: what else is declared protection that can never fire? Probed
three classes across the registry. All three are clean, and the reasons are worth keeping so nobody
re-probes them:

  - `invalidated_by` naming a flag the command never accepts — 0 found. That one would be nasty (a
    claim-voiding flag that can never be presented, so the claim stands when it should not).
  - `path_gate` on a flag no list declares — 0 real. The 2 apparent hits (`whisper --model_dir`,
    `--output_dir`) were a bug in the PROBE: its flag regex excluded underscores, so it never matched
    the declarations that do exist.
  - A flag in BOTH `standalone` and `valued` of one block — 1548 found, and SAFE. It is a deliberate
    idiom for optional-value flags (`--verbose` vs `--verbose=3`, `-color` vs `-color auto`), and
    `policy::consumes_next_value` only consumes the next token when it is NOT flag-shaped. Verified:
    `coqc -color --frobnicate` denies, so a dual-listed flag cannot swallow a following flag and hide
    it from the allowlist.

THE REAL GAP — reparenting WITHOUT duplication. `the_command_tree_has_no_duplicate_names` catches a
duplicated span because duplication necessarily repeats a name. It does NOT catch a `[[command.sub.sub]]`
that simply moved under the wrong parent: the pulumi corruption reparented `history`/`tag`/`graph`
under `config` AND duplicated, and only the duplication was detectable. A pure move would still pass.

Guarding it needs an expectation of which subs OWN nested subs, which the data does not currently
state. Two ways in, neither yet built:
  - "Assert every `[[command.sub.sub]]`'s parent actually dispatches nested" was investigated and is
    VACUOUS: `build_sub_kind` returns `DispatchKind::Branching` whenever `!toml.sub.is_empty()`, so
    every parent with nested subs dispatches them by construction. There is no such thing as an
    ignored nested sub. The nearest NON-vacuous variant — a `candidate` parent, whose nested subs
    `filter_candidates` silently drops — found 0 instances but is now asserted in `filter_candidates`
    so it cannot appear.
  - Pin the tree SHAPE per command — a fixture of `command sub subsub` triples, regenerated
    deliberately — so any move shows up as a diff rather than needing to be reasoned about.

Adjacent and also unbuilt: `[command.output]` or `[command.fallback]` declared on a command whose
dispatch never consults it — dead declarations that read as configuration.

## Refusal copy — SPEC WRITTEN, not implemented (docs/design/refusal-copy.md)

The message an agent meets when we do not auto-approve is jargon-laden and, worse, reads as a
verdict: "this command is not on the allowlist" sounds like the command was assessed and REJECTED,
whose natural response is to hunt for a spelling that passes. The true statement is nearly the
opposite — safe-chains only grants approvals for researched commands, and says nothing about the
rest.

Prompted by `RUSTDOCFLAGS=-D warnings cargo doc …`, refused because the unquoted assignment makes
`warnings` the COMMAND NAME. The message never said `warnings`, so the refusal looked arbitrary,
while naming it would have been an instant bug report — and the bug was otherwise silent, since
`bash: warnings: command not found` matches neither `^error` nor `^warning` and the user's own grep
would have swallowed it.

The spec's load-bearing rule: the copy is selected by the (capability, EMISSION) pair, not by harness
name. A deny-harness we ABSTAIN on produces an ordinary prompt, not a block, so "blocked" would be a
lie there. Deriving copy from emission is what stops it drifting when a harness's behaviour changes —
as Cursor's did when `allow` turned out to be ignored. Any implementation that hardcodes wording per
target reintroduces exactly the drift HARNESS-BEHAVIORS.md exists to prevent.

Also specified: always name the resolved command; neutral vocabulary with an explicit avoid-list;
a fallback that is vague about CONSEQUENCE but specific about CAUSE when the harness is unknown; and
a parse-surprise hint emitted only when the resolved name is an unknown bare word AND an
env-assignment prefix is present.

Review of the examples exposed a UX inconsistency worth fixing in the LOGIC: a credential path is
refused at `developer`, APPROVED at `local-admin` and `yolo`, and refused again at `network-admin`
(siblings, not a ladder). A path grant never opens it, by design. So the lever a user reaches for
first does nothing and nothing tells them the other lever exists. The spec records three fixes, the
cheapest being that a refusal should name the lever that WOULD work, plus a guard that a message
offers a grant if and only if a grant actually changes the verdict.

Four guards specified, each needing a red demo — three of this session's findings were in this same
message layer and every one looked correct until the demo showed the text had not moved.

## A grant should cover what it names — SPEC (docs/design/explicit-grants.md)

A user tired of approving `~/.ssh` reads writes `[[grant]] path = "~/.ssh", read = true` and expects
to stop being asked. Today nothing changes and nothing says why.

The rule that fixes it is already in the codebase, for hidden files: a grant covers the subtree it
NAMES, and carve-outs exist to stop a grant reaching into things it did not name. `remainder()` is
the path below the grant root, so a `~/` grant sees `.ssh/id_rsa` (hidden, refused) while a `~/.ssh`
grant sees `id_rsa` (dot-free, admitted). The comment says it outright: "grant such a directory
explicitly to reach inside it."

The secret carve-out does not follow that rule. `apply_grant` bails unconditionally on
`base.reads_secret`, never asking whether the grant named the store. Two carve-outs, one asks, one
does not, and the one that does not is the one users hit.

Change: compare the SHIELDED NODE against the grant root. Node strictly below the root means the
grant swept it up, so the shield wins. Root at or inside the node means the grant named it, so the
grant wins. No new syntax, no new locus rung, no new field.

Implementation cost is one thing: `apply_grant` only receives the resolved `Role`, which has lost
which node matched, so the shielded node's path has to survive `base_region` for the comparison.

Cannot be simplified to "delete the bail and let the hidden rule cover it": most credential stores
are dot-dirs so it would look right, but `/etc/shadow`, `/root`, macOS keychains and browser profiles
are not dot-prefixed, and a broad `/etc` grant would sweep them up.

FOUR carve-out kinds, not two (`role_is_protective`): `reads_secret`, `pinned`,
`write_locus > worktree` (write freezes: `.git`, `.envrc`, package-content, system-integrity) and
`read_locus > worktree-trusted`. That last one is NOT a carve-out — it is the `unknown` role that
grants exist to widen, and "fixing" it would break ordinary grants. Write freezes are the kind most
likely to be missed, because the bail never mentions them. `system-integrity` is DECIDED as absolute like `pinned`: it is `/etc/passwd`, `/etc/sudoers`,
`/etc/pam.d/*` and the loader, not `/System`, so an earlier "the OS refuses it anyway" argument was
false — those are writable with privilege. An agent that can write `/etc/sudoers` defeats the
machine's authorization substrate including safe-chains, which is the same category of risk that
makes `pinned` absolute. Ordinary `/etc` stays `machine` and grantable, so the cost is small.

`pinned` keeps its blanket bail. safe-chains' own config write stays un-grantable however
specifically it is named, because the risk is to the mechanism rather than to the user's data.

REJECTED (an earlier draft of this spec): an `acknowledge = "credential-store"` field. The user
config is already the trust root — user-only, unwritable by agents — so a grant typed there IS the
statement of intent, and demanding a second field to prove it is ceremony rather than safety.
Anyone willing to add the grant would add the acknowledgement.

## Why the corpus is authored under the facet model, not under levels

Decision (2026-07-16), kept here because it explains the shape of every TOML in
`commands/`: a command is characterised along the behavioural axes and the level falls out of that
profile, rather than a level being assigned directly. The level-based tail hid real credential
exposures — `vault read`, `security find-internet-password`,
`aws secretsmanager get-secret-value` — because "read-only" flattens the axes that decide safety.

The mechanism consequence, which IS this repo's job: every axis a description asserts must have
somewhere in the schema to live and something in the engine that reads it. Where it does not, that
is a Tier 1 or Tier 2 item above, not a per-command matter.

## Pre-1.0 hardening

- **Credential-exposure audit — the #1 correctness item (the one class that escapes the SafeWrite-local
  bound: a "read" that returns REMOTE secret material). SWEEP SPEC BUILT + partly gated.**
  - Two guards enforce the class: `credential_smelling_subs_are_classified_or_grandfathered` (sub NAME
    layer) and the new `credential_store_reads_are_denied` corpus ratchet (ARGUMENT / whole-tool layer).
    IMPORTANT: the class CANNOT be swept generatively — a blind `<read-verb> <secret-word>` probe is
    vacuous (1855 false hits: `alembic show secret` auto-approves because `show` takes any positional).
    So the ratchet is a curated researched worklist that only grows as secret-store CLIs are researched.
  - Gated this pass: `op item get`/`read`/`document get` (profile=credential-read; op is a whole secret
    store), `vault kv get` (the KV-v2 sugar for `vault read`). Regression-covered: aws secretsmanager
    get-secret-value / ecr get-login-password / sts get-session-token / ssm get-parameter
    --with-decryption, gcloud secrets versions access / auth print-*-token, az keyvault secret show, gh
    auth token, doctl auth init, security find-internet-password.
  - kubectl `get secret` — GATED (2026-07) via the new `credential_first_arg` mechanism (below): every
    name form denies — exact `secret`/`secrets`, the slash shorthand `secret/<name>`, qualified
    `secret.v1.core`, and flag-first `get -o yaml secret` — while pods/CRDs/`secretstore` stay read-only.
    Residual (minor): conservative — gates `get secrets` (name list) too; `describe secret` stays allowed
    (it redacts values).
  - NEW MECHANISM `credential_first_arg` (2026-07) — the value-dependent credential gate. A glob list on
    a Branching sub (dispatch_branching, flag-aware) that DENIES a first-positional match before the
    first_arg allow-glob. The declarative complement to `profile=credential-read` for the "a specific
    resource/key name discloses" class. Closes kubectl secret (all forms) AND `aws configure get
    aws_secret_access_key`/`aws_session_token` (region/output stay allowed). Guarded by
    `credential_first_arg_gates_every_secret_name_form`; documented in SAMPLE.toml.
  - Breadth sweep batch 1 (2026-07): gated `bw get`/`list` (Bitwarden), `pass show`/`grep`, `heroku
    config` (all profile=credential-read; conservative on the password managers). Verified already-safe:
    doppler, gopass, chamber, infisical, `az account get-access-token`, `gcloud auth print-*`, flyctl,
    step, kubeseal, gpg -d, `cat ~/.aws/credentials`. `wrangler secret list` = names-only (grandfathered).
  - DECRYPT-TO-SCREEN — DONE (2026-07). New `decrypt-read` archetype (operation=observe, secret=reads,
    disclosure=local-process → yolo, the same tier as a credential-store read) + a NEW top-level
    `[[command.flag]]` mechanism (the flat-command analog of `[[command.sub.flag]]`: a mode flag whose
    presence classifies the whole invocation as an archetype). Closed:
    - `sops` — restructured to 3.13 subcommands. `decrypt` sub + legacy `-d`/`--decrypt` flags →
      decrypt-read; `filestatus` → SafeRead; `encrypt`/`edit`/`rotate`/`set`/`exec-env`/`exec-file`/
      `updatekeys` → candidate (remote KMS / interactive / decrypt+execute). Closed BOTH the `-d` flag
      hole AND the newly-found subcommand hole (`sops decrypt FILE` was read as a filename → SafeWrite).
    - `age -d`/`--decrypt` → decrypt-read (encrypt stays SafeWrite).
    - `ansible-vault view` AND `decrypt` → decrypt-read (`decrypt --output -` streamed plaintext to the
      model — a bypass caught by adversarial review; `view` was gated but `decrypt` was left SafeWrite).
    - `gpg -d`/`--decrypt` → decrypt-read (top-level flag). Also `gpg secret.gpg` (bare-file IMPLICIT
      decrypt) now denies: gpg requires an inspection command (`require_any`), so a positional-only
      invocation can't auto-approve an implicit decrypt/verify.
    - `openssl` (all disclosure subs: rsa/pkey/ec/dsa/pkcs8/pkcs12/enc/smime/cms) → an ENGINE RESOLVER
      `resolve_openssl` (src/engine/resolve.rs), NOT declarative. openssl's flag grammar (single-dash
      long opts `--d`==`-d`, the `-text` side channel that dumps private components past `-pubout`/
      `-noout`, an `-out` whose VALUE can be stdout, parser token-swallow) defeated declarative
      matching over 3 review rounds. The resolver emits decrypt-read (→ yolo) only when private/
      decrypted material reaches the MODEL (stdout), and abstains for public-key mode, to-FILE
      extraction, `-noout` validate, and encrypt/sign. `openssl_output_reaches_model` is FAIL-CLOSED
      (model-reaching unless a single plain-file `-out`). The superseded declarative scaffolding
      (`unless_flags`, bimodal-sub walk, single-dash-long flag normalization) was REMOVED. A glued
      `-flag=path` pathgate gap (out-of-workspace read via `-in=~/.ssh/id_rsa`) was fixed generally in
      `pathgate::walk`. Key GENERATION (genrsa/genpkey/req -newkey) stays SafeWrite — the threat model
      is exfil of EXISTING secrets, not fresh keys (user-confirmed).
    Guarded by `decrypt_read_denies_at_the_band_and_is_a_secret_read` (registry-walking) +
    `decrypt_to_screen_corpus_denies` (MUST_DENY corpus + a complement of diverted/read forms that must
    stay allowed) + `openssl_output_destination_is_fail_closed` + `openssl_resolver_gates_model_
    disclosure_only` (golden) + `openssl_decrypt_triggers_gate_both_dash_spellings`. The user's rule:
    decrypt-to-screen is NOT auto-approved below local-admin (lands at yolo, refused below). CONVERGED
    after 7 adversarial-review rounds (the last comprehensive pass: clean, fail-closed by construction).
    - `aws configure get aws_secret_access_key` / `aws_session_token` — GATED (2026-07) via
      `credential_first_arg` on `configure get`; `get region`/`output` stay allowed. (Residual: the rare
      profile-qualified key form `get profile.x.aws_secret_access_key` needs suffix-glob support — the
      current globs are prefix-only.)
    - `terraform output -raw <name>` and `helm get values <release>` — VALUE-dependent: mostly non-secret
      outputs/config, but a sensitive output / a secret embedded in values discloses. Handler-class (can't
      gate the whole sub without over-denying the common read). Grouped with the value-dependent set.
  - INTENTIONALLY ALLOWED (verified, not holes): `kubectl get configmap -o yaml` (ConfigMaps are officially
    non-secret; gating over-denies config reads); `cat .env` (worktree-local — the workspace-boundary model
    lets the agent read its own project files; a remote EXFIL of it still denies).
  - Remaining sweep: keep researching secret-store / cloud CLIs and add each credential read to the ratchet.
    The value-dependent class (sops -d, aws/terraform/helm/kubectl-configmap) wants a shared "flag/first-arg
    triggers credential-read" mechanism — worth designing once rather than per-tool handlers.
- **Over-deny audit follow-ups — RESOLVED (2026-07).**
  - `terraform`: already fully covered (verified) — `plan`/`validate`/`show`/`fmt`/`output`/`state list`
    /`version` allow, `init`/`apply`/`destroy`/`import` deny. The old "not covered at all" note was stale.
  - `fd -x`/`--exec` / `-X`/`--exec-batch`: NOW delegates to the inner command like `find -exec`
    (`handler = "fd"`, `src/handlers/fd.rs`), bound to each search path (deny-absorbing); the no-`{}`
    batch form appends the match so `fd /etc -X cat` can't leak. Proptest `fd_exec_follows_the_inner_
    command_locus` guards the class.
  - Judgment calls MADE (keep denying — opaque/network-sourced code, the `./bill` line): `pnpm install`
    (postinstall) and `python3 -m <module>` deny. `npm run` already allowlists safe scripts via
    `first_arg` (`run test` allows, `run build` denies) — no change needed.
- **Harness verification grid — see the scorecard at the top of HARNESS-BEHAVIORS.md (source of truth).**
  Verified live: Codex, Antigravity `agy` (supersedes retired Gemini), Claude, Cursor, Copilot
  (v1.0.71, allow+deny both honored). Assumed (Claude-mirror): Qwen, Droid. opencode is static-config
  (no runtime hook).
- **Cursor target — DECIDED (2026-07): Deny harness.** cursor-agent v2026.07.16 honors a hook `deny`
  (blocks + shows our message) but IGNORES `allow` (a known cursor bug — forum.cursor.com/t/…/144244,
  allowlist wins). So `src/targets/cursor.rs` now emits `deny` for gated commands (protective, like
  Codex) and keeps `allow` for safe (inert until cursor honors it). REVISIT if the bug is fixed → switch
  back to allow-for-safe + Defer. Trade-off: a Deny harness hard-blocks every not-allowlisted command
  (escape = `~/.config/safe-chains.toml` grant), stricter than the prior abstain. See HARNESS-BEHAVIORS
  §Cursor.
- **opencode — DROPPED `--opencode-config` (decided 2026-07).** It rendered an empty allowlist (the
  `all_opencode_patterns()` stub) — misleading — so the flag, stub, and renderer were removed;
  `OpenCodeTarget` stays for detection with a "no usable hook yet" message. opencode has no runtime hook
  (plugin hook broken, opencode #7006) and a static glob can't express per-arg safety, so there is no
  meaningful integration to ship. **WATCH-LIST (revisit when upstream changes):** opencode #7006 (a real
  runtime hook) → then wire a per-command opencode target. Also Cursor forum 144244 (the ignored-`allow`
  bug) → when fixed, flip `targets/cursor.rs` back to allow-for-safe + Defer (see §Cursor).
- **`.safe-chains.toml` protected config location — WON'T-FIX before 1.0 (decided 2026-07).** Most
  harnesses do not expose a protected location, so there's nothing to implement. Best-effort holds: the
  command classifier denies every *command* write to the trust root (guarded); a non-command write
  (editor/`python -c`) escaping it is an accepted residual, out of scope for a string classifier.
- **cargo-fuzz — DONE (2026-07).** `fuzz/` standalone-workspace crate, `parse` target over
  `is_safe_command`, seed corpus, nightly Linux CI (`.github/workflows/fuzz.yml`). Verified live:
  builds under nightly + cargo-fuzz 0.13.2, 416k runs/26s clean. Run with `cargo +nightly fuzz run
  parse`. Follow-ups: pin a dated nightly for CI reproducibility; add a `command_verdict` target.

---

## Post-1.0 (deferred, not blocking 1.0)

- **ANSI-C quoting (`$'…'`) is unmodeled, so every use of it denies.** Found 2026-07-28 by a
  differential sweep against `bash -n`. The parser has no `$'…'` token: simple cases survive by
  accident (the `$` is dropped and `'…'` parses as an ordinary single-quoted string), but the
  quoting rules differ, so `echo $'don\'t'` fails to parse outright — in `$'…'` a `\'` is an
  ESCAPED quote, while in `'…'` a `'` always closes. Uniformly fail-closed today: even
  `cat $'README.md'` denies, so nothing is mis-approved and there is no hurry.

  The cost is over-denial of real idioms — `sort -t$'\t'`, `IFS=$'\n'`, `grep $'\t'`.

  Deliberately NOT fixed inline with the heredoc work, because decoding `$'…'` means decoding
  escapes (`\x2f`, `\057`, `\n`), and that is a new PATH-NORMALIZATION surface: `cat $'\x2f\x65\x74\x63/shadow'`
  must classify as `/etc/shadow`, not as an opaque literal. Adding the token without the decoding
  would turn today's uniform deny into a hole. Verified current behavior is safe: the hex and octal
  spellings of `/etc/shadow` and of `../outside.txt` all deny.

  **Done when:** `$'…'` is a real `WordPart` whose escapes are decoded before locus classification;
  `sort -t$'\t' file` and `echo $'don\'t'` approve; every escape spelling of an out-of-workspace
  path still denies, guarded by a property test over encoded/plain path pairs (the encoded form must
  never be more permissive than the literal one — the abstraction-soundness shape already used in
  `handler_property_tests`).

- **Em-dash sweep of command descriptions.** The hand-written guide docs (`docs/src/*.md`) and
  `README.md` are em-dash-free (done 2026-07). The generated Command Reference still carries them: 26
  em-dashes surface in `COMMANDS.md`, sourced from the `description` field of ~560 of 1257 command
  TOMLs. Sweep them context-aware (colon / comma / period / parens per usage, not a blind `sed`), then
  regenerate `COMMANDS.md` + the book. Also add a "no em-dashes in descriptions" note to the
  description-writing guidance in `AGENTS.md` so new commands don't reintroduce them. Deferred by user
  decision — not needed for 1.0.

---

## Output-flag writes — CLOSED, with a known small tail

The ungated-output-flag WRITE class is closed (202-command sweep, ~156 flag writers gated in
pathgates.toml; ~65 verified format-only rows sit on the ratchet worklist).

REMAINING, low severity: obscure positional converters the flag guard cannot enumerate (a
dedicated last_write audit would catch more); the single-char -d/-O set, kept out of the guard
for noise with specific ones gated; sub-positional writers like `hugo new site <path>`.

## Positional last-arg writer audit — DONE, with residual sub-classes
`positional_last_arg_writers_are_gated_or_acknowledged` (src/registry/tests.rs) drove a full-registry
audit of the last-positional / in-place WRITER class (the one the flag guard can't enumerate). Gated
~40 genuine writers via pathgates.toml [roles.X]:
  - converter families (shape="last_write"): the ghostscript wrappers (dvipdf, eps2eps, pdf2dsc,
    pfbtopfa, ps2epsi, ps2pdf12/13/14, ps2pdfwr, ps2ps), libtiff (pal2rgb, ppm2tiff, rgb2ycbcr,
    thumbnail, tiff2bw, tiff2icns, tiff2rgba, tiffcrop, tiffdither, tiffmedian), Little CMS
    jpgicc/tificc (shape merged onto their -o=read profile gate), and heif-thumbnailer, wkhtmltopdf,
    usdrecord, gdbm_dump, gdbm_load, pkgbuild, productbuild.
  - in-place mutators (shape="last_write", or positional="write" for multi-file): llvm-objcopy,
    llvm-strip, wasm-strip, install_name_tool, indent, resolveLinks, PlistBuddy, initdb,
    gatherheaderdoc; nbstripout + afscexpand (multi-file → positional="write").
The ~95 remaining auto-approvers are acknowledged NON-writers on tests/fixtures/positional_writer_worklist.tsv
(compilers/linkers → -o/a.out output, readers/viewers → stdout, test runners, linters, clipboard, flag
or derived output). The discovery ratchet is a description-heuristic best-effort (fail-OPEN on wording);
the fail-CLOSED guarantee for the known writers is positional_and_output_dir_writers_gate_sensitive_paths.

RESIDUAL SUB-CLASSES — RESOLVED (2026-07). On analysis none needed a brand-new primitive; each fit an
existing one, and the value-add was the proptests + one operation-aware mechanism.
  - ar-family (`ar`/`emar`/`llvm-ar`): NOT a first_write shape — a new pathgate `handler = "ar_archive"`
    (pathgate::handlers) reads the key-letter so r/q/d/m/s WRITE the archive and t/p/x READ it, and the
    add-ops read their members. Operation-awareness matters because read and write both deny a sensitive
    locus but DIVERGE at an in-workspace protected path (`.git/config`: readable, write-denied) — so a
    plain `positional = "write"` would over-deny `ar t ./.git/x.a`.
  - derived-output (`textutil`/`cap_mkdb`/`znew`/`pl2pm`): a sibling write lands in the input's directory,
    so write-gating the input path is locus-equivalent to gating the sibling. cap_mkdb/znew/pl2pm →
    `positional = "write"`. textutil has read modes too (`-info`/`-cat`) so it uses `handler =
    "textutil_mode"` (convert/strip write, info/cat read; -output/-outputdir are write targets).
  - scaffolders (`create-*`/`degit`): FACET model — a scaffolder CREATEs INERT CODE (the template is
    inert until the user runs it) into a NAMED directory. That is local SafeWrite; the axis to control is
    the LOCUS, so gate the target dir (`positional = "write"`, or last_write for degit) to keep the write
    in the workspace. Kept SafeWrite (not candidate) per the "inert code until run" nature; the npm-install
    step runs in the now-workspace-gated dir. (If we later want the install-exec itself gated, that is an
    execution-facet decision separate from this locus gate.)
  The operation-aware `handler` mechanism is guarded by pathgate_handler_names_resolve (name ⟺ fn) and
  proptests: ar/textutil "write is never more permissive than read" + "ops classify regardless of
  modifiers", with operation_aware_read_write_divergence_is_real pinning the .git/config case.

## `dispatch_executor` skips the flag policy when a positional is present

`ExecutorKind::File` returns `execute_file_verdict(first)` INSTEAD of `check_owned(tokens, policy)`,
so for any command declaring `executor = "file"` the flag policy — `max_positional`, the
standalone/valued allowlists — goes unenforced as soon as the first positional resolves. Only
positional COUNT and later positionals are affected; an unknown flag still denies, because it stops
`first_positional` resolving. (`ExecutorKind::Project` always checks the policy; `File` not doing so
looks like an oversight rather than a distinction.)

It cannot simply start calling `check_owned`: for an interpreter every token after the script is the
SCRIPT's argv (`python3 ./task.py --flag arg`), which the command's own grammar cannot describe.
That was tried and it false-denied. The fix is either to apply the policy to the tokens up to and
including the executor positional, or to declare which commands pass trailing args through.

Impact today is limited to commands that OPEN their extra positionals. `tilt` did — `tilt ./ok.erb
/etc/evil.erb` was admitted — and now carries `[command.path_gate] positional = "exec"` instead,
which composes with the grammar rather than replacing it. The interpreters (python3/ruby/node/go)
are unaffected: their trailing tokens are argv for workspace-local code, which the execution-origin
model trusts by design. `karma` had the same gap — `karma start ./ok.conf.js /etc/evil.conf.js` was admitted — and now
carries the same `path_gate`.

The structural half is now guarded: `capped_file_executors_declare_a_path_gate` (registry/tests.rs)
fails if any command declares a File executor WITH `max_positional` but no `path_gate`, so the next
one cannot inherit the hole silently.

DONE 2026-09-01 — but NOT by the fix this section proposed, and the difference is the useful part.

**"Enforce the policy over the pre-script prefix" does not work.** It was implemented and measured:
the prefix contains exactly one positional by construction, so `max_positional = 1` always passes
and the second config is still never counted. With karma's `path_gate` commented out,
`karma start ./ok.conf.js /etc/evil.conf.js` was still ADMITTED under that change. It is a no-op for
the case it targets.

What works is the section's other option — declare which commands pass trailing args through.
`passes_argv` on a File executor says the tokens after the script are the SCRIPT's argv, so only the
prefix is checked; **it defaults to false**, so the whole invocation is governed and `max_positional`
counts every operand. Declared on `python3`, `node`, `ruby` and `go run`; absent everywhere else,
which is where the enforcement comes from.

The payoff is that the compensating gates can go. `karma`'s `positional = "exec"` existed only
because of this gap — with the grammar enforced, at most one positional is possible and the executor
already locus-gates it — so it is removed and the two-config form still denies. Its FLAG roles stay:
`--format-error` names a JS module karma requires, and nothing else gates that.

Note `tilt` is not the witness this section assumed. It never used `fallback.executor` at all — it
deliberately chose a `path_gate` instead, for this very reason — so it does not exercise the dispatch
path and is unchanged. `karma start` (a File executor WITH `max_positional`) is the real case.

Guarded by `a_file_executor_enforces_its_grammar_unless_it_passes_argv`, red-demoed by setting
`passes_argv` on karma, which is only load-bearing because the compensating gate was removed first —
with it in place the guard passed either way and proved nothing.

## Two targets cannot self-filter on the tool — verify their envelopes

`no_target_decides_on_a_foreign_tool` requires a target to abstain when the envelope names a tool
other than its shell tool. Seven targets do. Two are exempt because their envelope, as we model it,
carries NO tool identifier:

- **cursor** — flat `{command, cwd, workspace_roots}`; nothing names the tool.
- **grok** — `{toolInput:{command}, workspaceRoot, cwd, sessionId}`; HARNESS-BEHAVIORS.md's live
  verification records no tool field.

Neither was given an invented field name. HARNESS-BEHAVIORS.md's rule is that the harness wins and
these contracts were verified live, so guessing a key is the mistake that fails silently. Both rely
on their configured matcher, which is what every target did until recently.

Risk while exempt: both are deny-harnesses, so a foreign-tool envelope can only produce an
over-deny, never a grant.

(Antigravity was on this list and should not have been. Its identifier is `toolCall.name` =
`run_command`, documented AND live-verified in HARNESS-BEHAVIORS.md — we simply were not
deserializing it. Now filtered. The lesson for the two below: check the doc before assuming the
field is absent.)

DONE when: each harness's PreToolUse envelope is checked for a tool-identifying field (drive the
TUI, dump a real envelope for a non-shell tool). Either add the field and the filter — the guard
picks it up as soon as `sample_envelope` returns `Some` — or record in HARNESS-BEHAVIORS.md that the
envelope genuinely has none.

## `--setup` silently rewrites a wrong-typed key on codex / cursor / antigravity

Four targets (claude, qwen, droid, gemini) used to PANIC when an existing settings file had e.g.
`"hooks": "a string"`; they now go through `targets::append_hook_entry`, which reports the problem
and leaves the file untouched. Codex, cursor and antigravity never panicked because they guard with
`if !hooks.is_object() { *hooks = json!({}); }` — they REPLACE the user's value instead.

Replacing is milder than crashing but is still a silent destructive edit to a file we did not
write. An unreadable value usually means a hand-edit or a schema we do not know, and the same
argument that made the other four refuse applies here.

Not folded into the panic fix on purpose: those three have shipped, tested behaviour and two of
them nest differently (antigravity puts `PreToolUse` at the top level, with no outer key), so
`append_hook_entry` does not drop straight in.

DONE when: all seven refuse rather than overwrite — either by generalizing `append_hook_entry` to
an optional outer key, or by each guarding in place — and a test asserts the pre-existing value
survives, the way `refuses_a_wrong_typed_outer_key_without_panicking` does for the shared helper.

DONE 2026-09-01, and two of the three were already fixed. codex and cursor had both been converted
to `append_hook_entry` since this was written, so their wrong-typed INNER keys were already refused
— codex even carries a doc comment describing that fix. Only antigravity still replaced, and it
genuinely cannot use the helper: its hook entry is a top-level key, so there is no outer key to
protect. It guards in place instead, which is the section's second option.

What re-reading found that this section did not name: the same silent replacement existed for a
non-object ROOT in `append_hook_entry` itself, so ALL SEVEN discarded a settings file that parsed to
`[1,2,3]` or `"a string"`. A test asserted that as intended — "a file whose ROOT is not an object
carries nothing to preserve" — which is the argument this very section rejects one level in. A root
we cannot read is not more disposable than a key we cannot read; it is less, being the whole file.
That test now asserts refusal instead.

Refusing is safe for a first-time `--setup`: every caller turns a MISSING file into an empty object
before the helper sees it, so the refusal only fires on a file that exists and parses to a
non-object. Guarded end to end through `AntigravityTarget::install` (the file survives byte-for-byte)
and over four non-object roots for the shared helper; red-demoed by restoring the replacement.

## `--suggest` appends to a `.safe-chains.toml` it cannot parse

`emit_suggestion` reads the existing file with `unwrap_or_default()`, merges the generated block in,
writes it back, reports "Added this to …", and prints a `[[trusted]]` pin. It never checks that the
existing content parses. Given a malformed file it produces a still-malformed one and tells the user
to pin it.

That used to chain into a fail-open: a pinned invalid file made `load_toml` panic on every
invocation, hook included, which lets the harness proceed. The panic half is fixed — such a file is
now skipped with a message — so the remaining damage is that `--suggest` claims success while
producing a file that will never load, and hands over a pin for it.

Same shape as the `--setup` panic already fixed: don't write into a config whose existing content we
could not read.

CODE FIX LANDED, TEST DID NOT (verified 2026-08-04). `emit_suggestion` now validates the existing
file first — `main.rs` refuses with "isn't valid TOML (…), so adding to it would leave a file
safe-chains can't load", prints the block for hand-placement, and exits 1 without writing. What is
still missing is the guard: nothing asserts the malformed file is byte-unchanged, so the behaviour
rests on the code alone and a refactor could drop it silently.

DONE when: a test asserts the malformed file is byte-unchanged, as
`refuses_a_wrong_typed_outer_key_without_panicking` does for the install path.

## Re-tokenize split words instead of refusing them

An unquoted expansion whose value holds whitespace becomes several arguments at run time. Two
dimensions were leaking and are now handled differently:

- **Paths** — `locus::classify_local` classifies each split piece and takes the worst, so
  `VAR="x /etc/shadow"; cat $VAR` denies while `VAR="-rf ./sub"; rm $VAR` keeps its real locus.
- **Flags** — `check::smuggles_a_flag` REFUSES outright, because the danger (`fd --exec rm`,
  `find -exec rm {} \;`) is a capability the grammar would have rejected, not a place a path points.

Refusing costs a false deny on a value that hides a harmless flag: `VAR="-rf ./sub"; rm $VAR` is an
ordinary worktree delete and now prompts. Pinned as an accepted trade in
`an_unquoted_expansion_is_split_into_words`.

The precise fix is to re-tokenize: `Word::expand` already turns one word into many for brace
expansion, and feeding split pieces through it would let each command's own flag grammar judge them
— no refusal, no over-deny. The blocker is that a bound value carries SEPARATE read and write
representatives for loop variables (`loop_reprs`), so tokenization would have to choose a face
before the face is known.

DONE when: `Word::expand` splits unquoted variable expansions, `VAR="-rf ./sub"; rm $VAR` is allowed
again while `VAR="--exec rm"; fd pat $VAR` still denies, and `smuggles_a_flag` is deleted rather
than left as a second gate.

## Nightly fuzz 2026-08-04: the classify budget does not cover its sibling entry points

Two nightly failures on the released v0.222.0 (`explain_render` crash, `suggest_roundtrip` OOM), one
root cause. `command_verdict` is budgeted — `ClassifyGuard::enter()` caps total work and fails
closed. The two OTHER public entry points that also walk and brace-expand a command are not.

Neither is a fail-open. Over-budget denies, so the leak can only over-deny. Verified there is no
bypass: `{,}` is a way to spell `-e` without the literal bytes, and every brace-spelled payload is
refused exactly like its plain form (`perl -{,}e system("id")` denies, as does `perl -e system("id")`).

### 1. `explain_inner` had no work budget of its own — FIXED 2026-08-04

`ClassifyGuard::enter()` resets `CLASSIFY_WORK` only on entry at depth 0 and NEVER clears it on exit.
`explain_inner` enters no guard, so it starts with whatever the previous classification left and
trips `MAX_CLASSIFY_WORK`. Same input, same thread, only the call order differs:

    explain first    explain=true   is_safe_command=true    agree
    enforce first    explain=FALSE  is_safe_command=true    DIVERGE

Minimized from the 244-byte artifact to 37 printable chars (libFuzzer `tmin` stalled at 130; a
delta-debugger in STRING space against the real predicate got the rest):

    perl {,} -{,}e{,}{,}{,}\~{,}{,}{,}{,}

Impact is transparency, and it is not cosmetic: the hook auto-approves the command while injecting
context reading "this command is not on the allowlist, so it is not auto-approved". That explanation
feeds both the human's approval decision and the model's context.

FIXED, in two parts — the first was necessary but NOT sufficient, which is worth recording:

  1. `ClassifyGuard::drop` now clears the counter when depth returns to 0, so a completed
     classification leaves nothing behind for a caller that takes no guard.
  2. That alone still left `explain` non-deterministic: it charges fan-out with no guard of its own
     while the per-segment classifications inside it reset the counter whenever one bottomed out at
     depth 0, so two consecutive `explain` calls on one dense input rendered DIFFERENT answers.
     `explain_inner` now takes a top-level `ClassifyGuard`, giving the whole explanation one budget
     and keeping every nested classification at depth >= 1 — the same shape `command_verdict` has.

Part 2 was found only because the guard's generator was extended (below); the original fuzz artifact
passed after part 1.

GUARDS, both red-demoed:
  - `explain_agrees_with_enforcement_whatever_the_call_order` — the invariant, over generated input.
  - `explain_agrees_with_enforcement_across_the_classify_budget_boundary` — a CONSTRUCTED sweep of
    both brace-group counts. The proptest alone is not enough and this was measured, not assumed:
    with the fix disabled it passed 1500 generated cases while the sweep failed on the first
    crossing. A divergence needs a command that is ALLOWED and within one call's residue of the cap,
    and random fragments almost always deny outright.

GENERATOR GAP CLOSED, and it was the real lesson: `arb_shell_fragment` emitted `{` and `}` as
separate space-joined fragments, so it could never produce a contiguous `{,}`. Brace expansion — a
real parser feature with a real cap and a real fan-out charge — was unreachable from the generator
every guard in that file is built on. It now emits brace GROUPS, including one dense enough to
approach the budget (8 groups = 256 alternatives); the space-joined form tops out at 224 against a
cap of 512 and so could never reach the boundary where a budget's interesting behaviour lives.

### 2. `is_safe_command` took 5.6 SECONDS on a 322-byte input — FIXED 2026-08-04 (brace fan-out)

CORRECTED 2026-08-04 after profiling. The first reading here was "`suggest::analyze` is unbudgeted"
(true — zero budget references in `src/suggest.rs`) and it was the WRONG diagnosis. Measured on the
release binary against the `suggest_roundtrip` artifact:

    cst::parse            8.3 us    parsed=true
    is_safe_command        5.6 s    -> false
    suggest::analyze       5.4 s    -> Generated(1 entries)

Three things follow, and the first is the one that matters:

  - The cost is in CLASSIFICATION, not parsing — parse is six orders of magnitude cheaper. That is
    the OPPOSITE of the nested-`$((` blow-up in "Three reported prompts A", where the cost was
    entirely in parse. Do not merge the two; they need different fixes.
  - It is in the MAIN path. `is_safe_command` is what the PreToolUse hook runs before every command,
    so a 322-byte line buying 5.6s is a hook-latency bug, not a `--suggest` one. `suggest` is a
    victim: it calls `command_verdict` first, then walks again.
  - `MAX_CLASSIFY_WORK` bounds the NUMBER of re-classifications (512) but not the COST of each. The
    comment on `charge_classify_work` already anticipated this shape ("512 delegations x 256 words
    is ~131k word checks — seconds of wall clock from a ~200-byte input") and charging fan-out was
    supposed to bound it. This input shows that mitigation is INSUFFICIENT, which is new information
    about a fix already believed complete.

It fails CLOSED (returns `false`), so this is latency/availability, not a bypass — but a hook slow
enough to look hung is its own failure.

PROFILED, then fixed. The method that worked was delta-debugging on ELAPSED TIME (shrink while
`is_safe_command` still takes >= 500ms), which cut 322 bytes to 165 and made the shape legible:
repeated `{…,,,,,,,…}` groups. A synthetic sweep then gave the scaling law outright —

    groups (8 alts each)   2: 38us   3: 179us   4: 1.38ms   5: 11.8ms   6: 107ms
    alts (3 groups)        4: 52us   8: 188us  16: 1.18ms  32: 8.34ms

— x8-9 per added group: time linear in words produced, words = alts^groups.

ROOT CAUSE, two defects in `brace_expand` (src/cst/eval.rs):

  1. The only guard was `s.matches('{').count() > 8` — a count of BRACES, not of words. Six groups of
     eight alternatives is six braces and 262144 words; eight groups is 16.7M, which is the ~7s that
     matched the artifact. `BRACE_EXPANSION_CAP` could not save it: the caller inspects `alts` only
     AFTER `brace_expand` has materialized the entire product.
  2. `brace_expand(&suffix)` was recomputed inside both loops although `suffix` is loop-invariant, so
     an identical expansion was redone once per alternative at every nesting level.

FIXED: the suffix expansion is hoisted, and the cap is enforced on WORDS PRODUCED inside the
innermost loop, so the product is never materialized before its size is checked. Measured after: the
whole series is FLAT at 30-63us out to 8 groups (was 38us -> 107ms), and `suggest::analyze` on the
original artifact went 5.4s -> 6.7ms. Both nightly artifacts (`suggest_roundtrip` OOM,
`explain_render` crash) now replay clean.

GUARD: `brace_expansion_stays_bounded_as_groups_are_added` sweeps 2..16 groups through
`finishes_within`. Red-demoed — with the cap disabled it fails at 8 groups from 149 bytes. The sweep
rather than a single size is deliberate: the defect is that cost grows WITH GROUP COUNT, so a guard
fixed at one count would sit at whatever margin that count happened to have.

Correctness spot-checked, since this changes an expansion everything depends on: `echo {a,b}`,
`cp file.{txt,bak}` and `mkdir -p ./src/{lib,bin}` still auto-approve, and `rm -{,}rf /` still denies.

Both artifacts are in the nightly run's `fuzz-findings-*` uploads. `equivalence` and `config_load`,
which had failed four nights running, both PASSED — those fixes held.

## `--suggest` can write OUTSIDE the worktree (nearest-ancestor walk)

Found 2026-08-04 researching why `safe-chains --suggest` prompts (it prompts correctly — see below).

`repo_config_path()` walks UP from the cwd and writes to the first `.safe-chains.toml` it finds,
falling back to creating one in the cwd only if none exists anywhere. So a `~/.safe-chains.toml`
makes every `--suggest` run from anywhere under `$HOME` rewrite THAT file — an out-of-worktree,
user-locus config write. Same command, different write target depending on where an ancestor config
happens to sit, with nothing on the command line indicating which.

Second consequence, quieter: if the repo config is already pinned and trusted, any `--suggest` run
rewrites it, changes its hash, and breaks the pin — silently disabling the user's customizations
until they re-pin. That direction is fail-CLOSED (`repo_is_trusted` requires the hash to match), so
it is a self-inflicted DoS rather than an escalation.

The escalation path is genuinely closed and was verified, not assumed: `registry::custom::
repo_is_trusted` honours a repo config only when the canonicalized parent matches a pinned
`[[trusted]] path` AND the SHA-256 matches; `user_config_level()` never consults the repo file, so a
repo `.safe-chains.toml` cannot raise the ceiling.

FIXED 2026-08-04. `repo_config_path_from(start)` now finds the project root (`.git`/`.jj`) first and
confines the search to it: the nearest existing config at or below that root, else one created AT the
root. With no project root there is no boundary to search within, so it does not search at all — it
uses the cwd rather than climbing. Split out from `repo_config_path()` so it takes the start
directory as a parameter and is testable without changing the process cwd.

GUARD: `tests/suggest_write_scope.rs::suggest_never_writes_a_config_above_the_project_root`,
red-demoed against the old walk (which wrote `…/.tmplYNqAT/.safe-chains.toml`, outside the project).

### `--suggest` stays OFF the allowlist — decided, with the reason

Our own `commands/tools/safe-chains.toml` is `level = "Inert"` and does not list `--suggest`, so it
denies by omission. That is CORRECT and should stay:

  - the write locus is not worktree-bounded (above), so it is not SafeWrite;
  - it writes safe-chains' own policy config — the `reconfigure future commands` facet, which the
    research rules separate from plain data persistence;
  - auto-approving buys NOTHING. The next step of the workflow is a human trust decision (paste the
    pin into `~/.config/safe-chains.toml`), so a prompt at the write costs the user nothing they
    were not about to do anyway, and it is the one moment worth noticing WHICH file is written.

### Our own description omits `--suggest`

`commands/tools/safe-chains.toml`'s `description` documents `--setup`/`--auto-detect`/`--tool`
("write the hook into another tool's config") and `--generate-book`, but never mentions the flag
that writes safe-chains' OWN config — the most trust-relevant write it has. The description is
meant to be the full behavioural profile.

FIXED 2026-08-04 — the description now covers `--suggest`: what it writes, that the generated file is
inert until the pin is added by hand, and that the write is confined to the project root.

Side effect worth knowing: documenting the write tripped
`positional_last_arg_writers_are_gated_or_acknowledged`, whose heuristic reads a writer-shaped
description and then probes `<cmd> in.dat ~/.ssh/authorized_keys`. safe-chains is a false positive —
its positional is the COMMAND STRING being classified, parsed as shell text and never opened, and
`--suggest`'s target comes from the cwd — so it is acknowledged in
`tests/fixtures/positional_writer_worklist.tsv` with that reasoning, which is what the guard's own
comment prescribes for false positives.

## `write_flags` declares a write but does NOT gate it — the sweep, and what it found

FOUND 2026-08-07 while measuring the denominator for the mode design. `write_flags` raises a
command's LEVEL to SafeWrite when the flag appears; it never constrains WHERE the write may land,
and nothing connected the two. So an author who correctly declares `write_flags` gets no protection
AND no warning — worse than not declaring it, because the entry reads as though the write is handled.

The tell is in the data: of the commands that WERE gated, almost none declares a `path_gate`
(`ameba`, `brakeman`, `fmt`, `gosec`, `http` — all zero). They were protected incidentally, by the
global `pathgates.toml` lists, not by anything they declared.

### The guard was widened, 62 -> 424 reported

`ambiguous_output_flags_do_not_write_sensitive_paths` already existed and missed all of this, for
three independent reasons — each worth knowing because each is a different kind of blind spot:

  - it matched a HARDCODED flag-name list (`-o`, `--output`, `--outdir`…), so `-i`, `-w`,
    `--in-place`, `--fix` were invisible — the in-place formatters and autofixers, which are the
    largest writer family in the tree;
  - it read `cmd.valued` only, never `write_flags` — i.e. never the entry's OWN declaration;
  - it walked top-level commands only, NOT subs. That is why `vegeta report --output
    ~/.ssh/authorized_keys` was missed, and it is the biggest multiplier: many holes are
    `<cmd> <sub> --output`.

It now derives the obligation from `write_flags` as well. A name list only finds flags someone
thought to write down; `write_flags` cannot drift from what the author claimed.

### The worklist is SEEDED, not acknowledged

424 reported, 362 seeded wholesale into `tests/fixtures/output_flag_worklist.tsv` under an
UNTRIAGED header. That was deliberate: it holds the line at today's exposure (nothing new can ship
ungated) instead of leaving the guard red and therefore ignored. Everything above that header was
triaged by a human; everything below carries no such claim and is a BURN-DOWN LIST.

Expect three populations in it: genuine holes; FORMAT flags where a path is a nonsense value
(`bazel query --output json`); and probe artifacts where the guard's fixed probe shape is not a
valid invocation for that command (`xattr -w` consumes the path as an attribute NAME). The last
group cannot be fixed and should be annotated in place, as `xattr` now is.

### Burned down so far: 424 -> 377 -> 295 (agents, 2026-08-08)

Two agents took the `a–f` and `g–p` slices; a third and fourth are on `q–s` and `t–z`. Combined they
closed 82 rows. The split of outcomes is the useful number, because it says what the residue IS:
roughly a third were genuine holes to gate, and most of the rest were FORMAT flags where a path is a
nonsense value — which is what the seeding note predicted, so the seeded-not-acknowledged call holds
up.

Three live holes were found that the guard could NOT have flagged, all escalated by the agents
rather than guessed:

    dart format        rewrites its positionals IN PLACE by DEFAULT, so the dangerous form carries
                       no flag at all, and `-o write|show|json|none` selects the mode BY VALUE.
                       Needed a handler; see the mode doc — it is the acceptance test there.
    afconvert          the output is the LAST POSITIONAL, not a flag. `shape = "last_write"`.
    gomodifytags       `-file X` is a read without `-w` and a write with it. Gated write
                       UNCONDITIONALLY as the fail-closed choice, which buys a narrow over-deny:
                       a read-only `gomodifytags -file ~/other/x.go -add-tags json` now denies.
                       Folded into docs/design/command-modes.md as customer 5.

`gomodifytags` is a DIFFERENT shape from every other customer and is why the mode doc grew a
requirement: modes must be able to re-role a FLAG's value, not only positionals. `write_when` cannot
express it — that field promotes positionals only.

### q–s slice: 295 -> 244, and two findings the guard structurally cannot make

48 rows gated, 26 verified format-only, plus THREE holes that were never on the worklist — which
matters, because the worklist is generated by a flag-NAME heuristic and these show what that
heuristic is blind to:

    safety scan --save-as FORMAT PATH   the write target is the flag's SECOND value. A flag->role
                                        map cannot reach it. Closed with positional = "write",
                                        which works only because safety takes no real positional —
                                        every input is a flag (-r, --target). Verified no over-deny.
    slither --triage-database PATH      a path flag whose NAME says nothing about output.
    safety --save-html/--save-json      declared `standalone` (boolean) but upstream VALUED. The
                                        mis-arity is why the probe fell through to a positional —
                                        an arity error masquerading as a gate gap.

The last one is the general lesson: a wrong `standalone`/`valued` classification silently changes
what the path-gate walk even sees. That is the same 234-scope overlap the mode doc lists as customer
6, showing up as a security bug rather than a modelling wart.

### t–z slice: 244 -> 219, and a THIRD population nobody predicted

25 rows gated across 7 commands, 11 kept as format-only, ZERO escalations. Two adjacent defects
found while gating, both of which had made the allowlist wrong in the *other* direction:

    trace trim -o          documented by the man page but MISSING from `valued`, so the short
                           spelling was a false deny before it was a hole.
    vitepress --outDir     VitePress parses argv with minimist, which does NO kebab->camel
                           conversion. We allowed `--out-dir`, which the tool silently IGNORES,
                           and denied the `--outDir` that actually works. Both now allowed, both
                           gated.

The seeding note predicted three populations (holes, format flags, probe artifacts). There is a
FOURTH, and it is the one worth naming: **PHANTOM entries — flags and subcommands that do not exist
upstream at all.** `tilt describe -o`, `tuist {build,generate,test} --destination`, and
`wasmer {wasm2wat,wat2wasm}` are in our TOMLs but not in the tools. Three of the five kept groups in
this slice were phantoms, not enums.

Phantoms are harmless to safety (the tool rejects the spelling) but they corrupt the DENOMINATOR of
every sweep: they inflate the worklist, they make coverage look worse than it is, and each one costs
a researcher a full lookup to disprove. Worth a dedicated guard — a flag we allow that upstream does
not define is exactly the kind of drift `researched_version` exists to catch.

### Path-named flags: a RATCHET on the re-research campaign, not a second campaign

`--cache-dir` and friends are ungated capabilities at the `user` rung (`trivy fs --cache-dir ~/.ssh`
auto-approves). The sweep missed them because its population is an output-flag NAME list plus
author-declared `write_flags`, and a flag named for its PURPOSE rather than its direction is in
neither.

`a_path_named_flag_records_its_facet_profile` + `tests/fixtures/path_named_flag_facets.tsv` hold the
line: a path-named flag that auto-approves a sensitive path must be recorded, with `operation` /
`locus` / `persistence` validated against `src/engine/facet.rs` so an invented or unfilled term is a
build error. Red-demoed four ways: unlisted candidate, stale row, invented facet term, missing
evidence note.

**Do NOT schedule a campaign against the 928 rows.** Measured: they span 297 commands, 225 of which
already carry a `researched_version` — so 714 of 928 sit on commands that were already researched
under the PRE-FACET standard. Redoing those under facets IS the re-research campaign; a second
worklist would duplicate it and compete for the same attention. Rows leave this file as a side
effect of that campaign (gate the flag → row stops auto-approving → stale check forces deletion).
Its standalone value is only that the backlog cannot GROW, plus an ordering signal: a command with
rows here has evidence of an ungated path capability and is worth re-researching sooner.

The facet framing is what makes the rows worth recording at all, because it splits them into three
capabilities a write/not-write question flattens into one:

    --cache-dir, --build-path     create · user · data
    --config-file, --conf-path    CONFIGURE · reconfiguring — changes what LATER commands do, and
                                  `pathgate::Role` cannot express it. Gating these as "write" would
                                  record the wrong capability and call it closed.
    --vault-password-file         SECRET READ on the disclosure axis; already denies elsewhere.

### Adversarial review of the sub-scoped gate — TWO fail-opens in code written the same day

Both were in the mechanism added hours earlier, both invisible to the tests that "proved" it worked,
and both reachable.

**1. A flag before the sub walked past the gate.** The lookup checked `tokens[1]` only. Measured with
a temporary gate on `helm list`:

    helm list ~/.ssh/authorized_keys                  DENY
    helm --namespace foo list ~/.ssh/authorized_keys  ALLOW   <- bypass

Fixed by trying EVERY bare token as a candidate sub, applying the gate from that offset. Scanning for
"the first bare token" would NOT have worked — a valued pre-flag's value is itself bare (`foo`
above), so it would have found the wrong token. Trying all of them needs no flag-arity knowledge at
this layer and fails closed; the cost is that a positional whose text equals a sub name engages that
sub's gate, which can only add a denial.

Why the original tests missed it: `rbs` and `smbutil` both REJECT pre-sub flags at dispatch, so the
gap could not show up on either of the two commands that use the mechanism. The regression test
therefore drives the token walk directly instead of relying on a real command to expose it.

**2. A sub ALIAS evaded the gate — and that one was hiding a live hole.** A key matches the literal
token, so gating the canonical spelling leaves every alias open:

    swiftlint fix ~/.ssh/authorized_keys          DENY
    swiftlint autocorrect ~/.ssh/authorized_keys  ALLOW   <- same code path, alias spelling

`swiftlint fix` rewrites files IN PLACE, so it was an ungated in-place rewriter before this — the
alias probe is what surfaced it. Both spellings are now gated, with examples. 35 subs in the registry
declare aliases, so this was a trap laid for the next author, not a one-off.

`a_sub_scoped_gate_covers_every_spelling_of_its_sub` now fails the build when a gate names some
spellings of a sub but not all. That is a GUARD, not the deeper fix: `should_deny` still cannot
resolve a sub alias to its canonical name (the pathgate layer has no sub-name canonicalizer). The
guard makes the gap impossible to spring silently, which is the property that matters until then.

**And `swiftlint`'s own description already said "aliased autocorrect … rewrites files in place"** —
the third entry this session whose text recorded the capability while no gate existed (`rbs annotate`,
`dart format`, now this). That pattern is the argument for deriving gates FROM the description's
facts rather than trusting a separate hand-written gate to agree with them.

### A `positional` gate is not free — it gates the VALUED flags too, and that cuts both ways

Gating `git diff`'s positionals to close the `--no-index` credential read also gated every valued
flag's value, which was right for `-O <orderfile>` (a file git reads — `git diff -O /etc/shadow`
denies, and the flag's NAME would never have flagged it to the path-named heuristic) and WRONG for
`-S`/`-G`, whose values are a search string and a regex. `git diff -S /etc/passwd` — looking for a
path-shaped literal in the diff, reading nothing — started denying.

So a `positional` role on a flag-rich command owes an explicit `flags` map covering every valued
flag: the path-bearing ones by role, the rest as `ignore`. An unlisted valued flag is treated as a
path, which is fail-CLOSED but shows up as a false deny that is hard to attribute.

`a_positional_gate_declares_a_role_for_every_valued_flag_in_its_scope` +
`tests/fixtures/positional_gate_undeclared_flags.tsv` now enforce it. Writing the guard showed the
problem was never about `git diff`: **157 valued flags across the already-gated commands have no
role**, so the hand-fix was one instance of a systemic gap.

They are LATENT, not live — the gate only fires on a path-shaped value, and `--max-line-length 100`
is not one. Classifying all 157 is a research campaign of the same kind as the path-named rows and
competes for the same attention, so **do not schedule one**. The fixture is a CAP: a new positional
gate must declare its scope's valued flags the way `git diff` now does, and a row leaves by being
given a role (the stale check then forces its deletion). Red-demoed both directions — removing a
declared role fails, and a row that gains one fails until deleted.

### Known limitations of the new guards — recorded, not fixed

- **`operand_write_probe_artifacts.tsv` masks future change.** A row says "these operands are not
  file targets". If the command later starts writing its operands, the row keeps suppressing the
  finding, and the stale check cannot see it (that only fires when a row STOPS auto-approving).
- **`a_path_named_flag_records_its_facet_profile` under-selects.** It probes `<scope> <flag>
  <sensitive>`; a flag needing further operands may deny for the WRONG reason (missing positional),
  so the candidate never enters the population. Fail-open in selection, documented in the fixture.
- **The guard shipped not compiling, and that hid 142 rows.** `src/registry/tests.rs` referenced
  `TomlSub` without importing it, so `cargo test --lib` failed to BUILD — which reads as a broken
  tree, not as a failing guard, and `cargo build` stayed green throughout. With the import added the
  nested-sub walk ran for the first time and the population went 786 → 928. The lesson is the one
  `--all-targets` already teaches for clippy: a guard is only as good as the last time it actually
  executed, and "the suite didn't compile" is the failure mode that looks least like one.
- **`shfmt --write=true ./a.sh` is a NEW false deny** introduced by moving `--write` from `valued` to
  `standalone`. The move is correct — with `--write` valued, `shfmt --write ~/.ssh/config` has the
  flag swallow the path and leaves no positional for `write_when`, which is the original hole. The
  real gap is that the parser rejects `=value` on a boolean flag, which Go's flag package accepts;
  `-w=true` already denied before this change, so the class predates it.

### Still-open holes the name heuristic cannot reach (from t–z)

    trivy fs|image|sbom --cache-dir DIR    a directory trivy WRITES its vulnerability DB into
    vitepress build --cache-dir DIR        same shape
    trivy --config/--ignorefile/--template  READ paths, lower severity

`--cache-dir` is the general lesson: the sweep looks for output-SHAPED names (`-o`, `--output`,
`--out`), so a write flag named for its PURPOSE rather than its direction is invisible to it. A
second sweep keyed on `dir`/`path`/`file` suffixes would find this class.

### NEW GUARD: `a_gated_flag_is_never_declared_value_less` — and the hole it found

A gated flag CONSUMES a value, so declaring it `standalone`-only is a contradiction the registry can
check with zero research: both halves are already written down. Added because `safety --save-html`
was found by hand, one command at a time.

It failed on its first run with 8 findings, and one was a LIVE HOLE:

    shfmt -w ./a.sh ~/.ssh/config     AUTO-APPROVED — overwrites a credential file with
    shfmt -w ./a.sh /etc/hosts        formatted shell. Both now denied.

`shfmt -w` is BOOLEAN — it rewrites every file OPERAND in place. Gating it as a path-VALUED flag
made the gate eat the first operand, so the one-operand probe (`shfmt -w ~/.ssh/config`) denied and
the gate looked correct. Only the two-operand form escaped. Fixed with `write_when` (not
`positional = "write"` — without `-w`, shfmt prints to stdout and `-d`/`-l` only report, so
unconditional gating would deny ordinary inspection).

**The lesson is about probe shape, not about shfmt.** A single-flag-single-path probe cannot
distinguish "the gate works" from "the gate is mis-typed and ate the path." Any sweep built on that
shape shares the blind spot — and the output-flag sweep IS built on it.

The other 5 were arity bugs where behaviour was already correct (`racc -O`, `tiff2ps -O`, `tsup -d`,
`webpack -o`, `wget --directory-prefix` — all valued upstream, all wrongly ALSO in `standalone`).
`shfmt --write` was additionally mis-declared `valued` when it too is boolean.

**Scope decision:** a flag the command declares `valued` in ANY scope is exempt. `smbutil -f` is a
path on `statshares` and boolean on `view`; `bundle --path` is valued on `install` and boolean on
`info`. Those are real defects — `smbutil view -f //server` FALSELY DENIES today — but they are the
sub-scoping gap below, not this contradiction, and folding them in would make this guard unfixable
without the schema change.

### The multi-operand sweep: shfmt was the ONLY real one

`a_boolean_write_flag_gates_its_operands` generalises the shfmt hole across the tree. Population is
derived, not guessed: a flag the entry declares in `write_flags` AND declares `standalone` without
`valued` — boolean, so the OPERANDS are what gets written. Probe puts a plausible first operand
ahead of the sensitive one, the shape the one-operand sweep structurally cannot see.

12 candidates, and after checking each against upstream, **all 12 are probe artifacts** — commands
whose positionals are not file targets at all (`borg check`'s is a REPOSITORY spec; `depcruise
--init` writes `.dependency-cruiser.cjs` in cwd; `gomodifytags`/`mtree`/`ncu` are flag-driven;
`xattr -w NAME VALUE FILE` eats the probe tokens as name and value; `qlmanage -t` READS). Recorded
with reasons in `tests/fixtures/operand_write_probe_artifacts.tsv`, failing both directions.

So the answer to "did the one-operand probe shape hide a class of holes?" is: it hid exactly one,
and it is fixed. That is worth knowing precisely, because the alternative — assuming the sweep was
sound — is what let shfmt sit there. The guard now keeps the shape covered for new entries.

### DONE: path gates ARE sub-scoped now — `[roles."<cmd> <sub>"]`

Implemented as a compound KEY in the existing central map, not a schema change. `should_deny` tries
`"<cmd> <sub>"` when the second token is a bare word, applying the spec to `tokens[1..]` so the sub
name lands where the walk expects the command name. ~10 lines, no `CommandSpec` field (which would
have meant editing ~10 construction sites), no codegen change, and the data reads exactly like a
command-scoped role.

Closed both known customers in one move:

    rbs annotate ~/.ssh/authorized_keys   was AUTO-APPROVED — upstream calls annotate_file(path)
                                          per positional. Siblings only read, and there is no flag
                                          to key on: the SUBCOMMAND is the mode. Now denied, with
                                          `rbs prototype rb ./lib` / `validate` / `list` untouched.
    smbutil view -f //server              FALSELY DENIED — `-f` is a share path on statshares and a
                                          BOOLEAN on view, so the command-wide gate ate the operand.
                                          Gate moved per-sub; the false deny is gone and the four
                                          path-taking subs still deny a sensitive value.

Note `rbs`'s own description ALREADY said "annotate rewrites RBS files in place". The knowledge was
recorded and the schema could not express the gate — which is the real argument for this change, and
the same argument the mode doc makes.

**`dart_mode` CANNOT be retired — measured, not assumed.** Swapped `[roles."dart"] handler` for
`[roles."dart format"] positional = "write"` and diffed a 12-invocation matrix. Nine rows identical;
three regress:

    dart format -o show .git/config        OK  ->  DENY
    dart format -o json .git/config        OK  ->  DENY
    dart format --output=show .git/config  OK  ->  DENY

Sub-scoping removed ONE of the handler's two jobs (scope to `format`), not both. The other is
selecting the role from `-o`'s VALUE — `write` rewrites, `show`/`json`/`none` print to stdout — and
no declarative mechanism reads a flag's value. `write_when` sees only presence, and `-o` is present
in every one of those forms.

The regression is invisible outside an in-workspace protected path, which is exactly why it needed
measuring rather than reasoning: read and write both deny a sensitive locus, so `~/.ssh/...` and
`/etc/hosts` agree under either implementation. `.git/config` (readable, write-denied) is the ONLY
place the two differ, so a matrix without it would have "proved" the handler retirable.

Retiring it needs the flag-with-value predicate from docs/design/command-modes.md. `dart format`
stays that document's acceptance test, and this is the measurement backing it.

### Superseded — kept for the reasoning: path gates were not SUB-scoped

`rbs annotate` REWRITES its RBS-file positionals in place; the sibling subs read. There is no flag to
key on: the SUBCOMMAND is the mode. `[command.path_gate]` applies command-wide, so the only
expressible choices are "gate every sub" (breaks `rbs prototype`, `rbs validate`) or "gate none"
(leaves the hole). `write_when` does not help — there is no flag.

This is a DIFFERENT gap from the mode doc's customers, and worth keeping distinct: modes are about
SELECTING a variant, this is about the gate's SCOPE. `[[command.sub]]` already selects correctly;
what it cannot do is carry a path-gate payload. Fixing it is plausibly small — let a sub declare its
own `path_gate` — and it would also serve `dart format` (currently a Rust handler for exactly this
reason: the gate had to apply to one sub only).

### `qlmanage -t` renders a credential file — for the READ-side sweep

`qlmanage -t ~/.ssh/id_rsa` generates a Quick Look preview of whatever it is pointed at. That is a
disclosure READ, not a write, so it is outside the output-flag guard's remit and no gate was added.
Recorded here so the read-side/disclosure sweep has it; it is the same shape as the deferred
`exiftool`/`ls`/`stat` read-gating question.

### The guard caught its author, which is the point

`a_gated_command_proves_its_safe_form_still_works` went red on `dart` — a gate I added myself,
missing its `examples_safe`. Not an agent's mistake. A guard that only ever catches other people's
work has not been tested.

### Burned down in the first pass: 424 -> 377

    11 in-place formatters   positional = "write"    clang-format, gofmt, gofumpt, yapf, autoflake,
                                                     autopep8, cmake-format, fourmolu, ocamlformat,
                                                     ormolu, goimports
    xattr                    handler                 -w/-d/-c write, -p/-l read, bare listing is
                                                     METADATA and stays ungated (matching this
                                                     file's standing policy for ls/stat/file)
    exiftool                 handler                 write-only; the READ-gating deferral for
                                                     disclosure inspectors is untouched
    rdfind                   handler                 -deleteduplicates/-makesymlinks destroy;
                                                     -dryrun disarms
    mtree                    handler                 -r REMOVES everything the spec omits, and the
                                                     tree is a `-p` FLAG value, never a positional —
                                                     which is why every positional sweep missed it
    ncu, jupytext            handler                 --upgrade / --sync decide the role
    8 autofix linters        write_when              declarative; see below
    clang-tidy               flags                   --export-fixes writes a FLAG VALUE, so
                                                     write_when does not cover it

### NEW MECHANISM: `write_when` on a pathgates role

Six hand-written handlers in, the shape was obvious enough to name: a tool that INSPECTS its
operands by default and REWRITES them under a mode flag. `write_when = ["--fix", …]` promotes the
positionals to writes when any listed flag is present, anywhere in the token list (the flag may
follow the paths). Eight linters closed with data instead of Rust.

Deliberately NOT general: it expresses only "flag present => positionals are writes". A tool whose
mode also MOVES the path (`mtree -p`, `ncu --packageFile`) or that needs disarming on another flag
(`rdfind -dryrun`) still needs a handler.

### This is the mode design's best evidence

SIX of the eight pathgate handlers were written in one sitting, every one because read-vs-write
depends on a flag and the TOML cannot say so. `mtree` is the sharpest: same binary, same operand,
and `-r` turns inspection into deletion. The metric to watch for docs/design/command-modes.md is
therefore the HANDLER COUNT, not the worklist count — each new handler is a mode the schema could
not express. `write_when` is the first narrow slice of that concept landing declaratively, and it
removed eight commands' worth of bespoke code on its first use.

### Two guards added, both red-demoed

`pathgates_toml_parses` and `known_safe_commands_are_still_auto_approved` (a canary: `ls`, `true`,
`pwd`, `echo hi`, `git status`, `cargo build`, `grep -rn foo ./src`).

The canary is the important one, and the lesson generalises: A SECURITY TEST THAT ONLY ASSERTS
DENIALS CANNOT DISTINGUISH "correctly gated" FROM "catastrophically broken". A duplicate
`[roles."x"]` table key makes pathgates.toml unparseable, the loader panics, and EVERY command
denies — which from outside looks like a flawless gate. That happened THREE times this session and
was caught each time only because the in-workspace control also denied. The canary makes that
instinct permanent.

### Differential regression check — clean, and the harness was proven first

1391 registry-example invocations (every `examples_safe`/`examples_denied` in the tree) run through
the pre-change binary and the current one: ZERO changed verdicts. Before believing that, the harness
was shown to detect BOTH directions — `A->D` on `clang-format -i /etc/hosts`, and `D->A` on the same
command with the binaries swapped — because a differential that cannot see a change reports a clean
run for a broken build.

What it does NOT prove: the corpus is the registry's own examples, which are in-workspace forms, so
it shows no FALSE DENIES on documented usage. It does not exercise the gates firing; that is what
the per-command safe-twin controls did.

RE-RUN 2026-08-08 after the adversarial review, because the first result predated the `mtree_mode`
fix and the `write_when` hardening — a differential is only evidence about the tree that produced
it. Same outcome: 1391 invocations, 0 changed verdicts, 0 in each direction. The harness was proved
again first, and this time the positive control included the bypass the review found
(`mtree -P -p ~/.ssh -r`, A->D), so the proof covers the change actually under test rather than an
older one.

### Adversarial review of the gate batch (2026-08-08) — one live bypass, one inconsistency

**BYPASS in my own handler, fixed.** `mtree_mode` listed `-P` and `-L` as VALUED. They are BOOLEAN
(do-not-follow / follow symlinks), so the walk consumed the following `-p` as their value and never
gated the tree:

    mtree -r -p ~/.ssh        denied
    mtree -P -p ~/.ssh -r     ALLOWED   <- same destructive -r, reordered

Exactly the defect class this gate exists to catch — an arity asserted without checking it —
committed while building the gate. VALUED is now only genuinely valued flags, verified in every
ordering with in-workspace controls.

**`write_when` hardened.** It matched exactly, so `--fix=all` (a real ansible-lint spelling) would
not have promoted. It now also matches `<flag>=`, and `--fixture` still does not match `--fix`. Unit
test added and red-demoed; the integration probes all use the bare form, so an exact-match regression
would have kept them green.

**OPEN — the formatter/linter split is inconsistent.** The 11 in-place formatters use
`positional = "write"` (their READS are write-gated); the 8 autofix linters use `write_when` with
`positional = "ignore"` (reads ungated). Same family, two policies. It shows at an in-workspace
PROTECTED path, where read and write differ:

    gofmt .git/config          DENIES   (read-only invocation: a false deny)
    ansible-lint .git/config   allows

The principled fix is `positional = "read"` + `write_when` on the formatters too. NOT done here,
because `fourmolu`/`ormolu` also accept `--mode inplace` — a VALUED mode selector `write_when`
cannot express — so converting them would trade a narrow false deny for a real hole. Needs the
per-command research, and is another customer for the mode design.

**Also latent:** a spec carrying BOTH `handler` and `write_when` silently drops the `write_when`,
because a handler replaces the positional walk. That is the same trap the `handler` doc comment
already records for `flags` (which was fixed by honouring them alongside). No spec does this today;
it should either be honoured or made a build error before one does.
