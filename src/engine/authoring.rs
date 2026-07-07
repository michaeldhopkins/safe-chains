//! Compiling level TOML into [`Level`] values (v1.4 §4.1) — the analogue of
//! `build_command` for the level language.
//!
//! A `[level.<name>]` table carries an optional `extends`, a list of `allow`
//! clauses, and (for the loosest level only) `deny` clauses. Each clause maps a
//! facet key to a constraint: an ordinal `"<= term"` / `">= term"` / `"term"`
//! (exact), or a categorical term / list of terms. Compound facets are nested
//! tables (`locus = { local = "<= worktree", remote = "none" }`).
//!
//! `extends` composes upward only (R27): an extending level inherits its base's
//! allow *and* deny clauses and may add only allow clauses — declaring `deny` on an
//! extending level is a compile error.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde::Deserialize;

use super::facet::FacetTerm;
use super::level::{Clause, Level, OrdBound};

/// The default level set, compiled once from the embedded `levels/default.toml`.
pub fn default_levels() -> &'static [Level] {
    static LEVELS: LazyLock<Vec<Level>> = LazyLock::new(|| {
        build_level_set(include_str!("../../levels/default.toml"))
            .expect("embedded levels/default.toml must compile")
    });
    &LEVELS
}

/// Compile a TOML level set into levels, resolving `extends` in dependency order.
pub fn build_level_set(source: &str) -> Result<Vec<Level>, String> {
    let set: TomlLevelSet = toml::from_str(source).map_err(|e| e.to_string())?;
    let mut pending: Vec<(String, TomlLevel)> = set.level.into_iter().collect();
    let mut built: Vec<Level> = Vec::new();
    let mut by_name: BTreeMap<String, usize> = BTreeMap::new();

    while !pending.is_empty() {
        let before = pending.len();
        let mut still = Vec::new();
        for (name, tl) in pending {
            let ready = tl.extends.as_ref().is_none_or(|base| by_name.contains_key(base));
            if ready {
                let level = compile_level(name.clone(), tl, &built, &by_name)?;
                by_name.insert(name, built.len());
                built.push(level);
            } else {
                still.push((name, tl));
            }
        }
        if still.len() == before {
            let names: Vec<&String> = still.iter().map(|(n, _)| n).collect();
            return Err(format!("unresolved `extends` (cycle or missing base) among {names:?}"));
        }
        pending = still;
    }
    Ok(built)
}

fn compile_level(
    name: String,
    tl: TomlLevel,
    built: &[Level],
    by_name: &BTreeMap<String, usize>,
) -> Result<Level, String> {
    let allow = tl
        .allow
        .into_iter()
        .map(build_clause)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("level `{name}`: {e}"))?;
    let deny = tl
        .deny
        .into_iter()
        .map(build_clause)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("level `{name}`: {e}"))?;

    match tl.extends {
        Some(base_name) => {
            if !deny.is_empty() {
                return Err(format!(
                    "level `{name}` extends `{base_name}` and declares `deny` — extends only \
                     loosens (R27); author a stricter level from a lower base instead"
                ));
            }
            let idx = *by_name
                .get(&base_name)
                .ok_or_else(|| format!("level `{name}`: unknown base `{base_name}`"))?;
            let base = built
                .get(idx)
                .ok_or_else(|| format!("level `{name}`: base index out of range"))?;
            Ok(Level::extend(base, name, allow))
        }
        None => Ok(Level { name, allow, deny }),
    }
}

fn build_clause(tc: TomlClause) -> Result<Clause, String> {
    let mut c = Clause::default();
    if let Some(v) = tc.operation {
        c.operation = Some(parse_set(&v)?);
    }
    if let Some(l) = tc.locus {
        c.local_locus = opt_bound(l.local.as_deref())?;
        c.remote_reach = opt_bound(l.remote.as_deref())?;
        if let Some(b) = l.binding {
            c.remote_binding = Some(parse_set(&b)?);
        }
    }
    c.scale = opt_bound(tc.scale.as_deref())?;
    c.authority = opt_bound(tc.authority.as_deref())?;
    c.isolation = opt_bound(tc.isolation.as_deref())?;
    c.reversibility = opt_bound(tc.reversibility.as_deref())?;
    if let Some(p) = tc.persistence {
        c.persistence_level = opt_bound(p.level.as_deref())?;
        if let Some(t) = p.trigger {
            c.trigger_escape = opt_bound(t.escape.as_deref())?;
            if let Some(k) = t.kind {
                c.trigger_kind = Some(parse_set(&k)?);
            }
        }
    }
    if let Some(d) = tc.disclosure {
        c.disclosure_audience = opt_bound(d.audience.as_deref())?;
        if let Some(ch) = d.channel {
            c.disclosure_channel = Some(parse_set(&ch)?);
        }
        if let Some(pr) = d.principal {
            c.disclosure_principal = Some(parse_set(&pr)?);
        }
    }
    if let Some(s) = tc.secret {
        c.secret_level = opt_bound(s.level.as_deref())?;
        if let Some(ch) = s.channel {
            c.secret_channel = Some(parse_set(&ch)?);
        }
        if let Some(pr) = s.principal {
            c.secret_principal = Some(parse_set(&pr)?);
        }
    }
    if let Some(n) = tc.network {
        c.net_direction = opt_bound(n.direction.as_deref())?;
        c.net_destination = opt_bound(n.destination.as_deref())?;
        c.net_payload = opt_bound(n.payload.as_deref())?;
    }
    c.execution = opt_bound(tc.execution.as_deref())?;
    c.cost = opt_bound(tc.cost.as_deref())?;
    Ok(c)
}

fn opt_bound<T: FacetTerm + Ord>(s: Option<&str>) -> Result<Option<OrdBound<T>>, String> {
    s.map(parse_bound).transpose()
}

/// Parse an ordinal constraint: `"<= term"`, `">= term"`, or `"term"` (exact).
fn parse_bound<T: FacetTerm + Ord>(s: &str) -> Result<OrdBound<T>, String> {
    let s = s.trim();
    let (make, rest): (fn(T) -> OrdBound<T>, &str) =
        if let Some(rest) = s.strip_prefix("<=") {
            (OrdBound::at_most, rest)
        } else if let Some(rest) = s.strip_prefix(">=") {
            (OrdBound::at_least, rest)
        } else if let Some(rest) = s.strip_prefix('=') {
            (OrdBound::exactly, rest)
        } else {
            (OrdBound::exactly, s)
        };
    Ok(make(parse_term(rest)?))
}

fn parse_set<T: FacetTerm>(v: &StringOrVec) -> Result<Vec<T>, String> {
    v.as_slice().iter().map(|s| parse_term(s)).collect()
}

fn parse_term<T: FacetTerm>(s: &str) -> Result<T, String> {
    T::from_term(s.trim()).ok_or_else(|| format!("unknown term `{}`", s.trim()))
}

// ── the TOML schema ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TomlLevelSet {
    #[serde(default)]
    level: BTreeMap<String, TomlLevel>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlLevel {
    extends: Option<String>,
    #[serde(default)]
    allow: Vec<TomlClause>,
    #[serde(default)]
    deny: Vec<TomlClause>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct TomlClause {
    operation: Option<StringOrVec>,
    locus: Option<TomlLocus>,
    scale: Option<String>,
    authority: Option<String>,
    isolation: Option<String>,
    reversibility: Option<String>,
    persistence: Option<TomlPersistence>,
    disclosure: Option<TomlDisclosure>,
    secret: Option<TomlSecret>,
    network: Option<TomlNetwork>,
    execution: Option<String>,
    cost: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlLocus {
    local: Option<String>,
    remote: Option<String>,
    binding: Option<StringOrVec>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlPersistence {
    level: Option<String>,
    trigger: Option<TomlTrigger>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlTrigger {
    escape: Option<String>,
    kind: Option<StringOrVec>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlDisclosure {
    audience: Option<String>,
    channel: Option<StringOrVec>,
    principal: Option<StringOrVec>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlSecret {
    level: Option<String>,
    channel: Option<StringOrVec>,
    principal: Option<StringOrVec>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlNetwork {
    direction: Option<String>,
    destination: Option<String>,
    payload: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrVec {
    One(String),
    Many(Vec<String>),
}

impl StringOrVec {
    fn as_slice(&self) -> &[String] {
        match self {
            StringOrVec::One(s) => std::slice::from_ref(s),
            StringOrVec::Many(v) => v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::facet::*;

    fn level<'a>(levels: &'a [Level], name: &str) -> &'a Level {
        levels.iter().find(|l| l.name == name).expect("level exists")
    }

    fn observe_at(local: LocalLocus) -> Profile {
        let mut c = Capability::new(Operation::Observe);
        c.locus.local = local;
        Profile::of(vec![c])
    }

    #[test]
    fn the_default_ladder_compiles() {
        let levels = default_levels();
        let names: Vec<&str> = levels.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["inert", "read-local", "write-local"]);
    }

    #[test]
    fn inert_admits_a_version_probe_but_not_reading_the_worktree() {
        let levels = default_levels();
        let inert = level(levels, "inert");
        assert!(inert.admits(&observe_at(LocalLocus::Process)), "node --version");
        assert!(!inert.admits(&observe_at(LocalLocus::Worktree)), "cat ./notes is above inert");
    }

    #[test]
    fn read_local_reads_the_worktree_but_refuses_a_secret_and_a_write() {
        let levels = default_levels();
        let read_local = level(levels, "read-local");
        assert!(read_local.admits(&observe_at(LocalLocus::Worktree)), "cat ./notes");
        assert!(read_local.admits(&observe_at(LocalLocus::User)), "cat ~/.config");

        let secret = {
            let mut c = Capability::new(Operation::Observe);
            c.locus.local = LocalLocus::User;
            c.secret.level = SecretLevel::Reads;
            Profile::of(vec![c])
        };
        assert!(!read_local.admits(&secret), "cat ~/.ssh/id_rsa");
        assert!(!read_local.admits(&Profile::of(vec![Capability::new(Operation::Create)])), "a write");
    }

    #[test]
    fn write_local_writes_the_worktree_but_not_installs_or_mass_ops() {
        let levels = default_levels();
        let write_local = level(levels, "write-local");

        let touch = {
            let mut c = Capability::new(Operation::Create);
            c.locus.local = LocalLocus::Worktree;
            Profile::of(vec![c])
        };
        assert!(write_local.admits(&touch), "touch build/out");
        // still reads (inherited)
        assert!(write_local.admits(&observe_at(LocalLocus::Worktree)));

        let install = {
            let mut c = Capability::new(Operation::Create);
            c.locus.local = LocalLocus::Worktree;
            c.persistence.level = PersistenceLevel::Installing;
            Profile::of(vec![c])
        };
        assert!(!write_local.admits(&install), "installing is above write-local");
    }

    #[test]
    fn the_ladder_nests() {
        let levels = default_levels();
        let (inert, read, write) =
            (level(levels, "inert"), level(levels, "read-local"), level(levels, "write-local"));
        // everything inert admits, read-local and write-local admit too
        for local in [LocalLocus::Process, LocalLocus::Temp] {
            let p = observe_at(local);
            assert!(inert.admits(&p) && read.admits(&p) && write.admits(&p));
        }
    }

    #[test]
    fn unknown_term_is_a_compile_error() {
        let src = r#"
            [level.x]
            [[level.x.allow]]
            scale = "<= enormous"
        "#;
        let err = build_level_set(src).unwrap_err();
        assert!(err.contains("enormous"), "{err}");
    }

    #[test]
    fn unknown_facet_key_is_a_compile_error() {
        let src = r#"
            [level.x]
            [[level.x.allow]]
            operashun = ["observe"]
        "#;
        assert!(build_level_set(src).is_err());
    }

    #[test]
    fn deny_on_an_extending_level_is_rejected() {
        let src = r#"
            [level.base]
            [[level.base.allow]]
            operation = ["observe"]

            [level.child]
            extends = "base"
            [[level.child.deny]]
            operation = ["destroy"]
        "#;
        let err = build_level_set(src).unwrap_err();
        assert!(err.contains("R27"), "{err}");
    }

    #[test]
    fn missing_base_is_a_compile_error() {
        let src = r#"
            [level.child]
            extends = "ghost"
            [[level.child.allow]]
            operation = ["observe"]
        "#;
        assert!(build_level_set(src).is_err());
    }

    #[test]
    fn ordinal_operators_parse() {
        let src = r#"
            [level.x]
            [[level.x.allow]]
            scale = ">= bounded"
            reversibility = "<= recoverable"
            authority = "root"
        "#;
        let levels = build_level_set(src).expect("compiles");
        let clause = &level(&levels, "x").allow[0];
        assert_eq!(clause.scale, Some(OrdBound::at_least(Scale::Bounded)));
        assert_eq!(clause.reversibility, Some(OrdBound::at_most(Reversibility::Recoverable)));
        assert_eq!(clause.authority, Some(OrdBound::exactly(Authority::Root)));
    }

    // ── facet-monotonicity: the coherence check on the authored levels ──────────────
    //
    // A level is coherent iff making any command *less* severe never flips it from
    // admitted to denied. An allow clause with an ordinal *floor* (or an exact bound
    // on a non-minimum term) would break this — the check exists to catch that in
    // hand-authored TOML.

    use crate::engine::testgen::{arb_profile, lowered_variants};
    use proptest::prelude::*;

    fn assert_monotone_from(lvl: &Level, boundary: Capability) {
        assert!(
            lvl.admits(&Profile::of(vec![boundary.clone()])),
            "{}: boundary capability should be admitted",
            lvl.name,
        );
        for lowered in lowered_variants(&boundary) {
            assert!(
                lvl.admits(&Profile::of(vec![lowered.clone()])),
                "{}: admitted a boundary cap but denied it after lowering one facet:\n  {:?}\n  {:?}",
                lvl.name,
                boundary,
                lowered,
            );
        }
    }

    #[test]
    fn authored_levels_are_monotone_at_their_ceilings() {
        let levels = default_levels();

        let mut inert_cap = Capability::new(Operation::Observe);
        inert_cap.locus.local = LocalLocus::Temp;
        inert_cap.disclosure.audience = DisclosureAudience::LocalProcess;
        inert_cap.execution = ExecutionTrust::SelfCode;
        assert_monotone_from(level(levels, "inert"), inert_cap);

        let mut read_cap = Capability::new(Operation::Observe);
        read_cap.locus.local = LocalLocus::User;
        read_cap.secret.level = SecretLevel::UsesAmbient;
        read_cap.network.direction = NetDirection::Loopback;
        read_cap.disclosure.audience = DisclosureAudience::LocalProcess;
        read_cap.execution = ExecutionTrust::SelfCode;
        assert_monotone_from(level(levels, "read-local"), read_cap);

        let mut write_cap = Capability::new(Operation::Mutate);
        write_cap.locus.local = LocalLocus::Worktree;
        write_cap.scale = Scale::Bounded;
        write_cap.reversibility = Reversibility::Recoverable;
        write_cap.persistence.level = PersistenceLevel::Data;
        write_cap.secret.level = SecretLevel::UsesAmbient;
        write_cap.disclosure.audience = DisclosureAudience::LocalProcess;
        write_cap.execution = ExecutionTrust::CallerInline;
        assert_monotone_from(level(levels, "write-local"), write_cap);
    }

    proptest! {
        /// For any profile an authored level admits, lowering any single ordinal facet
        /// of any capability keeps the profile admitted.
        #[test]
        fn authored_levels_are_facet_monotone(profile in arb_profile()) {
            for lvl in default_levels() {
                if !lvl.admits(&profile) {
                    continue;
                }
                for (i, cap) in profile.capabilities.iter().enumerate() {
                    for lowered in lowered_variants(cap) {
                        let mut lowered_profile = profile.clone();
                        lowered_profile.capabilities[i] = lowered;
                        prop_assert!(
                            lvl.admits(&lowered_profile),
                            "{} broke facet-monotonicity",
                            lvl.name,
                        );
                    }
                }
            }
        }
    }
}
