//! Engine bridge (v1.4 §4.5; annex `…-engine` §4). Projects a resolved capability profile
//! back to a legacy [`Verdict`] so the existing ceiling gate (`main::run_cli`) keeps working
//! unchanged. The engine is authoritative for every command it can resolve
//! (`resolve::resolve` → `Some`); the legacy classifier handles the rest. There is no
//! opt-out — `cst::check::leaf_verdict` calls `engine_verdict(tokens).unwrap_or(legacy)`.

use std::cell::Cell;

use super::authoring::default_levels;
use super::facet::Profile;
use super::level::Level;
use super::resolve;
use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

thread_local! {
    /// The level a `--level` threshold selected, when it is one of the UPPER band
    /// (`local-admin`/`network-admin`/`yolo`) that has no 3-value legacy equivalent. When set,
    /// `project` decides via `Level::admits` against THIS level instead of the lower-band
    /// projection — the only way a profile that only an upper level admits (`git push`, `sudo`)
    /// can be approved. `None` (the default) keeps the byte-for-byte lower-band behavior, so
    /// `command_verdict` / `is_safe_command` and every existing test are unaffected.
    static EVAL_LEVEL: Cell<Option<&'static Level>> = const { Cell::new(None) };
}

/// Evaluate the enclosed classification against `level` (an upper-band level). Restores the
/// previous context on drop. Mirrors `pathctx::enter`.
pub fn enter_eval_level(level: &'static Level) -> EvalLevelGuard {
    EvalLevelGuard(EVAL_LEVEL.with(|c| c.replace(Some(level))))
}

pub struct EvalLevelGuard(Option<&'static Level>);

impl Drop for EvalLevelGuard {
    fn drop(&mut self) {
        EVAL_LEVEL.with(|c| c.set(self.0));
    }
}

/// The engine's verdict for a command whose resolver exists, or `None` if it has none
/// (the caller keeps the legacy verdict).
pub fn engine_verdict(tokens: &[Token]) -> Option<Verdict> {
    resolve::resolve(tokens).map(|p| project(&p))
}

/// Project a resolved profile to a legacy [`Verdict`]: the **lowest** authored level
/// that admits it, mapped back to its legacy [`SafetyLevel`]; `Denied` if no
/// legacy-mapped level admits it (above the auto-approve band → worst-case, §0).
/// `default_levels()` builds the ascending chain (paranoid ⊂ reader ⊂ editor ⊂
/// developer), so the first match among the mapped levels is the minimum.
pub fn project(profile: &Profile) -> Verdict {
    if profile.capabilities.is_empty() {
        // Fail-closed (§0): an empty profile means the resolver produced NO capability.
        // Every level vacuously admits it (`all` of zero capabilities is true), so without
        // this guard it would project to the lowest level (`paranoid`) — the *most*
        // permissive, inverting the principle. A genuinely-inert command emits an explicit
        // observe capability, never an empty profile.
        return Verdict::Denied;
    }
    if let Some(level) = EVAL_LEVEL.with(Cell::get) {
        // An upper-band `--level` is authoritative via `admits`. Pass projects to `SafeWrite`
        // — the legacy ceiling every upper level shares — so `run_cli`'s existing `<= ceiling`
        // gate accepts it; a profile the level does not admit is `Denied`, dominating the chain.
        return if level.admits(profile) {
            Verdict::Allowed(SafetyLevel::SafeWrite)
        } else {
            Verdict::Denied
        };
    }
    for level in default_levels() {
        // Only the auto-approvable band (paranoid..developer) has a 3-value legacy
        // equivalent. The levels above it (local-admin, network-admin, yolo) have NO
        // legacy mapping, so a profile that only THEY admit projects to Denied — never
        // silently to SafeWrite (the old `_ => SafeWrite` catch-all would have
        // auto-approved sudo/terraform the moment those levels were added). Selecting
        // an upper level as a threshold is the separate harness-config change.
        if let Some(sl) = to_legacy(&level.name)
            && level.admits(profile)
        {
            return Verdict::Allowed(sl);
        }
    }
    Verdict::Denied
}

fn to_legacy(level_name: &str) -> Option<SafetyLevel> {
    match level_name {
        "paranoid" => Some(SafetyLevel::Inert),
        "reader" => Some(SafetyLevel::SafeRead),
        "editor" | "developer" => Some(SafetyLevel::SafeWrite),
        _ => None, // local-admin, network-admin, yolo — above the legacy 3-value ceiling
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

    /// The fail-open this refactor had to avoid: the levels above developer (local-admin,
    /// network-admin, yolo) have NO legacy `SafetyLevel`, so a profile only they admit must
    /// project to `Denied` — never to `SafeWrite`. The old `_ => SafeWrite` catch-all in
    /// `to_legacy` would have auto-approved every one of these.
    #[test]
    fn profiles_needing_an_upper_level_project_to_denied_not_safewrite() {
        // sudo systemctl restart — elevated authority on machine locus (local-admin)
        let sudo = Profile::of(vec![{
            let mut c = Capability::new(Operation::Control);
            c.locus.local = LocalLocus::Machine;
            c.authority = Authority::Root;
            c
        }]);
        assert_eq!(project(&sudo), Verdict::Denied, "sudo must not auto-approve");

        // terraform apply — remote reach over outbound network (network-admin)
        let remote = Profile::of(vec![{
            let mut c = Capability::new(Operation::Mutate);
            c.locus.remote = RemoteReach::Fixed;
            c.network.direction = NetDirection::Outbound;
            c
        }]);
        assert_eq!(project(&remote), Verdict::Denied, "remote infra must not auto-approve");

        // terraform destroy — irreversible remote destroy (yolo only)
        let catastrophe = Profile::of(vec![{
            let mut c = Capability::new(Operation::Destroy);
            c.locus.remote = RemoteReach::Fixed;
            c.reversibility = Reversibility::Irreversible;
            c
        }]);
        assert_eq!(project(&catastrophe), Verdict::Denied, "irreversible destroy must not auto-approve");
    }

    /// The legacy classifier's leaf verdict for `cmd` — what the engine falls back to for a
    /// command it can't resolve, and the baseline the never-looser gates compare against.
    fn legacy(cmd: &str) -> Verdict {
        crate::handlers::dispatch(&toks(&cmd.split_whitespace().collect::<Vec<_>>()))
    }

    #[test]
    fn the_engine_is_authoritative_with_legacy_fallback() {
        // a resolved command → the engine's (finer) verdict, end to end
        assert_eq!(
            crate::command_verdict("cat ./notes.md"),
            Verdict::Allowed(SafetyLevel::SafeRead),
            "cat resolves → engine tightens inert to read-local",
        );
        // an unresolvable command → the legacy classifier still decides
        let unresolved = resolve::UNRESOLVED_CMD.join(" ");
        assert_eq!(
            crate::command_verdict(&unresolved),
            legacy(&unresolved),
            "no resolver → legacy verdict",
        );
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
            // PCRE (-P/--perl-regexp) is benign — PCRE2 execs no code, just a regex engine
            "grep -P foo file", "grep -oP foo file", "grep --perl-regexp foo file",
            // unrecognized / dangerous flags must worst-case
            "cat --unknownflag ./x", "cat -Z ./x", "grep --wat foo file",
            // pattern-less grep (C1): legacy denies as a usage error, engine must too
            "grep", "grep -r", "grep -i", "grep -e foo", "grep -f patterns.txt",
        ];
        for cmd in cases {
            let base = legacy(cmd);
            let t = toks(&cmd.split_whitespace().collect::<Vec<_>>());
            let Some(engine) = engine_verdict(&t) else { continue };
            assert!(
                not_looser(base, engine),
                "engine LOOSER than legacy for `{cmd}`: legacy {base}, engine {engine}",
            );
        }
    }

    /// The never-looser invariant above holds over the commands legacy *allowlisted*. The
    /// `developer` level is the deliberate exception: it admits well-modeled operations the
    /// hand-built allowlist could only DENY — e.g. deleting your own project files. This
    /// test pins that divergence as intended, not a regression: it is exactly the kind of
    /// finer classification the engine exists to make, now that it is authoritative.
    #[test]
    fn developer_intentionally_admits_worktree_destroy_that_legacy_denies() {
        let rm = "rm -rf ./node_modules";
        assert_eq!(legacy(rm), Verdict::Denied, "legacy allowlist denies rm deletion");
        assert_eq!(crate::command_verdict(rm), Verdict::Allowed(SafetyLevel::SafeWrite), "engine (developer) admits it — intended");
        assert!(!not_looser(Verdict::Denied, Verdict::Allowed(SafetyLevel::SafeWrite)), "and it IS looser than legacy, by design");
    }

    /// sed/tar keep coarse legacy HANDLERS (`coreutils::sed`/`tar`) that `handlers::dispatch`
    /// consults before the TOML — so `legacy()` for them is that handler, which denies an in-place
    /// edit. The behavioral engine models `sed -i` on a worktree file correctly (a SafeWrite) and is
    /// authoritative. Pin the divergence as intended (not a regression) — the same shape as `rm`
    /// above — because the corpus gate's sed examples deliberately avoid this looser case.
    #[test]
    fn engine_intentionally_admits_worktree_in_place_edit_that_legacy_sed_handler_denies() {
        let sed = "sed -i s/a/b/ ./file.txt";
        assert_eq!(legacy(sed), Verdict::Denied, "legacy sed handler denies in-place edit");
        assert_eq!(crate::command_verdict(sed), Verdict::Allowed(SafetyLevel::SafeWrite), "engine admits worktree -i — intended");
        assert!(!not_looser(Verdict::Denied, Verdict::Allowed(SafetyLevel::SafeWrite)), "and it IS looser than the legacy sed handler, by design");
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

                // A PROFILED sub's legacy kind is deny-all — a fail-closed placeholder for when the
                // engine ABSTAINS (a global flag before the sub), NOT a real hand-built verdict. So the
                // never-looser comparison is meaningless for it: the engine is authoritative and
                // legitimately admits below the line (`npm ci --ignore-scripts` at developer). Its
                // landing is pinned by the archetype tests, not here.
                if crate::registry::sub_archetypes(&t).is_some() {
                    continue;
                }
                let engine = project(&profile);
                let base = legacy(ex);
                assert!(
                    not_looser(base, engine),
                    "engine LOOSER than legacy for `{ex}` ({name}): legacy {base}, engine {engine}",
                );
            }
        }
        // non-vacuity: the gate must actually resolve engine examples, or it is a green
        // test proving nothing (the trap that hid its own emptiness). Every resolvable
        // command must contribute at least one example.
        assert!(exercised >= 5, "corpus gate exercised only {exercised} engine resolutions — vacuous?");
    }

    /// Per-level threshold wiring end to end: an UPPER-band `--level` classifies via `admits`,
    /// unlocking profiles that only an upper level admits, while the lower band and the
    /// allowlist-only fail-closed reflex are untouched.
    #[test]
    fn upper_band_levels_admit_via_the_engine_end_to_end() {
        let net = crate::upper_level_by_name("network-admin").expect("network-admin exists");
        let yolo = crate::upper_level_by_name("yolo").expect("yolo exists");

        // git push origin — a network-admin op. THE payoff: denied at the default (developer)
        // band, admitted once the threshold IS an upper level.
        assert_eq!(crate::command_verdict("git push origin main"), Verdict::Denied, "developer denies push");
        assert!(crate::command_verdict_at_level("git push origin main", net).is_allowed(), "network-admin admits push");
        assert!(crate::command_verdict_at_level("git push origin main", yolo).is_allowed(), "yolo admits push");

        // rm -rf / — the one thing even yolo denies (destroy·irreversible·unbounded).
        assert_eq!(crate::command_verdict_at_level("rm -rf /", yolo), Verdict::Denied, "yolo denies rm -rf /");

        // a plain read passes at every upper level (they extend reader).
        assert!(crate::command_verdict_at_level("cat ./README.md", net).is_allowed(), "reads pass at network-admin");

        // a legacy-DENIED / unmodeled command stays denied even at yolo — allowlist-only: what
        // the engine cannot certify, no threshold can approve.
        assert_eq!(crate::command_verdict_at_level("frobnicate --wombat", yolo), Verdict::Denied, "unmodeled denied at yolo");

        // a chain is admitted only if EVERY segment is (a Denied dominates the combine).
        assert_eq!(crate::command_verdict_at_level("git push && rm -rf /", yolo), Verdict::Denied, "one bad segment sinks the chain");

        // the upper-band lookup rejects lower-band and unknown names (they keep the 3-value ceiling).
        assert!(crate::upper_level_by_name("developer").is_none());
        assert!(crate::upper_level_by_name("reader").is_none());
        assert!(crate::upper_level_by_name("nonsense").is_none());

        // the lower band is UNCHANGED — no eval-level context, projection still tightens cat to read.
        assert_eq!(crate::command_verdict("cat ./README.md"), Verdict::Allowed(SafetyLevel::SafeRead), "lower band untouched");
    }
}
