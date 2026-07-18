//! The profile resolver — turning a parsed command into its behavior profile
//! (annex `behavioral-taxonomy-engine`). Runs via `engine::bridge`, which is
//! AUTHORITATIVE for every command it can resolve (`engine_verdict(tokens).unwrap_or(legacy)`
//! in `cst::check::leaf_verdict`) — there is no opt-out.
//!
//! This file holds the dispatch (`resolve`) and the per-command `resolve_*` functions;
//! the shared toolkit they build on lives in submodules: [`flags`] (the getopt-style
//! flag walker), [`locus`] (`classify_locus` — the [`LocalLocus`] ladder that refines the
//! old `is_safe_write_target` boolean, v1.4 §2.2), and [`capability`] (the builders that
//! stamp out each `Capability` with the facet pairing its operation warrants).

use super::facet::*;
use crate::parse::{Token, has_flag};

mod capability;
mod flags;
mod locus;
pub(crate) mod regions;
#[cfg(test)]
mod scenarios;

use capability::{
    breadth_scale, creates, destroys, executes, mutates, observes, overwrites, reads_content,
    reads_to_model, relocates, transfer_profile, worst, writes_export_file,
};
use flags::{walk_positionals, walk_value};
use locus::{classify_locus, read_locus, write_locus};
pub(crate) use locus::reads_secret;

/// For `for VAR in ITEMS; do …$VAR…`, the representatives to bind `$VAR` to in the body: the
/// worst-READ item and the worst-WRITE item of the list (they can differ, so a read and a
/// write of `$VAR` each get their list's worst case). `$VAR` then inherits the list's locus
/// per operation — the `find … {}`→path binding, one layer up. `None` for an empty list, which
/// leaves `$VAR` fail-closed (machine). An item coming from a command substitution / arithmetic
/// is unpinnable, so it worst-cases to machine via a `$`-carrying sentinel representative.
pub(crate) fn loop_reprs(items: &[String]) -> Option<(String, String)> {
    if items.is_empty() {
        return None;
    }
    let faced: Vec<(String, LocalLocus, LocalLocus)> = items
        .iter()
        .map(|s| {
            if s.contains("__SAFE_CHAINS_") {
                ("$loop_sub".to_string(), LocalLocus::Machine, LocalLocus::Machine)
            } else {
                (s.clone(), read_locus(s), write_locus(s))
            }
        })
        .collect();
    let read_item = faced.iter().max_by_key(|(_, r, _)| *r).map(|(s, _, _)| s.clone())?;
    let write_item = faced.iter().max_by_key(|(_, _, w)| *w).map(|(s, _, _)| s.clone())?;
    // Freeze against the CURRENT (outer) loop bindings, so an inner representative like `$d/x`
    // doesn't carry a stale outer variable into the body — nested loops compose.
    let read_repr = crate::pathctx::expand_loop(&read_item, false).into_owned();
    let write_repr = crate::pathctx::expand_loop(&write_item, true).into_owned();
    Some((read_repr, write_repr))
}

/// The verdict for READING the content of `path` — used to gate an input-redirect source
/// (`cmd < path`) by its read locus, exactly as an operand read is gated, so `cat < /etc/shadow`
/// denies like `cat /etc/shadow`. `-` / stdin never reaches here (redirects always name a file).
pub(crate) fn read_content_verdict(path: &str) -> crate::verdict::Verdict {
    let cap = reads_content(read_locus(path), Scale::Single, "reads a redirect source");
    crate::engine::bridge::project(&Profile::of(vec![cap]))
}

/// The verdict for WRITING/overwriting `path` — used to gate a legacy writer command's file
/// operand (`tee`/`shred`/`bzip2`) by its write locus, so `shred /etc/hosts` denies.
pub(crate) fn write_target_verdict(path: &str) -> crate::verdict::Verdict {
    let cap = overwrites(write_locus(path), Scale::Single, false);
    crate::engine::bridge::project(&Profile::of(vec![cap]))
}

/// The verdict for EXECUTING the code in file `path` — used to gate an interpreter/runner's
/// script operand (`bash x.sh`, `python x.py`, `node x.js`, `go run pkg/`) by its EXECUTOR
/// locus. A worktree-local script is the dev loop → admitted at `developer`; a foreign one
/// (`/tmp/x.sh`, `~/x.py`, `/usr/local/bin/x`) or an unpinnable path (`$VAR`, glob, `..`
/// beyond cwd → `machine`) denies. `CallerFile` trust (code from a named file). See
/// docs/design/behavioral-taxonomy-execution-origin.md.
pub(crate) fn execute_file_verdict(path: &str) -> crate::verdict::Verdict {
    // A GLOB executor (`bash *.sh`) names no specific file — the matched code is unknown, so
    // it cannot be pinned to a worktree executor; deny (design §6). ($VAR/../cmdsub are already
    // worst-cased by classify_locus.) A glob stays fine as a read/write OPERAND, where every
    // match is locus-gated; only as an EXECUTOR is the code it would run unknowable.
    if path.contains(['*', '?', '[']) {
        return crate::engine::bridge::project(&worst("glob executor — the code that would run is unknown (§6)"));
    }
    let cap = executes(classify_locus(path), ExecutionTrust::CallerFile, "runs code from a named file");
    crate::engine::bridge::project(&Profile::of(vec![cap]))
}

/// The verdict for running the CURRENT PROJECT's own code — an implicit-project runner
/// (`cargo run`, `dotnet run`, `swift run`) with no path operand and no redirect out of the
/// worktree. `SelfCode` @ `Worktree` → admitted at `developer`. A runner redirected out of the
/// project (`cargo run --manifest-path ~/o/Cargo.toml`) resolves that path through
/// [`execute_file_verdict`] instead. See docs/design/behavioral-taxonomy-execution-origin.md.
pub(crate) fn execute_project_verdict() -> crate::verdict::Verdict {
    let cap = executes(LocalLocus::Worktree, ExecutionTrust::SelfCode, "runs the current project's own code");
    crate::engine::bridge::project(&Profile::of(vec![cap]))
}

/// Resolve a command's leaf tokens to its behavior profile, or `None` if the command
/// has no resolver yet (the caller then worst-cases / falls back to the legacy
/// classifier — §0 fail-closed). Redirects, substitutions, and chain semantics are the
/// surrounding CST's job, not this leaf's (annex `…-engine` §1).
pub fn resolve(tokens: &[Token]) -> Option<Profile> {
    let arg0 = tokens.first()?;
    // Canonicalize the invoked token through the registry's alias map (`gcat` → `cat`) BEFORE the
    // resolver lookup: Homebrew installs GNU coreutils as g-prefixed aliases, and without this
    // they'd miss every resolver and fall through to the ungated legacy classifier (a fail-open —
    // `gtee /etc/cron.d/job`, `gcat /etc/shadow`). The `tokens` are passed through unchanged; the
    // resolver gates operands by position, not by re-reading the command name.
    let canonical = crate::registry::canonical_name(arg0.command_name());
    // `sudo`/`doas` ELEVATE the wrapped command's authority — they are a delegating wrapper, not a
    // command of their own. Resolve the inner command and lift its authority to root (or `other-user`
    // for `-u`), so the safety of `sudo X` is the safety of `X` run privileged: `sudo cat ./notes`
    // → a root READ (local-admin), `sudo rm -rf /` → the catastrophe corner (denied everywhere).
    if canonical == "openssl" {
        return resolve_openssl(arg0, tokens);
    }
    if matches!(canonical, "sudo" | "doas") {
        return resolve_privilege_wrapper(arg0, tokens);
    }
    // Phase 1: a subcommand tagged with a facet archetype (`profile = …`) classifies as that
    // archetype's static capability — the derived, self-documenting successor to `candidate = true`.
    // Checked BEFORE command-level behavior, since a subcommand tool carries no `[command.behavior]`.
    if let Some(names) = crate::registry::sub_archetypes(tokens) {
        if !trusted_command_path(arg0.as_str()) {
            return Some(worst("resolvable name invoked from a non-standard path — possible spoof (§0)"));
        }
        // One capability per archetype (the sub's profile + each present escalating flag); the level
        // algebra takes the max. Fail-closed: an unknown archetype name → a worst capability, so a
        // typo or `unclassified` can never silently pass (a proptest catches typos at test time).
        let mut caps: Vec<Capability> = names
            .iter()
            .map(|n| {
                crate::engine::archetype::archetype(n).cloned().unwrap_or_else(|| {
                    Capability::worst("subcommand/flag declares an unknown archetype (§0)")
                })
            })
            .collect();
        // Destination-trust (exposure §4): a sub tagged `network_destination` gets its send TARGET
        // classified onto the base archetype's `locus.provenance` — established remote / literal URL
        // / opaque `$VAR` — or, for a command-transport form (`ext::…`), worst-cased as RCE.
        if let Some(dest) = crate::registry::sub_destination_token(tokens) {
            match destination_provenance(dest) {
                Some(prov) => {
                    if let Some(base) = caps.first_mut() {
                        base.locus.provenance = prov;
                    }
                }
                None => {
                    return Some(worst(
                        "send target is a command transport (ext::…) — runs a local command, RCE (§4)",
                    ));
                }
            }
        }
        // A `data-export` sub with an OUTPUT-FILE flag (`db dump -f out.sql`) writes its bulk result
        // to a local file — a SECOND capability beyond the remote read, gated at the file's locus
        // (worktree write vs a system-path clobber). Absent → the export streams to stdout, so the
        // profile is the remote read alone.
        if let Some(path) = crate::registry::sub_output_path_token(tokens) {
            caps.push(writes_export_file(classify_locus(path)));
        }
        return Some(Profile::of(caps));
    }
    // A flat command whose top-level classifying flag (`[[command.flag]]`) is present resolves to
    // that flag's archetype — the flag-triggered mode of a bimodal tool: `age -d` / `sops --decrypt`
    // reveal plaintext to the model (`decrypt-read`), while the bare/encrypt form falls through to
    // ordinary resolution below. Checked after the profiled-sub walk (a sub match wins) so a
    // subcommand form (`sops decrypt`) and the flag form (`sops -d`) both classify.
    if let Some(names) = crate::registry::command_flag_archetypes(tokens) {
        if !trusted_command_path(arg0.as_str()) {
            return Some(worst("resolvable name invoked from a non-standard path — possible spoof (§0)"));
        }
        let caps: Vec<Capability> = names
            .iter()
            .map(|n| {
                crate::engine::archetype::archetype(n).cloned().unwrap_or_else(|| {
                    Capability::worst("command flag declares an unknown archetype (§0)")
                })
            })
            .collect();
        return Some(Profile::of(caps));
    }
    // Every facet-classified command declares `[command.behavior]` (the coreutils are all ported;
    // dd/tar/sed/grep declare a `hook`). No declaration → the command is unresearched for the
    // engine, so return `None` (the caller falls back to the legacy classifier).
    let spec = crate::registry::command_behavior(canonical)?;
    // A resolvable basename reached via a NON-STANDARD path (`./cat`, `/tmp/cat`, `~/bin/grep`)
    // is not necessarily the real tool — a planted binary named `cat` would be certified as safe
    // coreutils. Don't certify it; worst-case (§0). Bare names and standard bin paths are
    // trusted. (Legacy classifies purely by basename and inherits the spoof; the engine is
    // stricter here, which keeps it never-looser.)
    if !trusted_command_path(arg0.as_str()) {
        return Some(worst("resolvable name invoked from a non-standard path — possible spoof (§0)"));
    }
    Some(resolve_behavior(spec, tokens))
}

/// `sudo`/`doas`: resolve the wrapped command and ELEVATE its authority. Authority is the axis every
/// level below `local-admin` pins to `user`, so a root capability lands at `local-admin` (or `yolo`)
/// — the projection does the rest. Fail-closed: an unknown sudo option, a root shell/editor
/// (`-i`/`-s`/`-e`), or an inner command from a non-standard path worst-cases; an unresolved inner
/// returns `None` so the caller's legacy fallback denies it (never *looser* than the bare command).
fn resolve_privilege_wrapper(arg0: &Token, tokens: &[Token]) -> Option<Profile> {
    if !trusted_command_path(arg0.as_str()) {
        return Some(worst("sudo/doas invoked from a non-standard path — possible spoof (§0)"));
    }
    let mut i = 1;
    let mut run_as_other = false;
    'scan: while let Some(tok) = tokens.get(i) {
        let t = tok.as_str();
        if t == "--" {
            i += 1;
            break;
        }
        if !t.starts_with('-') || t == "-" {
            break; // the inner command starts here
        }
        if let Some(long) = t.strip_prefix("--") {
            let (name, glued_val) = match long.split_once('=') {
                Some((n, _)) => (n, true),
                None => (long, false),
            };
            match name {
                "login" | "shell" | "edit" => {
                    return Some(worst("sudo -i/-s/-e runs a root shell or editor — arbitrary code as root (§0)"));
                }
                "user" | "other-user" => {
                    run_as_other = true;
                    if !glued_val { i += 1; }
                }
                "group" | "prompt" | "close-from" | "host" | "role" | "type"
                | "command-timeout" | "chroot" | "chdir" | "preserve-env" => {
                    // `--preserve-env` is boolean OR `--preserve-env=list`; only the space form of the
                    // others consumes a value. A bare `--preserve-env` just falls through (no skip).
                    if !glued_val && name != "preserve-env" { i += 1; }
                }
                "background" | "stdin" | "non-interactive" | "reset-timestamp"
                | "remove-timestamp" | "set-home" | "askpass" | "help" | "version"
                | "validate" | "list" | "bell" => {}
                _ => return Some(worst("sudo: unrecognized option — fail-closed (§0)")),
            }
        } else {
            // A short cluster (`-EH`, `-u root`, `-uroot`). Consume char by char; a valued flag eats
            // the rest of the token as its value, or the next token if the rest is empty.
            let rest = &t[1..];
            for (idx, c) in rest.char_indices() {
                match c {
                    'i' | 's' | 'e' => {
                        return Some(worst("sudo -i/-s/-e runs a root shell or editor — arbitrary code as root (§0)"));
                    }
                    'u' | 'U' | 'g' | 'p' | 'C' | 'h' | 'r' | 't' | 'T' | 'R' | 'D' => {
                        if c == 'u' || c == 'U' { run_as_other = true; }
                        if idx + c.len_utf8() == rest.len() { i += 1; } // value is the next token
                        i += 1;
                        continue 'scan; // rest of the token was this flag's value
                    }
                    'E' | 'H' | 'k' | 'K' | 'n' | 'b' | 'A' | 'S' | 'P' | 'B' | 'v' | 'l' => {}
                    _ => return Some(worst("sudo: unrecognized option — fail-closed (§0)")),
                }
            }
        }
        i += 1;
    }
    let inner = &tokens[i..];
    if inner.is_empty() {
        return None; // `sudo` / `sudo -v` / `sudo -l` — no command to elevate; legacy decides
    }
    let elevated = if run_as_other { Authority::OtherUser } else { Authority::Root };
    let caps = resolve(inner)?
        .capabilities
        .into_iter()
        .map(|mut c| {
            c.authority = c.authority.max(elevated);
            c
        })
        .collect();
    Some(Profile::of(caps))
}

/// openssl decrypt / private-key disclosure resolver. openssl's flag grammar defeats declarative
/// flag-gating — it accepts `--opt` as an alias for `-opt` on every subcommand, `-text` dumps the
/// PRIVATE key components to stdout past `-pubout`/`-noout`, and `-out`'s VALUE can itself be stdout
/// (`-out -`, `-out /dev/stdout`) — so the disclosure-prone subs are classified here in Rust. Returns
/// `decrypt-read` (→ yolo, denied below) only when private/decrypted material reaches the MODEL
/// (stdout); returns `None` for public-key ops, to-FILE extraction, encrypt/sign, and the ~30 benign
/// subs, which fall through to openssl's declarative (allow_all) classification. Fail-closed: a spoofed
/// path worst-cases; a disclosure sub always yields a verdict rather than abstaining to the permissive
/// legacy default.
fn resolve_openssl(arg0: &Token, tokens: &[Token]) -> Option<Profile> {
    if !trusted_command_path(arg0.as_str()) {
        return Some(worst("openssl invoked from a non-standard path — possible spoof (§0)"));
    }
    let sub = tokens.get(1)?.as_str();
    let args = &tokens[2..];
    let discloses = match sub {
        // Private-key subs: private material reaches the model UNLESS the input is public (`-pubin`),
        // or it's public-key output (`-pubout`) with no `-text` side channel — and then only if the
        // (private-key) output actually goes to stdout, not a file.
        "rsa" | "pkey" | "ec" | "dsa" => {
            if openssl_flag(args, "-pubin") {
                false
            } else if openssl_flag(args, "-text") {
                true // dumps the private exponent/primes to stdout regardless of -out/-noout/-pubout
            } else if openssl_flag(args, "-pubout") {
                false // public-key PEM out, no -text
            } else {
                openssl_output_reaches_model(args)
            }
        }
        // PKCS#8 is a private-key format with no public mode; disclosed if it reaches stdout.
        "pkcs8" => openssl_flag(args, "-text") || openssl_output_reaches_model(args),
        // Unencrypted key export (`-nodes`/`-noenc`, OpenSSL 3.0 spelling); disclosed if it hits stdout.
        "pkcs12" => {
            (openssl_flag(args, "-nodes") || openssl_flag(args, "-noenc"))
                && openssl_output_reaches_model(args)
        }
        // Symmetric decrypt: plaintext to the model only when it goes to stdout.
        "enc" => openssl_flag(args, "-d") && openssl_output_reaches_model(args),
        "smime" => openssl_flag(args, "-decrypt") && openssl_output_reaches_model(args),
        "cms" => {
            (openssl_flag(args, "-decrypt") || openssl_flag(args, "-EncryptedData_decrypt"))
                && openssl_output_reaches_model(args)
        }
        _ => return None, // benign subs — openssl's declarative (allow_all) classification
    };
    if discloses {
        let cap = crate::engine::archetype::archetype("decrypt-read")
            .cloned()
            .unwrap_or_else(|| Capability::worst("decrypt-read archetype missing (§0)"));
        Some(Profile::of(vec![cap]))
    } else {
        None // public / to-file / encrypt / benign → legacy allow_all classification
    }
}

/// Whether an openssl BOOLEAN flag (`-d`, `-text`, `-pubout`) is present, accepting the `--` twin
/// openssl honors on every subcommand (`--d`, `--text`). Value flags use [`openssl_flag_value`].
fn openssl_flag(args: &[Token], flag: &str) -> bool {
    args.iter().any(|t| {
        let s = t.as_str();
        s == flag || (s.starts_with("--") && s.len() > 2 && &s[1..] == flag)
    })
}

/// Whether the sub's OUTPUT reaches the model. FAIL-CLOSED (a path string cannot be soundly matched
/// against a denylist of device spellings — the OS collapses `//dev/stdout`, `/dev/./stdout`,
/// `/dev/fd//1` to the same device, and openssl honors the LAST of duplicate `-out`s): the output
/// reaches the model UNLESS it is provably diverted to a single plain FILE. So it's model-reaching
/// when `-noout` is absent AND NOT (exactly one `-out` whose value is a plain file). `-noout`
/// suppresses the PEM output (a validate); `-text` is checked by the caller BEFORE this, since it
/// dumps to stdout past both `-noout` and `-out`.
fn openssl_output_reaches_model(args: &[Token]) -> bool {
    if openssl_flag(args, "-noout") {
        return false;
    }
    let outs = openssl_flag_values(args, "-out");
    // Diverted to disk ONLY when there is exactly one `-out` naming a plain file. No `-out` (default
    // stdout), a duplicate `-out` (last-wins — the first is untrustworthy), or a device/`-` value all
    // reach the model.
    !matches!(outs.as_slice(), [only] if out_value_is_plain_file(only))
}

/// Whether an `-out` value names a plain FILE (a safe diversion), as opposed to stdout/`-`, or a
/// device / fd / console path (`/dev/stdout`, `/dev/stderr`, `/dev/fd/1`, `/proc/self/fd/1`). Collapses
/// redundant `/`, `.`, and `..` segments first so alternate spellings can't evade. Fail-closed: `-`,
/// empty, or any `/dev/…` or `/proc/…/fd/…` path is NOT a plain file. (Symlinks are classified by their
/// literal spelling — out of scope for a static classifier, per AGENTS.md.)
fn out_value_is_plain_file(value: &str) -> bool {
    if value.is_empty() || value == "-" {
        return false;
    }
    let norm = collapse_path(value).to_ascii_lowercase();
    let device_or_fd =
        norm == "/dev" || norm.starts_with("/dev/") || (norm.starts_with("/proc/") && norm.contains("/fd/"));
    !device_or_fd
}

/// Collapse a path's redundant `/` / `.` / `..` segments (what the kernel does before opening it), so
/// `//dev/stdout`, `/dev/./stdout`, `/dev/fd//1`, `/foo/../dev/stdout` all normalize to the device
/// path. A leading `..` on a relative path is kept (can't resolve above an unknown cwd).
fn collapse_path(p: &str) -> String {
    let absolute = p.starts_with('/');
    let mut stack: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if matches!(stack.last(), Some(&s) if s != "..") {
                    stack.pop();
                } else if !absolute {
                    stack.push("..");
                }
            }
            s => stack.push(s),
        }
    }
    let joined = stack.join("/");
    if absolute { format!("/{joined}") } else { joined }
}

/// Every value of a valued openssl flag (`-out file` / `--out file` / `-out=file` / `--out=file`),
/// accepting the `--` twin — ALL occurrences, in order (openssl honors the last; the caller fails
/// closed on duplicates).
fn openssl_flag_values<'a>(args: &'a [Token], flag: &str) -> Vec<&'a str> {
    let twin = format!("-{flag}"); // `-out` → `--out`
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let s = args[i].as_str();
        if let Some(v) = s
            .strip_prefix(flag)
            .or_else(|| s.strip_prefix(twin.as_str()))
            .and_then(|r| r.strip_prefix('='))
        {
            out.push(v);
        } else if (s == flag || s == twin)
            && let Some(next) = args.get(i + 1)
        {
            out.push(next.as_str());
            i += 1;
        }
        i += 1;
    }
    out
}

/// Classify a network-destination token's PROVENANCE (exposure §4). `None` (a bare invocation) is
/// the configured default → `Established`. A command-transport form (`ext::<cmd>`) is not a
/// destination but LOCAL CODE, signalled by a `None` return so the caller worst-cases it as RCE.
fn destination_provenance(dest: Option<&str>) -> Option<Provenance> {
    let Some(tok) = dest else {
        return Some(Provenance::Established);
    };
    if tok.starts_with("ext::") {
        return None; // `git push ext::sh -c …` runs a local command — RCE, not egress
    }
    // A variable / substitution: the actual target is not in the command string, so it cannot be
    // reviewed — the fail-closed case.
    if tok.contains('$') || tok.contains('`') {
        return Some(Provenance::Opaque);
    }
    // Spelled inline: a URL scheme, an scp-style `user@host:path`, or a filesystem path. Otherwise a
    // bare word is a reference to a configured remote (established by a prior `clone`/`remote add`).
    let literal = tok.contains("://")
        || (tok.contains('@') && tok.contains(':'))
        || tok.starts_with('/')
        || tok.starts_with("./")
        || tok.starts_with("../");
    Some(if literal { Provenance::Literal } else { Provenance::Established })
}

/// The generic, declaration-driven resolver: build a `Profile` from a command's
/// `[command.behavior]` (`BehaviorSpec`) and its tokens. This is the non-legacy classification
/// path expressed in TOML — the operation + operand-role + flag grammar are data, and this one
/// function replaces a hardcoded `resolve_*`. Irreducible token logic a declaration can't
/// express is delegated to a named `hook`.
fn resolve_behavior(spec: &crate::registry::types::BehaviorSpec, tokens: &[Token]) -> Profile {
    use crate::registry::types::{BehaviorHook, PositionalRole};
    if let Some(hook) = spec.hook {
        return match hook {
            // grep's hook supplies the operand set (the irreducible token logic); the declared
            // operation + the builders supply the facets — the composition seam (§8). grep is
            // observe-only, so its operands become content reads.
            BehaviorHook::Grep => {
                let Some(g) = grep_operands(tokens) else {
                    return worst("grep: unrecognized flag or missing pattern — worst-cased (§0)");
                };
                let mut caps: Vec<Capability> = g
                    .pattern_files
                    .iter()
                    .map(|f| reads_content(read_locus(f), Scale::Single, "reads a grep -f pattern file"))
                    .collect();
                caps.extend(reads_to_model(&g.files, g.scale));
                Profile::of(caps)
            }
            // dd/tar/sed parse their own irregular operand syntax (`key=value`, dashless mode
            // bundles, a mini-language script) AND build their own multi-role profiles, so their
            // hook returns the full `Profile` — the parser and the facets are entangled with the
            // parse and stay in Rust (their DATA — flag/param sets — is small and audited).
            BehaviorHook::Dd => resolve_dd(tokens),
            BehaviorHook::Tar => resolve_tar(tokens),
            BehaviorHook::Sed => resolve_sed(tokens),
        };
    }
    // No path operands (echo): a pure stdout emitter, handled BEFORE the flag walk — echo has no
    // flag grammar (it prints any `-x` verbatim), so walking would wrongly reject it. `observe`
    // with model disclosure and no fs/net/exec; its args touch nothing.
    if matches!(spec.positionals, PositionalRole::None) {
        return match spec.operation {
            Operation::Observe => {
                let mut c = Capability::new(Operation::Observe);
                c.disclosure.audience = DisclosureAudience::LocalProcess;
                c.because = "behavior: prints its arguments to stdout; no fs/net/exec/secret".to_string();
                Profile::of(vec![c])
            }
            _ => worst("behavior: none-operand role supports only observe (§0)"),
        };
    }
    let long: Vec<&str> = spec.long.iter().map(String::as_str).collect();
    let valued_long: Vec<&str> = spec.valued_long.iter().map(String::as_str).collect();
    let Some(operands) = walk_positionals(&spec.short, &spec.valued_short, &long, &valued_long, spec.numeric_shorthand, tokens) else {
        return worst("behavior: unrecognized flag — worst-cased (§0)");
    };
    let scale = behavior_scale(spec, &operands, tokens);
    // Path-flag values (e.g. `touch -r REF`) are gated alongside the positional operands.
    let flag_caps = path_flag_caps(spec, tokens);
    match spec.positionals {
        PositionalRole::Read => {
            let mut caps = reads_to_model(&operands, scale);
            caps.extend(flag_caps);
            Profile::of(caps)
        }
        PositionalRole::Write => {
            if operands.is_empty() {
                return worst("behavior: write operation with no operand — worst-cased (§0)");
            }
            let mut caps: Vec<Capability> = operands
                .iter()
                .map(|p| match spec.operation {
                    Operation::Destroy => destroys(classify_locus(p), scale),
                    Operation::Create => creates(classify_locus(p), scale),
                    Operation::Mutate => mutates(classify_locus(p), scale, "behavior: in-place mutate"),
                    _ => Capability::worst("behavior: unsupported write operation — worst-cased (§0)"),
                })
                .collect();
            caps.extend(flag_caps);
            Profile::of(caps)
        }
        PositionalRole::Transfer => resolve_transfer(spec, operands, flag_caps, tokens),
        // None is handled above (before the flag walk); pattern-then-read routes through a hook
        // (grep). Neither reaches here, so both fail closed.
        PositionalRole::None | PositionalRole::PatternThenRead => {
            worst("behavior: operand role not resolvable without a hook (§0)")
        }
    }
}

/// The transfer arm of `resolve_behavior` (cp/mv/ln): split the operands into sources and a
/// destination (`-t`/`--target-directory` value, else the last operand), gate each at its locus
/// — a relocate source at its WRITE face — and fold in any path-flag capabilities. Fails closed
/// on a missing spec, a missing dest, or a `-t` dest with no sources.
fn resolve_transfer(
    spec: &crate::registry::types::BehaviorSpec,
    operands: Vec<&str>,
    flag_caps: Vec<Capability>,
    tokens: &[Token],
) -> Profile {
    use crate::registry::types::TransferSource;
    let Some(t) = &spec.transfer else {
        return worst("behavior: transfer role without transfer spec — worst-cased (§0)");
    };
    let (sources, dest) = if let Some(d) = walk_value(&spec.valued_short, tokens, b't', "--target-directory") {
        if operands.is_empty() {
            return worst("behavior: transfer -t with no source operand — worst-cased (§0)");
        }
        (operands, d)
    } else {
        match operands.split_last() {
            Some((last, rest)) if !rest.is_empty() => (rest.to_vec(), *last),
            _ => return worst("behavior: transfer needs a source and a destination — worst-cased (§0)"),
        }
    };
    let no_clobber = if t.clobber_flags.is_empty() {
        t.no_clobber_flags.iter().any(|f| behavior_flag_present(tokens, f))
    } else {
        // A clobber flag PRESENT means overwrite; its absence is the no-clobber default.
        !t.clobber_flags.iter().any(|f| behavior_flag_present(tokens, f))
    };
    let recursive = t.recursive_flags.iter().any(|f| behavior_flag_present(tokens, f));
    let transfer_scale = breadth_scale(&sources, recursive);
    // A relocate REMOVES its source (a write), so gate the source at its write face.
    let source_writes = matches!(t.source, TransferSource::Relocate);
    let mut prof = transfer_profile(
        &sources,
        dest,
        transfer_scale,
        source_writes,
        |loc, sc| match t.source {
            TransferSource::Observe => observes(loc, sc, "transfer reads the source at its locus"),
            TransferSource::Relocate => relocates(loc, sc),
        },
        |loc, sc| overwrites(loc, sc, no_clobber),
    );
    prof.capabilities.extend(flag_caps);
    prof
}

/// Capabilities for a command's declared PATH-FLAGS: a valued flag whose value is a path
/// (`touch -r REF` reads REF's timestamp) is gated by its role's locus, exactly like an operand
/// — so an out-of-workspace value denies. Folds the `[command.path_gate]` idea into behavior.
fn path_flag_caps(spec: &crate::registry::types::BehaviorSpec, tokens: &[Token]) -> Vec<Capability> {
    use crate::registry::types::PathRole;
    let mut caps = Vec::new();
    for pf in &spec.path_flags {
        let short = pf.short.unwrap_or(0);
        let long = pf.long.as_deref().unwrap_or("");
        if let Some(v) = walk_value(&spec.valued_short, tokens, short, long) {
            caps.push(match pf.role {
                PathRole::Read => observes(read_locus(v), Scale::Single, "behavior: a flag value is a read path"),
                PathRole::Write => mutates(write_locus(v), Scale::Single, "behavior: a flag value is a write path"),
            });
        }
    }
    caps
}

/// The `Scale` for a behavior resolution: `single` always yields one item; `breadth` widens on
/// operand count, a glob, or a declared unbounded flag (`rm -r`) via `breadth_scale`.
fn behavior_scale(
    spec: &crate::registry::types::BehaviorSpec,
    operands: &[&str],
    tokens: &[Token],
) -> Scale {
    use crate::registry::types::ScaleModel;
    match spec.scale {
        ScaleModel::Single => Scale::Single,
        ScaleModel::Breadth => {
            let recursive = spec.unbounded_flags.iter().any(|f| behavior_flag_present(tokens, f));
            breadth_scale(operands, recursive)
        }
    }
}

/// Whether a declared behavior flag (a bare token like `-r` or `--recursive`) is present,
/// via the shared `has_flag` (which handles short clustering and `--flag=value`).
fn behavior_flag_present(tokens: &[Token], flag: &str) -> bool {
    if flag.starts_with("--") {
        has_flag(tokens, None, Some(flag))
    } else {
        has_flag(tokens, Some(flag), None)
    }
}

/// A command name with no resolver and no plausible future one — the stable stand-in for
/// "unresearched" across engine tests. Using a real tool here is a trap: when `rm` gained
/// a resolver, three tests that used `rm` as their unresearched example silently broke.
/// A name that will never be a real tool can never be silently repurposed.
#[cfg(test)]
pub(crate) const UNRESOLVED_CMD: &[&str] = &["safe-chains-unresolved-sentinel"];

/// Whether `arg0` is a trusted way to invoke a standard tool: a bare name (found via
/// `$PATH`) or an absolute path under a standard system bin directory. A path elsewhere
/// (`./x`, `/tmp/x`, `~/bin/x`) may be an impostor.
fn trusted_command_path(arg0: &str) -> bool {
    const STD_BINS: &[&str] =
        &["/usr/bin/", "/bin/", "/usr/local/bin/", "/opt/homebrew/bin/", "/sbin/", "/usr/sbin/"];
    !arg0.contains('/') || STD_BINS.iter().any(|p| arg0.starts_with(p))
}

/// The classified operand set of a `grep` invocation: the positional file operands (read at
/// `scale`, empty = stdin) and the `-f`/`--file` pattern files (each read once). This is the
/// irreducible token logic a `[command.behavior]` declaration can't express — grep's
/// pattern-vs-file disambiguation, `-e`/`-f` pattern flags, and the unknown-`--token`-is-a-
/// pattern heuristic. The declared `operation` (observe) and the builders turn these operands
/// into capabilities in `resolve_behavior`'s hook arm; this function assigns no facets.
struct GrepOperands<'a> {
    files: Vec<&'a str>,
    pattern_files: Vec<&'a str>,
    scale: Scale,
}

/// Walk a `grep` command into its `GrepOperands`, or `None` to fail closed (unrecognized flag,
/// or no pattern operand). The behavior hook (`BehaviorHook::Grep`) for `commands/text/grep.toml`.
fn grep_operands(tokens: &[Token]) -> Option<GrepOperands<'_>> {
    // `-r` (or --recursive); `-R`/--dereference-recursive is not benign and worst-cases
    // in the walk below, so it needn't be detected here.
    let recursive = has_flag(tokens, Some("-r"), Some("--recursive"));
    let scale = if recursive { Scale::Unbounded } else { Scale::Single };

    let mut files = Vec::new(); // positional file operands
    let mut pattern_files = Vec::new(); // -f/--file pattern files grep reads
    let mut pattern_from_flag = false;
    let mut unknown_flag = false;
    let mut flags_done = false;
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        let next = tokens.get(i + 1).map(Token::as_str);
        if !flags_done && t == "--" {
            flags_done = true;
            i += 1;
        } else if flags_done || !t.starts_with('-') || t == "-" {
            files.push(t);
            i += 1;
        } else if t.starts_with("--") {
            if let Some(v) = t.strip_prefix("--file=") {
                pattern_from_flag = true;
                pattern_files.push(v);
                i += 1;
            } else if t == "--file" {
                pattern_from_flag = true;
                pattern_files.extend(next);
                i += 2;
            } else if t == "--regexp" {
                pattern_from_flag = true;
                i += 2;
            } else if t.starts_with("--regexp=") {
                pattern_from_flag = true;
                i += 1;
            } else if grep_long_known(t) {
                i += 1;
            } else if grep_long_dangerous(t) {
                unknown_flag = true;
                i += 1;
            } else {
                // An unrecognized `--token` is not a grep flag: it is the search PATTERN
                // (grep patterns commonly look like `-->`, `---`, `--foo`). Treat it as a
                // positional so the file operands classify the read, matching legacy.
                files.push(t);
                i += 1;
            }
        } else {
            match grep_short_cluster(t, next) {
                GrepShort::Unrecognized => {
                    unknown_flag = true;
                    i += 1;
                }
                GrepShort::Standalone => i += 1,
                GrepShort::Pattern { file, consumes_next } => {
                    pattern_files.extend(file);
                    pattern_from_flag = true;
                    i += if consumes_next { 2 } else { 1 };
                }
                GrepShort::SkipValue { consumes_next } => i += if consumes_next { 2 } else { 1 },
            }
        }
    }

    if unknown_flag {
        return None; // unrecognized flag → fail closed (§0)
    }
    if files.is_empty() {
        // No positional operand → grep has no pattern (a `-e`/`-f` pattern still needs a
        // search target). This is a usage error; the legacy classifier denies it, so the
        // engine must not be looser — fail closed (§0).
        return None;
    }

    if !pattern_from_flag {
        files.remove(0); // the first positional is the PATTERN, not a file
    }
    if recursive && files.is_empty() {
        files.push("."); // grep -r with no path searches the cwd
    }

    Some(GrepOperands { files, pattern_files, scale })
}

/// The outcome of parsing one grep short-option cluster.
enum GrepShort<'a> {
    /// An unrecognized short (e.g. `-R`, symlink-dereferencing recursive) → the caller worst-cases.
    Unrecognized,
    /// All chars benign; no value taken.
    Standalone,
    /// `-e`/`-f` supplied the pattern (so positionals are files); `-f`'s value, if any,
    /// is a pattern file grep reads.
    Pattern { file: Option<&'a str>, consumes_next: bool },
    /// `-m`/`-A`/`-B`/`-C`/`-d` — a count/action value to skip.
    SkipValue { consumes_next: bool },
}

/// Parse a grep short-option cluster (e.g. `-ifpatterns`), honoring GNU semantics that a
/// value-taking short consumes the rest of its cluster (glued) or the next token.
fn grep_short_cluster<'a>(cluster: &'a str, next: Option<&'a str>) -> GrepShort<'a> {
    // NB: `r` (recursive) is benign, but `R` (--dereference-recursive) follows symlinks
    // and can escape the classified locus, so it is NOT benign — it worst-cases. `P`
    // (PCRE, `--perl-regexp`) IS benign: GNU grep's PCRE2 does not implement Perl's
    // `(?{code})` execution, so it runs no code — it's just another regex engine like `-E`/`-F`.
    const BENIGN: &[u8] = b"ivnclLoqswxHhaIrzZEFGbUP";
    let bytes = cluster.as_bytes();
    let mut k = 1;
    while k < bytes.len() {
        // Non-ASCII bytes aren't flags and would make `cluster[k + 1..]` slice mid-char.
        if !bytes[k].is_ascii() {
            return GrepShort::Unrecognized;
        }
        let glued = &cluster[k + 1..]; // safe: bytes[k] is ASCII → k+1 is a char boundary
        let has = !glued.is_empty();
        match bytes[k] {
            b'f' => {
                let file = if has { Some(glued) } else { next };
                return GrepShort::Pattern { file, consumes_next: !has };
            }
            b'e' => return GrepShort::Pattern { file: None, consumes_next: !has },
            b'm' | b'A' | b'B' | b'C' | b'd' => return GrepShort::SkipValue { consumes_next: !has },
            b if BENIGN.contains(&b) => k += 1,
            _ => return GrepShort::Unrecognized,
        }
    }
    GrepShort::Standalone
}

/// Whether a grep long flag (its `--name`, ignoring any `=value`) is recognized-benign.
/// `--dereference-recursive` and anything unlisted are not → worst-case (§0).
fn grep_long_known(flag: &str) -> bool {
    const KNOWN: &[&str] = &[
        "--recursive", "--ignore-case", "--invert-match", // NB: --dereference-recursive
        // (symlink-following) is intentionally absent → worst-case (M2)
        "--line-number", "--count", "--files-with-matches", "--files-without-match",
        "--only-matching", "--perl-regexp", "--word-regexp", "--line-regexp", "--fixed-strings",
        "--extended-regexp", "--basic-regexp", "--with-filename", "--no-filename",
        "--quiet", "--silent", "--no-messages", "--null", "--byte-offset", "--text",
        "--color", "--colour", "--help", "--version", "--after-context", "--before-context",
        "--context", "--max-count", "--include", "--exclude", "--exclude-dir",
        "--include-dir", "--binary-files", "--devices", "--directories",
    ];
    let name = flag.split('=').next().unwrap_or(flag);
    KNOWN.contains(&name)
}

/// The long spelling of the dangerous grep short `-R`: `--dereference-recursive` (follows
/// symlinks out of the classified locus, M2). Recognized so both spellings worst-case; every
/// OTHER unrecognized `--token` is a search pattern, not a flag. (`--perl-regexp`/`-P` is NOT
/// here — PCRE2 executes no code, so it is benign, like `-E`/`-F`.)
fn grep_long_dangerous(flag: &str) -> bool {
    let name = flag.split('=').next().unwrap_or(flag);
    matches!(name, "--dereference-recursive")
}

/// `dd if=IN of=OUT bs=… …` — the operand-model breaker: `dd` takes NO getopt flags or
/// positionals, only `key=value` operands, so the shared `Flags`/`positionals` toolkit does
/// not apply and it parses its own. `if=` reads (default stdin), `of=` writes (default
/// stdout). It is still a transfer at the facet level — `dd if=~/.ssh/id_rsa of=./x` denies
/// on the input locus, `dd if=./x of=/dev/rdisk0` denies on the output locus (a raw device
/// is beneath the fs) — but the roles arrive inside `key=value`, not positional slots, which
/// is why its conservation probe is `Operands::Custom`. `bs`/`count`/`conv`/… are benign
/// transfer parameters; any other key, or a non-`key=value` operand, worst-cases (§0).
fn resolve_dd(tokens: &[Token]) -> Profile {
    const PARAMS: &[&str] = &[
        "bs", "ibs", "obs", "cbs", "count", "skip", "seek", "conv", "iflag", "oflag", "status",
    ];
    let (mut input, mut output) = (None, None);
    for t in &tokens[1..] {
        let t = t.as_str();
        if t == "--help" || t == "--version" {
            continue;
        }
        let Some((key, val)) = t.split_once('=') else {
            return worst("dd: non key=value operand — worst-cased (§0)");
        };
        match key {
            "if" => input = Some(val),
            "of" => output = Some(val),
            k if PARAMS.contains(&k) => {}
            _ => return worst("dd: unrecognized operand — worst-cased (§0)"),
        }
    }
    // dd touches exactly one input and one output — a `single` blast radius, whatever the
    // data VOLUME. The disk-wipe danger of `of=/dev/rdisk0` is carried by its device locus,
    // not by scale.
    let input_locus = input.map_or(LocalLocus::Process, read_locus);
    match output {
        // of= names a sink: read the input into it (no model disclosure) + write the sink.
        Some(of) => Profile::of(vec![
            observes(input_locus, Scale::Single, "dd reads its input (if=) into the output"),
            overwrites(classify_locus(of), Scale::Single, false),
        ]),
        // no of= → output is stdout, so the input content reaches the model (like `cat`).
        None => Profile::of(vec![reads_content(
            input_locus,
            Scale::Single,
            "dd copies its input to stdout (→ the model)",
        )]),
    }
}

/// `tar` — the flag-SYNTAX breaker: its options may be written WITHOUT a leading dash
/// (`tar czf` == `tar -czf`), so the getopt walker misreads the cluster as a positional; tar
/// parses its own. The mode letter splits the profile sharply:
///   - create/append (`c`/`r`/`u`): reads each member (source) + writes the archive (dest) —
///     a bundler, so `tar czf - ~/.ssh` denies on the member locus (golden-set).
///   - list (`t`): reads the archive, prints member names to the model.
///   - extract (`x`) and the rarer modes: extraction writes an ARCHIVE-CONTROLLED set of
///     paths that `..`-traversal can send anywhere — unknowable without opening the archive,
///     so worst-case (§0). Any value-taking option we don't model (`-C`, `-T`, …) or an
///     unknown letter also worst-cases.
fn resolve_tar(tokens: &[Token]) -> Profile {
    let mut p = TarParse::default();
    // `-C DIR` changes the directory for the members that FOLLOW it, so a member's real locus
    // is `DIR/member` — the same `find … {}`→path binding. tar applies `-C` CUMULATIVELY: each
    // `-C` chdir's relative to the already-changed directory, so consecutive `-C /  -C etc`
    // resolves to `/etc`, not `etc`. Compose relative values onto the active dir (via the same
    // `tar_bound` join, which also lets an absolute value replace and routes any `..` through
    // the unpinnable guard); stamp each positional with the accumulated dir.
    let mut dir: Option<String> = None;
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if t == "-C" || t == "--directory" {
            dir = tokens.get(i + 1).map(|d| tar_bound(dir.as_deref(), d.as_str()));
            i += 2;
            continue;
        }
        if let Some(d) = t.strip_prefix("--directory=").or_else(|| t.strip_prefix("-C").filter(|d| !d.is_empty())) {
            dir = Some(tar_bound(dir.as_deref(), d));
            i += 1;
            continue;
        }
        if let Some(long) = t.strip_prefix("--") {
            p.long_option(long);
        } else if let Some(cluster) = t.strip_prefix('-').filter(|c| !c.is_empty()) {
            p.cluster(cluster);
        } else if i == 1 {
            p.cluster(t); // dashless old-style option bundle (only the first argument)
        } else {
            p.positionals.push((dir.clone(), t));
        }
        i += 1;
    }
    p.into_profile()
}

/// A tar positional: a member/archive path with the accumulated `-C` directory active when it
/// appeared (already composed across consecutive `-C` options).
type TarPositional<'a> = (Option<String>, &'a str);

/// A tar positional borrowed for classification: (`-C` dir, path).
type TarRef<'a> = (Option<&'a str>, &'a str);

/// A tar member/archive path resolved against an active `-C` directory: `DIR/path` for a
/// relative path, or `path` unchanged when there is no `-C` or the path is absolute (an
/// absolute member ignores `-C`).
fn tar_bound(dir: Option<&str>, path: &str) -> String {
    match dir {
        Some(d) if !path.starts_with('/') && !path.starts_with('~') && !path.starts_with('-') => {
            format!("{}/{}", d.trim_end_matches('/'), path)
        }
        _ => path.to_string(),
    }
}

/// Accumulated `tar` parse: the mode, whether `-f` wants an archive, and `reject` — set by
/// any option we can't model safely (an unknown letter, or a value-taking option like `-T`
/// / `-X` whose ordered operand consumption we don't track). `-C` IS modeled (see
/// `resolve_tar`); it only reaches `cluster` inside a mixed bundle, which still worst-cases.
#[derive(Default)]
struct TarParse<'a> {
    mode: Option<u8>,
    want_archive: bool,
    reject: bool,
    long_archive: Option<&'a str>,
    /// Each positional with the `-C` directory active when it appeared (`None` = cwd).
    positionals: Vec<TarPositional<'a>>,
}

impl<'a> TarParse<'a> {
    fn cluster(&mut self, cluster: &str) {
        const NOVAL: &[u8] = b"vzjJZpkmOwhSlPa"; // benign no-value option letters
        for b in cluster.bytes() {
            match b {
                b'c' | b'x' | b't' | b'r' | b'u' | b'A' | b'd' => self.mode = Some(b),
                b'f' => self.want_archive = true,
                b'C' | b'T' | b'X' | b'b' | b'H' | b'g' | b'K' | b'N' => self.reject = true,
                x if NOVAL.contains(&x) => {}
                _ => self.reject = true,
            }
        }
    }

    fn long_option(&mut self, long: &'a str) {
        let name = long.split('=').next().unwrap_or(long);
        match name {
            "create" => self.mode = Some(b'c'),
            "extract" | "get" => self.mode = Some(b'x'),
            "list" => self.mode = Some(b't'),
            "append" => self.mode = Some(b'r'),
            "update" => self.mode = Some(b'u'),
            "file" => match long.split_once('=') {
                Some((_, v)) => self.long_archive = Some(v),
                None => self.want_archive = true,
            },
            "gzip" | "bzip2" | "xz" | "zstd" | "compress" | "verbose" | "preserve-permissions"
            | "same-permissions" | "to-stdout" | "help" | "version" | "dereference" | "totals" => {}
            _ => self.reject = true,
        }
    }

    fn into_profile(self) -> Profile {
        let Some(mode) = self.mode.filter(|_| !self.reject) else {
            return worst("tar: unrecognized/unmodeled option — worst-cased (§0)");
        };
        // Separate the archive from the members. `--file=X` names it directly; a bare `f`
        // (dashless `czf` or dashed `-czf`) takes the FIRST positional as the archive.
        let (archive, members): (Option<TarRef>, &[TarPositional]) =
            if let Some(a) = self.long_archive {
                (Some((None, a)), &self.positionals)
            } else if self.want_archive {
                match self.positionals.split_first() {
                    Some((first, rest)) => (Some((first.0.as_deref(), first.1)), rest),
                    None => return worst("tar: -f without an archive — worst-cased (§0)"),
                }
            } else {
                (None, &self.positionals) // archive is stdin/stdout
            };
        // A `-` archive (or none) is a stdout/stdin stream, not a file to gate.
        let archive_file = archive.filter(|(_, a)| *a != "-");

        match mode {
            b'c' | b'r' | b'u' => {
                let mut caps: Vec<Capability> = members
                    .iter()
                    .map(|(dir, m)| observes(read_locus(&tar_bound(dir.as_deref(), m)), Scale::Bounded, "tar reads a member into the archive"))
                    .collect();
                if let Some((dir, a)) = archive_file {
                    caps.push(overwrites(classify_locus(&tar_bound(dir, a)), Scale::Single, false));
                }
                if caps.is_empty() {
                    return worst("tar create with no members — worst-cased (§0)");
                }
                Profile::of(caps)
            }
            b't' => {
                let loc = archive_file.map_or(LocalLocus::Process, |(dir, a)| classify_locus(&tar_bound(dir, a)));
                Profile::of(vec![reads_content(loc, Scale::Single, "tar lists the archive's members (names → the model)")])
            }
            // x (extract) and A/d: archive-controlled, ..-escapable writes → worst-case.
            _ => worst("tar extract writes an archive-controlled, ..-escapable path set — worst-cased (§0)"),
        }
    }
}

/// `sed` — the read-becomes-WRITE breaker: `sed 's/…/…/' FILE` reads FILE and prints to the
/// model, but `sed -i` edits the SAME file operands **in place** (a mutate), so a single
/// flag flips the operation on the same slots. Two more wrinkles: `-i` takes an OPTIONAL
/// glued suffix (`-i.bak`) the getopt walker can't express, and — like `grep` — the first
/// positional is the SCRIPT unless `-e`/`-f` supplied it (`-f` also reads a script file).
/// So `sed` parses its own flags.
fn resolve_sed(tokens: &[Token]) -> Profile {
    // HP-7: sed is a mini-language. Its `e` command/modifier executes text as a shell command
    // (RCE), and its `w`/`W`/`r`/`R` commands write/read arbitrary files EMBEDDED in the script —
    // both invisible to flag parsing. Scan the script(s): an `e`/unknown command worst-cases; the
    // file commands' filenames get gated by locus below (a local write is fine, `/etc/cron.d/x` is
    // not), exactly like the operand files.
    let script = crate::handlers::coreutils::sed::scan_sed(tokens);
    if script.exec || script.unknown {
        return worst("sed: script has an `e` exec or unmodeled command — worst-cased (§0, HP-7)");
    }
    // A `-f`/`--file` script comes from a file we can't read — its `e`/`w`/`r` commands are invisible,
    // so we can't verify it (like `awk -f`, `bash script.sh`, mlr `--load`). Worst-case it.
    if script.script_file {
        return worst("sed: -f runs a script file we can't inspect — worst-cased (§0)");
    }
    const BOOL: &[u8] = b"nrEsuz"; // no-value short flags
    let mut in_place = false;
    let mut script_from_flag = false;
    let mut script_files: Vec<&str> = Vec::new(); // -f FILE — sed reads these
    let mut files: Vec<&str> = Vec::new();
    let mut flags_done = false;
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        let next = tokens.get(i + 1).map(Token::as_str);
        if !flags_done && t == "--" {
            flags_done = true;
            i += 1;
        } else if flags_done || t == "-" || !t.starts_with('-') {
            files.push(t);
            i += 1;
        } else if let Some(long) = t.strip_prefix("--") {
            match sed_long(long, next, &mut in_place, &mut script_from_flag, &mut script_files) {
                Some(consumed) => i += consumed,
                None => return worst("sed: unrecognized flag — worst-cased (§0)"),
            }
        } else {
            match sed_cluster(&t[1..], next, BOOL) {
                SedShort::Bad => return worst("sed: unrecognized flag — worst-cased (§0)"),
                SedShort::InPlace => {
                    in_place = true;
                    i += 1;
                }
                SedShort::Standalone => i += 1,
                SedShort::Script { consumes_next } => {
                    script_from_flag = true;
                    i += usize::from(consumes_next) + 1;
                }
                SedShort::ScriptFile { file, consumes_next } => {
                    script_from_flag = true;
                    script_files.extend(file);
                    i += usize::from(consumes_next) + 1;
                }
                SedShort::SkipValue { consumes_next } => i += usize::from(consumes_next) + 1,
            }
        }
    }
    // Without -e/-f, the first positional is the SCRIPT, not a file.
    if !script_from_flag && !files.is_empty() {
        files.remove(0);
    }
    // Blast radius: a glob (`sed -i … *`) or several operands is bounded, not single — so a
    // sweeping in-place edit is scored honestly (still worktree-bound by locus; a system or
    // home path denies whatever the scale).
    let scale = breadth_scale(&files, false);
    let mut caps: Vec<Capability> =
        script_files.iter().map(|f| observes(read_locus(f), Scale::Single, "sed reads an -f script file")).collect();
    // Script-embedded file commands (`w`/`W` write, `r`/`R` read, `s///w` write) — gate each target
    // by its locus, just like an operand file.
    caps.extend(script.writes.iter().map(|f| mutates(classify_locus(f), Scale::Single, "sed w/W writes a file")));
    caps.extend(script.reads.iter().map(|f| observes(read_locus(f), Scale::Single, "sed r/R reads a file")));
    if in_place {
        caps.extend(files.iter().map(|f| mutates(classify_locus(f), scale, "sed -i edits the file in place")));
    } else {
        caps.extend(reads_to_model(&files, scale));
    }
    Profile::of(caps)
}

/// The outcome of parsing one `sed` short-option cluster.
enum SedShort<'a> {
    Bad,
    Standalone,
    InPlace,                                                    // -i (rest is the optional suffix)
    Script { consumes_next: bool },                            // -e SCRIPT
    ScriptFile { file: Option<&'a str>, consumes_next: bool }, // -f FILE
    SkipValue { consumes_next: bool },                         // -l N
}

fn sed_cluster<'a>(cluster: &'a str, next: Option<&'a str>, boolset: &[u8]) -> SedShort<'a> {
    let bytes = cluster.as_bytes();
    let mut k = 0;
    while k < bytes.len() {
        // A flag byte is ASCII; a non-ASCII lead/continuation byte is not a flag, and slicing
        // `cluster[k + 1..]` at it would land mid-char and panic. Bail as unrecognized.
        if !bytes[k].is_ascii() {
            return SedShort::Bad;
        }
        let glued = &cluster[k + 1..]; // safe: bytes[k] is ASCII → k+1 is a char boundary
        let has = !glued.is_empty();
        match bytes[k] {
            b'i' => return SedShort::InPlace, // -i[SUFFIX]: the rest of the cluster is the suffix
            b'e' => return SedShort::Script { consumes_next: !has },
            b'f' => {
                let file = if has { Some(glued) } else { next };
                return SedShort::ScriptFile { file, consumes_next: !has };
            }
            b'l' if has || next.is_some() => return SedShort::SkipValue { consumes_next: !has }, // -l N
            b if boolset.contains(&b) => k += 1,
            _ => return SedShort::Bad,
        }
    }
    SedShort::Standalone
}

/// Parse a `sed` long option, returning how many tokens it consumed, or `None` if unknown.
fn sed_long<'a>(
    long: &'a str,
    next: Option<&'a str>,
    in_place: &mut bool,
    script_from_flag: &mut bool,
    script_files: &mut Vec<&'a str>,
) -> Option<usize> {
    let name = long.split('=').next().unwrap_or(long);
    match name {
        "in-place" => *in_place = true, // --in-place[=SUFFIX] (glued only)
        "expression" => {
            *script_from_flag = true;
            return Some(if long.contains('=') { 1 } else { 2 });
        }
        "file" => {
            *script_from_flag = true;
            match long.split_once('=') {
                Some((_, v)) => script_files.push(v),
                None => {
                    script_files.extend(next);
                    return Some(2);
                }
            }
        }
        "quiet" | "silent" | "regexp-extended" | "null-data" | "separate" | "unbuffered"
        | "posix" | "help" | "version" | "debug" | "follow-symlinks" | "sandbox"
        | "zero-terminated" | "line-length" => {}
        _ => return None,
    }
    Some(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(parts: &[&str]) -> Vec<Token> {
        parts.iter().map(|p| Token::from_test(p)).collect()
    }

    fn level(name: &str) -> &'static crate::engine::level::Level {
        crate::engine::authoring::default_levels()
            .iter()
            .find(|l| l.name == name)
            .expect("level exists")
    }

    fn inert() -> &'static crate::engine::level::Level {
        level("paranoid")
    }

    fn read_local() -> &'static crate::engine::level::Level {
        level("reader")
    }

    /// `resolve_openssl` contract: a private-key/decrypt form reaching the MODEL classifies as
    /// decrypt-read (secret=reads → refused by developer, admitted only by yolo); a public/to-file/
    /// validate form ABSTAINS (None → openssl's legacy allow_all); a spoofed path worst-cases.
    #[test]
    fn openssl_resolver_gates_model_disclosure_only() {
        let (dev, yolo) = (level("developer"), level("yolo"));
        for parts in [
            &["openssl", "rsa", "-in", "priv.pem"][..],
            &["openssl", "rsa", "-in", "priv.pem", "-pubout", "-text"], // -text past -pubout
            &["openssl", "rsa", "-in", "priv.pem", "-out", "/dev/stdout"], // -out value is stdout
            &["openssl", "rsa", "-in", "priv.pem", "-noout", "-text"],
            &["openssl", "pkcs8", "-in", "priv.pem"],
            &["openssl", "enc", "--d", "-k", "p", "-in", "c"], // --opt alias
            &["openssl", "cms", "-EncryptedData_decrypt", "-in", "m"],
            &["openssl", "pkcs12", "-in", "f.p12", "-noenc"],
        ] {
            let p = resolve(&toks(parts)).unwrap_or_else(|| panic!("resolves: {parts:?}"));
            assert!(
                p.capabilities.iter().any(|c| c.secret.level == SecretLevel::Reads),
                "secret=reads: {parts:?}",
            );
            assert!(!dev.admits(&p), "developer refuses: {parts:?}");
            assert!(yolo.admits(&p), "yolo admits: {parts:?}");
        }
        for parts in [
            &["openssl", "rsa", "-in", "priv.pem", "-pubout"][..],
            &["openssl", "rsa", "-in", "priv.pem", "-noout"],       // validate, no output
            &["openssl", "rsa", "-in", "enc.pem", "-out", "clean.pem"], // to a FILE, off the model
            &["openssl", "pkey", "-in", "pub.pem", "-pubin", "-text"], // public input → public text
            &["openssl", "pkcs12", "-in", "f.p12", "-nodes", "-out", "k.pem"],
            &["openssl", "enc", "-e", "-in", "x", "-out", "x.enc", "-k", "p"],
            &["openssl", "x509", "-in", "c", "-noout", "-text"],
        ] {
            assert!(resolve(&toks(parts)).is_none(), "resolver abstains (→ legacy): {parts:?}");
        }
    }

    /// The output-destination check is FAIL-CLOSED: only a single plain-file `-out` diverts the key
    /// off the model. Path-normalization spellings, `/dev/stderr`, and duplicate `-out` (openssl honors
    /// the last) must all read as model-reaching — the sign-off review found the old device-spelling
    /// denylist let these through.
    #[test]
    fn openssl_output_destination_is_fail_closed() {
        let dev = level("developer");
        let reaches_model = |args: &[&str]| {
            let mut parts = vec!["openssl", "rsa", "-in", "priv.pem"];
            parts.extend_from_slice(args);
            // decrypt-read (secret=reads, refused by developer) ⇔ the output reached the model; a
            // diverted output makes the resolver ABSTAIN (None → openssl's benign legacy).
            match resolve(&toks(&parts)) {
                None => false,
                Some(p) => {
                    p.capabilities.iter().any(|c| c.secret.level == SecretLevel::Reads) && !dev.admits(&p)
                }
            }
        };
        for evasion in [
            &["-out", "//dev/stdout"][..],
            &["-out", "/dev/./stdout"],
            &["-out", "//dev/fd/1"],
            &["-out", "/dev/fd//1"],
            &["-out=//dev/stdout"],
            &["-out", "/dev/stderr"],
            &["-out", "/foo/../dev/stdout"],
            &["-out", "dup.pem", "-out", "/dev/stdout"], // last-wins
            &["-out", "-"],
        ] {
            assert!(reaches_model(evasion), "must read as model-reaching: {evasion:?}");
        }
        for diverted in [
            &["-out", "clean.pem"][..],
            &["-out", "./sub/key.pem"],
            &["-out", "devnotes.pem"], // "dev" prefix on a filename is not the /dev device
            &["-out", "/home/u/key.pem"],
            &["-noout"],
        ] {
            assert!(!reaches_model(diverted), "must divert off the model: {diverted:?}");
        }
    }

    #[test]
    fn echo_resolves_to_a_benign_inert_profile() {
        let p = resolve(&toks(&["echo", "hi"])).expect("echo has a resolver");
        assert_eq!(p.capabilities.len(), 1);
        let c = &p.capabilities[0];
        assert_eq!(c.operation, Operation::Observe);
        assert_eq!(c.locus.local, LocalLocus::Process);
        assert_eq!(c.disclosure.audience, DisclosureAudience::LocalProcess);
        assert!(!c.because.is_empty(), "a structural certification cites its reason");
        // admitted at the *strictest* level — every facet (network/exec/secret/…) is zero
        assert!(inert().admits(&p), "echo is fully certified and inert-safe");
    }

    #[test]
    fn echo_flags_do_not_change_its_profile() {
        let bare = resolve(&toks(&["echo", "hi"])).expect("echo");
        let flagged = resolve(&toks(&["echo", "-n", "-e", "hi"])).expect("echo -n -e");
        assert_eq!(bare, flagged);
        assert!(inert().admits(&flagged));
    }

    #[test]
    fn an_unresearched_command_has_no_resolver() {
        assert!(resolve(&toks(UNRESOLVED_CMD)).is_none(), "unresearched → caller worst-cases");
        assert!(resolve(&[]).is_none(), "empty tokens");
    }

    #[test]
    fn cat_of_a_worktree_file_is_read_local() {
        let p = resolve(&toks(&["cat", "./notes.md"])).expect("cat");
        assert!(read_local().admits(&p), "cat ./notes.md");
        assert!(!inert().admits(&p), "reading a real file is above inert");
    }

    #[test]
    fn cat_beyond_the_worktree_is_denied_by_locus() {
        // Secrets, private home, unpinnable, and unrecognized system paths stay denied…
        for path in ["~/.ssh/id_rsa", "~/notes", "/etc/shadow", "$SECRET", "../outside", "/var/lib/mysql/data"] {
            let p = resolve(&toks(&["cat", path])).expect("cat");
            assert!(!read_local().admits(&p), "cat {path} is above read-local by locus");
        }
    }

    #[test]
    fn cat_of_a_system_file_is_not_admitted() {
        // the retreat: reading a system path is no longer admitted (it prompts, or the user grants).
        for path in ["/etc/hosts", "/etc/os-release", "/usr/share/doc/x"] {
            let p = resolve(&toks(&["cat", path])).expect("cat");
            assert!(!read_local().admits(&p), "cat {path} is no longer auto-approved");
        }
    }

    #[test]
    fn cat_stdin_is_process_scoped() {
        assert!(inert().admits(&resolve(&toks(&["cat"])).expect("cat")), "no operand → stdin");
        assert!(inert().admits(&resolve(&toks(&["cat", "-"])).expect("cat -")), "- → stdin");
    }

    #[test]
    fn cat_reads_every_file_operand_and_one_home_read_sinks_it() {
        let p = resolve(&toks(&["cat", "-n", "a.txt", "src/b.rs"])).expect("cat");
        assert_eq!(p.capabilities.len(), 2, "-n is a flag; two files");
        assert!(read_local().admits(&p), "both worktree");

        let mixed = resolve(&toks(&["cat", "a.txt", "~/.ssh/id_rsa"])).expect("cat");
        assert!(!read_local().admits(&mixed), "one home read sinks the whole profile");
    }

    #[test]
    fn cat_double_dash_treats_the_rest_as_files() {
        let p = resolve(&toks(&["cat", "--", "-n"])).expect("cat");
        assert_eq!(p.capabilities.len(), 1, "-n after -- is a filename");
        assert!(read_local().admits(&p));
    }

    #[test]
    fn head_tail_wc_read_like_cat_and_honor_numeric_shorthand() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};
        // worktree reads → read-local (SafeRead); home reads → denied by locus, same as cat.
        for cmd in [
            vec!["head", "README.md"],
            vec!["head", "-n", "5", "src/main.rs"],
            vec!["head", "-20", "src/main.rs"],   // obsolete -NUM form must parse
            vec!["tail", "-f", "./log.txt"],       // follow is still a bounded read
            vec!["tail", "-n", "100", "./log.txt"],
            vec!["wc", "-l", "./notes.md"],
        ] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("read")), Verdict::Allowed(SafetyLevel::SafeRead), "{cmd:?}");
        }
        // reading stdin (`-`) is process-scoped → inert, like `cat -`.
        assert_eq!(project(&resolve(&toks(&["wc", "-c", "-"])).expect("wc")), Verdict::Allowed(SafetyLevel::Inert), "wc stdin");
        for cmd in [vec!["head", "~/.ssh/id_rsa"], vec!["tail", "/etc/shadow"], vec!["wc", "-l", "$SECRET"]] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("read")), Verdict::Denied, "{cmd:?} beyond worktree");
        }
        // -NUM consumes no operand: `head -20 file` reads exactly `file`, not a phantom "20".
        let p = resolve(&toks(&["head", "-20", "src/main.rs"])).expect("head");
        assert_eq!(p.capabilities.len(), 1, "-20 is the count, not a file");
        // wc --files0-from reads an unpinnable set → worst-case → denied.
        assert_eq!(project(&resolve(&toks(&["wc", "--files0-from=list"])).expect("wc")), Verdict::Denied, "--files0-from");
        assert_eq!(project(&resolve(&toks(&["wc", "--files0-from", "-"])).expect("wc")), Verdict::Denied, "--files0-from -");
        // unknown flags fail closed.
        assert_eq!(project(&resolve(&toks(&["head", "-Z", "x"])).expect("head")), Verdict::Denied, "unknown flag");
    }

    #[test]
    fn grep_reads_its_files_not_the_pattern() {
        let p = resolve(&toks(&["grep", "foo", "file.txt"])).expect("grep");
        assert_eq!(p.capabilities.len(), 1, "the pattern is not a file");
        assert!(read_local().admits(&p));
    }

    #[test]
    fn grep_beyond_the_worktree_is_denied() {
        for args in [
            vec!["grep", "foo", "~/.ssh/config"],
            vec!["grep", "-r", "foo", "~"],
            vec!["grep", "foo", "$DIR"],
        ] {
            let p = resolve(&toks(&args)).expect("grep");
            assert!(!read_local().admits(&p), "{args:?}");
        }
    }

    #[test]
    fn grep_recursive_is_unbounded_and_defaults_to_cwd() {
        let p = resolve(&toks(&["grep", "-r", "foo", "src/"])).expect("grep");
        assert!(p.capabilities.iter().all(|c| c.scale == Scale::Unbounded), "-r → unbounded");
        assert!(read_local().admits(&p), "recursive worktree search");

        let cwd = resolve(&toks(&["grep", "-r", "foo"])).expect("grep");
        assert!(cwd.capabilities.iter().all(|c| c.locus.local == LocalLocus::Worktree), "cwd, not stdin");
        assert!(read_local().admits(&cwd));
    }

    #[test]
    fn grep_e_and_f_supply_the_pattern_so_positionals_are_files() {
        // -e: pattern is the flag's value; file.txt is the only file
        let e = resolve(&toks(&["grep", "-e", "foo", "file.txt"])).expect("grep -e");
        assert_eq!(e.capabilities.len(), 1);
        assert!(read_local().admits(&e));

        // -f: the pattern FILE is itself a read
        let f = resolve(&toks(&["grep", "-f", "patterns.txt", "file.txt"])).expect("grep -f");
        assert_eq!(f.capabilities.len(), 2, "patterns.txt + file.txt");
        assert!(read_local().admits(&f));

        let home = resolve(&toks(&["grep", "-f", "~/.secret-patterns", "file.txt"])).expect("grep -f");
        assert!(!read_local().admits(&home), "a home pattern file is denied by locus");

        // glued short value: -fpatterns.txt and -ifpatterns.txt both name a pattern file
        let glued = resolve(&toks(&["grep", "-fpatterns.txt", "file.txt"])).expect("grep -f glued");
        assert_eq!(glued.capabilities.len(), 2, "glued -f value is still a read");
        let glued_home = resolve(&toks(&["grep", "-if~/.secrets", "x"])).expect("grep -if glued");
        assert!(!read_local().admits(&glued_home), "glued home pattern file denied by locus");
    }

    #[test]
    fn grep_long_flags() {
        // --file / --file= name a pattern file grep also reads (2 caps)
        assert_eq!(resolve(&toks(&["grep", "--file", "p.txt", "f.txt"])).expect("grep").capabilities.len(), 2);
        assert_eq!(resolve(&toks(&["grep", "--file=p.txt", "f.txt"])).expect("grep").capabilities.len(), 2);

        // --regexp supplies the pattern; the positional is the file
        let r = resolve(&toks(&["grep", "--regexp", "foo", "f.txt"])).expect("grep");
        assert_eq!(r.capabilities.len(), 1);
        assert!(read_local().admits(&r));

        // a space-separated long value (`--max-count 5`) is imprecise — `5` is read as a
        // phantom positional — but FAIL-SAFE: still worktree-bounded, admitted at
        // read-local, never looser. (Precise handling needs the TOML flag schema.)
        let m = resolve(&toks(&["grep", "--max-count", "5", "foo", "f.txt"])).expect("grep");
        assert!(read_local().admits(&m), "--max-count 5 is fail-safe (imprecise)");

        // --perl-regexp (PCRE2) runs no code — benign like any regex-engine flag; reads read-local.
        let pcre = resolve(&toks(&["grep", "--perl-regexp", "foo", "f"])).expect("grep");
        assert!(read_local().admits(&pcre), "grep --perl-regexp reads a file, it does not exec");
    }

    #[test]
    fn grep_dash_patterns_are_search_patterns_not_flags() {
        // A `--`-prefixed token that is not a recognized grep flag is a SEARCH PATTERN, not
        // an unknown flag — grep patterns commonly look like `-->`, `---`, `--foo`. The
        // engine must read the file operand at read-local, matching the legacy handler, not
        // worst-case it.
        for args in [
            vec!["grep", "-->", "file.txt"],
            vec!["grep", "---", "file.txt"],
            vec!["grep", "--some-pattern", "file.txt"],
            vec!["grep", "-rn", "-->", "src/"],
            vec!["grep", "-i", "-r", "-n", "-->", "src/"],
        ] {
            let p = resolve(&toks(&args)).expect("grep");
            assert!(read_local().admits(&p), "dash-pattern should read-local: {args:?}");
            assert!(!inert().admits(&p), "it still reads a file: {args:?}");
        }
        // but the genuinely-dangerous long (--dereference-recursive, symlink escape) worst-cases
        for args in [vec!["grep", "--dereference-recursive", "foo", "dir"]] {
            let p = resolve(&toks(&args)).expect("grep");
            assert!(!read_local().admits(&p), "dangerous long must worst-case: {args:?}");
        }
        // PCRE flags now read-local (PCRE2 execs no code): -P short, --perl-regexp long, -oP combined.
        for args in [
            vec!["grep", "-P", "foo", "f"],
            vec!["grep", "--perl-regexp", "foo", "f"],
            vec!["grep", "-oP", "foo", "f"],
        ] {
            let p = resolve(&toks(&args)).expect("grep");
            assert!(read_local().admits(&p), "grep PCRE flag should read-local: {args:?}");
        }
    }

    #[test]
    fn grep_stdin_and_standalone_flags() {
        assert!(inert().admits(&resolve(&toks(&["grep", "foo"])).expect("grep")), "no file → stdin");
        let p = resolve(&toks(&["grep", "-i", "-n", "foo", "file.txt"])).expect("grep");
        assert_eq!(p.capabilities.len(), 1, "-i -n standalone; foo pattern; file.txt file");
        assert!(read_local().admits(&p));
    }

    /// The complete resolved capability for a single-capability invocation, with
    /// `because` cleared so the assertion is over the **facets** (not the prose).
    fn one_cap(cmd: &[&str]) -> Capability {
        let p = resolve(&toks(cmd)).expect("resolves");
        assert_eq!(p.capabilities.len(), 1, "{cmd:?} is a single-capability invocation");
        let mut c = p.capabilities[0].clone();
        c.because = String::new();
        c
    }

    /// Golden profiles: assert **every** facet of the resolved capability for
    /// representative invocations. This is the "all facets covered" check (§0) — struct
    /// equality means a facet the resolver forgot (left at a wrong default) or set wrong
    /// fails the test, per command. When commands carry TOML profiles, the expected
    /// profile is derived from the TOML instead of hand-built here.
    #[test]
    fn golden_profiles_cover_every_facet() {
        // echo — the reference `structural` profile: observe, process-scoped, output to
        // the model, and every other axis provably zero.
        let mut echo = Capability::new(Operation::Observe);
        echo.disclosure.audience = DisclosureAudience::LocalProcess;
        assert_eq!(one_cap(&["echo", "hi"]), echo, "echo");

        // cat of a worktree file — observe · worktree · content-to-model.
        let mut cat = Capability::new(Operation::Observe);
        cat.locus.local = LocalLocus::Worktree;
        cat.disclosure.audience = DisclosureAudience::LocalProcess;
        assert_eq!(one_cap(&["cat", "./notes.md"]), cat, "cat ./notes.md");

        // cat of a plain home file — home is no longer admitted, so locus rises to machine (deny).
        let mut cat_home = cat.clone();
        cat_home.locus.local = LocalLocus::Machine;
        assert_eq!(one_cap(&["cat", "~/notes.txt"]), cat_home, "cat ~/notes.txt");

        // cat of a home CREDENTIAL store rises further, to machine (HP-20 credential role).
        let mut cat_cred = cat.clone();
        cat_cred.locus.local = LocalLocus::Machine;
        assert_eq!(one_cap(&["cat", "~/.ssh/id_rsa"]), cat_cred, "cat ~/.ssh/id_rsa");

        // grep of a worktree file — like cat, bounded to the single searched file.
        assert_eq!(one_cap(&["grep", "foo", "file.txt"]), cat, "grep foo file.txt");

        // grep -r — the recursive search raises scale to unbounded and nothing else.
        let mut grep_r = cat.clone();
        grep_r.scale = Scale::Unbounded;
        assert_eq!(one_cap(&["grep", "-r", "foo", "src/"]), grep_r, "grep -r foo src/");

        // rm — destroy · worktree · effortful; no net/exec/secret.
        let mut rm = Capability::new(Operation::Destroy);
        rm.locus.local = LocalLocus::Worktree;
        rm.reversibility = Reversibility::Effortful;
        assert_eq!(one_cap(&["rm", "./x"]), rm, "rm ./x");

        // mkdir — create · worktree · trivial · leaves data. A fresh dir is rmdir-removable.
        let mut mkdir = Capability::new(Operation::Create);
        mkdir.locus.local = LocalLocus::Worktree;
        mkdir.reversibility = Reversibility::Trivial;
        mkdir.persistence.level = PersistenceLevel::Data;
        assert_eq!(one_cap(&["mkdir", "./build"]), mkdir, "mkdir ./build");

        // touch — the same create · worktree · trivial · data shape as mkdir.
        assert_eq!(one_cap(&["touch", "./new.txt"]), mkdir, "touch ./new.txt");

        // cp -n ./a ./b — a guaranteed-non-clobbering copy is TWO capabilities:
        // a source read (observe, worktree, NO model disclosure) and a trivial dest create.
        let cp = resolve(&toks(&["cp", "-n", "./a", "./b"])).expect("cp");
        assert_eq!(cp.capabilities.len(), 2, "cp = source read + dest write");
        let mut src = Capability::new(Operation::Observe);
        src.locus.local = LocalLocus::Worktree; // disclosure.audience stays `none`: file→file
        assert_eq!(clear_because(&cp.capabilities[0]), src, "cp source read");
        let mut dst = Capability::new(Operation::Create);
        dst.locus.local = LocalLocus::Worktree;
        dst.reversibility = Reversibility::Trivial; // -n → cannot overwrite
        dst.persistence.level = PersistenceLevel::Data;
        assert_eq!(clear_because(&cp.capabilities[1]), dst, "cp -n dest write");

        // mv ./a ./b — a relocation: source MUTATE (trivial, transient — the entry leaves)
        // + dest CREATE (recoverable overwrite). Contrast cp's source, which is an observe.
        let mv = resolve(&toks(&["mv", "./a", "./b"])).expect("mv");
        let mut mv_src = Capability::new(Operation::Mutate);
        mv_src.locus.local = LocalLocus::Worktree;
        mv_src.reversibility = Reversibility::Trivial;
        assert_eq!(clear_because(&mv.capabilities[0]), mv_src, "mv source relocation");
        let mut mv_dst = Capability::new(Operation::Create);
        mv_dst.locus.local = LocalLocus::Worktree;
        mv_dst.reversibility = Reversibility::Recoverable;
        mv_dst.persistence.level = PersistenceLevel::Data;
        assert_eq!(clear_because(&mv.capabilities[1]), mv_dst, "mv dest write");

        // ln ./a ./b — target bridged (observe, no model disclosure) + link create (trivial,
        // no -f). Same facet shapes as cp -n, the point being ln reuses `observes`.
        let ln = resolve(&toks(&["ln", "./a", "./b"])).expect("ln");
        let mut ln_tgt = Capability::new(Operation::Observe);
        ln_tgt.locus.local = LocalLocus::Worktree;
        assert_eq!(clear_because(&ln.capabilities[0]), ln_tgt, "ln target bridge");
        let mut ln_link = Capability::new(Operation::Create);
        ln_link.locus.local = LocalLocus::Worktree;
        ln_link.reversibility = Reversibility::Trivial;
        ln_link.persistence.level = PersistenceLevel::Data;
        assert_eq!(clear_because(&ln.capabilities[1]), ln_link, "ln link create");
    }

    fn clear_because(c: &Capability) -> Capability {
        let mut c = c.clone();
        c.because = String::new();
        c
    }

    #[test]
    fn mkdir_creates_in_the_worktree_but_not_beyond_it() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};
        // a fresh dir is a trivial-reversibility create → write-local (SafeWrite)
        for cmd in [vec!["mkdir", "./build"], vec!["mkdir", "-p", "a/b/c"], vec!["mkdir", "-m", "755", "./x"]] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("mkdir")), Verdict::Allowed(SafetyLevel::SafeWrite), "{cmd:?}");
        }
        // outside the worktree → denied by locus
        for cmd in [vec!["mkdir", "/etc/evil"], vec!["mkdir", "~/newdir"], vec!["mkdir", "$HOME/x"]] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("mkdir")), Verdict::Denied, "{cmd:?}");
        }
        // a glued valued short (-m755) and its value must not be read as operands
        let g = resolve(&toks(&["mkdir", "-m755", "./x"])).expect("mkdir");
        assert_eq!(g.capabilities.len(), 1, "-m755 glued: only ./x is an operand");
        assert_eq!(g.capabilities[0].locus.local, LocalLocus::Worktree);
        // fail-closed on an unknown flag / no operand
        assert_eq!(project(&resolve(&toks(&["mkdir", "-Q", "x"])).expect("mkdir")), Verdict::Denied, "unknown flag");
        assert_eq!(project(&resolve(&toks(&["mkdir"])).expect("mkdir")), Verdict::Denied, "no operand");
    }

    #[test]
    fn cp_splits_source_and_dest_loci_and_overwrite_gates_the_level() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};

        // a copy is a create/overwrite, not a destroy → write-local (SafeWrite), matching
        // echo > config.json. Overwriting is recoverable; -n can't clobber (trivial). Both
        // write-local — the destroy-vs-create boundary keeps cp below rm.
        let plain = resolve(&toks(&["cp", "./a", "./b"])).expect("cp");
        assert_eq!(plain.capabilities.last().unwrap().reversibility, Reversibility::Recoverable, "dest overwrite");
        assert_eq!(project(&plain), Verdict::Allowed(SafetyLevel::SafeWrite), "cp ./a ./b");
        let nc = resolve(&toks(&["cp", "-n", "./a", "./b"])).expect("cp");
        assert_eq!(nc.capabilities.last().unwrap().reversibility, Reversibility::Trivial, "-n cannot clobber");
        assert_eq!(project(&nc), Verdict::Allowed(SafetyLevel::SafeWrite), "cp -n ./a ./b");

        // reading a home/system SOURCE is denied by the source locus — no secret detector,
        // just the read locus (cp can't smuggle ~/.ssh/id_rsa into the worktree).
        assert_eq!(project(&resolve(&toks(&["cp", "~/.ssh/id_rsa", "./x"])).expect("cp")), Verdict::Denied, "home source");
        assert_eq!(project(&resolve(&toks(&["cp", "/etc/shadow", "./x"])).expect("cp")), Verdict::Denied, "system source");
        // writing a home/system DEST is denied by the dest locus.
        assert_eq!(project(&resolve(&toks(&["cp", "./x", "~/backdoor"])).expect("cp")), Verdict::Denied, "home dest");
        assert_eq!(project(&resolve(&toks(&["cp", "./x", "/etc/cron.d/x"])).expect("cp")), Verdict::Denied, "system dest");

        // -t DIR makes every positional a source; the dir is the dest. All three spellings
        // (separate, --long=, and glued short) must parse the same way.
        for form in [
            vec!["cp", "-t", "./dest", "./a", "./b"],
            vec!["cp", "--target-directory=./dest", "./a", "./b"],
            vec!["cp", "-t./dest", "./a", "./b"], // glued short — previously worst-cased
        ] {
            let t = resolve(&toks(&form)).expect("cp -t");
            assert_eq!(t.capabilities.len(), 3, "{form:?}: 2 sources + 1 dest");
            assert_eq!(project(&t), Verdict::Allowed(SafetyLevel::SafeWrite), "{form:?}");
        }
        // a glued -t pointing outside the worktree is still denied by the dest locus.
        assert_eq!(project(&resolve(&toks(&["cp", "-t/etc", "./a"])).expect("cp")), Verdict::Denied, "cp -t/etc");

        // optional-argument longs (--backup[=X], --preserve[=X]) must NOT swallow the
        // source operand: bare and glued forms both leave ./a a source and ./b the dest.
        for form in [
            vec!["cp", "--backup", "./a", "./b"],
            vec!["cp", "--preserve", "./a", "./b"],
            vec!["cp", "--preserve=mode", "./a", "./b"],
        ] {
            let c = resolve(&toks(&form)).expect("cp");
            assert_eq!(c.capabilities.len(), 2, "{form:?}: source read + dest write");
            assert_eq!(project(&c), Verdict::Allowed(SafetyLevel::SafeWrite), "{form:?}");
        }

        // recursion raises scale to unbounded; a lone operand / unknown flag worst-cases.
        assert_eq!(resolve(&toks(&["cp", "-r", "./a", "./b"])).expect("cp").capabilities[0].scale, Scale::Unbounded);
        assert_eq!(project(&resolve(&toks(&["cp", "./only"])).expect("cp")), Verdict::Denied, "no dest");
        assert_eq!(project(&resolve(&toks(&["cp", "-Q", "./a", "./b"])).expect("cp")), Verdict::Denied, "unknown flag");
        // -t naming a dest with NO source operands is a usage error → fail closed (not a lone,
        // benign dest write).
        assert_eq!(project(&resolve(&toks(&["cp", "-t", "./dest"])).expect("cp")), Verdict::Denied, "-t no source");
    }

    #[test]
    fn mv_relocates_within_the_worktree_and_gates_both_loci() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};

        // a move within the worktree is a mutate (source) + create (dest), both trivial/
        // recoverable → write-local, NOT developer. Unlike rm, a move relocates, not destroys.
        let m = resolve(&toks(&["mv", "./a", "./b"])).expect("mv");
        assert_eq!(m.capabilities[0].operation, Operation::Mutate, "source is a relocation, not a destroy");
        assert_eq!(m.capabilities[0].reversibility, Reversibility::Trivial, "mv back");
        assert_eq!(project(&m), Verdict::Allowed(SafetyLevel::SafeWrite), "mv ./a ./b");

        // both loci are gated as writes: source-out and dest-out both deny.
        assert_eq!(project(&resolve(&toks(&["mv", "~/.ssh/id_rsa", "./x"])).expect("mv")), Verdict::Denied, "source in home");
        assert_eq!(project(&resolve(&toks(&["mv", "./x", "~/exfil"])).expect("mv")), Verdict::Denied, "dest in home");
        // moving a worktree-TRUSTED file mutates .git → denied, even though cp of it is
        // allowed (cp only READS .git/config; the dest write puts cp at SafeWrite).
        assert_eq!(project(&resolve(&toks(&["mv", ".git/config", "./x"])).expect("mv")), Verdict::Denied, "mv .git/config");
        assert_eq!(project(&resolve(&toks(&["cp", ".git/config", "./x"])).expect("cp")), Verdict::Allowed(SafetyLevel::SafeWrite), "cp .git/config reads");

        // The relocate source gates at its WRITE face, not its read face. safe-chains' own config
        // READS at worktree-trusted but WRITES at machine (un-grantable): `mv`ing it REMOVES it,
        // so the removal must gate at the write face (machine); a `cp` of it only READS
        // (worktree-trusted). Both deny by verdict, so assert the source LOCUS to pin the face —
        // this is the case a read-face relocate would fail open on.
        let cfg = "~/.config/safe-chains.toml";
        assert_eq!(
            resolve(&toks(&["mv", cfg, "./x"])).expect("mv").capabilities[0].locus.local,
            LocalLocus::Machine,
            "mv source removal gates at the WRITE face",
        );
        assert_eq!(
            resolve(&toks(&["cp", cfg, "./x"])).expect("cp").capabilities[0].locus.local,
            LocalLocus::WorktreeTrusted,
            "cp source read gates at the READ face",
        );

        // -t DIR and glued forms; fail-closed on unknown flag / lone operand.
        let t = resolve(&toks(&["mv", "-t", "./dest", "./a", "./b"])).expect("mv -t");
        assert_eq!(t.capabilities.len(), 3, "2 sources + 1 dest");
        assert_eq!(project(&resolve(&toks(&["mv", "./only"])).expect("mv")), Verdict::Denied, "no dest");
        assert_eq!(project(&resolve(&toks(&["mv", "-Q", "./a", "./b"])).expect("mv")), Verdict::Denied, "unknown flag");
    }

    #[test]
    fn ln_is_cp_by_reference_and_gates_the_target_locus() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};

        // a worktree link (hard or symbolic) is target-read + link-create → write-local.
        for cmd in [vec!["ln", "./a", "./b"], vec!["ln", "-s", "./target", "./link"]] {
            let p = resolve(&toks(&cmd)).expect("ln");
            assert_eq!(p.capabilities[0].operation, Operation::Observe, "target is a bridged read");
            assert_eq!(project(&p), Verdict::Allowed(SafetyLevel::SafeWrite), "{cmd:?}");
        }
        // the cp-bypass is closed: linking a SECRET/unreadable TARGET denies on the target
        // locus, exactly as `cp` of it would (a link would otherwise alias the secret in).
        assert_eq!(project(&resolve(&toks(&["ln", "~/.ssh/id_rsa", "./x"])).expect("ln")), Verdict::Denied, "hard link to home credential");
        assert_eq!(project(&resolve(&toks(&["ln", "-s", "/etc/shadow", "./x"])).expect("ln")), Verdict::Denied, "symlink to secret");
        // the retreat: linking to a NON-workspace target denies on the target locus (as cp would).
        assert_eq!(project(&resolve(&toks(&["ln", "-s", "/etc/hosts", "./x"])).expect("ln")), Verdict::Denied, "symlink to system path");
        // writing the LINK outside the worktree denies on the link locus.
        assert_eq!(project(&resolve(&toks(&["ln", "-s", "./a", "~/evil"])).expect("ln")), Verdict::Denied, "link into home");
        // -t DIR, lone operand, unknown flag.
        assert_eq!(resolve(&toks(&["ln", "-t", "./dir", "./a", "./b"])).expect("ln -t").capabilities.len(), 3);
        assert_eq!(project(&resolve(&toks(&["ln", "./only"])).expect("ln")), Verdict::Denied, "no link name");
        assert_eq!(project(&resolve(&toks(&["ln", "-Q", "./a", "./b"])).expect("ln")), Verdict::Denied, "unknown flag");
        // -f (a clobber flag PRESENT) flips the link-create from the no-clobber default
        // (`trivial`) to `recoverable` — still write-local. Exercises the `clobber_flags`-present
        // branch of the transfer arm, the inverse of cp/mv's `no_clobber_flags`.
        let forced = resolve(&toks(&["ln", "-f", "./a", "./b"])).expect("ln -f");
        assert_eq!(project(&forced), Verdict::Allowed(SafetyLevel::SafeWrite), "ln -f worktree link");
        assert_eq!(forced.capabilities.last().unwrap().reversibility, Reversibility::Recoverable, "ln -f overwrites → recoverable");
        assert_eq!(
            resolve(&toks(&["ln", "./a", "./b"])).expect("ln").capabilities.last().unwrap().reversibility,
            Reversibility::Trivial,
            "ln default no-clobber → trivial",
        );
    }

    #[test]
    fn dd_parses_key_value_operands_and_gates_both_sides() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};

        // a worktree-to-worktree copy → write-local; params (bs/count/conv) are ignored.
        assert_eq!(
            project(&resolve(&toks(&["dd", "if=./a", "of=./b", "bs=1M", "count=10"])).expect("dd")),
            Verdict::Allowed(SafetyLevel::SafeWrite),
            "dd worktree copy",
        );
        // input from stdout (no of=) discloses the input content to the model, like cat.
        assert_eq!(project(&resolve(&toks(&["dd", "if=./notes"])).expect("dd")), Verdict::Allowed(SafetyLevel::SafeRead), "dd to stdout");
        assert_eq!(project(&resolve(&toks(&["dd"])).expect("dd")), Verdict::Allowed(SafetyLevel::Inert), "bare dd is stdin→stdout");

        // both sides gated by locus: a home INPUT or a device/home OUTPUT denies.
        for cmd in [
            vec!["dd", "if=~/.ssh/id_rsa", "of=./x"], // read a home secret
            vec!["dd", "if=./x", "of=/dev/rdisk0"],   // write a raw device (disk wipe)
            vec!["dd", "if=./x", "of=/dev/sda"],
            vec!["dd", "if=./x", "of=~/backup"],      // write into home
            vec!["dd", "if=~/.ssh/id_rsa"],           // home secret to stdout (→ model)
        ] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("dd")), Verdict::Denied, "{cmd:?}");
        }
        // fail-closed: a non key=value operand, or an unknown key, worst-cases.
        assert_eq!(project(&resolve(&toks(&["dd", "./file"])).expect("dd")), Verdict::Denied, "positional operand");
        assert_eq!(project(&resolve(&toks(&["dd", "exec=evil", "of=./x"])).expect("dd")), Verdict::Denied, "unknown key");
    }

    #[test]
    fn tar_parses_dashless_bundles_and_splits_by_mode() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};

        // dashless `czf` and dashed `-czf` and the long form all parse the same: a create is
        // members-read + archive-write → write-local for a worktree backup.
        for cmd in [
            vec!["tar", "czf", "backup.tar", "./src"],
            vec!["tar", "-czf", "backup.tar", "./src"],
            vec!["tar", "--create", "--file=backup.tar", "./src"],
        ] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("tar")), Verdict::Allowed(SafetyLevel::SafeWrite), "{cmd:?}");
        }
        // list reads the archive → read-local.
        assert_eq!(project(&resolve(&toks(&["tar", "tzf", "backup.tar"])).expect("tar")), Verdict::Allowed(SafetyLevel::SafeRead), "list");

        // the bundler-exfil case (golden-set): a home member denies on the member locus,
        // whether the archive goes to stdout or a file.
        assert_eq!(project(&resolve(&toks(&["tar", "czf", "-", "~/.ssh"])).expect("tar")), Verdict::Denied, "bundle secret to stdout");
        assert_eq!(project(&resolve(&toks(&["tar", "czf", "out.tar", "~/.aws"])).expect("tar")), Verdict::Denied, "bundle home member");
        // a home/system ARCHIVE denies on the archive write locus.
        assert_eq!(project(&resolve(&toks(&["tar", "cf", "~/backup.tar", "./src"])).expect("tar")), Verdict::Denied, "archive into home");

        // extract is archive-controlled (..-escapable) → worst-case, even for a benign archive.
        assert_eq!(project(&resolve(&toks(&["tar", "xzf", "release.tar"])).expect("tar")), Verdict::Denied, "extract");
        // `tar cf backup.tar` with no members creates an empty archive — a benign worktree
        // write, so SafeWrite (not a fail-closed case).
        assert_eq!(project(&resolve(&toks(&["tar", "cf", "backup.tar"])).expect("tar")), Verdict::Allowed(SafetyLevel::SafeWrite), "empty archive");
        // fail-closed: an unmodeled value option (-C), no mode, an empty profile, a bad letter.
        assert_eq!(project(&resolve(&toks(&["tar", "-C", "/etc", "xf", "a.tar"])).expect("tar")), Verdict::Denied, "-C unmodeled");
        assert_eq!(project(&resolve(&toks(&["tar", "c"])).expect("tar")), Verdict::Denied, "create to stdout, no members");
        assert_eq!(project(&resolve(&toks(&["tar", "zf", "backup.tar"])).expect("tar")), Verdict::Denied, "no mode letter");
    }

    #[test]
    fn sed_i_flips_read_to_write_and_locus_stops_system_wide_damage() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};

        // -i turns the file operands from reads into in-place MUTATES.
        let read = resolve(&toks(&["sed", "s/x/y/", "./foo"])).expect("sed");
        assert_eq!(read.capabilities[0].operation, Operation::Observe, "no -i → read");
        assert_eq!(project(&read), Verdict::Allowed(SafetyLevel::SafeRead), "sed read");
        let edit = resolve(&toks(&["sed", "-i", "s/x/y/", "./foo"])).expect("sed");
        assert_eq!(edit.capabilities[0].operation, Operation::Mutate, "-i → in-place write");
        assert_eq!(project(&edit), Verdict::Allowed(SafetyLevel::SafeWrite), "sed -i worktree");

        // THE CONCERN: a stray system-wide `sed -i` is stopped by LOCUS — a system, home, or
        // unpinnable target denies whatever the scale. Damage needs a target above the
        // worktree, and every such target is denied.
        for cmd in [
            vec!["sed", "-i", "s/a/b/", "/etc/passwd"],
            vec!["sed", "-i", "s/a/b/", "/etc/hosts"],
            vec!["sed", "-i", "s/a/b/", "~/.bashrc"],
            vec!["sed", "-i", "s/a/b/", "$CONFIG"],      // unpinnable
            vec!["sed", "-i", "s/a/b/", "../outside"],   // escapes the worktree
            vec!["sed", "-i", "-e", "s/a/b/", "/etc/x"], // -e script, system file
        ] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("sed")), Verdict::Denied, "{cmd:?} must deny");
        }

        // A worktree-scoped sweep IS allowed — bounded, recoverable, your own project files.
        // The glob/multi-operand blast radius is scored as `bounded`, still write-local.
        let glob = resolve(&toks(&["sed", "-i", "s/a/b/", "*"])).expect("sed");
        assert_eq!(glob.capabilities[0].scale, Scale::Bounded, "a glob is a bounded blast radius");
        assert_eq!(project(&glob), Verdict::Allowed(SafetyLevel::SafeWrite), "sed -i * (worktree)");
        assert_eq!(project(&resolve(&toks(&["sed", "-i", "s/a/b/", "a", "b", "c"])).expect("sed")), Verdict::Allowed(SafetyLevel::SafeWrite), "multi-file");

        // -i.bak (optional glued suffix) still parses as in-place.
        assert_eq!(project(&resolve(&toks(&["sed", "-i.bak", "s/a/b/", "./foo"])).expect("sed")), Verdict::Allowed(SafetyLevel::SafeWrite), "-i.bak");
        // -f runs a script file we can't inspect (its e/w/r commands are invisible) → denied, like
        // `awk -f`, `bash script.sh`, mlr `--load`.
        assert_eq!(project(&resolve(&toks(&["sed", "-f", "script.sed", "./foo"])).expect("sed")), Verdict::Denied, "-f script file unanalyzable");
        // a home file read (no -i) still denies by locus, like cat.
        assert_eq!(project(&resolve(&toks(&["sed", "s/a/b/", "~/.ssh/id_rsa"])).expect("sed")), Verdict::Denied, "read home secret");
        assert_eq!(project(&resolve(&toks(&["sed", "-Q", "./foo"])).expect("sed")), Verdict::Denied, "unknown flag");
    }

    #[test]
    fn sed_exec_command_is_worst_cased_at_parity_with_legacy() {
        use crate::engine::bridge::project;
        use crate::verdict::Verdict;
        // The `e` command/modifier executes text as a shell command (RCE). The resolver must
        // worst-case it — flag parsing alone treated the script as opaque and let it through.
        for cmd in [
            vec!["sed", "s/test/touch tmp/e", "file"],   // s///e modifier
            vec!["sed", "-e", "s/x/cmd/e", "file"],       // via -e
            vec!["sed", "s/x/cmd/ew", "file"],            // e flag BEFORE the greedy w flag
            vec!["sed", "1e", "file"],                    // address + e
            vec!["sed", "e"],                             // bare e
            vec!["sed", "-e", "e"],
            vec!["sed", "1e reboot", "file"],             // address + e WITH a command argument
            vec!["sed", "p;e id", "file"],                // e after a `;` separator
        ] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("sed")), Verdict::Denied, "{cmd:?}: exec must deny");
        }
        // `s/x/cmd/we` is NOT here: `w` is greedy-to-EOL, so `we` writes to a file named `e` (a
        // local SafeWrite), not w-then-e exec. `sed '1e reboot'` — the former residual gap — is now
        // caught by the sed sub-parser (`scan_sed`).
    }

    /// HP-19 #1 (engine): `classify_locus` now resolves relative paths against the ambient
    /// cwd/root. With no context it falls back to relative-is-worktree (status quo); under a
    /// `cd /etc` context the same operands resolve to `/etc/*` and deny.
    #[test]
    fn classify_locus_resolves_relative_operands_against_the_cwd_context() {
        use crate::engine::bridge::project;
        use crate::pathctx::PathCtx;
        use crate::verdict::{SafetyLevel, Verdict};

        // No context → relative is worktree (fallback), and a sweeping edit is write-local.
        for p in ["*", "passwd", "config"] {
            assert_eq!(classify_locus(p), LocalLocus::Worktree, "{p}: no ctx → worktree");
        }
        assert_eq!(project(&resolve(&toks(&["sed", "-i", "s/a/b/", "*"])).expect("sed")), Verdict::Allowed(SafetyLevel::SafeWrite), "no ctx: sed -i *");

        // Context says the shell is in /etc → relative operands are /etc/* → machine → deny.
        let _g = crate::pathctx::enter(PathCtx { cwd: Some("/etc".into()), root: Some("/home/u/proj".into()) });
        for p in ["*", "hosts", "config", "cron.d"] {
            assert_eq!(classify_locus(p), LocalLocus::Machine, "{p}: cwd=/etc → machine");
        }
        // /etc/passwd is the identity substrate: its WRITE face worst-cases to system-integrity
        // (above machine → above local-admin), even reached as a relative operand from cwd=/etc.
        assert_eq!(classify_locus("passwd"), LocalLocus::SystemIntegrity, "passwd: cwd=/etc → system-integrity");
        assert_eq!(project(&resolve(&toks(&["sed", "-i", "s/a/b/", "*"])).expect("sed")), Verdict::Denied, "cwd=/etc: sed -i * denied");
        assert_eq!(project(&resolve(&toks(&["dd", "if=./x", "of=passwd"])).expect("dd")), Verdict::Denied, "cwd=/etc: dd of=passwd denied");
        assert_eq!(project(&resolve(&toks(&["cp", "./payload", "config"])).expect("cp")), Verdict::Denied, "cwd=/etc: cp denied");
    }

    #[test]
    fn touch_creates_in_the_worktree_and_gates_the_reference_path() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};
        for cmd in [
            vec!["touch", "./new.txt"],
            vec!["touch", "-c", "existing"],
            vec!["touch", "-r", "ref.txt", "./out"], // worktree reference: a read + a create, both worktree
            vec!["touch", "-d", "-1 day", "./out"],  // -d takes a DATE literal (not a path), dash-leading value
        ] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("touch")), Verdict::Allowed(SafetyLevel::SafeWrite), "{cmd:?}");
        }
        // `-r REF` reads REF's timestamp — a path-flag gated by REF's locus. A worktree ref is a
        // worktree read (allowed, 2 caps), but an out-of-workspace reference DENIES (it would
        // otherwise be an mtime/existence oracle for arbitrary paths).
        let p = resolve(&toks(&["touch", "-r", "ref.txt", "./out"])).expect("touch");
        assert_eq!(p.capabilities.len(), 2, "./out create + ref.txt read");
        assert!(p.capabilities.iter().any(|c| c.operation == Operation::Observe), "the -r reference is a read");
        assert_eq!(project(&resolve(&toks(&["touch", "-r", "~/.bashrc", "./out"])).expect("touch")), Verdict::Denied, "home reference");
        assert_eq!(project(&resolve(&toks(&["touch", "-r", "/etc/shadow", "./out"])).expect("touch")), Verdict::Denied, "system reference");
        assert_eq!(project(&resolve(&toks(&["touch", "--reference=/etc/shadow", "./out"])).expect("touch")), Verdict::Denied, "long glued reference");
        assert_eq!(project(&resolve(&toks(&["touch", "--reference", "/etc/shadow", "./out"])).expect("touch")), Verdict::Denied, "long spaced reference");
        // -d's dash-leading date literal is NOT a path and is NOT gated.
        assert_eq!(project(&resolve(&toks(&["touch", "-d", "-1 day", "/tmp/../etc/x"])).expect("touch")), Verdict::Denied, "operand still gated");
        // beyond the worktree, and fail-closed cases
        assert_eq!(project(&resolve(&toks(&["touch", "/etc/x"])).expect("touch")), Verdict::Denied, "system path");
        assert_eq!(project(&resolve(&toks(&["touch", "-Z", "x"])).expect("touch")), Verdict::Denied, "unknown flag");
        assert_eq!(project(&resolve(&toks(&["touch"])).expect("touch")), Verdict::Denied, "no operand");
    }

    #[test]
    fn worst_case_is_denied_even_by_a_permissive_yolo_shaped_level() {
        use crate::engine::level::{Clause, Level, OrdBound};
        // a yolo-shaped level: allow anything local up to `machine`, minus a destroy corner
        let yolo = Level::new("yolo-ish")
            .allowing(Clause {
                local_locus: Some(OrdBound::at_most(LocalLocus::Machine)),
                ..Default::default()
            })
            .denying(Clause {
                operation: Some(vec![Operation::Destroy]),
                reversibility: Some(OrdBound::at_least(Reversibility::Irreversible)),
                ..Default::default()
            });
        let wc = Profile::of(vec![Capability::worst("test")]);
        assert!(!yolo.admits(&wc), "worst_case (locus=kernel) exceeds even a machine-capped allow");
    }

    #[test]
    fn rm_within_the_worktree_projects_to_developer_but_beyond_it_denies() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};
        // `developer` admits destroy WITHIN the worktree (golden-set decision 2), even
        // recursive/effortful; it maps to the legacy SafeWrite ceiling.
        for cmd in [
            vec!["rm", "./stale.log"],
            vec!["rm", "-rf", "./node_modules"],
            vec!["rm", "a", "b", "c"],
            vec!["rm", "--interactive=always", "./x"], // optional-arg long: must not worst-case
        ] {
            let p = resolve(&toks(&cmd)).expect("rm resolves");
            assert!(p.capabilities.iter().all(|c| c.operation == Operation::Destroy), "{cmd:?} destroys");
            assert_eq!(project(&p), Verdict::Allowed(SafetyLevel::SafeWrite), "{cmd:?} → developer");
        }
        // Deletion that reaches beyond the worktree (home/system) is above `developer`,
        // denied by locus — no clause admits a machine/user-scoped destroy.
        for cmd in [vec!["rm", "-rf", "/"], vec!["rm", "-rf", "~/notes"], vec!["rm", "/etc/hosts"]] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("rm")), Verdict::Denied, "{cmd:?} beyond worktree");
        }
    }

    /// End-to-end: `rm -rf /` resolves to the `destroy · irreversible · unbounded` corner and
    /// is the one thing even a maximally-permissive yolo refuses — by facet, not by name.
    /// Everything one facet away stays yolo-admitted.
    #[test]
    fn rm_rf_root_is_the_one_thing_even_yolo_denies() {
        let yolo = level("yolo");
        let root = resolve(&toks(&["rm", "-rf", "/"])).expect("rm");
        assert_eq!(root.capabilities[0].reversibility, Reversibility::Irreversible, "rm -rf / is irreversible");
        assert_eq!(root.capabilities[0].scale, Scale::Unbounded);
        assert!(!yolo.admits(&root), "rm -rf / denied even at yolo");
        assert!(!yolo.admits(&resolve(&toks(&["rm", "-rf", "~/notes"])).expect("rm")), "rm -rf ~ likewise");
        // adjacent-by-one-facet stays yolo-allowed:
        assert!(yolo.admits(&resolve(&toks(&["rm", "-rf", "./node_modules"])).expect("rm")), "recoverable worktree");
        assert!(yolo.admits(&resolve(&toks(&["rm", "/etc/hosts"])).expect("rm")), "single (bounded) system delete");
    }

    /// Phase 1 end-to-end: a subcommand tagged `profile = "<archetype>"` resolves (through the
    /// nested `<resource> <action>` grammar) to that archetype's exact static capability, so its
    /// verdict is DERIVED from facets, not hand-marked. Untagged sibling subs leave the engine
    /// abstaining (→ legacy).
    #[test]
    fn a_subcommand_profile_resolves_to_its_archetype() {
        let p = resolve(&toks(&["koyeb", "apps", "delete", "myapp"])).expect("koyeb apps delete resolves");
        assert_eq!(p.capabilities.len(), 1);
        assert_eq!(
            &p.capabilities[0],
            crate::engine::archetype::archetype("remote-destroy-recoverable").unwrap(),
            "the sub resolves to its declared archetype's capability",
        );
        // a differently-tagged action gets a different archetype
        let create = resolve(&toks(&["koyeb", "apps", "create", "myapp"])).expect("resolves");
        assert_eq!(create.capabilities[0].operation, Operation::Create);
        // an untagged read sub: no profile, no command behavior → the engine abstains (legacy decides)
        assert!(resolve(&toks(&["koyeb", "apps", "list"])).is_none(), "untagged sub → engine abstains");
    }

    /// Per-flag escalation (Phase 1 layer): a dangerous flag ADDS a capability to the sub's profile,
    /// and the level algebra takes the max — so a benign base + a destructive flag lands at the
    /// flag's tier. `git push` is vcs-sync (network-admin); `git push --force` adds
    /// remote-destroy-irreversible and escalates past it, to yolo.
    #[test]
    fn an_escalating_flag_adds_a_capability_and_raises_the_tier() {
        let destroy = crate::engine::archetype::archetype("remote-destroy-irreversible").unwrap();
        // The vcs-sync base now carries the destination's provenance (exposure §4): `origin` and the
        // bare `--force` form (default remote) are both `established`.
        let vcs_sync = {
            let mut c = crate::engine::archetype::archetype("vcs-sync").unwrap().clone();
            c.locus.provenance = Provenance::Established;
            c
        };

        let base = resolve(&toks(&["git", "push", "origin", "main"])).expect("git push resolves");
        assert_eq!(base.capabilities, vec![vcs_sync.clone()], "base is vcs-sync, established destination");

        let forced = resolve(&toks(&["git", "push", "--force"])).expect("resolves");
        assert_eq!(forced.capabilities.len(), 2);
        assert!(forced.capabilities.contains(&vcs_sync) && forced.capabilities.contains(destroy),
            "--force ADDS remote-destroy-irreversible to the vcs-sync base");

        // the escalation MATTERS at the level layer: network-admin admits the base but not the
        // forced push; the flag pushed it up to yolo.
        let network_admin = level("network-admin");
        assert!(network_admin.admits(&base), "git push is network-admin");
        assert!(!network_admin.admits(&forced), "git push --force escalated past network-admin");
        assert!(level("yolo").admits(&forced), "and lands at yolo");

        // the -f short form escalates identically
        assert_eq!(resolve(&toks(&["git", "push", "-f"])).unwrap().capabilities.len(), 2);
    }

    /// Destination-trust (exposure §4): `git push`'s send TARGET is classified onto
    /// `locus.provenance`, and an `ext::` command-transport worst-cases as RCE. The one resolver
    /// that makes the `locus.provenance` facet actually bind to a command.
    #[test]
    fn git_push_destination_provenance_is_classified() {
        use crate::engine::bridge::project;
        use crate::verdict::Verdict;

        let prov = |cmd: &[&str]| resolve(&toks(cmd)).expect("push resolves").capabilities[0].locus.provenance;

        // bare (configured default) and a bare remote NAME → established (a prior deliberate act).
        assert_eq!(prov(&["git", "push"]), Provenance::Established, "bare push = default remote");
        assert_eq!(prov(&["git", "push", "origin", "main"]), Provenance::Established, "remote name");
        // a flag before the target doesn't hide it.
        assert_eq!(prov(&["git", "push", "--force", "origin"]), Provenance::Established, "flag then name");
        // spelled inline → literal (visible but injectable): URL, scp-path, filesystem path.
        assert_eq!(prov(&["git", "push", "https://h/x.git", "main"]), Provenance::Literal, "url");
        assert_eq!(prov(&["git", "push", "git@h:x.git"]), Provenance::Literal, "scp-style");
        assert_eq!(prov(&["git", "push", "/srv/mirror.git"]), Provenance::Literal, "path");
        // a variable / substitution → opaque (unreviewable).
        assert_eq!(prov(&["git", "push", "$REMOTE"]), Provenance::Opaque, "variable");

        // network-admin admits established + literal, refuses opaque; the ext:: transport is RCE.
        let net = level("network-admin");
        assert!(net.admits(&resolve(&toks(&["git", "push", "origin"])).unwrap()), "established at network-admin");
        assert!(net.admits(&resolve(&toks(&["git", "push", "https://h/x.git"])).unwrap()), "literal URL at network-admin");
        assert!(!net.admits(&resolve(&toks(&["git", "push", "$REMOTE"])).unwrap()), "opaque above network-admin");
        // ext::<cmd> runs a local command — worst-cased, denied below yolo.
        assert_eq!(project(&resolve(&toks(&["git", "push", "ext::sh"])).unwrap()), Verdict::Denied, "ext:: is RCE");
        assert!(!net.admits(&resolve(&toks(&["git", "push", "ext::sh"])).unwrap()), "ext:: not at network-admin");

        // `--repo=<dest>` OVERRIDES the positional (the fail-open the review found: `--repo=ext::sh`
        // slipping past a benign `origin`). Glued and space forms; a bare remote name still allows.
        assert_eq!(project(&resolve(&toks(&["git", "push", "--repo=ext::sh", "origin"])).unwrap()), Verdict::Denied, "--repo=ext:: is RCE");
        assert!(!net.admits(&resolve(&toks(&["git", "push", "--repo", "$VAR", "origin"])).unwrap()), "--repo $VAR is opaque");
        assert_eq!(prov(&["git", "push", "--repo=https://h/x.git", "main"]), Provenance::Literal, "--repo URL is literal");
        assert!(net.admits(&resolve(&toks(&["git", "push", "--repo=upstream", "main"])).unwrap()), "--repo=<remote name> is established");
    }

    /// The `data-export` resolver: a bulk remote export (`supabase db dump`) is a read that
    /// auto-approves to stdout, but its OUTPUT-FILE form (`-f path`) adds a SECOND, path-gated local
    /// write — a dump to the worktree stays local (SafeWrite) while one to a system path gates on
    /// locus (denied), and the glued short `-f/path` spelling can't slip that gate. The unbounded
    /// `scale` records the volume without itself gating the read. See `behavioral-taxonomy-exposure.md`.
    #[test]
    fn data_export_gates_its_output_file() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};

        // to stdout: the bulk remote read alone — one capability, auto-approves as a read.
        let stdout = resolve(&toks(&["supabase", "db", "dump", "--data-only"])).expect("dump resolves");
        assert_eq!(stdout.capabilities.len(), 1, "stdout dump = the remote read only");
        assert_eq!(stdout.capabilities[0].scale, Scale::Unbounded, "a dump records its volume");
        assert_eq!(project(&stdout), Verdict::Allowed(SafetyLevel::SafeRead), "bulk read auto-approves");

        // -f into the worktree: read + a worktree write → still auto-approves (SafeWrite).
        for cmd in [
            vec!["supabase", "db", "dump", "-f", "dump.sql"],
            vec!["supabase", "db", "dump", "--file=dump.sql", "--data-only"],
        ] {
            let p = resolve(&toks(&cmd)).expect("dump resolves");
            assert_eq!(p.capabilities.len(), 2, "{cmd:?}: the remote read + a local write");
            assert_eq!(project(&p), Verdict::Allowed(SafetyLevel::SafeWrite), "{cmd:?}");
        }

        // -f onto a system path: the write gates on locus → denied, in every spelling — space,
        // glued `=`, AND the glued short `-f/path` that mustn't be a bypass.
        for cmd in [
            vec!["supabase", "db", "dump", "-f", "/etc/passwd"],
            vec!["supabase", "db", "dump", "--file=/etc/passwd"],
            vec!["supabase", "db", "dump", "-f/etc/passwd"],
        ] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("resolves")), Verdict::Denied, "{cmd:?} writes a system path");
        }
    }

    /// `sudo`/`doas` elevate the wrapped command's AUTHORITY — the resolver that finally gives
    /// `local-admin` something to admit. `sudo <safe cmd>` = a root op (above every user-authority
    /// band); `sudo rm -rf /` stays the catastrophe corner; `-u`/`-i` and unknown options fail up.
    #[test]
    fn sudo_elevates_the_wrapped_commands_authority() {
        use crate::engine::bridge::project;
        use crate::verdict::Verdict;
        let (dev, local, net, yolo) = (level("developer"), level("local-admin"), level("network-admin"), level("yolo"));

        // sudo cat ./notes — a ROOT read. Authority lifts to root; every band below local-admin pins
        // authority=user, so it lands at local-admin (and yolo), NOT developer/network-admin.
        let read = resolve(&toks(&["sudo", "cat", "./notes.md"])).expect("sudo cat resolves");
        assert_eq!(read.capabilities[0].authority, Authority::Root, "authority lifted to root");
        assert!(!dev.admits(&read) && !net.admits(&read), "a root op is above the user-authority bands");
        assert!(local.admits(&read) && yolo.admits(&read), "a root read is local-admin");

        // benign flag clusters are skipped without losing the inner command (space + glued values too).
        assert_eq!(resolve(&toks(&["sudo", "-EH", "cat", "./x"])).unwrap().capabilities[0].authority, Authority::Root);
        assert_eq!(resolve(&toks(&["sudo", "-n", "-p", "pw", "cat", "./x"])).unwrap().capabilities[0].authority, Authority::Root);

        // bumping authority does NOT rescue the catastrophe corner.
        assert_eq!(project(&resolve(&toks(&["sudo", "rm", "-rf", "/"])).unwrap()), Verdict::Denied, "sudo rm -rf / denied everywhere");

        // -u (run as another user) → other-user authority → yolo-only (identity confusion tops the ladder).
        let other = resolve(&toks(&["sudo", "-u", "bob", "cat", "./x"])).expect("sudo -u resolves");
        assert_eq!(other.capabilities[0].authority, Authority::OtherUser, "-u = run as other user");
        assert!(!local.admits(&other) && yolo.admits(&other), "other-user is yolo-only");
        assert_eq!(resolve(&toks(&["sudo", "-ubob", "cat", "./x"])).unwrap().capabilities[0].authority, Authority::OtherUser, "glued -ubob");

        // -i / -s / -e launch a root shell or editor → arbitrary code, worst-cased.
        assert_eq!(project(&resolve(&toks(&["sudo", "-i"])).unwrap()), Verdict::Denied, "sudo -i is a root shell");
        assert!(!local.admits(&resolve(&toks(&["sudo", "-s", "bash"])).unwrap()), "root shell not local-admin");

        // an UNRECOGNIZED sudo option fails closed.
        assert_eq!(project(&resolve(&toks(&["sudo", "--nonsense", "cat", "./x"])).unwrap()), Verdict::Denied, "unknown option worst-cases");

        // an UNRESOLVED inner → None, so the caller's legacy fallback denies (never looser than bare).
        assert!(resolve(&toks(&["sudo", "totallyunknowncmd", "x"])).is_none(), "unresolved inner → legacy denies");
        // `sudo` with no command → None (legacy decides).
        assert!(resolve(&toks(&["sudo", "-v"])).is_none(), "no inner command");

        // doas is the same wrapper.
        assert_eq!(resolve(&toks(&["doas", "cat", "./x"])).unwrap().capabilities[0].authority, Authority::Root);

        // NEVER LOOSER: at the default band every `sudo …` is denied (root authority is auto-approved
        // by NO level below local-admin), exactly like the legacy classifier, which denies sudo whole.
        for cmd in ["sudo cat ./notes.md", "sudo rm -rf ./build", "sudo -EH cat ./x", "sudo -u bob ls"] {
            assert_eq!(crate::command_verdict(cmd), Verdict::Denied, "`{cmd}` must not auto-approve at the default band");
        }
    }

    /// systemctl — the first REAL command user of the `local-privileged` archetype. Read subs stay
    /// SafeRead (any band); service-management subs land at local-admin; and `sudo systemctl restart`
    /// (previously fail-closed, since systemctl's inner sub was unmodeled) now resolves to local-admin.
    #[test]
    fn systemctl_service_management_is_local_admin() {
        use crate::verdict::Verdict;
        let (dev, local, net, yolo) = (level("developer"), level("local-admin"), level("network-admin"), level("yolo"));

        // service management → local-privileged: local-admin and yolo admit; developer/network-admin don't.
        for sub in ["restart", "start", "stop", "enable", "disable", "mask", "daemon-reload", "kill"] {
            let p = resolve(&toks(&["systemctl", sub, "nginx"])).unwrap_or_else(|| panic!("systemctl {sub} resolves"));
            assert!(!dev.admits(&p) && !net.admits(&p), "systemctl {sub} is above developer/network-admin");
            assert!(local.admits(&p) && yolo.admits(&p), "systemctl {sub} is local-admin");
        }
        // reads stay auto-approvable (SafeRead), a power-state sub denies by omission (not modeled).
        assert!(crate::command_verdict("systemctl status nginx").is_allowed(), "status reads");
        assert_eq!(crate::command_verdict("systemctl reboot"), Verdict::Denied, "reboot omitted → denied");

        // the fail-closed case is fixed: `sudo systemctl restart` resolves the inner sub (already root).
        let sudo_restart = resolve(&toks(&["sudo", "systemctl", "restart", "nginx"])).expect("resolves");
        assert!(local.admits(&sudo_restart), "sudo systemctl restart is local-admin");
        assert_eq!(crate::command_verdict("sudo systemctl restart nginx"), Verdict::Denied, "still not auto-approved at default");
    }

    /// The flag-conditional-archetype resolver (the `when_absent` mechanism, npm exemplar):
    /// `npm ci --ignore-scripts` is a PINNED, scripts-off install → `local-install-pinned`
    /// (developer). Dropping `--ignore-scripts` escalates it to `supply-chain-build` (yolo — runs
    /// fetched code at install). `npm install`/`i` are FLOATING → always `supply-chain-build`. This
    /// is the pattern the package-manager fan-out replicates.
    #[test]
    fn npm_install_is_classified_by_pinning_and_scripts_off() {
        let (dev, yolo) = (level("developer"), level("yolo"));

        // pinned (ci) + scripts-off → developer.
        let safe = resolve(&toks(&["npm", "ci", "--ignore-scripts"])).expect("npm ci --ignore-scripts");
        assert!(dev.admits(&safe), "pinned, scripts-off ci is developer");

        // pinned but scripts-ON → the --ignore-scripts ABSENCE escalates to supply-chain-build → yolo.
        let scripts_on = resolve(&toks(&["npm", "ci"])).expect("npm ci");
        assert!(!dev.admits(&scripts_on), "ci without --ignore-scripts runs fetched code → above developer");
        assert!(yolo.admits(&scripts_on), "and lands at yolo");

        // floating installs → supply-chain-build regardless of flags.
        for c in [&["npm", "install"][..], &["npm", "install", "left-pad"], &["npm", "i", "react"], &["npm", "install", "--ignore-scripts"]] {
            let p = resolve(&toks(c)).unwrap_or_else(|| panic!("{c:?} resolves"));
            assert!(!dev.admits(&p) && yolo.admits(&p), "{c:?}: floating install → supply-chain (yolo)");
        }
    }

    #[test]
    fn rm_flag_and_operand_fail_closed() {
        use crate::engine::bridge::project;
        use crate::verdict::Verdict;
        for cmd in [
            vec!["rm", "--no-preserve-root", "-rf", "/"], // enables rm -rf / → must worst-case
            vec!["rm", "-Z", "x"],                        // unknown flag
            vec!["rm"],                                   // no operand (usage error)
            vec!["./rm", "x"],                            // basename spoof
        ] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("resolves")), Verdict::Denied, "{cmd:?}");
        }
    }

    #[test]
    fn rm_scale_and_force_semantics() {
        let cap = |cmd: &[&str]| resolve(&toks(cmd)).expect("rm").capabilities[0].clone();
        assert_eq!(cap(&["rm", "./x"]).scale, Scale::Single);
        assert_eq!(cap(&["rm", "a", "b"]).scale, Scale::Bounded, "multiple operands");
        assert_eq!(cap(&["rm", "*.log"]).scale, Scale::Bounded, "a glob");
        assert_eq!(cap(&["rm", "-r", "./dir"]).scale, Scale::Unbounded, "recursive");
        // -f only suppresses prompts — it does NOT raise reversibility for rm
        assert_eq!(cap(&["rm", "./x"]).reversibility, Reversibility::Effortful);
        assert_eq!(cap(&["rm", "-f", "./x"]).reversibility, Reversibility::Effortful, "-f is not a raiser");
    }

    #[test]
    fn a_resolvable_name_from_a_non_standard_path_worst_cases() {
        // ./cat, /tmp/cat, ~/bin/grep may be impostors → worst-case, not certified safe
        for cmd in [vec!["./cat", "x"], vec!["/tmp/cat", "x"], vec!["~/bin/grep", "foo", "f"]] {
            let p = resolve(&toks(&cmd)).expect("resolvable name");
            assert!(!read_local().admits(&p), "{cmd:?} from a non-standard path must worst-case");
        }
        // bare names and standard bin paths resolve normally
        assert!(read_local().admits(&resolve(&toks(&["cat", "./notes.md"])).expect("cat")));
        assert!(read_local().admits(&resolve(&toks(&["/usr/bin/cat", "./notes.md"])).expect("cat")));
        // a non-resolvable command from any path → None (the engine doesn't claim it)
        assert!(resolve(&toks(&["/tmp/mytool", "x"])).is_none());
    }

    #[test]
    fn unrecognized_flags_worst_case_fail_closed() {
        for cmd in [
            vec!["cat", "-Z", "./x"],
            vec!["cat", "--wat", "./x"],
            vec!["grep", "-Q", "foo", "f"], // unknown grep short char (-Z is benign: --null)
            vec!["grep", "-R", "foo", "dir"], // -R follows symlinks → escapes locus (M2)
        ] {
            let p = resolve(&toks(&cmd)).expect("resolver");
            assert!(!inert().admits(&p) && !read_local().admits(&p), "{cmd:?} must worst-case");
        }
        // recognized-benign flags still resolve normally
        assert!(read_local().admits(&resolve(&toks(&["cat", "-nA", "./x"])).expect("cat")));
        assert!(read_local().admits(&resolve(&toks(&["grep", "-rin", "foo", "src/"])).expect("grep")));
    }

    use proptest::prelude::*;

    /// The content-transfer commands: every one moves/bridges content between a source and
    /// a destination operand, so BOTH roles must be locus-gated. Extend this list as
    /// `install`/`dd`/`rsync`/`tar` land — a resolver that forgets to gate a role then fails
    /// the property below (the `ln` cp-bypass class, §HP re: capability laundering).
    const TRANSFER_CMDS: &[&str] = &["cp", "mv", "ln"];

    /// A sensitive path that must never be laundered through a transfer command, in any
    /// role. Covers each locus rung above the worktree AND the two unpinnable markers.
    const HOT_PATHS: &[&str] = &["/etc/shadow", "~/.ssh/id_rsa", "$SECRET", "../out", "~/.aws"];

    proptest! {
        /// No capability laundering: a hot path in EITHER operand role of a transfer command
        /// denies — you can neither pull a secret in (`cp ~/.ssh/id_rsa ./x`) nor push one
        /// out (`cp ./x /etc/cron.d/y`). This is the STRICT property that catches an ignored
        /// operand; plain locus-monotonicity does not, because ignoring a role leaves the
        /// verdict unchanged, and unchanged is "not looser".
        #[test]
        fn transfer_commands_gate_both_operand_roles(
            cmd in prop::sample::select(TRANSFER_CMDS),
            hot in prop::sample::select(HOT_PATHS),
        ) {
            use crate::engine::bridge::project;
            use crate::verdict::Verdict;
            let hot_source = resolve(&toks(&[cmd, hot, "./safe"])).expect("resolves");
            prop_assert_eq!(project(&hot_source), Verdict::Denied, "{} hot SOURCE ({})", cmd, hot);
            let hot_dest = resolve(&toks(&[cmd, "./safe", hot])).expect("resolves");
            prop_assert_eq!(project(&hot_dest), Verdict::Denied, "{} hot DEST ({})", cmd, hot);
        }
    }

    /// The exact roster of commands classified by `[command.behavior]`. Pinning it turns a
    /// DROPPED or typo'd behavior block into a test failure: `TomlCommand` deliberately lacks
    /// `deny_unknown_fields` (it must tolerate `[[trusted]]`), so a mistyped top-level key
    /// (`behaviour = …`) is silently dropped and the command reverts to its PERMISSIVE legacy
    /// fallback — a fail-open the enumeration guards can't see (they `continue` on `None`). This
    /// roster is that missing tripwire, and the guards below derive their non-vacuity floors from
    /// it so the floors track reality. Update deliberately when porting a command. `echo` is a
    /// none-role printer; `dd`/`tar`/`sed` are hook commands; `grep` is a hook + pattern-then-read;
    /// the other 10 are the plain positional coreutils.
    const EXPECTED_BEHAVIOR_COMMANDS: &[&str] = &[
        "cat", "cp", "dd", "echo", "grep", "head", "ln", "mkdir", "mv", "rm", "sed", "tail",
        "tar", "touch", "wc",
    ];

    /// The behavior roster is exactly `EXPECTED_BEHAVIOR_COMMANDS` — no command silently lost its
    /// `[command.behavior]` (fail-open) and none was added without being pinned. Red→green: delete
    /// one command's behavior block and this fails.
    #[test]
    fn behavior_command_roster_is_pinned() {
        use std::collections::BTreeSet;
        let actual: BTreeSet<&str> = crate::registry::toml_command_names()
            .into_iter()
            .filter(|n| crate::registry::command_behavior(n).is_some())
            .collect();
        let expected: BTreeSet<&str> = EXPECTED_BEHAVIOR_COMMANDS.iter().copied().collect();
        assert_eq!(
            actual, expected,
            "behavior-command roster drifted — a [command.behavior] block was added, dropped, or \
             typo'd. A dropped block silently reverts the command to its fail-open legacy path."
        );
    }

    /// Hot-path probes for a `[command.behavior]` command, keyed on its declared operand role
    /// (the parallel of `probes` for the `Operands` enum). A `@` in a slot is the hot path.
    fn behavior_probes(cmd: &str, role: crate::registry::types::PositionalRole, hot: &str) -> Vec<Vec<String>> {
        use crate::registry::types::PositionalRole;
        let inv = |slots: &[&str]| -> Vec<String> {
            std::iter::once(cmd.to_string()).chain(slots.iter().map(|s| s.replace('@', hot))).collect()
        };
        match role {
            PositionalRole::None => vec![],
            PositionalRole::Read | PositionalRole::Write => vec![inv(&["@"])],
            PositionalRole::PatternThenRead => vec![inv(&["PATTERN", "@"])],
            PositionalRole::Transfer => vec![inv(&["@", "./safe"]), inv(&["./safe", "@"])],
        }
    }

    /// Hot-path probes for a HOOK command, whose irregular operand syntax `behavior_probes`
    /// (positional roles) can't express — dd's `key=value`, tar's dashless mode bundles, sed's
    /// script. The `match` is EXHAUSTIVE, so a new `BehaviorHook` variant must declare its probe
    /// rows here or the build breaks — restoring the "new entry covered automatically" property the
    /// deleted `every_touched_path_operand_is_gated` had via `Operands::Custom`. `@` = the hot slot.
    fn hook_probes(hook: crate::registry::types::BehaviorHook, cmd: &str, hot: &str) -> Vec<Vec<String>> {
        use crate::registry::types::BehaviorHook;
        let inv = |slots: &[&str]| -> Vec<String> {
            std::iter::once(cmd.to_string()).chain(slots.iter().map(|s| s.replace('@', hot))).collect()
        };
        match hook {
            // grep is pattern-then-read → already probed by `behavior_probes`; no extra rows.
            BehaviorHook::Grep => vec![],
            BehaviorHook::Dd => vec![inv(&["if=@", "of=./safe"]), inv(&["if=./safe", "of=@"])],
            BehaviorHook::Tar => vec![inv(&["cf", "./s.tar", "@"]), inv(&["cf", "@", "./s"]), inv(&["tf", "@"])],
            BehaviorHook::Sed => vec![inv(&["s/x/y/", "@"]), inv(&["-i", "s/x/y/", "@"])],
        }
    }

    /// Fail-closed, enumerated over the REGISTRY: every `[command.behavior]` command denies an
    /// operand on a hot path (a secret, home, system, or unpinnable locus), AND a write-role
    /// command denies a write into the worktree-trusted rung (`.git/config`). Restores and
    /// generalizes `every_touched_path_operand_is_gated` for the declarative path — a command
    /// ported off Rust is covered automatically. Red→green: make `resolve_behavior` skip
    /// `classify_locus` and this fails on the first probe.
    #[test]
    fn every_behavior_command_gates_hot_operands() {
        use crate::engine::bridge::project;
        use crate::registry::types::PositionalRole;
        use crate::verdict::Verdict;

        let deny = |cmd: &[String], why: &str| {
            let refs: Vec<&str> = cmd.iter().map(String::as_str).collect();
            let profile = resolve(&toks(&refs)).expect("behavior command resolves");
            assert_eq!(project(&profile), Verdict::Denied, "{cmd:?}: {why}");
        };

        let mut path_bearing = 0usize;
        let mut hook_bearing = 0usize;
        for name in crate::registry::toml_command_names() {
            let Some(b) = crate::registry::command_behavior(name) else { continue };
            if !matches!(b.positionals, PositionalRole::None) {
                path_bearing += 1;
            }
            if b.hook.is_some() {
                hook_bearing += 1;
            }
            for hot in HOT_PATHS {
                for cmd in behavior_probes(name, b.positionals, hot) {
                    deny(&cmd, "touched hot path not gated");
                }
                // Hook commands (dd/tar/sed) have irregular operand syntax, so they are swept by
                // their own probe table — restoring the enumerated coverage the deleted RESOLVERS
                // sweep gave them.
                if let Some(hook) = b.hook {
                    for cmd in hook_probes(hook, name, hot) {
                        deny(&cmd, "hook: touched hot path not gated");
                    }
                }
            }
            // Worktree-trusted is a WRITE boundary only: reading `.git/config` (cat/grep) is
            // legitimately allowed, but a write/destroy/relocate into it must deny. Probe the
            // write face — the destination slot for a transfer, the operand for a plain write.
            let inv = |slots: &[&str]| -> Vec<String> {
                std::iter::once(name.to_string()).chain(slots.iter().map(|s| s.to_string())).collect()
            };
            match b.positionals {
                PositionalRole::Write => deny(&inv(&[".git/config"]), "write into worktree-trusted not gated"),
                PositionalRole::Transfer => deny(&inv(&["./safe", ".git/config"]), "transfer dest into worktree-trusted not gated"),
                _ => {}
            }
        }
        // Non-vacuity: every path-bearing AND every hook command on the roster was reached and
        // probed. Derived from the roster (not a magic number) — none-role printers (echo) don't
        // positionally gate; hook commands (dd/tar/sed) gate via `hook_probes`.
        let count = |pred: fn(&crate::registry::types::BehaviorSpec) -> bool| {
            EXPECTED_BEHAVIOR_COMMANDS
                .iter()
                .filter(|n| crate::registry::command_behavior(n).is_some_and(pred))
                .count()
        };
        assert_eq!(
            path_bearing,
            count(|b| !matches!(b.positionals, PositionalRole::None)),
            "path-bearing behavior commands: saw {path_bearing}"
        );
        assert_eq!(hook_bearing, count(|b| b.hook.is_some()), "hook behavior commands: saw {hook_bearing}");
    }

    /// Fail-closed on unknown flags, enumerated over the REGISTRY: every DECLARATIVE flag-walking
    /// behavior command (a Read/Write/Transfer role, hookless) worst-cases an unrecognized flag —
    /// the `walk_positionals` → `worst` path. Exempt: `grep` (its hook treats an unknown `--token`
    /// as a search pattern — keyed on `BehaviorHook::Grep` SPECIFICALLY, not `hook.is_some()`, so a
    /// future hook variant is not auto-exempted), and none-role commands (echo prints its args;
    /// dd/tar/sed parse their own irregular syntax — all covered by their own resolver tests, and
    /// none-role commands take no positional path operands, so an unknown flag can't unlock danger).
    #[test]
    fn every_hookless_behavior_command_worst_cases_unknown_flags() {
        use crate::engine::bridge::project;
        use crate::registry::types::{BehaviorHook, PositionalRole};
        use crate::verdict::Verdict;

        let exempt = |b: &crate::registry::types::BehaviorSpec| {
            matches!(b.hook, Some(BehaviorHook::Grep)) || matches!(b.positionals, PositionalRole::None)
        };
        let mut checked = 0usize;
        for name in crate::registry::toml_command_names() {
            let Some(b) = crate::registry::command_behavior(name) else { continue };
            if exempt(b) {
                continue;
            }
            let profile = resolve(&toks(&[name, "--xyzzy-unknown-42", "./safe"])).expect("resolves");
            assert_eq!(project(&profile), Verdict::Denied, "{name}: unknown flag not worst-cased");
            checked += 1;
        }
        let expected = EXPECTED_BEHAVIOR_COMMANDS
            .iter()
            .filter(|n| crate::registry::command_behavior(n).is_some_and(|b| !exempt(b)))
            .count();
        assert_eq!(checked, expected, "declarative flag-walking behavior commands: saw {checked}");
    }

    /// Fail-closed authoring guard: `path_flag_caps` (which gates a valued flag's path VALUE, e.g.
    /// `touch -r REF`) runs ONLY on the declarative Read/Write/Transfer path — the None arm (echo)
    /// and the hook arm (grep/dd/tar/sed) both return before it. So a `[command.behavior.flags]`
    /// path-role declared on a none-role or hook command would be SILENTLY UNGATED — a fail-open.
    /// Assert no command does that. Red→green: add `kind = "read"` to a hook command's flags.
    #[test]
    fn no_none_or_hook_command_declares_ungated_path_flags() {
        use crate::registry::types::PositionalRole;
        for name in crate::registry::toml_command_names() {
            let Some(b) = crate::registry::command_behavior(name) else { continue };
            if b.path_flags.is_empty() {
                continue;
            }
            assert!(
                b.hook.is_none() && !matches!(b.positionals, PositionalRole::None),
                "{name}: behavior path-flags are gated only on the Read/Write/Transfer path; on a \
                 none-role or hook command they would be silently ungated (fail-open)"
            );
        }
    }
}
