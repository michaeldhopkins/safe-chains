//! HP-20 filesystem region model. A positive, structured classifier over paths — the
//! read/write analogue of the command allowlist. `regions/default.toml` maps path shapes to
//! ROLES; each role projects to the `LocalLocus` ladder through two faces (a read face and a
//! write face) plus a `reads_secret` bit, so the same path read is safe / written is denied.
//!
//! Matching is most-specific-wins (exact > longer prefix > segment), OS-scoped to the running
//! platform, and fail-closed: an absolute/home path matching nothing is `unknown` (deny),
//! a bare relative path is `worktree`. Runs AFTER the `$VAR`/`..` guard in `locus.rs`.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::engine::facet::{FacetTerm, LocalLocus};

/// A role's projection: the locus a READ of such a path reaches, the locus a WRITE reaches,
/// and whether reading it extracts a secret.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Role {
    pub read_locus: LocalLocus,
    pub write_locus: LocalLocus,
    pub reads_secret: bool,
}

#[derive(Deserialize)]
struct RegionsFile {
    #[serde(default)]
    role: HashMap<String, RoleDef>,
    #[serde(default)]
    region: Vec<RegionDef>,
}

#[derive(Deserialize)]
struct RoleDef {
    read_locus: String,
    write_locus: String,
    #[serde(default)]
    reads_secret: bool,
    #[serde(default)]
    #[allow(dead_code)] // policy prose, not consumed by the classifier
    description: String,
}

#[derive(Deserialize)]
struct RegionDef {
    path: String,
    role: String,
    os: Option<Vec<String>>,
    // `note` / `researched` are dated provenance (mirrors `researched_version`) — parsed so
    // the file validates, but not consumed by the classifier.
    #[serde(default)]
    #[allow(dead_code)]
    note: String,
    #[serde(default)]
    #[allow(dead_code)]
    researched: String,
}

enum Matcher {
    /// `/etc/hosts`, `~` — the whole path (a `Prefix` also matches its own dir, sans slash).
    Exact(String),
    /// `/proc/sys/` — the subtree under it (path is a descendant, or the dir itself).
    Prefix(String),
    /// `/dev/sd*` — a raw string prefix (device families: matches `/dev/sda`, `/dev/sda1`).
    StringPrefix(String),
    /// `.git`, `.envrc` — any path component equal to it, at any depth.
    Segment(String),
}

impl Matcher {
    fn from_path(path: &str) -> Matcher {
        if let Some(p) = path.strip_suffix('*') {
            Matcher::StringPrefix(p.to_string())
        } else if path.ends_with('/') {
            Matcher::Prefix(path.to_string())
        } else if path.starts_with('/') || path.starts_with('~') {
            Matcher::Exact(path.to_string())
        } else {
            Matcher::Segment(path.to_string())
        }
    }

    /// Specificity of a match against `path`, or `None` if it doesn't match. Higher = more
    /// specific: exact ≫ any prefix ≫ any segment, and within a kind, longer wins.
    fn specificity(&self, path: &str) -> Option<usize> {
        match self {
            Matcher::Exact(s) => (path == s).then_some(1_000_000 + s.len()),
            Matcher::Prefix(s) => {
                let dir = s.strip_suffix('/').unwrap_or(s);
                (path.starts_with(s.as_str()) || path == dir).then_some(1_000 + s.len())
            }
            Matcher::StringPrefix(s) => path.starts_with(s.as_str()).then_some(1_000 + s.len()),
            Matcher::Segment(seg) => path.split('/').any(|c| c == seg).then_some(seg.len()),
        }
    }
}

struct Node {
    matcher: Matcher,
    role: Role,
    os: Option<Vec<String>>,
}

impl Node {
    fn applies_here(&self) -> bool {
        match &self.os {
            None => true,
            Some(list) => list.iter().any(|o| o == current_os()),
        }
    }
}

#[cfg(test)]
thread_local! {
    static OS_OVERRIDE: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
}

/// Run `f` with the platform forced (tests only): lets the scenario suite exercise BOTH the
/// linux and macOS region sets on any host, instead of `cfg`-gating half of them away.
#[cfg(test)]
pub(crate) fn with_os<T>(os: &'static str, f: impl FnOnce() -> T) -> T {
    struct Reset(Option<&'static str>);
    impl Drop for Reset {
        fn drop(&mut self) {
            OS_OVERRIDE.with(|c| c.set(self.0));
        }
    }
    let _reset = Reset(OS_OVERRIDE.with(|c| c.replace(Some(os))));
    f()
}

fn current_os() -> &'static str {
    #[cfg(test)]
    if let Some(o) = OS_OVERRIDE.with(std::cell::Cell::get) {
        return o;
    }
    std::env::consts::OS
}

struct Regions {
    nodes: Vec<Node>,
    worktree: Role,
    unknown: Role,
}

fn parse_locus(s: &str) -> LocalLocus {
    LocalLocus::from_term(s).unwrap_or_else(|| panic!("regions: unknown locus rung `{s}`"))
}

static REGIONS: LazyLock<Regions> = LazyLock::new(|| {
    let src = include_str!("../../../regions/default.toml");
    let file: RegionsFile = toml::from_str(src).expect("regions/default.toml is invalid TOML");

    let role_of = |name: &str| -> Role {
        let def = file
            .role
            .get(name)
            .unwrap_or_else(|| panic!("regions: role `{name}` is not defined"));
        Role {
            read_locus: parse_locus(&def.read_locus),
            write_locus: parse_locus(&def.write_locus),
            reads_secret: def.reads_secret,
        }
    };

    let nodes = file
        .region
        .iter()
        .map(|r| Node {
            matcher: Matcher::from_path(&r.path),
            role: role_of(&r.role),
            os: r.os.clone(),
        })
        .collect();

    Regions {
        nodes,
        worktree: role_of("worktree"),
        unknown: role_of("unknown"),
    }
});

// ── User trust grants ──────────────────────────────────────────────────────────────────────
// A user WIDENS the default classification for directories they own by listing them in
// `~/.config/safe-chains.toml`. A grant admits reads and/or writes under a subtree — the
// read/write asymmetry is the point (`read = true, write = false` = a readable-but-not-written
// install dir). Grants only ever widen, are user-level only (never a repo file — an agent
// could drop one to escalate), and NEVER override a secret carve-out (`~/.ssh/id_rsa` stays
// denied even under a `~/` grant).

struct Grant {
    matcher: Matcher,
    read: bool,
    write: bool,
}

#[derive(Deserialize)]
struct GrantEntry {
    path: String,
    #[serde(default)]
    read: bool,
    #[serde(default)]
    write: bool,
}

#[derive(Deserialize)]
struct GrantFile {
    #[serde(default)]
    grant: Vec<GrantEntry>,
}

fn load_user_grants() -> Vec<Grant> {
    if std::env::var_os("SAFE_CHAINS_NO_LOCAL").is_some() {
        return Vec::new();
    }
    let Some(dir) = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
    else {
        return Vec::new();
    };
    let Ok(src) = std::fs::read_to_string(dir.join("safe-chains.toml")) else {
        return Vec::new();
    };
    toml::from_str::<GrantFile>(&src)
        .map(|f| f.grant)
        .unwrap_or_default()
        .into_iter()
        .map(|g| Grant { matcher: Matcher::from_path(&g.path), read: g.read, write: g.write })
        .collect()
}

#[cfg(not(test))]
static USER_GRANTS: LazyLock<Vec<Grant>> = LazyLock::new(load_user_grants);

#[cfg(test)]
thread_local! {
    static TEST_GRANTS: std::cell::RefCell<Vec<Grant>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Run `f` with the given grants active (tests only): `(path, read, write)`.
#[cfg(test)]
pub(crate) fn with_grants<T>(grants: &[(&str, bool, bool)], f: impl FnOnce() -> T) -> T {
    let parsed = grants
        .iter()
        .map(|&(p, read, write)| Grant { matcher: Matcher::from_path(p), read, write })
        .collect();
    TEST_GRANTS.with(|g| *g.borrow_mut() = parsed);
    let out = f();
    TEST_GRANTS.with(|g| g.borrow_mut().clear());
    out
}

/// The most-specific grant matching `path`, as `(read, write)`.
fn best_grant(path: &str) -> Option<(bool, bool)> {
    let pick = |grants: &[Grant]| {
        grants
            .iter()
            .filter_map(|g| g.matcher.specificity(path).map(|s| (s, g.read, g.write)))
            .max_by_key(|&(s, ..)| s)
            .map(|(_, r, w)| (r, w))
    };
    #[cfg(test)]
    {
        TEST_GRANTS.with(|g| pick(&g.borrow()))
    }
    #[cfg(not(test))]
    {
        pick(&USER_GRANTS)
    }
}

/// Widen `base` by a matching user grant. A grant never lowers a secret carve-out, and each
/// face is admitted only if the grant grants it — `read`/`write` are independent.
fn apply_grant(path: &str, base: Role) -> Role {
    if base.reads_secret {
        return base;
    }
    let Some((read, write)) = best_grant(path) else {
        return base;
    };
    Role {
        read_locus: if read { base.read_locus.min(LocalLocus::WorktreeTrusted) } else { base.read_locus },
        write_locus: if write { base.write_locus.min(LocalLocus::Worktree) } else { base.write_locus },
        reads_secret: base.reads_secret,
    }
}

/// The role for `path`. Most-specific applicable node wins; ties break toward the more
/// restrictive role (higher write locus, then read locus) — a safety backstop. No match →
/// fail-closed default: an absolute or home path is `unknown` (deny), a relative one is
/// `worktree`. Then a user trust grant may widen the result. `path` is expected already
/// resolved and past the `$`/`..` guard.
pub(crate) fn classify_region(path: &str) -> Role {
    apply_grant(path, base_region(path))
}

fn base_region(path: &str) -> Role {
    let r = &*REGIONS;
    let mut best: Option<(usize, Role)> = None;
    for node in &r.nodes {
        if !node.applies_here() {
            continue;
        }
        let Some(spec) = node.matcher.specificity(path) else {
            continue;
        };
        let take = match best {
            None => true,
            Some((bs, br)) => spec > bs || (spec == bs && more_restrictive(node.role, br)),
        };
        if take {
            best = Some((spec, node.role));
        }
    }
    if let Some((_, role)) = best {
        return role;
    }
    if path.starts_with('/') || path.starts_with('~') {
        r.unknown
    } else {
        r.worktree
    }
}

fn more_restrictive(a: Role, b: Role) -> bool {
    (a.write_locus, a.read_locus) > (b.write_locus, b.read_locus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_region_file_compiles_and_defaults_exist() {
        // forces the LazyLock; panics here if the TOML is malformed or a role is undefined
        let _ = classify_region("/etc/hosts");
        assert_eq!(classify_region("relative/file.txt").write_locus, LocalLocus::Worktree);
        assert_eq!(classify_region("/some/unmapped/path").write_locus, LocalLocus::Machine);
        assert_eq!(classify_region("/some/unmapped/path").read_locus, LocalLocus::Machine);
    }

    #[test]
    fn most_specific_wins() {
        // ~/.ssh/ (credential) beats ~/ (home)
        let ssh = classify_region("~/.ssh/id_rsa");
        assert_eq!(ssh.read_locus, LocalLocus::Machine);
        assert!(ssh.reads_secret);
        // ~/notes falls to the ~/ home prefix
        assert_eq!(classify_region("~/notes.txt").read_locus, LocalLocus::User);
        // exact carve-out beats the containing prefix: /var/log/ is readable, auth.log isn't
        assert_eq!(classify_region("/var/log/syslog").read_locus, LocalLocus::WorktreeTrusted);
    }

    #[test]
    fn read_and_write_faces_differ_for_public_config() {
        let hosts = classify_region("/etc/hosts");
        assert_eq!(hosts.read_locus, LocalLocus::WorktreeTrusted, "read is admitted at read-local");
        assert_eq!(hosts.write_locus, LocalLocus::Machine, "write reaches system → denied");
    }

    #[test]
    fn user_grant_widens_read_and_write() {
        with_grants(&[("~/projects/", true, true)], || {
            let r = classify_region("~/projects/other/src/main.rs");
            assert_eq!(r.write_locus, LocalLocus::Worktree, "write admitted");
            assert!(r.read_locus <= LocalLocus::WorktreeTrusted, "read admitted");
        });
        // grant gone → back to home default
        assert_eq!(classify_region("~/projects/other/src/main.rs").write_locus, LocalLocus::User);
    }

    #[test]
    fn read_only_grant_admits_read_but_not_write() {
        with_grants(&[("~/.local/share/mise/", true, false)], || {
            let r = classify_region("~/.local/share/mise/installs/python/bin/python");
            assert!(r.read_locus <= LocalLocus::WorktreeTrusted, "read admitted");
            assert!(r.write_locus > LocalLocus::Worktree, "write NOT admitted");
        });
    }

    #[test]
    fn grant_never_widens_a_secret_carveout() {
        with_grants(&[("~/", true, true)], || {
            let r = classify_region("~/.ssh/id_rsa");
            assert_eq!(r.read_locus, LocalLocus::Machine, "secret stays denied under a ~/ grant");
            assert!(r.reads_secret);
        });
    }

    #[test]
    fn grant_takes_effect_end_to_end() {
        with_grants(&[("~/projects/", true, true)], || {
            assert!(crate::is_safe_command("cat ~/projects/sibling/notes.txt"));
            assert!(crate::is_safe_command("cp ./a ~/projects/sibling/b"));
        });
    }

    /// Provenance discipline (mirrors `researched_version`): no node may ship without a `note`
    /// and a `researched` date, and every referenced role must resolve.
    #[test]
    fn every_region_carries_provenance_and_a_valid_role() {
        let src = include_str!("../../../regions/default.toml");
        let file: RegionsFile = toml::from_str(src).expect("valid TOML");
        for r in &file.region {
            assert!(!r.note.trim().is_empty(), "region `{}` is missing a note", r.path);
            assert!(!r.researched.trim().is_empty(), "region `{}` is missing a researched date", r.path);
            assert!(file.role.contains_key(&r.role), "region `{}` names undefined role `{}`", r.path, r.role);
        }
        assert!(file.region.len() > 30, "seed region set unexpectedly small ({})", file.region.len());
    }
}
