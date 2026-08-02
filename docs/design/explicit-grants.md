# Explicit grants: letting a user's own statement cut through

## The defect

A user who wants safe-chains to stop asking about `~/.ssh/config` does the obvious thing and grants
the path in `~/.config/safe-chains.toml`. Nothing changes. There is no message saying why, and no
hint that another lever exists.

The lever that does work is raising the LEVEL to `local-admin` or `yolo`. That is machine-wide. So
the narrow, precise instrument silently fails and the blunt one succeeds, and a user following the
path of least resistance ends up with far more granted than they asked for. For a safety tool that
is the wrong way round.

Measured, at the default level:

```
--level developer      deny     cat ~/.ssh/id_rsa
--level local-admin    APPROVE
--level network-admin  deny
--level yolo           APPROVE
```

## Why it happens: two axes, one vocabulary

Both levers are described to users as "allowing a path". They are not the same motion.

- A GRANT re-classifies a path. `~/projects/` granted moves from `machine` to `worktree`, separately
  for read and write.
- A LEVEL raises the ceiling: how far up the locus ladder is auto-approved.

The grant moves the path down toward the ceiling. The level moves the ceiling up toward the path.
Three carve-outs refuse to be moved by a grant at all: secret stores, hidden components under a
broad grant, and safe-chains' own config write.

So for `~/.ssh/id_rsa` the grant is a no-op not because the path sits too high for the level, but
because the shield pins it at `machine` and will not move.

## What the shield is actually protecting against

Worth being precise, because it decides the fix. The shield exists so that a BROAD grant does not
sweep up credentials by accident. `~/` granted should not silently hand over every key on the
machine. That is an accident-prevention rule, and a good one.

It is not an argument against a user naming `~/.ssh` and saying "yes, that one". Those are different
acts, and the current design cannot tell them apart because a grant carries no statement of intent
beyond its path.

## Proposal: an acknowledged grant, not a new status

A new locus rung (`user-approved-read`) is the wrong shape. Adding a rung to the ladder invites the
question of where it sits relative to the others, and the answer would have to be "above everything,
conditionally", which is not a rung. What is missing is not a classification. It is a record that the
user knew what they were asking for.

```toml
[[grant]]
path = "~/.ssh"
read = true
write = false
acknowledge = "credential-store"
```

`acknowledge` names the carve-out being overridden. Without it the grant behaves exactly as today.
With it, and only for the named carve-out, the grant wins.

Four properties this has that bare exactness would not:

1. It cannot happen by accident. A copied-and-pasted path does not acquire an acknowledgement.
2. It is self-documenting. Someone reading the config later sees that a credential store was opened
   deliberately, not swept up.
3. It is auditable. `acknowledge` values are enumerable, so "what has this machine opened" is a
   grep, not an inference over path shapes.
4. It states the real cost. Reading a credential is not only a locus question. `secret = reads` also
   means the content enters the agent's context, where it can be logged or sent onward. The word
   `acknowledge` is doing honest work: the user is accepting THAT, not just widening a directory.

An alternative considered and rejected: infer intent from grant SPECIFICITY, so an exact grant of
`~/.ssh` cuts through while a prefix grant of `~/` does not. It reuses the exact-beats-prefix rule
the region matcher already has, and needs no new syntax. It was rejected because the strength of the
statement would depend on how the path happened to be written, with no way to tell a deliberate
exact grant from one that is exact by coincidence, and nothing in the file recording that a
credential store was involved.

## What must not change

- User config only. A repo-level file can never carry a grant, acknowledged or not. An agent that
  can write `.safe-chains.toml` must not be able to open a credential store.
- safe-chains' own config write stays un-grantable regardless of acknowledgement. An agent must not
  be able to grant itself write access to the file that governs it, and no user intent argument
  changes that, because the risk is not to the user's data but to the mechanism itself.
- The default stays refused. Absent `acknowledge`, behaviour is exactly as today.
- Hidden-component sweeping stays off for broad grants. `acknowledge` is per-carve-out, not a
  blanket override.

## Copy, once this exists

The refusal in `refusal-copy.md` example 5 currently points at the level, which is the only lever
that works today. With acknowledged grants it should point at the narrow one first:

```
safe-chains did not auto-approve this. It reads `~/.ssh/id_rsa`, which holds
credentials. If that was not what you meant to do, stop and check the command.
A plain path grant does not cover credential paths, so that a broad grant cannot
hand over keys by accident. To allow this one, add acknowledge = "credential-store"
to the grant for ~/.ssh in ~/.config/safe-chains.toml.
```

## Testing

1. A grant without `acknowledge` leaves a credential path refused. This is today's behaviour and
   must not regress.
2. A grant WITH `acknowledge = "credential-store"` admits the read, and only under the granted
   subtree. `~/.ssh` acknowledged does not open `~/.aws`.
3. `acknowledge` in a REPO-level config is ignored entirely, and the presence of one does not make
   the file load differently otherwise.
4. safe-chains' own config write stays refused under every acknowledgement value.
5. The refusal message offers the acknowledged-grant remedy only when that remedy would actually
   change the verdict, which is the guard already specified in `refusal-copy.md`.

Every one needs a red demo. This is a mechanism whose entire purpose is to open something normally
closed, so a test that passes for the wrong reason is worse here than anywhere else in the codebase.
