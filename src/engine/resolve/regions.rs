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
    /// A user grant may NOT widen this role (like a secret store). Used for safe-chains' own
    /// config: an agent must not be able to grant itself write access to the file that governs it.
    pub pinned: bool,
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
    pinned: bool,
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

    /// The part of `path` below this matcher's root — used to keep a grant from widening a
    /// HIDDEN (dot-prefixed) file or dir it swept up. A `~/` grant matches `~/.ssh` and
    /// `~/projects`, but only the latter's remainder is dot-free.
    fn remainder<'a>(&self, path: &'a str) -> &'a str {
        match self {
            Matcher::Prefix(s) | Matcher::StringPrefix(s) => path.strip_prefix(s.as_str()).unwrap_or(""),
            Matcher::Exact(_) => "",
            Matcher::Segment(_) => path,
        }
    }
}

/// Whether `remainder` (a path below a grant root) contains a hidden component — a dotfile/
/// dotdir like `.ssh`, `.env`, `.git-credentials`. Credentials and config live in these, so a
/// broad grant must not sweep them up; grant such a directory explicitly to reach inside it.
fn has_hidden_component(remainder: &str) -> bool {
    remainder.split('/').any(|seg| seg.len() > 1 && seg.starts_with('.'))
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
            pinned: def.pinned,
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

// Grants are read from the user config in the real binary; tests inject them via `with_grants`.
#[cfg(not(test))]
#[derive(Deserialize)]
struct GrantEntry {
    path: String,
    #[serde(default)]
    read: bool,
    #[serde(default)]
    write: bool,
}

#[cfg(not(test))]
#[derive(Deserialize)]
struct GrantFile {
    #[serde(default)]
    grant: Vec<GrantEntry>,
}

#[cfg(not(test))]
fn load_user_grants() -> Vec<Grant> {
    if std::env::var_os("SAFE_CHAINS_NO_LOCAL").is_some() {
        return Vec::new();
    }
    // ~/.config/safe-chains.toml only — `XDG_CONFIG_HOME` is deliberately not honored so a
    // redirected env var can't point the trust root at an agent-writable dir (see custom.rs).
    let Some(dir) = std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config"))
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
        .flat_map(|g| grant_matchers(&g.path).into_iter().map(move |m| Grant { matcher: m, read: g.read, write: g.write }))
        .collect()
}

/// The matcher(s) for a grant path: the path as written, PLUS the other spelling of a home path
/// so a `~/` grant and a `/Users/you/` grant both cover a home file however the agent spells it.
fn grant_matchers(path: &str) -> Vec<Matcher> {
    let home = || std::env::var_os("HOME").and_then(|h| h.into_string().ok());
    let mut out = vec![Matcher::from_path(path)];
    if let Some(rest) = path.strip_prefix('~') {
        if let Some(h) = home() {
            out.push(Matcher::from_path(&format!("{h}{rest}")));
        }
    } else if let Some(h) = home()
        && let Some(rest) = path.strip_prefix(h.as_str())
    {
        out.push(Matcher::from_path(&format!("~{rest}")));
    }
    out
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
        .flat_map(|&(p, read, write)| grant_matchers(p).into_iter().map(move |m| Grant { matcher: m, read, write }))
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
            .filter_map(|g| {
                let spec = g.matcher.specificity(path)?;
                // A grant never widens a hidden file/dir it happened to sweep up (`~/` grant vs
                // `~/.git-credentials`); grant the dotdir explicitly to reach inside it.
                (!has_hidden_component(g.matcher.remainder(path))).then_some((spec, g.read, g.write))
            })
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
    if base.reads_secret || base.pinned {
        return base; // a grant never widens a secret store or safe-chains' own config
    }
    let Some((read, write)) = best_grant(path) else {
        return base;
    };
    Role {
        read_locus: if read { base.read_locus.min(LocalLocus::WorktreeTrusted) } else { base.read_locus },
        write_locus: if write { base.write_locus.min(LocalLocus::Worktree) } else { base.write_locus },
        reads_secret: base.reads_secret,
        pinned: base.pinned,
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
        // A specific region (credential shield, .git freeze) already won above; only a path matching
        // NOTHING reaches here. If it is a SIBLING of the workspace, it earns `adjacent` (a peer
        // project) rather than the `unknown`/machine deny — the co-located-repo pattern.
        adjacent_role(path).unwrap_or(r.unknown)
    } else {
        r.worktree
    }
}

/// Classify `path` as a direct SIBLING of the workspace — a peer project under the same parent
/// (`../branchdiff/src/x`) — earning the `adjacent` role (reads at reader, create/mutate at
/// developer; DESTROY stays worktree-only via the levels). `None` (→ `unknown`, denied) unless every
/// guard holds:
///  - the workspace root sits at depth >= 2 below `$HOME`, so its parent is never `$HOME` itself
///    (else a workspace at `~/work` would make `~/.ssh` a "sibling"); outside `$HOME`, no adjacency.
///  - the path is strictly UNDER the parent and NOT under the workspace itself.
///  - no HIDDEN (dot) component in the remainder below the parent — mirrors the grant shield
///    (`has_hidden_component`): a peer project's `.env`/`.git`/`.aws` stays denied, never adjacent.
///
/// `path` is already canonicalized to `~`-form; the workspace root is normalized to match.
fn adjacent_role(path: &str) -> Option<Role> {
    matches!(peer_kind(path), PeerKind::Ordinary).then_some(Role {
        read_locus: LocalLocus::Adjacent,
        write_locus: LocalLocus::Adjacent,
        reads_secret: false,
        pinned: false,
    })
}

/// Whether `path` is a peer path that would be `adjacent` EXCEPT that a hidden (dot) component
/// shields it (`../peer/.github/…`, `../peer/sub/.env`). This is NOT a deny reason of its own — the
/// shield already denies it — but it lets the overreach nudge say *why* a peer path is frozen
/// (hidden-in-peer) instead of the misleading generic "outside the working directory". `path` is in
/// the same `~`-form `adjacent_role` sees.
pub(crate) fn is_hidden_peer(path: &str) -> bool {
    matches!(peer_kind(path), PeerKind::Hidden)
}

enum PeerKind {
    /// A peer project's ordinary file — earns `adjacent`.
    Ordinary,
    /// Under the common parent, would be `adjacent`, but a hidden component shields it.
    Hidden,
    /// Not a co-located peer at all (fails a structural guard).
    NotPeer,
}

/// The single structural truth behind both `adjacent_role` and `is_hidden_peer`: is `path` a
/// co-located peer of the workspace, and if so is it shielded by a hidden component? Every guard is
/// shared so the two callers can never drift.
fn peer_kind(path: &str) -> PeerKind {
    let Some(home) = std::env::var("HOME").ok().filter(|h| h.starts_with('/')) else {
        return PeerKind::NotPeer;
    };
    let Some(root_raw) = crate::pathctx::root() else {
        return PeerKind::NotPeer;
    };
    let root = if root_raw == home {
        "~".to_string()
    } else if let Some(rest) = root_raw.strip_prefix(&home).filter(|r| r.starts_with('/')) {
        format!("~{rest}")
    } else if root_raw.starts_with('~') {
        root_raw
    } else {
        return PeerKind::NotPeer; // workspace outside $HOME (e.g. /opt/app) — conservative
    };
    let root = root.trim_end_matches('/');
    // depth >= 2 below home: root = "~/a/b…" with >= 2 components after "~".
    let Some(stripped) = root.strip_prefix("~/") else {
        return PeerKind::NotPeer;
    };
    let comps: Vec<&str> = stripped.split('/').filter(|s| !s.is_empty()).collect();
    let Some(last) = comps.last().filter(|_| comps.len() >= 2) else {
        return PeerKind::NotPeer;
    };
    let parent = &root[..root.len() - last.len() - 1]; // strip the trailing "/<last>"
    // strictly under the parent …
    let Some(under_parent) = path.strip_prefix(parent).filter(|r| r.starts_with('/')) else {
        return PeerKind::NotPeer;
    };
    // … but NOT the workspace itself or inside it.
    if path == root || path.strip_prefix(root).is_some_and(|r| r.starts_with('/')) {
        return PeerKind::NotPeer;
    }
    // a peer project's hidden files (.env/.git/.aws/.npmrc) are secrets/config — shielded, not adjacent.
    if has_hidden_component(under_parent.trim_start_matches('/')) {
        return PeerKind::Hidden;
    }
    PeerKind::Ordinary
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
    fn system_integrity_substrate_write_worst_cases_above_machine() {
        // Identity/auth files (cross-platform): readable as ordinary machine config, but a WRITE
        // worst-cases to system-integrity (above machine → above local-admin, yolo-only). The
        // loader/boot regions are `os = ["linux"]`, so they're not asserted here (this test is
        // platform-portable); their routing is the same role.
        for p in ["/etc/passwd", "/etc/group", "/etc/sudoers", "/etc/sudoers.d/pkg", "/etc/pam.d/sshd"] {
            assert_eq!(classify_region(p).write_locus, LocalLocus::SystemIntegrity, "write {p}");
            assert_eq!(classify_region(p).read_locus, LocalLocus::Machine, "read {p}");
        }
        // Ordinary /etc app config is NOT the trust substrate — it stays machine (unknown → machine).
        assert_eq!(classify_region("/etc/nginx/nginx.conf").write_locus, LocalLocus::Machine, "ordinary /etc stays machine");
        assert_eq!(classify_region("/usr/local/bin/tool").write_locus, LocalLocus::Machine, "/usr/local is admin-managed, stays machine");
    }

    /// The sibling-workspace (`adjacent`) classifier and its guards — the edge cases that make it
    /// safe rather than a home-wide hole.
    #[test]
    fn adjacent_sibling_classification() {
        use crate::pathctx::{enter, PathCtx};
        let ws = |root: &str, path: &str| {
            let _g = enter(PathCtx { cwd: Some(root.to_string()), root: Some(root.to_string()) });
            classify_region(path)
        };
        const WS: &str = "~/projects/safe-chains";

        // A sibling's ORDINARY files → adjacent (peer project the agent reaches into).
        assert_eq!(ws(WS, "~/projects/branchdiff/src/main.rs").read_locus, LocalLocus::Adjacent);
        assert_eq!(ws(WS, "~/projects/branchdiff/src/main.rs").write_locus, LocalLocus::Adjacent);
        assert_eq!(ws(WS, "~/projects/notes.txt").read_locus, LocalLocus::Adjacent, "a file peer to the workspace dir");

        // A sibling's HIDDEN files (.env/.git/.aws/.npmrc) are peer secrets/config → NOT adjacent.
        // (.git/.ssh are also caught by the segment shields; a bare .env has no shield node, so the
        // hidden-component guard is what protects it — the key edge case.)
        assert_ne!(ws(WS, "~/projects/branchdiff/.env").read_locus, LocalLocus::Adjacent, "peer .env stays denied");
        assert_ne!(ws(WS, "~/projects/branchdiff/.npmrc").read_locus, LocalLocus::Adjacent, "peer .npmrc stays denied");
        assert_eq!(ws(WS, "~/projects/branchdiff/.git/hooks/pre-commit").write_locus, LocalLocus::WorktreeTrusted, "peer .git hook stays frozen");

        // THE danger case: a workspace at `~/work` (depth 1) must NOT make `~/.ssh` / `~/x` siblings.
        assert_ne!(ws("~/work", "~/.ssh/id_rsa").read_locus, LocalLocus::Adjacent, "~/.ssh is never adjacent");
        assert_ne!(ws("~/work", "~/other-notes.txt").read_locus, LocalLocus::Adjacent, "depth-1 workspace has no siblings");
        // …nor a workspace at `~` itself (depth 0).
        assert_ne!(ws("~", "~/anything.txt").read_locus, LocalLocus::Adjacent);

        // A COUSIN (different parent) is not adjacent.
        assert_ne!(ws(WS, "~/other/thing.txt").read_locus, LocalLocus::Adjacent, "different parent → not a sibling");
        // A prefix-collision sibling name is a real sibling (peer dir), not the workspace.
        assert_eq!(ws(WS, "~/projects/safe-chains-fork/x").read_locus, LocalLocus::Adjacent);
        // The workspace's own absolute spelling is not "adjacent" (it's the workspace).
        assert_ne!(ws(WS, "~/projects/safe-chains/x").read_locus, LocalLocus::Adjacent);

        // A workspace OUTSIDE $HOME (e.g. /opt) gets no adjacency — conservative.
        assert_ne!(ws("/opt/app", "/opt/other/x").read_locus, LocalLocus::Adjacent);

        // No workspace context → no adjacency (fail-closed).
        assert_ne!(classify_region("~/projects/branchdiff/src/main.rs").read_locus, LocalLocus::Adjacent);
    }

    #[test]
    fn hidden_peer_predicate_tracks_the_dot_shield() {
        use crate::pathctx::{enter, PathCtx};
        let hp = |root: &str, path: &str| {
            let _g = enter(PathCtx { cwd: Some(root.to_string()), root: Some(root.to_string()) });
            is_hidden_peer(path)
        };
        const WS: &str = "~/projects/safe-chains";
        // A hidden file in a peer project — would be adjacent but for the dot-shield → HiddenPeer.
        assert!(hp(WS, "~/projects/branchdiff/.env"));
        assert!(hp(WS, "~/projects/branchdiff/.github/workflows/ci.yml"));
        assert!(hp(WS, "~/projects/branchdiff/sub/.config/app.toml"), "hidden component anywhere in the remainder");
        // Ordinary peer files ARE adjacent, so NOT hidden-peer — this is the pair that must never
        // collapse (else the nudge would call readable peer source a shielded path, or vice-versa).
        assert!(!hp(WS, "~/projects/branchdiff/src/main.rs"));
        // The workspace's OWN hidden file is in-workspace, not a peer.
        assert!(!hp(WS, "~/projects/safe-chains/.env"));
        // A cousin (different parent) is not a peer at all — it's genuinely outside.
        assert!(!hp(WS, "~/other/.env"));
        // Depth-1 workspace has no peers → never hidden-peer (the ~/.ssh danger case).
        assert!(!hp("~/work", "~/.ssh/id_rsa"));
        // No workspace context → not a peer (fail-closed, mirrors adjacency).
        assert!(!is_hidden_peer("~/projects/branchdiff/.env"));
    }

    #[test]
    fn most_specific_wins() {
        // the .ssh SEGMENT shield fires at any depth/spelling and reads_secret
        let ssh = classify_region("~/.ssh/id_rsa");
        assert_eq!(ssh.read_locus, LocalLocus::Machine);
        assert!(ssh.reads_secret);
        assert!(classify_region("myproj/.ssh/id_rsa").reads_secret, "segment bites a relative spelling too");
        // ~/notes has no node → unknown → denied (home is not admitted)
        assert_eq!(classify_region("~/notes.txt").read_locus, LocalLocus::Machine);
    }

    #[test]
    fn in_project_trusted_files_read_but_do_not_write() {
        let git = classify_region(".git/config");
        assert_eq!(git.read_locus, LocalLocus::WorktreeTrusted, "read is admitted at read-local");
        assert_eq!(git.write_locus, LocalLocus::WorktreeTrusted, "above the worktree write ceiling → frozen");
    }

    #[test]
    fn user_grant_widens_read_and_write() {
        with_grants(&[("~/projects/", true, true)], || {
            let r = classify_region("~/projects/other/src/main.rs");
            assert_eq!(r.write_locus, LocalLocus::Worktree, "write admitted");
            assert!(r.read_locus <= LocalLocus::WorktreeTrusted, "read admitted");
        });
        // grant gone → unknown/deny (home is not admitted)
        assert_eq!(classify_region("~/projects/other/src/main.rs").write_locus, LocalLocus::Machine);
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
    fn safe_chains_config_is_read_ok_write_denied_and_ungrantable() {
        let cfg = "~/.config/safe-chains.toml";
        assert!(classify_region(cfg).read_locus <= LocalLocus::WorktreeTrusted, "read is fine");
        assert_eq!(classify_region(cfg).write_locus, LocalLocus::Machine, "write denied");
        // even a broad ~/ grant cannot widen the write (the trust root is pinned)
        with_grants(&[("~/", true, true)], || {
            assert_eq!(classify_region(cfg).write_locus, LocalLocus::Machine, "grant can't unlock the config write");
            assert!(classify_region(cfg).read_locus <= LocalLocus::WorktreeTrusted);
        });
    }

    #[test]
    fn a_grant_does_not_widen_hidden_files_or_system_secrets() {
        with_grants(&[("~/", true, true)], || {
            assert_eq!(classify_region("~/projects/foo/main.rs").write_locus, LocalLocus::Worktree);
            // hidden dotfiles/dirs (where credentials live) are NOT swept up by a broad grant
            for p in ["~/.git-credentials", "~/.npmrc", "~/.config/gh/hosts.yml", "~/.pgpass", "~/.SSH/id_rsa"] {
                assert_eq!(classify_region(p).read_locus, LocalLocus::Machine, "hidden not widened: {p}");
            }
        });
        // a `/` grant cannot reach a system credential store (un-grantable shield)
        with_grants(&[("/", true, true)], || {
            assert_eq!(classify_region("/etc/ssl/private/server.key").read_locus, LocalLocus::Machine);
            assert_eq!(with_os("linux", || classify_region("/etc/shadow").read_locus), LocalLocus::Machine);
        });
        // an EXPLICIT dotdir grant still reaches its non-hidden contents
        with_grants(&[("~/.runner-scripts/", true, true)], || {
            assert_eq!(classify_region("~/.runner-scripts/deploy.sh").write_locus, LocalLocus::Worktree);
        });
        // macOS ~/Library credential stores are NOT dot-prefixed, so the dotfile rule can't catch
        // them under `grant ~/` — the shields must (un-grantable, like the dotdirs).
        with_grants(&[("~/", true, true)], || {
            for p in [
                "~/Library/Keychains/login.keychain-db",
                "~/Library/Cookies/Cookies.binarycookies",
                "~/Library/Application Support/Firefox/Profiles/x.default/logins.json",
                "~/Library/Application Support/Google/Chrome/Default/Login Data",
                "~/.config/git/credentials",
            ] {
                assert_eq!(with_os("macos", || classify_region(p).read_locus), LocalLocus::Machine, "shield: {p}");
            }
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
            // a redirect write honors the grant too (not just engine writers)
            assert!(crate::is_safe_command("echo hi > ~/projects/sibling/out.txt"));
        });
    }

    #[test]
    fn a_home_grant_matches_both_tilde_and_absolute_spellings() {
        let Some(home) = std::env::var_os("HOME").and_then(|h| h.into_string().ok()) else {
            return;
        };
        with_grants(&[("~/work/", true, true)], || {
            assert!(classify_region("~/work/a.txt").write_locus == LocalLocus::Worktree);
            assert!(classify_region(&format!("{home}/work/a.txt")).write_locus == LocalLocus::Worktree);
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
        assert!(file.region.len() > 10, "region set unexpectedly small ({})", file.region.len());
    }
}
