# Command substitution: bounding the value, not trusting the command

## 0. The problem

`cst/eval.rs` evaluates every `$( … )` and backtick to a single opaque sentinel,
`__SAFE_CHAINS_CMDSUB__`, which `locus::is_unpinnable` worst-cases to `machine`. Any operand
or redirect target derived from a substitution therefore denies:

```
rg -rn "after_save" $(fd -a solr_indexing.rb app/ lib/ | head -1)   # locus.local machine
grep -rn "time_for_a_boolean" $(bundle show merchants_support)/lib  # worst-cased (§0)
OUT=$(cat .out-dir); grep -iE sentry Gemfile.lock >"$OUT/raw/x.txt" # uncertain target
```

This is correct as a floor — a substitution's value is not knowable statically — but it denies a
large class of ordinary work, and the three commands above are real prompts from one week.

## 1. The trap: inner safety says nothing about the value

The obvious move is "if the inner command is allowlisted and read-only, admit the outer operand."
**This is a fail-open and must not be built.**

```
echo /etc/shadow        # allowlisted, inert, observes nothing
$(echo /etc/shadow)     # …and its VALUE names a credential file
cat $(echo /etc/shadow) # would auto-approve
```

`echo` is as safe as a command gets. `fd` is a read-only searcher. Neither fact bounds where
their output points:

```
fd -a passwd /          # read-only, and every line is an absolute machine path
pwd                     # read-only, and its value is whatever the cwd is
```

The property that matters is not *is the inner command safe to run* but *what can the inner
command's stdout name*. Call it the substitution's **output locus**. It is a different axis from
every facet the engine already carries, and it has to be researched per command like any other.

## 2. The rule

A substitution's value inherits the worst output locus its inner command can produce. A command
that has not declared an output-locus rule stays unpinnable — allowlist-shaped, fail-closed, and
identical to today's behavior.

Declared per command in `[command.output]`:

| `locus_from` | Meaning | Example |
|---|---|---|
| `operands` | Output names paths beneath the command's own path operands. Locus = worst `read_locus` over those operands; with none, the cwd. | `fd`, `rg -l`, `find`, `git ls-files` |
| `cwd` | Output names the working directory. | `pwd`, `git rev-parse --show-toplevel` |
| *(absent)* | Unpinnable. | everything else, including `echo` |

`fd x app/ lib/` → worktree. `fd x /` → machine. `fd -a x app/` → worktree, because `-a` only
makes worktree-relative output absolute; it does not widen the search root. The existing
`classify_locus` already scores those operand paths, so the rule reuses the machinery rather than
introducing a second path model.

**`echo` is deliberately absent.** Its output is its argument, so `locus_from = "operands"` would
be wrong (the argument is not a search root it descends) and any rule that admitted it would
readmit `$(echo /etc/shadow)`. A command whose output is caller-controlled text has no output
locus and never gets one.

## 3. Representation

`eval_part` emits a locus-tagged sentinel — `__SAFE_CHAINS_CMDSUB_WORKTREE__` — instead of the
opaque one when the inner command declares a rule and resolves to a bounded locus. `is_unpinnable`
returns false for a tagged sentinel; `classify_locus` maps it to the tagged rung. Untagged
substitutions keep the existing sentinel and the existing verdict, so the change is additive:
nothing that denies today starts approving unless its inner command was researched and declared.

## 4. What this does not do

- It does not make the value *known*. A worktree-tagged substitution is admitted at the worktree
  rung, not resolved to a filename; a write through it is still a worktree write, gated as one.
- It does not make an UNDECLARED command safe by association. Nesting composes only because a
  tagged sentinel classifies like the path it stands for, so `$(fd pat $(pwd))` is worktree and
  `$(fd pat $(fd d /etc))` is machine — each layer is still answered by a declaration. An
  undeclared inner (`$(fd pat $(hostname))`) leaves its root unpinnable and the whole thing
  collapses.
- It does not cover `bundle show gem` (task 8's case). That prints a path in the gem install
  root, which is genuinely outside the workspace; admitting it is a question about the
  `adjacent`/dependency-source rung, not about substitution.
