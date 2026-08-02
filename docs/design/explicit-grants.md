# A grant covers what it names

## What a user expects

Someone tired of approving reads under `~/.ssh` writes this and expects to stop being asked:

```toml
[[grant]]
path = "~/.ssh"
read = true
write = true
```

Today nothing changes, and nothing says why. That is the defect. The user config is the trust root:
it is user-only, a repo file can never carry a grant, and an agent cannot write it. A grant typed
there IS the statement of intent. Asking for a second field to prove the user meant it is ceremony,
not safety, and anyone willing to add the grant would add the ceremony too.

An earlier draft of this document proposed exactly that (`acknowledge = "credential-store"`). It was
wrong. safe-chains is not in the business of making users attest that they know what they are doing.

## The rule

**A grant covers the subtree it names. Carve-outs exist to stop a grant reaching into things it did
not name.**

That is not a new idea here. It is already the rule for hidden files:

```rust
/// The part of `path` below this matcher's root ... A `~/` grant matches `~/.ssh` and
/// `~/projects`, but only the latter's remainder is dot-free.
fn remainder<'a>(&self, path: &'a str) -> &'a str
```

```rust
// A grant never widens a hidden file/dir it happened to sweep up (`~/` grant vs
// `~/.git-credentials`); grant the dotdir explicitly to reach inside it.
(!has_hidden_component(g.matcher.remainder(path))).then_some((spec, g.read, g.write))
```

`remainder` is the part of the path BELOW the grant root, so:

- grant `~/`, path `~/.ssh/id_rsa` — remainder `.ssh/id_rsa`, hidden, not widened. Correct: the
  grant named home, not the keys.
- grant `~/.ssh`, path `~/.ssh/id_rsa` — remainder `id_rsa`, dot-free, widened. Correct: the grant
  named the directory.

The secret carve-out does not follow that rule. It bails unconditionally:

```rust
fn apply_grant(path: &str, base: Role) -> Role {
    if base.reads_secret || base.pinned {
        return base; // a grant never widens a secret store or safe-chains' own config
    }
```

It never asks whether the grant named the store. Two carve-outs, one asks, one does not, and the one
that does not is the one users hit.

## The change

Replace the blanket bail with the same question the hidden rule asks: **is the shielded node at or
below the grant root, or is it strictly below it?**

- Shield node strictly BELOW the grant root — the grant swept it up. Shield wins.
- Grant root AT or INSIDE the shielded node — the grant named it. Grant wins.

Worked through:

| grant | path | shield node | outcome |
|---|---|---|---|
| `~/` | `~/.ssh/id_rsa` | `~/.ssh` | below grant root, shield wins |
| `~/.ssh` | `~/.ssh/id_rsa` | `~/.ssh` | grant root is the node, grant wins |
| `~/.ssh` | `~/.aws/credentials` | `~/.aws` | not under the grant at all, shield wins |
| `~/projects` | `~/projects/app/.ssh/key` | `.ssh` segment | below grant root, shield wins |
| `/etc` | `/etc/shadow` | `/etc/shadow` | below grant root, shield wins |
| `/etc/shadow` | `/etc/shadow` | `/etc/shadow` | grant names it, grant wins |

The fourth row is why this cannot be simplified to "remove the bail and let the hidden rule handle
it". Most credential stores are dot-directories, so the hidden rule would cover them by accident.
But `/etc/shadow`, `/root`, the macOS keychains and browser profiles are not dot-prefixed, and a
broad `/etc` grant would sweep them up with the bail gone. The comparison has to be against the
shielded node, not against dot-ness.

## What this needs from the code

`apply_grant` currently receives only the resolved `Role`, which has lost the information about
WHICH node matched. The shielded node's root has to survive the region lookup so the comparison can
be made. That is the whole implementation: thread the matched node's path out of `base_region`
alongside the role, then compare it with the grant root the same way `remainder` already compares.

No new config syntax. No new locus rung. No new field on `[[grant]]`.

## What must not change

- **`pinned` stays absolute.** safe-chains' own config write is un-grantable regardless of how
  specifically it is named. The risk there is not to the user's data but to the mechanism: an agent
  that can grant itself write access to the governing file has defeated everything else. That is a
  different kind of rule from a credential shield and it keeps its blanket bail.
- **User config only.** A repo-level `.safe-chains.toml` can never carry a grant. This is what makes
  "the user typed it" a trustworthy signal at all.
- **Broad grants still do not sweep.** `~/` does not reach `~/.ssh`, and that is the same rule, not
  an exception to it.
- **Read and write stay independent.** `read = true, write = false` on `~/.ssh` admits reads and
  keeps writes refused.

## Partial-implementation risks

The change is one line in spirit and five places in practice. Each of these is a way to ship
something that looks done and is not.

**1. The shielded node has to survive the region lookup.** `apply_grant` receives a resolved `Role`,
which has already forgotten which node matched, so the comparison cannot be made without threading
that out of `base_region`. This is not optional plumbing: without it the only implementable rule is
the blanket bail we are replacing. If this gets hard, the temptation will be to approximate with
dot-ness, which fails on `/etc/shadow` (see the table above).

**2. `base_region` has its OWN secret-first pass.** It short-circuits on `reads_secret` before
ordinary specificity so an admit node cannot outrank the shield. That shortcut and `apply_grant`
must agree on which node they mean, or a grant naming a store gets past one and not the other. Two
places deciding "which shield applies here" is the shape of the original bug.

**3. The peer/adjacent path applies the same idea separately.** A peer project's `.env`/`.git`/`.aws`
stays denied via `has_hidden_component` on a different code path. After this change, a grant naming
`../peer/.ssh` should behave exactly like one naming `~/.ssh`. If only `apply_grant` learns the new
rule, the two diverge and the difference is invisible until someone hits it.

**4. Case folding.** On macOS the shield folds, so `~/.SSH/id_rsa` matches the `.ssh` store. If the
GRANT matcher does not fold the same way, granting `~/.ssh` covers one spelling and not the other.
That is worse than not implementing it, because the user believes the directory is granted.

**5. `pinned` must not come along.** It shares the bail line with `reads_secret`. Deleting the
condition wholesale makes safe-chains' own config write grantable, which is the one thing that must
never be reachable by any user statement. The edit is to split the condition, not remove it.

A useful completeness check, cheap to write: for every carve-out kind, assert both directions — a
grant that NAMES it widens, a grant ABOVE it does not. If a carve-out has no such pair, it has not
been considered.

## Every carve-out kind, and what the rule says about each

`role_is_protective` is the authoritative list of what makes a node stricter than an ordinary
worktree. There are FOUR conditions, not the two the bail names, so "does a grant that names it
win?" has to be answered for each or the change is only partly specified.

```rust
fn role_is_protective(role: &Role) -> bool {
    role.reads_secret
        || role.pinned
        || role.write_locus > LocalLocus::Worktree
        || role.read_locus > LocalLocus::WorktreeTrusted
}
```

**1. `read_locus > worktree-trusted` is NOT a carve-out.** It is the ordinary case. The `unknown`
role is `read = machine`, and widening exactly that is what grants are for. Listing it here is the
point: it looks protective by the predicate above, but a grant naming a directory of your own files
already widens it today and must keep doing so. Do not "fix" this one.

**2. `reads_secret` — a naming grant WINS.** `credential-store`, the motivating case. `~/.ssh`
granted covers `~/.ssh/id_rsa`; `~/` granted does not.

**3. `pinned` — absolute, no grant ever.** `safe-chains-config`. The risk is to the mechanism rather
than to the user's data: an agent that can grant itself write access to the governing file has
defeated everything else. Keeps its blanket bail, and that asymmetry is deliberate rather than an
oversight to be tidied later.

**4. `write_locus > worktree` (write freezes) — a naming grant WINS, with one open question.**
This is the kind most likely to be forgotten, because the bail never mentions it. It covers several
different things:

   - `.git` write freeze (`.git/config`, `.git/hooks/pre-commit`). A hook is an installed
     executable, so this is execution persistence, not just a file write. The rule still applies:
     a user naming `.git` in their own config has said what they mean. A grant on the REPO does not
     reach it, which is the case that matters for an agent.
   - `.envrc`, same shape.
   - `package-content` (`/usr/share`, `~/.cargo/registry`) at `write = machine`. Naming it grants
     it. Note the read face is already `adjacent`, so only the write is at stake here.
   - `system-integrity` (`write = system-integrity`). This is the open question. Following the rule
     uniformly is the simplest story and is arguably harmless, since the OS refuses these writes
     anyway on a protected volume, so the grant would buy a failed syscall rather than a real
     capability. Treating it like `pinned` is also defensible. DECIDE THIS EXPLICITLY rather than
     letting the implementation pick, because whichever way it goes it should be a sentence in this
     file, not an accident of where the condition was split.

## The completeness check

For every kind above, assert BOTH directions in the same test:

- a grant that NAMES the node widens it, and
- a grant ABOVE the node does not.

A carve-out with only one of the two has not been considered. The pairing is what catches the
failure modes the risks section lists: the peer-path divergence shows up as a naming grant that
works for `~/.ssh` and not `../peer/.ssh`, and the case-folding gap shows up as one that works for
`.ssh` and not `.SSH`. Neither is visible from a single-direction test.

Two rows of that table are deliberately asymmetric and should be written as such: `pinned` has no
widening direction at all, and `read_locus > worktree-trusted` has no refusing direction. If either
ever grows its missing half, something has gone wrong.

## Copy, once this lands

`refusal-copy.md` example 5 currently points at raising the level, because that is the only lever
that works today. After this it should point at the narrow one:

```
safe-chains did not auto-approve this. It reads `~/.ssh/id_rsa`, which holds
credentials.
If you want reads there approved, grant that directory in
~/.config/safe-chains.toml:

  [[grant]]
  path = "~/.ssh"
  read = true

A grant on a parent directory will not cover it. Credential paths are only covered
by a grant that names them.
```

The last line is the part worth keeping. It explains the one behaviour that would otherwise look
arbitrary, and it does so in terms of a rule rather than an exception.

## Testing

1. A grant of `~/.ssh` admits reads of `~/.ssh/id_rsa`. This is the user-facing point of the change.
2. A grant of `~/` does NOT admit them. This is today's behaviour and the accident-prevention
   property.
3. A grant of `~/.ssh` does not admit `~/.aws/credentials`. Naming one store does not name another.
4. A grant of `/etc` does not admit `/etc/shadow`; a grant of `/etc/shadow` does. This is the
   non-dot case the hidden rule cannot cover, so it is the one most likely to be got wrong.
5. `read = true, write = false` on `~/.ssh` admits the read and refuses the write.
6. safe-chains' own config write stays refused under a grant naming it exactly.
7. A grant in a REPO-level config is ignored, with and without a credential path.

Each needs a red demo. This mechanism exists to open something normally closed, so a test passing
for the wrong reason costs more here than anywhere else in the codebase.
