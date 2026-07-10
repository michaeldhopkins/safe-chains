//! Coexistence bridge (v1.4 §4.5; annex `…-engine` §4). Runs the profile resolver
//! alongside the legacy classifier behind the `SAFE_CHAINS_ENGINE` selector, and
//! projects a resolved profile back to a legacy [`Verdict`] so the existing ceiling
//! gate (`main::run_cli`) keeps working unchanged.
//!
//! Default is `legacy` — the engine does not run and behavior is byte-identical. Only
//! commands with a resolver (`resolve::resolve` → `Some`) are ever computed by the
//! engine; everything else stays on the legacy path.

use std::sync::LazyLock;

use super::authoring::default_levels;
use super::facet::Profile;
use super::resolve;
use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

/// The rollout selector (annex `…-engine` §5).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Current classifier is authoritative; the engine does not run (default).
    Legacy,
    /// Legacy is authoritative; the engine runs alongside and divergences are reported.
    Shadow,
    /// The engine is authoritative for commands it can resolve; legacy for the rest.
    New,
}

static MODE: LazyLock<Mode> =
    LazyLock::new(|| parse_mode(std::env::var("SAFE_CHAINS_ENGINE").ok().as_deref()));

fn parse_mode(value: Option<&str>) -> Mode {
    match value {
        Some("shadow") => Mode::Shadow,
        Some("new") => Mode::New,
        _ => Mode::Legacy,
    }
}

/// The active engine mode (read once from `SAFE_CHAINS_ENGINE`).
pub fn mode() -> Mode {
    *MODE
}

/// Combine the legacy verdict with the engine's, per mode — the seam `cst::check`'s
/// `leaf_verdict` calls. `legacy` is the classifier's verdict for the same tokens.
pub fn apply_mode(mode: Mode, legacy: Verdict, tokens: &[Token]) -> Verdict {
    match mode {
        Mode::Legacy => legacy,
        Mode::New => engine_verdict(tokens).unwrap_or(legacy),
        Mode::Shadow => {
            if let Some(engine) = engine_verdict(tokens)
                && engine != legacy
            {
                let name = tokens.first().map_or("", |t| t.as_str());
                eprintln!("safe-chains[shadow]: `{name}` legacy={legacy} engine={engine}");
            }
            legacy
        }
    }
}

/// The engine's verdict for a command whose resolver exists, or `None` if it has none
/// (the caller keeps the legacy verdict).
pub fn engine_verdict(tokens: &[Token]) -> Option<Verdict> {
    resolve::resolve(tokens).map(|p| project(&p))
}

/// Project a resolved profile to a legacy [`Verdict`]: the **lowest** authored level
/// that admits it, mapped back to its legacy [`SafetyLevel`]; `Denied` if no authored
/// level admits it (above the ladder → worst-case, §0). `default_levels()` is the
/// ascending local chain (inert ⊂ read-local ⊂ write-local), so the first match is the
/// minimum.
pub fn project(profile: &Profile) -> Verdict {
    if profile.capabilities.is_empty() {
        // Fail-closed (§0): an empty profile means the resolver produced NO capability.
        // Every level vacuously admits it (`all` of zero capabilities is true), so without
        // this guard it would project to the lowest level (`inert`) — the *most*
        // permissive, inverting the principle. A genuinely-inert command emits an explicit
        // observe capability, never an empty profile.
        return Verdict::Denied;
    }
    for level in default_levels() {
        if level.admits(profile) {
            return Verdict::Allowed(to_legacy(&level.name));
        }
    }
    Verdict::Denied
}

fn to_legacy(level_name: &str) -> SafetyLevel {
    match level_name {
        "inert" => SafetyLevel::Inert,
        "read-local" => SafetyLevel::SafeRead,
        _ => SafetyLevel::SafeWrite, // write-local, developer, …
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::facet::*;

    fn toks(parts: &[&str]) -> Vec<Token> {
        parts.iter().map(|p| Token::from_test(p)).collect()
    }

    #[test]
    fn project_maps_profiles_to_the_lowest_admitting_level() {
        // echo — inert
        let echo = Profile::of(vec![{
            let mut c = Capability::new(Operation::Observe);
            c.disclosure.audience = DisclosureAudience::LocalProcess;
            c
        }]);
        assert_eq!(project(&echo), Verdict::Allowed(SafetyLevel::Inert));

        // cat ./notes — read-local
        let read = Profile::of(vec![{
            let mut c = Capability::new(Operation::Observe);
            c.locus.local = LocalLocus::Worktree;
            c.disclosure.audience = DisclosureAudience::LocalProcess;
            c
        }]);
        assert_eq!(project(&read), Verdict::Allowed(SafetyLevel::SafeRead));

        // cat ~/.ssh/id_rsa — above the authored ladder → Denied
        let home = Profile::of(vec![{
            let mut c = Capability::new(Operation::Observe);
            c.locus.local = LocalLocus::User;
            c.disclosure.audience = DisclosureAudience::LocalProcess;
            c
        }]);
        assert_eq!(project(&home), Verdict::Denied);

        // touch build/out — create·worktree·data → write-local → SafeWrite (the
        // to_legacy `_ => SafeWrite` arm; no resolver emits this yet)
        let write = Profile::of(vec![{
            let mut c = Capability::new(Operation::Create);
            c.locus.local = LocalLocus::Worktree;
            c.scale = Scale::Bounded;
            c.reversibility = Reversibility::Recoverable;
            c.persistence.level = PersistenceLevel::Data;
            c
        }]);
        assert_eq!(project(&write), Verdict::Allowed(SafetyLevel::SafeWrite));

        // an EMPTY profile must fail closed (Denied), NOT project to inert — every level
        // vacuously admits it, so the guard is what stops "resolved to nothing" = "safe".
        assert_eq!(project(&Profile::of(vec![])), Verdict::Denied);
    }

    #[test]
    fn parse_mode_reads_the_selector() {
        assert_eq!(parse_mode(Some("shadow")), Mode::Shadow);
        assert_eq!(parse_mode(Some("new")), Mode::New);
        assert_eq!(parse_mode(None), Mode::Legacy);
        assert_eq!(parse_mode(Some("garbage")), Mode::Legacy);
        assert_eq!(parse_mode(Some("")), Mode::Legacy);
    }

    #[test]
    fn apply_mode_dispatches_the_leaf_by_mode() {
        let cat = toks(&["cat", "./notes.md"]);
        let legacy = Verdict::Allowed(SafetyLevel::Inert); // legacy classifies cat as inert
        let engine = engine_verdict(&cat).expect("cat resolves"); // read-local → SafeRead
        assert_ne!(engine, legacy, "cat: the engine tightens inert → read-local");

        assert_eq!(apply_mode(Mode::Legacy, legacy, &cat), legacy, "legacy authoritative");
        assert_eq!(apply_mode(Mode::New, legacy, &cat), engine, "new: engine authoritative");
        assert_eq!(apply_mode(Mode::Shadow, legacy, &cat), legacy, "shadow: legacy still decides");

        // an unresolvable command → New falls back to legacy
        let unresolved = toks(resolve::UNRESOLVED_CMD);
        assert_eq!(apply_mode(Mode::New, legacy, &unresolved), legacy, "no resolver → legacy");
    }

    #[test]
    fn engine_verdict_is_none_for_unresearched_commands() {
        assert!(engine_verdict(&toks(resolve::UNRESOLVED_CMD)).is_none());
        assert_eq!(engine_verdict(&toks(&["echo", "hi"])), Some(Verdict::Allowed(SafetyLevel::Inert)));
        assert_eq!(
            engine_verdict(&toks(&["cat", "./notes.md"])),
            Some(Verdict::Allowed(SafetyLevel::SafeRead)),
        );
        assert_eq!(engine_verdict(&toks(&["cat", "~/.ssh/id_rsa"])), Some(Verdict::Denied));
    }

    /// The engine may deny what legacy allowed (intended tightening) or classify higher,
    /// but must **never allow what legacy denied**, nor classify lower.
    fn not_looser(legacy: Verdict, engine: Verdict) -> bool {
        match (legacy, engine) {
            (_, Verdict::Denied) => true,
            (Verdict::Denied, Verdict::Allowed(_)) => false,
            (Verdict::Allowed(l), Verdict::Allowed(e)) => e >= l,
        }
    }

    /// The rollout safety gate on hand-picked forms — including the ones the wiring and
    /// the review flushed (unrecognized/dangerous flags, and pattern-less grep, which
    /// legacy denies as a usage error).
    #[test]
    fn the_engine_is_never_looser_than_legacy() {
        let cases = [
            "echo hi", "echo", "cat ./notes.md", "cat -n ./notes.md", "cat ~/.ssh/id_rsa",
            "cat /etc/hosts", "cat a.txt b.txt", "grep foo src/main.rs", "grep -r foo src/",
            "grep -r foo ~", "grep foo bar.txt",
            // unrecognized / dangerous flags must worst-case
            "cat --unknownflag ./x", "cat -Z ./x", "grep -P foo file",
            "grep --perl-regexp foo file", "grep --wat foo file",
            // pattern-less grep (C1): legacy denies as a usage error, engine must too
            "grep", "grep -r", "grep -i", "grep -e foo", "grep -f patterns.txt",
        ];
        for cmd in cases {
            let legacy = crate::command_verdict(cmd);
            let t = toks(&cmd.split_whitespace().collect::<Vec<_>>());
            let Some(engine) = engine_verdict(&t) else { continue };
            assert!(
                not_looser(legacy, engine),
                "engine LOOSER than legacy for `{cmd}`: legacy {legacy}, engine {engine}",
            );
        }
    }

    /// The never-looser invariant above holds over the commands legacy *allowlisted*. The
    /// `developer` level is the deliberate exception: it admits well-modeled operations the
    /// hand-built allowlist could only DENY — e.g. deleting your own project files. This
    /// test pins that divergence as intended, not a regression, and as the reason `new`
    /// mode is opt-in (default `legacy` keeps the conservative behavior; §4.5).
    #[test]
    fn developer_intentionally_admits_worktree_destroy_that_legacy_denies() {
        let rm = "rm -rf ./node_modules";
        assert_eq!(crate::command_verdict(rm), Verdict::Denied, "legacy allowlist denies rm deletion");
        let engine = engine_verdict(&toks(&rm.split_whitespace().collect::<Vec<_>>())).expect("rm resolves");
        assert_eq!(engine, Verdict::Allowed(SafetyLevel::SafeWrite), "developer admits it — intended");
        assert!(!not_looser(Verdict::Denied, engine), "and it IS looser than legacy, by design");
    }

    /// The data-driven corpus gate (the systematic test C1 slipped past): run **every**
    /// command's real `examples_safe`/`examples_denied` through the engine and assert,
    /// per resolvable example, the dimensions that hold today —
    ///   1. **never looser** than legacy (engine ≤ legacy; also subsumes "an
    ///      examples_denied that resolves stays denied", since legacy denies it),
    ///   2. **justified** — every resolved capability cites a `because` (§5),
    ///   3. **total** — resolution and projection never panic.
    /// It grows automatically as commands convert; today it exercises the resolvable
    /// commands and skips the rest. Only bare single commands are comparable at the leaf
    /// (chains/redirects/substitutions are the CST's job). The full per-facet completeness
    /// dimension is the golden-profile check (`resolve::golden_profiles_cover_every_facet`)
    /// and becomes TOML-derived when commands carry profile data (§7).
    #[test]
    fn the_engine_corpus_gate() {
        let mut exercised = 0usize;
        for (name, safe, denied) in crate::registry::corpus_examples() {
            for ex in safe.iter().chain(denied.iter()) {
                if ex.contains(['|', '>', '<', '&', ';', '$', '`', '(', '\n']) {
                    continue; // not a bare single command
                }
                let t = toks(&ex.split_whitespace().collect::<Vec<_>>());
                let Some(profile) = crate::engine::resolve::resolve(&t) else { continue };
                exercised += 1;

                for c in &profile.capabilities {
                    assert!(!c.because.is_empty(), "unjustified capability for `{ex}` ({name})");
                }

                let engine = project(&profile);
                let legacy = crate::command_verdict(ex);
                assert!(
                    not_looser(legacy, engine),
                    "engine LOOSER than legacy for `{ex}` ({name}): legacy {legacy}, engine {engine}",
                );
            }
        }
        // non-vacuity: the gate must actually resolve engine examples, or it is a green
        // test proving nothing (the trap that hid its own emptiness). Every resolvable
        // command must contribute at least one example.
        assert!(exercised >= 5, "corpus gate exercised only {exercised} engine resolutions — vacuous?");
    }

    /// A broad corpus of single commands touching every resolver, in varied loci/forms, for
    /// the shadow-validation sweep below. Single commands only (no chains) so
    /// `command_verdict` equals the leaf verdict the engine would see.
    const SHADOW_CORPUS: &[&str] = &[
        // read family — engine tightens legacy's `inert` to `read-local`, and denies reads
        // above the worktree (the intended locus tightening).
        "cat notes.md", "cat /etc/hosts", "cat ~/.ssh/id_rsa", "cat -n src/main.rs",
        "head -n 5 src/main.rs", "head -20 file", "tail -f app.log", "tail -n 100 log",
        "wc -l notes.md", "wc /etc/passwd", "grep foo src/main.rs", "grep -r foo ~",
        "grep -P foo file",
        // create / mutate
        "mkdir ./build", "mkdir -p a/b/c", "mkdir /etc/x", "touch ./f", "touch ~/.bashrc",
        "touch /etc/cron.d/x",
        // destroy
        "rm ./stale.log", "rm -rf ./node_modules", "rm -rf /", "rm ~/x",
        // transfer
        "cp a b", "cp ~/.ssh/id_rsa ./x", "cp ./x ~/y", "mv a b", "mv ~/x ./y",
        "ln -s ./t ./l", "ln -s /etc/hosts hosts", "ln ~/.ssh/id_rsa ./k",
        // key=value / dashless / flag-flip
        "dd if=./a of=./b", "dd if=./x of=/dev/rdisk0", "dd if=~/.ssh/id_rsa of=./x",
        "tar czf out.tar ./src", "tar czf - ~/.ssh", "tar xzf archive.tar",
        "sed s/a/b/ file", "sed -i s/a/b/ file", "sed -i s/a/b/ /etc/x",
        "echo hi",
    ];

    /// Commands the engine INTENDS to allow at a level (or at all) that legacy blanket-denies
    /// — the engine models a tool the coarse allowlist could only refuse (rm deletion, dd,
    /// tar, sed). Every looser-than-legacy divergence must be listed here; an unlisted one is
    /// a new engine bug. (Keep in sync as resolvers/levels evolve — that is the point.)
    const INTENDED_LOOSER: &[&str] = &[
        "rm ./stale.log", "rm -rf ./node_modules",
        "dd if=./a of=./b", "dd --version",
        "tar czf out.tar ./src",
        "sed s/a/b/ file", "sed -i s/a/b/ file",
    ];

    /// Shadow validation (rerunnable, HP-19-adjacent): sweep `SHADOW_CORPUS`, comparing the
    /// engine leaf verdict to legacy. The engine may be STRICTER (a higher level, or a deny)
    /// — always fine — or INTENDED-looser, which must be enumerated in `INTENDED_LOOSER`. Any
    /// *other* looser divergence is a regression and fails here. Re-run after touching a
    /// resolver or a level to catch drift.
    #[test]
    fn shadow_validation_sweep() {
        let mut resolved = 0usize;
        for &cmd in SHADOW_CORPUS {
            let t = toks(&cmd.split_whitespace().collect::<Vec<_>>());
            let Some(engine) = engine_verdict(&t) else { continue };
            resolved += 1;
            let legacy = crate::command_verdict(cmd);
            if !not_looser(legacy, engine) {
                assert!(
                    INTENDED_LOOSER.contains(&cmd),
                    "UNEXPECTED engine-looser-than-legacy for `{cmd}`: legacy {legacy}, engine {engine} \
                     — a new bug, or add to INTENDED_LOOSER if the engine is deliberately finer here",
                );
            }
        }
        assert!(resolved >= 30, "shadow sweep resolved only {resolved} — corpus went stale?");
    }
}
