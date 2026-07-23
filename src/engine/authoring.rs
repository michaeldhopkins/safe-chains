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

use serde::{Deserialize, Serialize};

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
        c.provenance = opt_bound(l.provenance.as_deref())?;
        if let Some(b) = l.binding {
            c.remote_binding = Some(parse_set(&b)?);
        }
    }
    c.scale = opt_bound(tc.scale.as_deref())?;
    c.retrieval = opt_bound(tc.retrieval.as_deref())?;
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
    c.execution_trust = opt_bound(tc.execution.as_deref())?;
    if let Some(sc) = tc.supply_chain {
        if let Some(s) = sc.source {
            c.supply_source = Some(parse_set(&s)?);
        }
        c.pinning = opt_bound(sc.pinning.as_deref())?;
        if let Some(e) = sc.exec_surface {
            c.exec_surface = Some(parse_set(&e)?);
        }
    }
    c.cost = opt_bound(tc.cost.as_deref())?;
    Ok(c)
}

fn opt_bound<T: FacetTerm + Ord>(s: Option<&str>) -> Result<Option<OrdBound<T>>, String> {
    s.map(parse_bound).transpose()
}

/// Parse an ordinal constraint: `"<= term"`, `">= term"`, `"term"` (exact), or a
/// two-sided range `">= lo, <= hi"` (a comma-separated floor and ceiling, order
/// insensitive). A range is the only form that pins both ends — needed where an
/// admit set is an interior band of the ladder (e.g. an executor locus that is
/// worktree-local but neither below it, `temp`, nor above it, `user`).
fn parse_bound<T: FacetTerm + Ord>(s: &str) -> Result<OrdBound<T>, String> {
    let parts: Vec<&str> = s.split(',').map(str::trim).collect();
    if parts.len() == 1 {
        let p = parts[0];
        return if let Some(rest) = p.strip_prefix("<=") {
            Ok(OrdBound::at_most(parse_term(rest)?))
        } else if let Some(rest) = p.strip_prefix(">=") {
            Ok(OrdBound::at_least(parse_term(rest)?))
        } else {
            Ok(OrdBound::exactly(parse_term(p.strip_prefix('=').unwrap_or(p))?))
        };
    }
    let (mut min, mut max) = (None, None);
    for p in parts {
        if let Some(rest) = p.strip_prefix("<=") {
            if max.replace(parse_term(rest)?).is_some() {
                return Err(format!("bound `{s}` sets `<=` more than once"));
            }
        } else if let Some(rest) = p.strip_prefix(">=") {
            if min.replace(parse_term(rest)?).is_some() {
                return Err(format!("bound `{s}` sets `>=` more than once"));
            }
        } else {
            return Err(format!("bound `{s}`: each part of a range must be `<=`/`>=`"));
        }
    }
    Ok(OrdBound { min, max })
}

fn parse_set<T: FacetTerm>(v: &StringOrVec) -> Result<Vec<T>, String> {
    v.as_slice().iter().map(|s| parse_term(s)).collect()
}

fn parse_term<T: FacetTerm>(s: &str) -> Result<T, String> {
    T::from_term(s.trim()).ok_or_else(|| format!("unknown term `{}`", s.trim()))
}

// ── the TOML schema ────────────────────────────────────────────────────────────

// Serialization mirrors deserialization so a compiled level round-trips back to
// equivalent TOML (`skip_serializing_if` keeps unset facets out of the output).

#[derive(Deserialize, Serialize)]
struct TomlLevelSet {
    #[serde(default)]
    level: BTreeMap<String, TomlLevel>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TomlLevel {
    #[serde(skip_serializing_if = "Option::is_none")]
    extends: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    allow: Vec<TomlClause>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    deny: Vec<TomlClause>,
}

#[derive(Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
struct TomlClause {
    #[serde(skip_serializing_if = "Option::is_none")]
    operation: Option<StringOrVec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    locus: Option<TomlLocus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retrieval: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    authority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    isolation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reversibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    persistence: Option<TomlPersistence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disclosure: Option<TomlDisclosure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secret: Option<TomlSecret>,
    #[serde(skip_serializing_if = "Option::is_none")]
    network: Option<TomlNetwork>,
    #[serde(skip_serializing_if = "Option::is_none")]
    execution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    supply_chain: Option<TomlSupplyChain>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cost: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TomlLocus {
    #[serde(skip_serializing_if = "Option::is_none")]
    local: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    binding: Option<StringOrVec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provenance: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TomlPersistence {
    #[serde(skip_serializing_if = "Option::is_none")]
    level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trigger: Option<TomlTrigger>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TomlTrigger {
    #[serde(skip_serializing_if = "Option::is_none")]
    escape: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<StringOrVec>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TomlDisclosure {
    #[serde(skip_serializing_if = "Option::is_none")]
    audience: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel: Option<StringOrVec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    principal: Option<StringOrVec>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TomlSecret {
    #[serde(skip_serializing_if = "Option::is_none")]
    level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel: Option<StringOrVec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    principal: Option<StringOrVec>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TomlNetwork {
    #[serde(skip_serializing_if = "Option::is_none")]
    direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TomlSupplyChain {
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<StringOrVec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pinning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exec_surface: Option<StringOrVec>,
}

#[derive(Deserialize, Serialize)]
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
        let mut names: Vec<&str> = levels.iter().map(|l| l.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            ["developer", "editor", "local-admin", "network-admin", "paranoid", "reader", "yolo"],
        );
        // yolo is a base level (carries the catastrophe `deny`), so build order isn't the ladder
        // order — but the mapped auto-approve band MUST stay ascending, since `bridge::project`
        // returns the first admitting mapped level as the minimum.
        let raw: Vec<&str> = levels.iter().map(|l| l.name.as_str()).collect();
        let pos = |n| raw.iter().position(|&x| x == n).expect("level present");
        assert!(
            pos("paranoid") < pos("reader") && pos("reader") < pos("editor") && pos("editor") < pos("developer"),
            "mapped band out of order: {raw:?}",
        );
    }

    #[test]
    fn inert_admits_a_version_probe_but_not_reading_the_worktree() {
        let levels = default_levels();
        let inert = level(levels, "paranoid");
        assert!(inert.admits(&observe_at(LocalLocus::Process)), "node --version");
        assert!(!inert.admits(&observe_at(LocalLocus::Worktree)), "cat ./notes is above paranoid");
    }

    #[test]
    fn read_local_reads_the_worktree_but_refuses_home_extraction_and_writes() {
        let levels = default_levels();
        let read_local = level(levels, "reader");
        assert!(read_local.admits(&observe_at(LocalLocus::Worktree)), "cat ./notes");
        assert!(read_local.admits(&observe_at(LocalLocus::WorktreeTrusted)), "git status reads .git");

        // home content read — denied by LOCUS, not by any secret detection
        // (cat ~/.ssh/id_rsa: locus=user, secret=none — cat extracts no credential)
        assert!(!read_local.admits(&observe_at(LocalLocus::User)), "cat ~/.ssh/id_rsa");

        // a credential-extraction command — denied by the positive secret claim
        // (security find-generic-password -w: secret=reads, regardless of locus)
        let extraction = {
            let mut c = Capability::new(Operation::Observe);
            c.secret.level = SecretLevel::Reads;
            Profile::of(vec![c])
        };
        assert!(!read_local.admits(&extraction), "keychain extraction");

        assert!(!read_local.admits(&Profile::of(vec![Capability::new(Operation::Create)])), "a write");
    }

    /// reader reads LOCAL and REMOTE alike (a pure fetch is a read), but the network read is a
    /// pure fetch, never an egress: `sends-host-data` (exfil) and any remote WRITE stay above it.
    #[test]
    fn reader_admits_a_pure_remote_fetch_but_not_exfil_or_remote_writes() {
        let reader = level(default_levels(), "reader");

        let fetch = {
            let mut c = Capability::new(Operation::Observe);
            c.locus.remote = RemoteReach::Arbitrary;
            c.network.direction = NetDirection::Outbound;
            c.network.payload = NetPayload::Fetches;
            c.disclosure.audience = DisclosureAudience::LocalProcess;
            Profile::of(vec![c])
        };
        assert!(reader.admits(&fetch), "curl GET / koyeb list — a pure remote fetch");

        // exfil: the request carries host data OUT — above reader
        let exfil = {
            let mut c = Capability::new(Operation::Observe);
            c.locus.remote = RemoteReach::Arbitrary;
            c.network.direction = NetDirection::Outbound;
            c.network.payload = NetPayload::SendsHostData;
            Profile::of(vec![c])
        };
        assert!(!reader.admits(&exfil), "sends-host-data (curl -d @secret) is not a read");

        // a remote WRITE — above reader (this is the nuance that lives on the write side)
        let remote_write = {
            let mut c = Capability::new(Operation::Mutate);
            c.locus.remote = RemoteReach::Fixed;
            c.network.direction = NetDirection::Outbound;
            Profile::of(vec![c])
        };
        assert!(!reader.admits(&remote_write), "a remote write is network-admin, not reader");

        // paranoid still blocks the network entirely
        assert!(!level(default_levels(), "paranoid").admits(&fetch), "paranoid blocks all network");
    }

    #[test]
    fn write_local_writes_the_worktree_but_not_installs_or_mass_ops() {
        let levels = default_levels();
        let write_local = level(levels, "editor");

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
    fn developer_deletes_within_the_worktree_but_not_beyond_it() {
        let levels = default_levels();
        let (write_local, developer) = (level(levels, "editor"), level(levels, "developer"));

        let destroy_at = |local| {
            let mut c = Capability::new(Operation::Destroy);
            c.locus.local = local;
            c.scale = Scale::Unbounded; // rm -rf
            c.reversibility = Reversibility::Effortful;
            Profile::of(vec![c])
        };
        // recursive/effortful worktree delete admits at developer, but not at write-local
        assert!(!write_local.admits(&destroy_at(LocalLocus::Worktree)), "rm waits for developer");
        assert!(developer.admits(&destroy_at(LocalLocus::Worktree)), "rm -rf ./node_modules");
        // .git/ (worktree-trusted), home, and system deletion stay above developer
        assert!(!developer.admits(&destroy_at(LocalLocus::WorktreeTrusted)), "rm -rf .git");
        assert!(!developer.admits(&destroy_at(LocalLocus::User)), "rm -rf ~");
        assert!(!developer.admits(&destroy_at(LocalLocus::Machine)), "rm -rf /");

        // the boundary is destroy vs create/overwrite: overwriting your own worktree file
        // (a recoverable create — echo > f, cp ./a ./b) stays at write-local, NOT developer.
        let overwrite = {
            let mut c = Capability::new(Operation::Create);
            c.locus.local = LocalLocus::Worktree;
            c.reversibility = Reversibility::Recoverable;
            c.persistence.level = PersistenceLevel::Data;
            Profile::of(vec![c])
        };
        assert!(write_local.admits(&overwrite), "cp ./a ./b is write-local (create), not developer");
        // developer still inherits every write-local grant
        let touch = {
            let mut c = Capability::new(Operation::Create);
            c.locus.local = LocalLocus::Worktree;
            Profile::of(vec![c])
        };
        assert!(developer.admits(&touch), "developer ⊇ write-local");
    }

    // Running code: the discriminator is the EXECUTOR-ORIGIN band, not blast radius. developer runs
    // code that LIVES in the worktree (bash ./x.sh) but refuses FOREIGN code below the band
    // (/tmp/x.sh, inline `python -c`) and SYSTEM code above it (~/x.sh, /usr/local/bin/x). The band's
    // FLOOR (`>= sandbox-scope`) makes locus.local non-monotone for execute, so the coherence
    // generator skips lowering it there (testgen::lowered_variants); this pins the band's exact edges
    // so a future mis-authoring can't drop the floor or slide the ceiling undetected — the coverage
    // gap that let the monotonicity break hide from the deterministic ceiling test.
    #[test]
    fn developer_runs_worktree_code_but_not_foreign_or_system() {
        let levels = default_levels();
        let developer = level(levels, "developer");
        let exec_at = |local| {
            let mut c = Capability::new(Operation::Execute);
            c.locus.local = local;
            c.execution.trust = ExecutionTrust::CallerFile; // bash ./x.sh — code from a named file
            Profile::of(vec![c])
        };
        // In the band [sandbox-scope, worktree-trusted]: worktree-local (and sibling) code runs.
        for local in [
            LocalLocus::SandboxScope,
            LocalLocus::Worktree,
            LocalLocus::Adjacent,
            LocalLocus::WorktreeTrusted,
        ] {
            assert!(developer.admits(&exec_at(local)), "developer runs worktree-scope code: {local:?}");
        }
        // Below the band: foreign/downloaded (temp) or inline (process) code is denied.
        assert!(!developer.admits(&exec_at(LocalLocus::Temp)), "bash /tmp/x.sh is foreign");
        assert!(!developer.admits(&exec_at(LocalLocus::Process)), "inline `python -c` is below the band");
        // Above the band: home/system executables are denied.
        assert!(!developer.admits(&exec_at(LocalLocus::User)), "~/x.sh waits for a higher level");
        assert!(!developer.admits(&exec_at(LocalLocus::Machine)), "/usr/local/bin/x waits for a higher level");
    }

    #[test]
    fn the_ladder_nests() {
        let levels = default_levels();
        let (inert, read, write) =
            (level(levels, "paranoid"), level(levels, "reader"), level(levels, "editor"));
        // everything inert admits, read-local and write-local admit too
        for local in [LocalLocus::Process, LocalLocus::Temp] {
            let p = observe_at(local);
            assert!(inert.admits(&p) && read.admits(&p) && write.admits(&p));
        }
    }

    /// The two admin flavors are INCOMPARABLE siblings above developer — each flexes a
    /// disjoint facet region (local-admin down into the machine, network-admin out to the
    /// network), and BOTH keep developer's `reversibility <= effortful` cap. Only yolo lifts
    /// it. This is the partial-order the old linear `SafetyLevel` enum could not express.
    #[test]
    fn the_admin_flavors_flex_disjoint_regions_and_only_yolo_is_irreversible() {
        let levels = default_levels();
        let developer = level(levels, "developer");
        let local_admin = level(levels, "local-admin");
        let network_admin = level(levels, "network-admin");
        let yolo = level(levels, "yolo");

        // sudo: elevated authority on the machine — local-admin admits, network-admin refuses
        let sudo = {
            let mut c = Capability::new(Operation::Mutate);
            c.locus.local = LocalLocus::Machine;
            c.authority = Authority::Root;
            Profile::of(vec![c])
        };
        assert!(!developer.admits(&sudo), "sudo is above developer");
        assert!(local_admin.admits(&sudo), "local-admin runs this machine");
        assert!(!network_admin.admits(&sudo), "network-admin never sudo's the box");

        // remote mutate over the network — network-admin admits, local-admin refuses
        let remote = {
            let mut c = Capability::new(Operation::Mutate);
            c.locus.remote = RemoteReach::Arbitrary;
            c.network.direction = NetDirection::Outbound;
            Profile::of(vec![c])
        };
        assert!(!developer.admits(&remote), "remote reach is above developer");
        assert!(network_admin.admits(&remote), "network-admin operates remotes");
        assert!(!local_admin.admits(&remote), "local-admin never reaches the network");

        // the reversibility spine: irreversible destroy is reserved for yolo, on ANY locus
        let irreversible = |local, remote| {
            let mut c = Capability::new(Operation::Destroy);
            c.locus.local = local;
            c.locus.remote = remote;
            c.reversibility = Reversibility::Irreversible;
            Profile::of(vec![c])
        };
        let mkfs = irreversible(LocalLocus::Device, RemoteReach::None); // disk wipe
        let tf_destroy = irreversible(LocalLocus::Process, RemoteReach::Fixed); // terraform destroy
        assert!(!local_admin.admits(&mkfs), "mkfs (irreversible) is above local-admin");
        assert!(!network_admin.admits(&tf_destroy), "terraform destroy (irreversible) is above network-admin");
        assert!(yolo.admits(&mkfs) && yolo.admits(&tf_destroy), "irreversible destroy is reserved for yolo");

        // but recoverable/effortful destruction in each direction stays at the flavor
        let effortful_machine = {
            let mut c = Capability::new(Operation::Destroy);
            c.locus.local = LocalLocus::Machine;
            c.scale = Scale::Unbounded;
            c.reversibility = Reversibility::Effortful;
            Profile::of(vec![c])
        };
        assert!(local_admin.admits(&effortful_machine), "sudo rm -rf /var (recoverable) is local-admin");
    }

    /// yolo lifts every cap EXCEPT the one catastrophe corner, carved purely by facets:
    /// `destroy · irreversible · unbounded` (rm -rf /). Everything adjacent — bounded or
    /// single-target irreversible destroy, or recoverable mass destroy — stays admitted,
    /// distinguished by facet alone, never by command name.
    #[test]
    fn yolo_denies_only_unbounded_irreversible_destroy() {
        let levels = default_levels();
        let yolo = level(levels, "yolo");

        let destroy = |scale, rev| {
            let mut c = Capability::new(Operation::Destroy);
            c.scale = scale;
            c.reversibility = rev;
            c.locus.local = LocalLocus::Machine;
            Profile::of(vec![c])
        };
        // the one refusal: rm -rf / — destroy the world, no recovery, no bound
        assert!(
            !yolo.admits(&destroy(Scale::Unbounded, Reversibility::Irreversible)),
            "rm -rf / is denied even at yolo",
        );
        // everything one facet away stays yolo-allowed, by facet:
        assert!(yolo.admits(&destroy(Scale::Bounded, Reversibility::Irreversible)), "terraform destroy (bounded)");
        assert!(yolo.admits(&destroy(Scale::Single, Reversibility::Irreversible)), "mkfs (single device)");
        assert!(yolo.admits(&destroy(Scale::Unbounded, Reversibility::Effortful)), "rm -rf ./x (recoverable)");
        // and yolo still admits the non-destroy extremes it exists for
        let wild = {
            let mut c = Capability::new(Operation::Execute);
            c.execution.trust = ExecutionTrust::NetworkSourced;
            c.locus.local = LocalLocus::Kernel;
            Profile::of(vec![c])
        };
        assert!(yolo.admits(&wild), "yolo still admits everything but the catastrophe corner");
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
    fn scalar_facet_values_parse() {
        // a set-valued facet given as a scalar (StringOrVec::One), not an array
        let src = r#"
            [level.x]
            [[level.x.allow]]
            operation = "observe"
            locus = { binding = "pinned" }
        "#;
        let levels = build_level_set(src).expect("compiles");
        let c = &level(&levels, "x").allow[0];
        assert_eq!(c.operation, Some(vec![Operation::Observe]));
        assert_eq!(c.remote_binding, Some(vec![RemoteBinding::Pinned]));
    }

    #[test]
    fn a_mutual_extends_cycle_is_a_compile_error() {
        let src = r#"
            [level.a]
            extends = "b"
            [level.b]
            extends = "a"
        "#;
        assert!(build_level_set(src).is_err());
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

    use crate::engine::testgen::{arb_capability, arb_profile, lowered_variants, predecessor};
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
        inert_cap.execution.trust = ExecutionTrust::SelfCode;
        assert_monotone_from(level(levels, "paranoid"), inert_cap);

        let mut read_cap = Capability::new(Operation::Observe);
        read_cap.locus.local = LocalLocus::WorktreeTrusted;
        read_cap.secret.level = SecretLevel::UsesAmbient;
        read_cap.network.direction = NetDirection::Loopback;
        read_cap.disclosure.audience = DisclosureAudience::LocalProcess;
        read_cap.execution.trust = ExecutionTrust::SelfCode;
        assert_monotone_from(level(levels, "reader"), read_cap);

        let mut write_cap = Capability::new(Operation::Mutate);
        write_cap.locus.local = LocalLocus::Worktree;
        write_cap.scale = Scale::Bounded;
        write_cap.reversibility = Reversibility::Recoverable;
        write_cap.persistence.level = PersistenceLevel::Data;
        write_cap.secret.level = SecretLevel::UsesAmbient;
        write_cap.disclosure.audience = DisclosureAudience::LocalProcess;
        write_cap.execution.trust = ExecutionTrust::CallerInline;
        assert_monotone_from(level(levels, "editor"), write_cap);
    }

    // ── union-level completeness: flat DNF's failure mode is a silent gap ────────────
    //
    // A level authored as a UNION of allow clauses to mean "allow almost everything" must admit a
    // capability IFF it is not in that level's ONE intended hole. A missing clause leaves an
    // accidental gap (a benign capability nothing admits → over-deny); a too-wide clause leaks the
    // hole (the corner slips in → fail-open). This proves the union has EXACTLY its declared gap —
    // the guard flat DNF needs before we lean on union constructions. Table-driven: add a row when
    // a new union-level is authored, and the whole class stays covered.
    proptest! {
        #[test]
        fn union_levels_admit_everything_but_their_declared_gap(cap in arb_capability()) {
            let gaps: &[(&str, fn(&Capability) -> bool)] = &[
                // yolo withholds only `destroy · irreversible · unbounded` (rm -rf /), carved by
                // the union of its allow clauses — never by a deny.
                ("yolo", |c: &Capability| {
                    c.operation == Operation::Destroy
                        && c.reversibility == Reversibility::Irreversible
                        && c.scale == Scale::Unbounded
                }),
            ];
            let levels = default_levels();
            for (name, gap) in gaps {
                let lvl = levels.iter().find(|l| &l.name == name).expect("level present");
                let admitted = lvl.admits(&Profile::of(vec![cap.clone()]));
                prop_assert_eq!(
                    admitted, !gap(&cap),
                    "level `{}`: capability {:?} admitted={} but intended_admit={}",
                    name, cap, admitted, !gap(&cap),
                );
            }
        }
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

    // execute·locus is the single facet `lowered_variants` skips (the executor-origin band), so the
    // proptest above cannot see a non-monotone execute band added to another level. This guard closes
    // that gap DIRECTLY at the level: for every level, admitting execute at a locus must admit it one
    // rung lower — checking the level as a whole, so a floored clause that a WIDER clause covers (a
    // level that `extend`s developer and re-admits below the band) is correctly monotone. Only the
    // levels whose executor-origin band is intentionally floored are exempt; a new non-monotone
    // execute band on any other level fails CLOSED here.
    #[test]
    fn execute_locus_is_monotone_except_the_intended_origin_bands() {
        // Only developer AUTHORS an execute·locus floor (`>= sandbox-scope`: temp/process below are
        // foreign code). network-admin `extend`s developer and inherits that clause verbatim without
        // re-admitting below it, so it shares developer's exact band. That is safe by construction:
        // `extend` only ADDS allow clauses, which only WIDEN the admit set — an extender can fill the
        // band's floor (local-admin does, via `<= machine`, and is monotone) but can never introduce a
        // floor worse than the one developer authored. So the shared band is fully pinned by
        // developer's edge test (`developer_runs_worktree_code_but_not_foreign_or_system`); any level
        // NOT listed here must be fully monotone in execute·locus, and a new base-level floor fails
        // closed until it is declared here with its own edge test.
        let intended: &[&str] = &["developer", "network-admin"];
        for lvl in default_levels() {
            if intended.contains(&lvl.name.as_str()) {
                continue;
            }
            let exec_at = |local| {
                let mut c = Capability::new(Operation::Execute);
                c.locus.local = local;
                c.execution.trust = ExecutionTrust::CallerFile;
                Profile::of(vec![c])
            };
            for local in LocalLocus::all() {
                let Some(lower) = predecessor(*local) else { continue };
                if lvl.admits(&exec_at(*local)) {
                    assert!(
                        lvl.admits(&exec_at(lower)),
                        "level `{}`: admits execute at {:?} but denies it one rung lower at {:?} — a \
                         non-monotone execute band. If deliberate, add `{}` to `intended` WITH an edge \
                         test; otherwise widen or remove the floor.",
                        lvl.name, local, lower, lvl.name,
                    );
                }
            }
        }
    }

    // ── round-trip: Level -> TOML -> Level is identity ──────────────────────────────
    //
    // The reverse of build_clause: a compiled clause serializes back to equivalent
    // TOML that recompiles to the same clause. Mirrors every operator the parser
    // produces (<=, >=, exact, and the two-sided range).

    fn bound_str<T: FacetTerm>(b: OrdBound<T>) -> String {
        match (b.min, b.max) {
            (Some(lo), Some(hi)) if lo == hi => lo.as_str().to_string(),
            (Some(lo), Some(hi)) => format!(">= {}, <= {}", lo.as_str(), hi.as_str()),
            (None, Some(hi)) => format!("<= {}", hi.as_str()),
            (Some(lo), None) => format!(">= {}", lo.as_str()),
            (None, None) => panic!("empty bound has no representation"),
        }
    }

    fn opt_bound_str<T: FacetTerm>(b: Option<OrdBound<T>>) -> Option<String> {
        b.map(bound_str)
    }

    fn set_str<T: FacetTerm>(v: &[T]) -> StringOrVec {
        StringOrVec::Many(v.iter().map(|t| t.as_str().to_string()).collect())
    }

    fn clause_to_toml(c: &Clause) -> TomlClause {
        let locus = (c.local_locus.is_some()
            || c.remote_reach.is_some()
            || c.remote_binding.is_some()
            || c.provenance.is_some())
        .then(|| TomlLocus {
            local: opt_bound_str(c.local_locus),
            remote: opt_bound_str(c.remote_reach),
            binding: c.remote_binding.as_deref().map(set_str),
            provenance: opt_bound_str(c.provenance),
        });
        let persistence = (c.persistence_level.is_some()
            || c.trigger_escape.is_some()
            || c.trigger_kind.is_some())
        .then(|| TomlPersistence {
            level: opt_bound_str(c.persistence_level),
            trigger: (c.trigger_escape.is_some() || c.trigger_kind.is_some()).then(|| TomlTrigger {
                escape: opt_bound_str(c.trigger_escape),
                kind: c.trigger_kind.as_deref().map(set_str),
            }),
        });
        let disclosure = (c.disclosure_audience.is_some()
            || c.disclosure_channel.is_some()
            || c.disclosure_principal.is_some())
        .then(|| TomlDisclosure {
            audience: opt_bound_str(c.disclosure_audience),
            channel: c.disclosure_channel.as_deref().map(set_str),
            principal: c.disclosure_principal.as_deref().map(set_str),
        });
        let secret = (c.secret_level.is_some()
            || c.secret_channel.is_some()
            || c.secret_principal.is_some())
        .then(|| TomlSecret {
            level: opt_bound_str(c.secret_level),
            channel: c.secret_channel.as_deref().map(set_str),
            principal: c.secret_principal.as_deref().map(set_str),
        });
        let network = (c.net_direction.is_some()
            || c.net_destination.is_some()
            || c.net_payload.is_some())
        .then(|| TomlNetwork {
            direction: opt_bound_str(c.net_direction),
            destination: opt_bound_str(c.net_destination),
            payload: opt_bound_str(c.net_payload),
        });
        let supply_chain = (c.supply_source.is_some()
            || c.pinning.is_some()
            || c.exec_surface.is_some())
        .then(|| TomlSupplyChain {
            source: c.supply_source.as_deref().map(set_str),
            pinning: opt_bound_str(c.pinning),
            exec_surface: c.exec_surface.as_deref().map(set_str),
        });
        TomlClause {
            operation: c.operation.as_deref().map(set_str),
            locus,
            scale: opt_bound_str(c.scale),
            retrieval: opt_bound_str(c.retrieval),
            authority: opt_bound_str(c.authority),
            isolation: opt_bound_str(c.isolation),
            reversibility: opt_bound_str(c.reversibility),
            persistence,
            disclosure,
            secret,
            network,
            execution: opt_bound_str(c.execution_trust),
            supply_chain,
            cost: opt_bound_str(c.cost),
        }
    }

    fn round_trip(levels: &[Level]) -> Vec<Level> {
        let level = levels
            .iter()
            .map(|l| {
                let tl = TomlLevel {
                    extends: None,
                    allow: l.allow.iter().map(clause_to_toml).collect(),
                    deny: l.deny.iter().map(clause_to_toml).collect(),
                };
                (l.name.clone(), tl)
            })
            .collect();
        let source = toml::to_string(&TomlLevelSet { level }).expect("serialize");
        build_level_set(&source).expect("re-parse serialized levels")
    }

    fn assert_round_trips(levels: &[Level]) {
        let round = round_trip(levels);
        for original in levels {
            let back = round.iter().find(|l| l.name == original.name).expect("level survives");
            assert_eq!(original.allow, back.allow, "{} allow clauses", original.name);
            assert_eq!(original.deny, back.deny, "{} deny clauses", original.name);
        }
    }

    #[test]
    fn authored_levels_round_trip() {
        assert_round_trips(default_levels());
    }

    #[test]
    fn every_facet_round_trips() {
        // a kitchen-sink level exercising every reverse-conversion branch
        let src = r#"
            [level.sink]
            [[level.sink.allow]]
            operation = ["observe", "create", "destroy"]
            locus = { local = "<= machine", remote = "<= fixed", binding = ["pinned", "ambient"] }
            scale = "<= bounded"
            authority = "<= root"
            isolation = "<= vm"
            reversibility = "<= effortful"
            persistence = { level = "<= installing", trigger = { escape = "<= boot", kind = ["clock", "event"] } }
            disclosure = { audience = "<= public", channel = ["filesystem", "network"], principal = ["own"] }
            secret = { level = ">= reads", channel = ["credential-store"], principal = ["cross"] }
            network = { direction = "<= outbound", destination = "<= arbitrary", payload = "<= sends-host-data" }
            execution = "<= network-sourced"
            supply_chain = { source = ["public-registry", "signed-repo"], pinning = ">= version", exec_surface = ["build-script", "install-hook"] }
            cost = "<= quota"
            [[level.sink.deny]]
            operation = ["destroy"]
            reversibility = ">= irreversible"
        "#;
        let levels = build_level_set(src).expect("compiles");
        assert_round_trips(&levels);
    }
}
