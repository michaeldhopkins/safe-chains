# Isolation — deep-dive (spec v1.1)

Status: draft (2026-07-01). Deepens the isolation frame from
`behavioral-taxonomy-delegation.md`. Also fixes the recursion bound (§4).

Isolation is the inverse of everything else in the model: most facets record
authority a command *has*; isolation records authority a frame *removes*. It is a
**capability-reduction frame** wrapping a nested profile — and its breaches are
**capability re-grants**. That framing (reduce-by-default, re-grant explicitly) is
the object-capability ideal (see `safety-foundations.md`), which is why isolation
is where the taxonomy touches real safe-API theory.

## 1. Isolation strength (an ordered facet)

Not all sandboxes are equal. The strength ordering is itself operationally
defined — each rung is distinguished by *what a hostile nested program can still
reach*.

| strength | mechanism | what still leaks (the discriminator) |
|---|---|---|
| `none` | — | full ambient authority of the user |
| `view` | `chroot` | filesystem *view* only; **root trivially escapes**; no PID/net/IPC isolation → not a security boundary |
| `namespace` | `docker/podman run`, `systemd-run` sandboxing | separate mount/PID/net/IPC/UTS namespaces + seccomp + dropped caps; contained to container-scope unless breached |
| `userns` | rootless podman, `bwrap`, user-namespaced containers | as above **and** container-root maps to an unprivileged host UID → kernel attack surface is the main residual |
| `vm` | `qemu`, `lima`, `multipass`, gVisor (`runsc`), Kata | separate kernel; escape requires a hypervisor/kernel-emulation bug |
| `ocap` | `wasmtime`/WASI, `pledge`+`unveil` | **deny-by-default**: no filesystem/network/exec authority except what was explicitly granted |

Key facts that pin the rungs:
- **`chroot` is `view`, not isolation.** A process with root (or certain caps) can
  escape a chroot; there is no process, network, or IPC separation. Treat
  `chroot DIR CMD` as "filesystem view narrowed" but locus otherwise unreduced.
- **`ocap` is the ceiling and the ideal.** `wasmtime --dir=. prog` grants the
  guest *only* the cwd; no ambient FS, no network unless `--tcplisten`/`--dir`
  add it. `pledge("stdio rpath")` / `unveil("/etc","r")` are a process declaring
  its own least authority. These are the north star for the "good CLI design"
  database — the safe shape is *deny-by-default, grant-by-designation*.

## 2. The frame transform

For an isolation frame over nested profile *P*:

1. **Contain.** Every capability in *P* whose locus is `worktree|user|machine`
   is clamped to **sandbox-scope** (a new locus rung meaning "inside the sandbox,
   not the host"). Network is clamped to none/loopback unless the sandbox shares
   it. Authority inside is contained even if `root` (container-root ≠ host-root at
   `namespace`+ strength).
2. **Re-grant per breach flag.** Each breach flag re-adds a specific authority
   (the catalog, §3). A volume mount re-adds a host-locus capability at
   `classify_locus(hostpath)`; `--privileged` re-adds full `machine`+root; etc.
3. **Resolve to worst-case on the unknowns.** An unrecognized flag on an
   isolation runner, or an unresolved image/command, makes the frame's containment
   *unverifiable* → treat as `none` (no containment credit). Containment is only
   credited when we can see it is intact.

Rule of thumb: **containment is earned, breaches are cheap.** We only downgrade
nested locus when we can confirm the sandbox and see no breach; any doubt keeps
the nested profile's own (worst-case) loci.

## 3. Breach catalog (the research)

Each row is registry data with `because` + evidence. Grouped by runner.

### Docker / Podman `run`
| flag | re-grant | note |
|---|---|---|
| `-v HOST:CT` / `--mount` / `--volume` | locus = `classify_locus(HOST)` | `-v /:/host` → whole host FS; write-through |
| `-v /var/run/docker.sock:…` | `machine` + `root` (host) | **docker socket = host root**: the guest can spawn a `--privileged` container |
| `--privileged` | `machine`, `authority=root`, all devices, seccomp/AppArmor off | total breach |
| `--pid=host` | `control` over host processes | can signal/inspect host PIDs |
| `--network=host` | network reach = host (loopback + host services) | reach host-only services |
| `--ipc=host` / `--uts=host` | shared IPC / hostname namespaces | |
| `--cap-add=…` | specific kernel caps; `SYS_ADMIN` ≈ near-root; `NET_ADMIN`, `SYS_PTRACE`, `DAC_OVERRIDE` notable | |
| `--device=/dev/…` | host device access (`/dev/mem`, block devs) | |
| `--security-opt seccomp=unconfined` / `apparmor=unconfined` | removes syscall/LSM filtering | widens nested surface |
| `--userns=host` | disables UID remap → container-root = host-root | drops `userns` to `namespace` |
| `--user 0` (alone) | contained-root | **not** a breach by itself at `namespace`+ strength |

Latent-authority note: **membership in the `docker` group (or a reachable daemon)
is host-root-equivalent**, because any such user can `-v / --privileged`. The
taxonomy classifies the *specific invocation* (contained if no breach flags), but
the registry should carry a standing note that the *ability to run docker at all*
implies latent `machine`+root — relevant to level design.

### bubblewrap (`bwrap`) / Flatpak
| flag | re-grant |
|---|---|
| `--bind SRC DST` / `--dev-bind` | host path into sandbox (like `-v`) |
| `--share-net` | network |
| Flatpak `--filesystem=host` / `--device=all` / `--share=network` / `--talk-name=…` | portal-mediated grants of FS / devices / net / IPC |
Default bwrap/flatpak is `userns`-strength deny-by-default (closer to `ocap`).

### firejail
Setuid sandbox with profiles. `--net=none` tightens; `--noprofile` loosens to
near-`none`. Firejail's own setuid surface is a caveat (it has had escapes).

### systemd-run / systemd sandboxing
`systemd-run -p ProtectSystem=strict -p PrivateTmp=yes -p PrivateNetwork=yes CMD`
composes isolation from directives; each `-p` directive is a *tightening* grant to
model (the inverse of docker breaches — here you add safety, not holes).

### chroot
`view` strength only (§1). Filesystem view narrowed to DIR; everything else
(net, PID, authority) unreduced; root escapes. Minimal containment credit.

### VM / gVisor / Kata
`vm` strength. Breaches: shared folders (`--mount`, vagrant synced_folders),
port-forwards (net reach), clipboard/agent channels. Escape needs a
hypervisor/emulation bug — highest practical containment.

### WASI runtimes (`wasmtime`, `wasmer`) — the `ocap` exemplar
Deny-by-default. Grants are *additive and explicit*: `--dir HOST::GUEST` (fs),
`--env` (a var), `--tcplisten`/`--tcpconnect` (net). With no grants, a WASM
module has `locus=process`, `network=none`, `execution=self`. This is the shape
the "good CLI design" database points every tool toward.

## 4. Recursion bound → a compute budget (replaces "depth 3")

A fixed depth is arbitrary. Bound resolution by **work**, not depth:

- Resolution runs against a **classification budget** (units ≈ grammar nodes /
  nested re-parses consumed). A linear chain (`sudo ssh h 'sudo …'`) is cheap and
  should resolve fully; the real risk is **fan-out** (a nested command that itself
  delegates to many, or pathological/adversarial input).
- Each nested re-parse debits the budget by its size. When the budget is
  exhausted, every not-yet-resolved nested computation becomes `opaque` →
  worst-case. Result stays safe; only precision is lost.
- The budget is a single tunable ceiling (a sanity limit against runaway/DoS
  input), set high enough that all realistic chains resolve. Depth is then an
  emergent consequence of cost, not a hardcoded semantic limit.

This matches the intent: no arbitrary semantic depth, just a guard against
computation blowing up.

## 5. Feedback into the spec

- **New locus rung `sandbox-scope`** between `temp` and `worktree` (effects
  contained to a sandbox, not the host).
- **Isolation strength** is an ordered facet on delegation frames (`none` … `ocap`).
- **Containment is earned**: only credit a downgrade when the sandbox is confirmed
  and no breach/unknown flag is present; otherwise `none`.
- **Breach catalog** is registry data (per-runner flag → re-grant), anchoring the
  golden-set.
- **Compute budget** replaces the depth cap.
