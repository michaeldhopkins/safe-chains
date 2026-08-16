# Behavioral taxonomy — relocation, and the locus a thing came FROM

*Refinement of v1.4 §2.1 (the act) and §2.2 (reach). Status: design, 2026-08-16. **Deferred — not
in 0.227.0.** Written up so the next person starts from the diagnosis rather than the symptom.*

## 1. The case that surfaced it

Sibling checkouts are readable and writable at `developer`, and sibling DELETE is deliberately
refused (see TODO.md, decided 2026-08-16). Then:

```
rm ../branchdiff/a            DENY   ← the decision
mv ../branchdiff/a ./b        ALLOW
mv ../branchdiff/src ./here   ALLOW  ← a whole directory
```

The first reading is "`mv` is a hole in the sibling-delete rule." **That reading is wrong**, and
worth recording as wrong because it is the tempting one: it argues from consequence (the peer
project ends up missing a directory) to classification (so treat it as a destroy). `mv` is not a
destroy. The bytes are intact, the act is reversible, and a taxonomy that calls it deletion is
lying about what happened. Flattening behaviors into their worst-sounding neighbour is the exact
failure the facet model exists to prevent.

The real finding is that the model cannot currently say what `mv ../branchdiff/src ./here` IS.

## 2. The concept exists — and is discarded at the last layer

This is not a missing idea. It is an idea that survives two layers and dies in the third.

**TOML layer** (`registry/types.rs`) — the distinction is declared:

```rust
pub(crate) enum TransferSource {
    /// cp/ln: read the source into the destination/link (no disclosure to the model).
    Observe,
    /// mv: remove the source from its old location (trivially reversible).
    Relocate,
}
```

**Locus layer** (`engine/resolve/locus.rs`) — it gets its own face, with its own resolution:

```rust
pub(crate) enum Face {
    Read,
    Write,
    /// Changes what the NAME refers to, rather than the bytes underneath it: `rm` unbinds it,
    /// `ln` points it elsewhere, `mv` takes it away.
    Rebind,
}
```

**Facet layer** (`engine/resolve/capability.rs`) — and here it is thrown away:

```rust
pub(super) fn relocates(locus: LocalLocus, scale: Scale) -> Capability {
    writes(Operation::Mutate, locus, scale, Reversibility::Trivial, PersistenceLevel::Transient,
           "mv removes the source from its old location (trivially reversible: mv back)")
}
```

`Operation::Mutate`. So **"remove this file from its directory" and "edit this file's contents" are
the same term** by the time the level algebra reads the capability. The `because` string still
tells the truth; nothing that decides anything can see it.

The information is discarded at precisely the layer where policy would use it.

## 3. Two distinct deficiencies

### 3.1 Relocation has no term of its own

`Operation` is the "what act is this" axis, and relocation is squeezed into `Mutate` for want of
somewhere better. Consequences:

- A level cannot say anything about relocation without also saying it about every in-place edit.
- The sibling decision had to attach to `destroy`, which `mv` correctly does not carry — so the
  rule expresses "don't delete from a peer checkout" and enforces it for `rm` only. That is not a
  bug in the rule; it is the rule being inexpressible.
- `Reversibility::Trivial` is doing double duty. It is true of the DATA (you can `mv` it back) and
  says nothing about the peer project's state having changed. One word, two claims.

### 3.2 A capability has one locus, so nothing can CROSS

```rust
pub struct Locus {
    pub local: LocalLocus,
    pub remote: RemoteReach,
    pub binding: RemoteBinding,
    pub provenance: Provenance,
}
```

One place. A transfer emits two independent capabilities — a mutate at the source, a create at the
destination — and **nothing links them**. The model can say "an effect landed at `worktree`" and
"an effect happened at `adjacent`", but never *these are the same bytes, and they crossed*.

So these two produce the same shape of facts, differing only in which loci incidentally appear:

```
mv a b                     rename inside one project
mv ../branchdiff/a ./b     transplant across a boundary
```

## 4. Proposal (a) — `Operation::Relocate`

Add a term beside `Mutate` and `Destroy`. `relocates()` stops lying; `TransferSource::Relocate` and
`Face::Rebind` finally reach the algebra they were computed for.

Cost is small and mostly mechanical: one `categorical_term!` entry, one builder, and the level
files gain the term wherever `mutate` currently stands in for it. Every level that admits `mutate`
today must decide whether it also admits `relocate` — **the migration is the design work**, and it
should be done as "what did we mean here", not by sed.

Care needed: `Operation` is a closed set the level TOMLs parse. Adding a term is a schema change
for `levels/*.toml` and any user-authored level, and an unknown term must keep failing closed.

This alone does NOT express crossing. It buys the ability to talk about relocation at all.

## 5. Proposal (b) — `Locus.origin`

Give the capability the place the effect came FROM:

```rust
pub struct Locus {
    pub local: LocalLocus,
    pub origin: Option<LocalLocus>,   // where these bytes were, if they moved
    ...
}
```

Crossing becomes a first-class fact, and levels can express rules that are currently unsayable:

- "data may move DOWN into the workspace, but not UP out of it"
- "a relocate whose origin is above the workspace wants a prompt, though a copy does not"
- "a rename within one locus is uninteresting at any level" — `origin == Some(local)`

### Why this is the right shape, not a bolt-on

The taxonomy **already models exactly this on the network side.** Egress is
payload × destination-trust (see `behavioral-taxonomy-exposure.md`): what is moving, and where it
is going. Data crossing a boundary is that same idea pointed at the filesystem instead of the
network. The write side simply never got the analog.

That symmetry is the argument for (b) over ad-hoc alternatives (a `crosses: bool`, a special-case
in the `mv` resolver). It also suggests `origin` may eventually want to carry more than a
`LocalLocus` — the read side pairs it with sensitivity — but a rung is enough to start.

### Cost

Bigger than (a). Every capability builder gains a field (defaulting to `None`, so most are
untouched), `transfer_profile` becomes the place that stamps it, and the level algebra needs a
comparison term for it. The `admits` implementation and the level-algebra proptests
(`engine/testgen.rs`) both grow a dimension.

## 6. What this does NOT presuppose

Getting the vocabulary right must not smuggle in a policy. Once `mv ../branchdiff/a ./b` is
expressible as *relocate, origin `adjacent`, destination `worktree`, trivially reversible*, the
right answer may well be **allow it exactly as today** — moving a file out of a peer checkout is a
reasonable thing to do in a multi-repo tree, and it is not a delete.

The point of the work is to be able to STATE the rule. It is not evidence that the current verdict
is wrong.

## 7. Open questions

- **Does `cp` want an origin too?** `cp ../sibling/a ./b` also changes custody, without removing
  anything. Probably yes for symmetry, and it costs nothing since the field is optional — but it
  makes `origin` a fact about transfers generally rather than relocations specifically.
- **What is the origin of a sweep?** `cp -r ~ ./x` already denies as an unclearable read. If
  `origin` existed, would that rule be better expressed as a crossing rule? Possibly — and if so,
  (b) subsumes some of what `sweeps_unnameable` does by hand.
- **Does `ln` fit?** It rebinds its DESTINATION rather than its source, and its target read is
  cp-by-reference. `Face::Rebind` already covers it; whether it wants `Operation::Relocate` is
  unclear — the bytes do not move.
- **Rename detection.** `origin == Some(local)` is the cheap test for "this is a rename". Is a
  rename within `adjacent` (`mv ../branchdiff/a ../branchdiff/b`, allowed today) something a level
  should be able to refuse separately? It restructures a peer project without removing anything
  from it.

## 8. Status

Deferred out of 0.227.0 deliberately. That release opened local reads and replaced a rung-based
bound with a name-based one; it is large enough, and (a) is a schema change to the level files
that wants its own release and its own migration review.

Nothing in 0.227.0 depends on this. The behaviour it describes — `mv` out of a sibling being
allowed while `rm` is refused — ships as-is and is recorded in TODO.md next to the sibling-delete
decision.
