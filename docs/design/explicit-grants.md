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
