# Fuzzing — the composition invariant (a fail-open oracle)

*Status: design, 2026-07-24. Companion to the `parse` fuzz target (`fuzz/fuzz_targets/parse.rs`).*

## 1. The gap this closes

The `parse` fuzz target feeds arbitrary bytes to `is_safe_command` and asserts one thing: it never
panics or hangs. That hardens **availability** — a hostile string can't crash the hook. It says
**nothing about correctness**, because it *discards the verdict*. By construction it cannot catch a
**fail-open**: a dangerous command wrongly returned `Allowed`. Fail-open is the security-critical
failure, and it is the one no byte-level no-panic fuzzer can see.

Catching a fail-open needs an **oracle** — knowledge of the *right* answer for an arbitrary command.
The design question is where that oracle comes from.

## 2. Why not differential

The obvious move is a second, independent classifier and a diff. Rejected here:

> A *sound* reference for an allowlist of 1,200+ commands **is safe-chains re-implemented** — the
> oracle would be as hard to get right as the thing under test, and a bug shared by both hides. A
> deliberately *simpler* reference only ever proves one direction: "the trivial-safe set is
> allowed" catches over-**denial**, never fail-**open**.

There is no cheap independent oracle for this allowlist. So we don't build one.

## 3. The metamorphic oracle: safe-chains judges itself

A metamorphic test asserts a **relationship between related inputs** that must hold whatever the
right answer is — no external oracle. safe-chains' defining rule hands us one for free:

> **A compound command is safe only if every part that executes is safe.** (AGENTS.md: "the moment
> one segment isn't allowlisted the entire chain drops to a manual prompt.")

That rule is a relationship between the verdict of a whole and the verdicts of its parts, with the
classifier as its own oracle. Two directions fall out of it.

### 3.1 Forward — composition safety (the fail-open detector)

> **F.** For every execution-joining operator `⊕`: `Allowed(A ⊕ B)` ⟹ `Allowed(A)` ∧ `Allowed(B)`.

Contrapositive: **a denied segment must poison the whole chain.** Build a command that *contains* a
denied segment; if the whole ever returns `Allowed`, the composition logic **laundered** the denied
segment — a fail-open. This is exactly the bypass class the byte target can't reach and that today's
one-off guards each cover a slice of (wrapper re-validation, brace expansion, interpreter-escape).

Operators for **F** (all genuinely execute both operands, so **F** is sound — §5):

```
&&   ||   ;   |   &   \n            $(…)   `…`   (…)   { …; }
```

### 3.2 Reverse — composition completeness (the over-denial detector)

> **R.** For sequencing operators only: `Allowed(A)` ∧ `Allowed(B)` ⟹ `Allowed(A ⊕ B)`.

Catches an over-denial regression: two individually-safe commands that a chain-handling bug denies
when joined. **R is not the security direction** (over-denial is friction, not danger) and it is
**only sound for a restricted operator set** — this is the load-bearing subtlety.

Operators for **R** (pure sequencing, no output re-interpretation):

```
&&   ||   ;   &   \n
```

**R deliberately EXCLUDES** the operators where composing safe parts can *correctly* produce a
denied whole — asserting R there would false-alarm on correct behavior:

- **Substitution** `$(B)` / `` `B` `` — the operand's *output executes*. `$(echo rm -rf /)` has a
  perfectly safe leaf (`echo rm -rf /`) but the whole must be **denied**. R is *false* here, by
  design.
- **Pipe** `A | B` — B may be an interpreter (`sh`, `python`, `xargs`). `echo evil | sh` has two safe
  leaves but must be **denied**. R is *false* here.
- **Deep nesting** — a large tree of safe leaves can trip safe-chains' recursion/length budget and
  fail **closed**. That is a *correct* denial, so R is bounded to **short** trees (§4).

## 4. The target: structure-aware, clean-leaf

Raw bytes almost never form a valid multi-segment command, so this is a **structure-aware** target
driven by `arbitrary`, not a byte target. libFuzzer mutates the **tree**; the leaves are fixed.

```
CmdTree = Leaf(pool_index)
        | Node(CmdTree, Op, CmdTree)
Op      = And | Or | Semi | Pipe | Background | Newline      // forward: all
        | Subst | Backtick | Subshell | Group                // forward: all
// reverse checks consider only { And, Or, Semi, Background, Newline }
```

**Leaf pool = the registry examples, filtered.** `examples_safe` ∪ `examples_denied` from
`commands/**/*.toml` — the same corpus the seed generator uses (`gen-fuzz-corpus`). The
`examples_denied` entries are the **poison leaves**: real, known-denied commands whose laundering is
the bug F hunts. Each leaf's own verdict is computed once at startup.

**The pool is filtered to atomic-and-clean** (§5) and that filter is *load-bearing for soundness*.

**Harness (per generated tree):**

```
whole = render(tree)
if is_safe(whole):                      # F: fail-open check — ALL operators
    for leaf in tree.executed_leaves():
        assert is_safe(leaf), FAIL_OPEN(whole, leaf)
if tree.is_sequencing_only() and tree.size() <= BOUND:   # R: over-denial — sequencing, short
    if all(is_safe(leaf) for leaf in tree.leaves()):
        assert is_safe(whole), OVER_DENIAL(whole)
```

A violation of **F** is a genuine fail-open; a violation of **R** (within its scope) is a genuine
over-denial. Neither can fire on correct behavior — provided the soundness contract holds.

## 5. Soundness contract (why the target never cries wolf)

A muted target is worse than no target. Two conditions make every reported violation real:

**5.1 Leaves are atomic and clean.** A leaf must be a *single* command that cannot capture or be
captured by the operator syntax around it. Concretely, a pool candidate is admitted only if it has:
balanced quotes; no unquoted shell-control metacharacter (`#`, `;`, `&`, `|`, `` ` ``, `$(`, `<`,
`>`, `\n`, brace/paren groups); no trailing `\`. The filter runs at startup and the pool size is
logged, so a too-aggressive filter is visible rather than silent. *Why it matters:* a leaf like
`echo hi #` would comment out a following `&& <denied>`, the chain would be *correctly* `Allowed`,
and F would false-alarm. Clean leaves make "`A ⊕ B` executes both A and B as written" true.

**5.2 Operators genuinely execute their operands.** The `Op` set is exactly the constructs where
both operands run as commands. Quoting and brace-*expansion* (`{a,b}`) do **not** execute an operand
and are excluded. This is what makes F's implication true for every operator it covers.

Given 5.1 + 5.2, `Allowed(whole) ⟹ each executed leaf is Allowed` is a theorem, not a heuristic — so
any counterexample libFuzzer finds is a real laundering bug.

## 6. Known limitation — emergent danger is out of scope

F catches **denied laundered into allowed**. It does **not** catch **danger that emerges from
individually-safe parts**: `echo evil | sh` passes F because both leaves are `Allowed`, yet the whole
is dangerous. That is a *different* property ("a command feeding an interpreter is denied"), enforced
by safe-chains' interpreter/pipe handling and exercised by the byte target + examples — not by
decomposition. This target does not claim it, and the doc says so rather than implying total
coverage (per the "no silent caps" discipline).

## 7. Relationship to what exists

- **Byte `parse` target** owns *leaf-content* coverage (parser edge cases on single commands, already
  seeded from the registry). This target owns *composition* coverage. Disjoint by construction —
  leaves here are fixed and clean.
- **Existing one-off guards** (wrapper re-validation, brace-expansion, interpreter-escape corpus in
  `tests/` and `src/registry/tests.rs`) are hand-picked instances of F. This target is their
  generative generalization: same property, coverage-guided over the whole operator × example space.
- **Level-algebra proptests** (`src/engine/testgen.rs`) assert invariants on the *engine*; this
  asserts an invariant on the *whole classifier* (parse → CST → engine → handlers).

## 8. Build increments

1. **F only, sequencing operators**, examples-only leaves + the atomic-clean filter. Prove the target
   compiles, runs under cargo-fuzz, and is non-vacuous (temporarily weaken a chain check → F fires).
2. **Add the exotic operators to F** (`$()`, backticks, subshell, group). This is where real bypasses
   live; verify each renders soundly.
3. **Add R**, scoped to sequencing + `size ≤ BOUND`. Non-vacuity: temporarily deny a valid `A && B`
   → R fires.
4. **Wire into CI** as a second cargo-fuzz target (its own corpus, shares the nightly shape).

## 9. Future — the metamorphic family

F and R are two instances of "adding structure can't loosen." Same machinery extends to:

- **Wrapper monotonicity** — `xargs`/`sudo`/`env`/`timeout`/`nohup` of a denied inner stays denied.
- **Redirect gating** — any command with `> <protected-path>` appended denies.

Both are the `Allowed(whole) ⟹ Allowed(inner)` shape with a fixed wrapper/suffix instead of a binary
operator, and fold into the same target once F + R are proven.
