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

/// Which faces a user grant may NOT widen.
///
/// Two distinct needs, and collapsing them was a real over-deny. safe-chains' own config must not
/// be WRITTEN at all, because any write to it decides what gets approved next. Its parent directory
/// is different: writing a file into `~/.config` is ordinary, and what must not happen is the
/// directory being replaced by something pointing elsewhere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Frozen {
    /// A grant widens every face. The default.
    Nothing,
    /// A grant may not widen REBIND. Writing into the node is still grantable.
    Rebind,
    /// A grant may not widen WRITE, and therefore not REBIND either.
    Write,
}

/// A role's projection: the locus a READ reaches, the locus a WRITE reaches, the locus a REBIND
/// reaches, and whether reading it extracts a secret.
///
/// REBIND is the third face, and it answers a question the other two cannot: may this operation
/// change what the NAME refers to? `rm` removes the binding, `ln` points it somewhere else, `mv`
/// takes it away — all rebinds. `cp`, `touch` and a redirect write THROUGH the name to the bytes
/// underneath, which is an ordinary write. The distinction cannot be read off the operation alone:
/// `cp x DIR` and `ln -s y DIR` are both `create`/`transfer` to the engine.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Role {
    pub read_locus: LocalLocus,
    pub write_locus: LocalLocus,
    /// Defaults to `write_locus`; only a grant, or an explicit `rebind_locus`, separates them.
    pub rebind_locus: LocalLocus,
    pub reads_secret: bool,
    pub frozen: Frozen,
}

impl Role {
    /// Whether a grant may widen this role's WRITE face.
    fn write_grantable(&self) -> bool {
        self.frozen != Frozen::Write && self.write_locus < LocalLocus::SystemIntegrity
    }

    /// Whether a grant may widen this role's REBIND face. Strictly stronger: freezing the write
    /// necessarily freezes the rebind, since a rebind is the more destructive of the two.
    fn rebind_grantable(&self) -> bool {
        self.frozen == Frozen::Nothing && self.write_locus < LocalLocus::SystemIntegrity
    }
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
    /// Absent → the write locus. Only a role that must be writable-into but not replaceable states
    /// it separately.
    #[serde(default)]
    rebind_locus: Option<String>,
    #[serde(default)]
    reads_secret: bool,
    /// `"rebind"` or `"write"`; absent → nothing is frozen.
    #[serde(default)]
    frozen: Option<String>,
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
    /// specific: exact ≫ any prefix ≫ any segment, and within a kind, longer wins. When `fold`,
    /// comparisons are ASCII-case-insensitive — used for DENY-shield nodes on a case-insensitive
    /// filesystem (macOS), so a case-variant spelling (`~/.AWS`, `.GIT/hooks`) can't evade a
    /// credential store or a write-freeze that, on that filesystem, names the very same file.
    fn specificity(&self, path: &str, fold: bool) -> Option<usize> {
        let eq = |a: &str, b: &str| if fold { a.eq_ignore_ascii_case(b) } else { a == b };
        let starts = |h: &str, p: &str| if fold { ci_starts_with(h, p) } else { h.starts_with(p) };
        match self {
            Matcher::Exact(s) => eq(path, s).then_some(1_000_000 + s.len()),
            Matcher::Prefix(s) => {
                let dir = s.strip_suffix('/').unwrap_or(s);
                (starts(path, s.as_str()) || eq(path, dir)).then_some(1_000 + s.len())
            }
            Matcher::StringPrefix(s) => starts(path, s.as_str()).then_some(1_000 + s.len()),
            Matcher::Segment(seg) => path.split('/').any(|c| eq(c, seg)).then_some(seg.len()),
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

    /// The root this matcher occupies IN `path` — the naming test's left-hand side.
    ///
    /// Computed from the match POSITION, not from the matcher's text, because a `Segment` has no
    /// fixed root: `.aws` roots at `~/.aws` in one path and at `~/projects/app/.aws` in another,
    /// and comparing against the bare text `".aws"` would rank every grant as being below it.
    fn root_in(&self, path: &str, fold: bool) -> Option<String> {
        self.specificity(path, fold)?;
        Some(match self {
            Matcher::Exact(s) => s.clone(),
            Matcher::Prefix(s) => s.strip_suffix('/').unwrap_or(s).to_string(),
            Matcher::StringPrefix(s) => s.clone(),
            // SLICED from `path`, never rebuilt by joining components: an absolute path's leading
            // `/` is an empty first component, so re-joining silently produced `root/.ssh` for
            // `/root/.ssh/id_rsa` and the grant that named it then failed to match its own node.
            Matcher::Segment(seg) => {
                let eq = |a: &str, b: &str| if fold { a.eq_ignore_ascii_case(b) } else { a == b };
                let mut offset = 0usize;
                let mut end = None;
                for comp in path.split('/') {
                    if eq(comp, seg) {
                        end = Some(offset + comp.len());
                        break;
                    }
                    offset += comp.len() + 1;
                }
                path[..end?].to_string()
            }
        })
    }
}

/// Whether `inner` sits at or below `outer` as a path, by whole components.
///
/// `~/.ssh` is at-or-below `~/.ssh` and `~/.ssh/known_hosts` is below it, but `~/.sshfoo` is not —
/// hence the component boundary rather than a bare `starts_with`.
fn at_or_below(inner: &str, outer: &str) -> bool {
    let inner = inner.trim_end_matches('/');
    let outer = outer.trim_end_matches('/');
    // An empty `outer` would make every absolute path read as "below" it. No node roots at `/`
    // today, so this is a fail-closed backstop rather than a live case: nothing NAMES a node that
    // claims the whole filesystem.
    if outer.is_empty() {
        return false;
    }
    inner == outer || inner.strip_prefix(outer).is_some_and(|rest| rest.starts_with('/'))
}

/// Whether `remainder` (a path below a grant root) contains a hidden component — a dotfile/
/// dotdir like `.ssh`, `.env`, `.git-credentials`. Credentials and config live in these, so a
/// broad grant must not sweep them up; grant such a directory explicitly to reach inside it.
fn has_hidden_component(remainder: &str) -> bool {
    remainder.split('/').any(|seg| seg.len() > 1 && seg.starts_with('.'))
}

/// ASCII-case-insensitive `starts_with`, zero-alloc (for case-folded shield matching).
fn ci_starts_with(haystack: &str, prefix: &str) -> bool {
    haystack.len() >= prefix.len()
        && haystack.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

/// Whether a role is a PROTECTION (a credential/secret shield, the pinned config, or a
/// write-freeze) rather than an admit — it makes some face stricter than an ordinary worktree.
/// Only protection nodes are matched case-insensitively on a case-insensitive filesystem: folding
/// an ADMIT (`/tmp`, worktree) could admit a case-variant that is a DIFFERENT path on a
/// case-sensitive volume (fail-open), whereas folding a protection only ever denies more.
fn role_is_protective(role: &Role) -> bool {
    role.reads_secret
        || role.frozen != Frozen::Nothing
        || role.write_locus > LocalLocus::Worktree
        || role.read_locus > LocalLocus::WorktreeTrusted
}

struct Node {
    matcher: Matcher,
    role: Role,
    os: Option<Vec<String>>,
    /// Match this node's path case-insensitively on a case-insensitive filesystem — set for
    /// protection nodes only (see `role_is_protective`).
    fold: bool,
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
        let write_locus = parse_locus(&def.write_locus);
        Role {
            read_locus: parse_locus(&def.read_locus),
            write_locus,
            // Absent → the same rung as the write face. The two only diverge under a grant, or
            // where a role states the rebind face explicitly.
            rebind_locus: def.rebind_locus.as_deref().map(parse_locus).unwrap_or(write_locus),
            reads_secret: def.reads_secret,
            frozen: match def.frozen.as_deref() {
                None => Frozen::Nothing,
                Some("rebind") => Frozen::Rebind,
                Some("write") => Frozen::Write,
                Some(other) => panic!("regions: role `{name}` has unknown frozen face `{other}` (known: rebind, write)"),
            },
        }
    };

    // Both spellings of a `~/`-anchored node, for the same reason `grant_matchers` generates both:
    // `pathctx::resolve` deliberately does NOT fold `/Users/you/x` into `~/x` (it leaves `~` to the
    // classifiers), so a node written `~/Library/Keychains/` matched that spelling alone.
    //
    // Without a grant that failed safe — the absolute form fell through to `unknown`, which denies,
    // so nothing looked wrong. WITH a grant it did not, because a grant DOES carry both spellings:
    // the grant matched the absolute path, the shield never claimed it, and the protection came
    // off. Spelling `~/.config` absolutely was enough to make `rm -rf` of the trust root grantable.
    let nodes = file
        .region
        .iter()
        .flat_map(|r| {
            let role = role_of(&r.role);
            let node = |path: &str| Node {
                matcher: Matcher::from_path(path),
                role,
                os: r.os.clone(),
                fold: role_is_protective(&role),
            };
            let mut out = vec![node(&r.path)];
            if let Some(rest) = r.path.strip_prefix('~')
                && let Some(home) = std::env::var_os("HOME").and_then(|h| h.into_string().ok())
            {
                out.push(node(&format!("{home}{rest}")));
            }
            out
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

/// Where a grant came from. Only a grant the user wrote FOR safe-chains may name a credential
/// store: a `Read(~/.ssh/**)` rule in `~/.claude/settings.json` was written to answer Claude's
/// permission prompt, and does not say the user wants every command touching `~/.ssh` auto-approved
/// here. Borrowing those rules is a convenience, and it stops short of the expensive case.
#[derive(Clone, Copy, PartialEq)]
enum GrantSource {
    UserConfig,
    Derived,
}

struct Grant {
    matcher: Matcher,
    read: bool,
    write: bool,
    source: GrantSource,
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
    let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        return Vec::new();
    };
    let mut grants = Vec::new();
    // ~/.config/safe-chains.toml — safe-chains' own grant list (read and/or write). XDG is
    // deliberately not honored so a redirected env var can't point the trust root at an
    // agent-writable dir (see custom.rs).
    if let Ok(src) = std::fs::read_to_string(home.join(".config/safe-chains.toml")) {
        grants.extend(
            toml::from_str::<GrantFile>(&src)
                .map(|f| f.grant)
                .unwrap_or_default()
                .into_iter()
                .flat_map(|g| {
                    grant_matchers(&g.path)
                        .into_iter()
                        .map(move |m| Grant { matcher: m, read: g.read, write: g.write, source: GrantSource::UserConfig })
                }),
        );
    }
    // ~/.claude/settings.json Read(...) rules — the harness's own read approvals, honored
    // read-only (an Edit()/Write() rule never becomes a write grant). The command-grant
    // analogue lives in `allowlist.rs`.
    grants.extend(claude_settings_read_grants(&home));
    grants
}

/// A Claude Code `Read(<pattern>)` permission rule translated into a grant-path prefix — or
/// `None` when the pattern can't be a clean prefix. Only ABSOLUTE (`//…`) and HOME (`~/…`)
/// patterns become grants: a relative / gitignore-style rule describes a workspace-local read
/// that is already auto-approved, so there is nothing to widen. The result is trimmed to a
/// glob-free prefix; a mid-path glob (`//Users/*/x`) or a bare filesystem/home root is refused
/// (fail closed — a "read anything" harness rule does not turn into a filesystem free pass; the
/// user can still grant that explicitly in `~/.config/safe-chains.toml`).
fn translate_read_pattern(inner: &str) -> Option<String> {
    let inner = inner.trim();
    let base = if let Some(rest) = inner.strip_prefix("//") {
        format!("/{rest}")
    } else if inner == "~" || inner.starts_with("~/") {
        inner.to_string()
    } else {
        return None;
    };
    // Strip a trailing directory glob; remember we did, so the grant becomes a subtree Prefix
    // (trailing slash) rather than an Exact single-path match — see `Matcher::from_path`.
    let mut prefix = base.as_str();
    let mut had_glob = false;
    while let Some(p) = prefix.strip_suffix("/**").or_else(|| prefix.strip_suffix("/*")) {
        prefix = p;
        had_glob = true;
    }
    let prefix = prefix.strip_suffix('/').unwrap_or(prefix);
    if prefix.contains(['*', '?']) || prefix.is_empty() || prefix == "/" || prefix == "~" {
        return None;
    }
    Some(if had_glob { format!("{prefix}/") } else { prefix.to_string() })
}

/// Grant-path prefixes derived from `Read(...)` allow-rules in a Claude Code `settings.json`
/// body. Only `permissions.allow` is consulted — the same trusted field the command allowlist
/// reads (see `allowlist.rs`).
fn claude_read_grant_paths(settings_json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(settings_json) else {
        return Vec::new();
    };
    let Some(arr) = value
        .get("permissions")
        .and_then(|v| v.get("allow"))
        .and_then(|v| v.as_array())
    else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|e| e.as_str())
        .filter_map(|entry| entry.strip_prefix("Read(").and_then(|s| s.strip_suffix(')')))
        .filter_map(translate_read_pattern)
        .collect()
}

/// Read-only grants sourced from `~/.claude/settings.json`. Only the user-global home settings
/// are trusted; a project's `.claude/settings.json` lives in the tree the agent edits and is
/// never read (mirrors `allowlist.rs`).
fn claude_settings_read_grants(home: &std::path::Path) -> Vec<Grant> {
    let Ok(src) = std::fs::read_to_string(home.join(".claude/settings.json")) else {
        return Vec::new();
    };
    claude_read_grant_paths(&src)
        .into_iter()
        .flat_map(|p| {
            grant_matchers(&p)
                .into_iter()
                .map(|m| Grant { matcher: m, read: true, write: false, source: GrantSource::Derived })
        })
        .collect()
}

/// The matcher(s) for a grant path: the path as written, PLUS the other spelling of a home path
/// so a `~/` grant and a `/Users/you/` grant both cover a home file however the agent spells it.
fn grant_matchers(path: &str) -> Vec<Matcher> {
    let home = || std::env::var_os("HOME").and_then(|h| h.into_string().ok());
    let mut out = vec![Matcher::from_path(&as_subtree(path))];
    if let Some(rest) = path.strip_prefix('~') {
        if let Some(h) = home() {
            out.push(Matcher::from_path(&as_subtree(&format!("{h}{rest}"))));
        }
    } else if let Some(h) = home()
        && let Some(rest) = path.strip_prefix(h.as_str())
    {
        out.push(Matcher::from_path(&as_subtree(&format!("~{rest}"))));
    }
    out
}

/// A grant path in subtree form: `~/.ssh` covers `~/.ssh/id_rsa`, not just the directory entry.
///
/// Without this a grant written the natural way is nearly inert. `Matcher::from_path` reads a path
/// with no trailing slash as `Exact`, which matches the directory itself and nothing inside it, so
/// `path = "~/projects"` would grant only `~/projects` while `path = "~/projects/"` granted the
/// tree. Nobody means the first. A `Prefix` still matches the bare directory too, so the stricter
/// reading is not lost, and a `*` path keeps its `StringPrefix` form.
fn as_subtree(path: &str) -> String {
    if path.ends_with('*') || path.ends_with('/') { path.to_string() } else { format!("{path}/") }
}

#[cfg(not(test))]
static USER_GRANTS: LazyLock<Vec<Grant>> = LazyLock::new(load_user_grants);

#[cfg(test)]
thread_local! {
    static TEST_GRANTS: std::cell::RefCell<Vec<Grant>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Run `f` with the given grants active (tests only): `(path, read, write)`. These carry
/// user-config semantics, so they may name a credential store; `with_derived_grants` is the
/// borrowed-from-another-tool flavor that may not.
#[cfg(test)]
pub(crate) fn with_grants<T>(grants: &[(&str, bool, bool)], f: impl FnOnce() -> T) -> T {
    with_grants_of_kind(grants, GrantSource::UserConfig, f)
}

/// Run `f` with grants that came from another tool's config (tests only) — read-only in practice,
/// and never able to name a credential store.
#[cfg(test)]
pub(crate) fn with_derived_grants<T>(grants: &[(&str, bool, bool)], f: impl FnOnce() -> T) -> T {
    with_grants_of_kind(grants, GrantSource::Derived, f)
}

#[cfg(test)]
fn with_grants_of_kind<T>(grants: &[(&str, bool, bool)], source: GrantSource, f: impl FnOnce() -> T) -> T {
    let parsed = grants
        .iter()
        .flat_map(|&(p, read, write)| {
            grant_matchers(p).into_iter().map(move |m| Grant { matcher: m, read, write, source })
        })
        .collect();
    TEST_GRANTS.with(|g| *g.borrow_mut() = parsed);
    let out = f();
    TEST_GRANTS.with(|g| g.borrow_mut().clear());
    out
}

/// The most-specific grant matching `path`, as `(read, write)`.
///
/// `secret_root`, when set, is the root of the credential-store node covering `path`, and switches
/// on the NAMING test: a grant reaches a secret only if it names it, meaning the grant's own root
/// sits at or below that node's root. `~/.ssh` and `~/.ssh/known_hosts` name `~/.ssh`; `~/` does
/// not. Someone who grants a path inside a store has decided about the store as surely as someone
/// who grants the store itself.
///
/// The hidden-component rule below cannot stand in for this. It only covers stores that are
/// dot-prefixed, and most are not — `/etc/shadow`, `/root`, `~/Library/Keychains`,
/// `~/Library/Messages` and every browser profile under `Application Support` all have dot-free
/// remainders, so without the naming test a `~/Library/` grant would silently unlock Keychains,
/// Safari history and Messages.
fn best_grant(path: &str, secret_root: Option<&str>) -> Option<(bool, bool)> {
    let pick = |grants: &[Grant]| {
        grants
            .iter()
            .filter_map(|g| {
                let spec = g.matcher.specificity(path, false)?;
                if let Some(root) = secret_root {
                    if g.source != GrantSource::UserConfig {
                        return None;
                    }
                    // Grants never fold case, so the root is computed unfolded here too; a grant
                    // covers the spelling it names (see `role_is_protective`).
                    let grant_root = g.matcher.root_in(path, false)?;
                    if !at_or_below(&grant_root, root) {
                        return None;
                    }
                }
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

/// Widen `base` by a matching user grant. Each face is admitted only if the grant grants it —
/// `read`/`write` are independent.
///
/// A grant covers what it NAMES. That is the whole rule, and a credential store is not an
/// exception to it: a grant naming `~/.ssh` reaches `~/.ssh`, a grant on `~/` does not (the naming
/// test lives in `best_grant`). Forcing a user to acknowledge that they meant it is not this
/// program's job; declining to let a grant reach somewhere it never mentioned is.
///
/// Faces that stay frozen regardless of naming, because granting them forfeits the ability to
/// enforce anything afterwards:
///   - `frozen = "write"` — safe-chains' own config and the harness settings file it reads
///     permissions from. An agent that can write one can decide what gets approved next.
///   - `frozen = "rebind"` — the directories those files live in. Writing a file INTO `~/.config`
///     is ordinary; replacing `~/.config` itself points the trust root somewhere else.
///   - `system-integrity` — `/etc/passwd`, `/etc/sudoers`, `/etc/pam.d`, the loader and boot. These
///     decide who may log in and what they may do, so a write there is compromise-complete.
///
/// All stay READABLE by a naming grant; it is only the writing face that cannot be handed over.
fn apply_grant(path: &str, base: Role) -> Role {
    // Fail closed: a secret role whose node we cannot locate gets no grant at all, rather than
    // falling through to the un-named case.
    let secret_root = if base.reads_secret {
        match secret_node_root(path) {
            Some(root) => Some(root),
            None => return base,
        }
    } else {
        None
    };
    let Some((read, write)) = best_grant(path, secret_root.as_deref()) else {
        return base;
    };
    Role {
        read_locus: if read { base.read_locus.min(LocalLocus::WorktreeTrusted) } else { base.read_locus },
        write_locus: if write && base.write_grantable() {
            base.write_locus.min(LocalLocus::Worktree)
        } else {
            base.write_locus
        },
        rebind_locus: if write && base.rebind_grantable() {
            base.rebind_locus.min(LocalLocus::Worktree)
        } else {
            base.rebind_locus
        },
        reads_secret: base.reads_secret,
        frozen: base.frozen,
    }
}

/// The role for `path`. Most-specific applicable node wins; ties break toward the more
/// restrictive role (higher write locus, then read locus) — a safety backstop. No match →
/// fail-closed default: an absolute or home path is `unknown` (deny), a relative one is
/// `worktree`. Then a user trust grant may widen the result. `path` is expected already
/// resolved and past the `$`/`..` guard.
pub(crate) fn classify_region(path: &str) -> Role {
    if let Some(role) = scratchpad_role(path) {
        return role;
    }
    apply_grant(path, base_region(path))
}

/// This session's SCRATCHPAD — the harness's own per-session working directory — earns
/// `sandbox-scope`: a trusted working area that is not the worktree.
///
/// Why a distinct rung rather than just `temp`: the scratchpad is where an agent stages its OWN
/// work (a generated script, an extracted archive, intermediate data). Classified `temp` it is read-
/// and write-able but **not executable**, because `temp` sits BELOW the execute clause's
/// `>= sandbox-scope` floor — the floor that (correctly) treats `/tmp/x.sh` as downloaded, foreign
/// code. That floor is right for anonymous `/tmp` and wrong for the agent's own workspace, and
/// `sandbox-scope` is precisely the rung the level model reserved for "trusted, not the worktree"
/// (levels/default.toml, the executor-origin band). So a recognized scratchpad becomes runnable
/// while every other `/tmp` path stays foreign.
///
/// Recognition is anchored on the unforgeable session id, never on a guessable layout — see
/// `pathctx::in_session_scratchpad` for why that is both safe and durable. This runs BEFORE the
/// region table so the scratchpad is not first captured by the generic `/tmp` node; it deliberately
/// does NOT bypass anything else, because a non-matching path falls straight through to the normal
/// classification.
fn scratchpad_role(path: &str) -> Option<Role> {
    crate::pathctx::in_session_scratchpad(path).then_some(Role {
        read_locus: LocalLocus::SandboxScope,
        write_locus: LocalLocus::SandboxScope,
        rebind_locus: LocalLocus::SandboxScope,
        reads_secret: false,
        frozen: Frozen::Nothing,
    })
}

/// The credential-store node covering `path`, if any. Shared by `base_region` (which wants the
/// role) and `apply_grant` (which wants the root the naming test compares against) so the two can
/// never disagree about which node is in play.
fn secret_node(path: &str) -> Option<&'static Node> {
    REGIONS
        .nodes
        .iter()
        .filter(|n| n.applies_here() && n.role.reads_secret)
        .find(|n| n.matcher.specificity(path, n.fold && current_os() == "macos").is_some())
}

/// The root the naming test compares a grant against: the DEEPEST matching credential-store node.
///
/// Deepest, not first-declared. A path can match more than one secret node (`/root/.ssh/id_rsa`
/// matches both the `/root/` prefix and the `.ssh` segment), and the deeper root is the safer
/// choice because it demands a more specific grant to name it. Taking whichever node the table
/// happened to declare first would let a shallower root govern a store nested inside it, which is
/// the failure the naming test exists to prevent. Today the segment nodes are declared first and
/// happen to give the deeper root; that is an accident of file order, not a property to rely on.
fn secret_node_root(path: &str) -> Option<String> {
    let fold_shields = current_os() == "macos";
    REGIONS
        .nodes
        .iter()
        .filter(|n| n.applies_here() && n.role.reads_secret)
        .filter_map(|n| n.matcher.root_in(path, n.fold && fold_shields))
        .max_by_key(|r| (r.split('/').count(), r.len()))
}

fn base_region(path: &str) -> Role {
    let r = &*REGIONS;
    // macOS's default filesystem (APFS) is case-insensitive, so `~/.AWS` and `.GIT/hooks` name the
    // same files as `~/.aws`/`.git` — a shield must fire on the case-variant too. Admit nodes are
    // never folded (a case-variant of `/tmp` on a case-sensitive volume is a different dir).
    // Best-effort by OS, not by volume: safe-chains never inspects the filesystem (§0.2, TOCTOU), so
    // a NON-default case-insensitive Linux mount (ext4 `casefold`, ciopfs, vfat) is not covered, and
    // a case-sensitive macOS volume over-denies a genuinely-distinct `.GIT` (fail-safe). Matching the
    // OS is the honest proxy for the default case.
    let fold_shields = current_os() == "macos";

    // A SECRET-BEARING region wins outright, before specificity is considered at all.
    //
    // Specificity ranks exact ≫ prefix ≫ segment, so ANY subtree admit outranks the shield's
    // segment match however deep the shield sits. Adding read-admits for package content made that
    // concrete: `/usr/share/.ssh/id_rsa` was approved, because `/usr/share/` is a prefix and
    // `.ssh` is only a segment. The shield's whole purpose is to be un-widenable — `apply_grant`
    // already refuses to let a grant reach through it, and an admit node must not either.
    //
    // This is the same failure that retired the previous admit map: a broad prefix quietly
    // swallowing something sensitive underneath it.
    if let Some(node) = secret_node(path) {
        return node.role;
    }

    let mut best: Option<(usize, Role)> = None;
    for node in &r.nodes {
        if !node.applies_here() {
            continue;
        }
        let Some(spec) = node.matcher.specificity(path, node.fold && fold_shields) else {
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
        rebind_locus: LocalLocus::Adjacent,
        reads_secret: false,
        frozen: Frozen::Nothing,
    })
}

/// Every region PATH the model declares, straight from `regions/default.toml`. Test-only, and it
/// exists so the abstraction-soundness property draws its witnesses from the region table rather
/// than from a hand-picked list: a newly-protected path becomes a witness the moment it is
/// declared, without anyone remembering to extend a corpus.
#[cfg(test)]
pub(crate) fn declared_region_paths() -> Vec<String> {
    let src = include_str!("../../../regions/default.toml");
    let file: RegionsFile = toml::from_str(src).expect("regions/default.toml is invalid TOML");
    file.region.into_iter().map(|r| r.path).collect()
}

enum PeerKind {
    /// A peer project's file — earns `adjacent`.
    ///
    /// Hidden components used to split off a `Hidden` variant here, shielding a peer's `.env`,
    /// `.github` and so on. Removed after a fortnight of real use: it fired constantly on ordinary
    /// committed content (`.github/workflows`, `.vscode`, `.cargo/config.toml`) while the things it
    /// was reaching for — `.ssh`, `.aws`, `.netrc`, `~/.config/gh` — are named by the credential
    /// shield, which is segment-matched and bites at any depth in any project. Hidden-ness was a
    /// second, structural vote on secrecy layered over a shield that already names what is secret,
    /// and it disagreed with the first vote far more often than it added to it.
    ///
    /// What this did give up: a peer's `.env`, `.npmrc` and `.git/config` are now readable — as the
    /// SAME files already were in the workspace the agent is rooted at.
    Ordinary,
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
    let _ = under_parent;
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
            let _g = enter(PathCtx { cwd: Some(root.to_string()), root: Some(root.to_string()), ..Default::default() });
            classify_region(path)
        };
        const WS: &str = "~/projects/safe-chains";

        // A sibling's ORDINARY files → adjacent (peer project the agent reaches into).
        assert_eq!(ws(WS, "~/projects/branchdiff/src/main.rs").read_locus, LocalLocus::Adjacent);
        assert_eq!(ws(WS, "~/projects/branchdiff/src/main.rs").write_locus, LocalLocus::Adjacent);
        assert_eq!(ws(WS, "~/projects/notes.txt").read_locus, LocalLocus::Adjacent, "a file peer to the workspace dir");

        // A sibling's HIDDEN files are ordinary peer content now — the dot-shield is gone, and what
        // stops a peer's secrets is the credential shield (segment-matched, any depth). `.env` and
        // `.npmrc` are the two the shield does NOT name, so they read exactly as the same files in
        // the workspace the agent is rooted at already did.
        assert_eq!(ws(WS, "~/projects/branchdiff/.env").read_locus, LocalLocus::Adjacent, "peer .env reads as peer content");
        assert_eq!(ws(WS, "~/projects/branchdiff/.npmrc").read_locus, LocalLocus::Adjacent, "peer .npmrc reads as peer content");
        assert_eq!(ws(WS, "~/projects/branchdiff/.ssh/id_rsa").read_locus, LocalLocus::Machine, "the shield still bites in a peer");
        // The .git WRITE freeze is a separate guard and is unaffected by dropping the dot-shield.
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

    /// A peer project's HIDDEN files are ordinary peer content now, and the credential shield is
    /// what still stops the secrets.
    ///
    /// The dot-shield used to freeze every hidden component under a peer. Two weeks of real use
    /// said it fired overwhelmingly on committed project content — `.github/workflows`, `.vscode`,
    /// `.cargo/config.toml` — while everything it was reaching for is NAMED by the credential
    /// shield, which is segment-matched and bites at any depth in any project. It was a second,
    /// structural vote on secrecy over a shield that already names what is secret.
    #[test]
    fn a_peers_hidden_files_are_adjacent_and_the_shield_still_holds() {
        use crate::pathctx::{enter, PathCtx};
        let at = |root: &str, path: &str| {
            let _g = enter(PathCtx { cwd: Some(root.to_string()), root: Some(root.to_string()), ..Default::default() });
            classify_region(path).read_locus
        };
        const WS: &str = "~/projects/safe-chains";

        // Hidden peer content is adjacent — the same rung its ordinary source has.
        for p in [
            "~/projects/branchdiff/.github/workflows/ci.yml",
            "~/projects/branchdiff/.vscode/settings.json",
            "~/projects/branchdiff/.cargo/config.toml",
            "~/projects/branchdiff/.env",
            "~/projects/branchdiff/sub/.config/app.toml",
        ] {
            assert_eq!(at(WS, p), LocalLocus::Adjacent, "hidden peer content should be adjacent: {p}");
        }
        // Ordinary peer source is unchanged.
        assert_eq!(at(WS, "~/projects/branchdiff/src/main.rs"), LocalLocus::Adjacent);

        // The SHIELD is what still refuses, at any depth, in a peer as anywhere else. This is the
        // half that must never regress: removing the dot rule leaned the whole guarantee onto it.
        for p in [
            "~/projects/branchdiff/.ssh/id_rsa",
            "~/projects/branchdiff/.aws/credentials",
            "~/projects/branchdiff/a/b/c/.netrc",
            "~/projects/branchdiff/deep/.gnupg/secring.gpg",
        ] {
            assert_eq!(at(WS, p), LocalLocus::Machine, "the credential shield must still bite: {p}");
        }
        // A cousin under a different parent is still not a peer at all.
        assert_ne!(at(WS, "~/other/notes.txt"), LocalLocus::Adjacent);
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
    fn translate_read_pattern_only_honors_absolute_and_home_prefixes() {
        // a directory glob (`/**`, `/*`) becomes a subtree Prefix (trailing slash)
        assert_eq!(translate_read_pattern("//Users/me/x/**"), Some("/Users/me/x/".into()));
        assert_eq!(translate_read_pattern("~/.gem/**"), Some("~/.gem/".into()));
        assert_eq!(translate_read_pattern("//Users/me/x/*"), Some("/Users/me/x/".into()));
        // a glob-free path stays an exact match (a single file or dir entry)
        assert_eq!(translate_read_pattern("~/.gem"), Some("~/.gem".into()));
        assert_eq!(translate_read_pattern("//etc/hosts"), Some("/etc/hosts".into()));
        // relative / gitignore-style / settings-dir-relative → workspace-local, not a grant
        assert_eq!(translate_read_pattern("src/**"), None);
        assert_eq!(translate_read_pattern("/logs/**"), None);
        // a mid-path glob can't be represented as a prefix
        assert_eq!(translate_read_pattern("//Users/*/mise/**"), None);
        assert_eq!(translate_read_pattern("~/**/*.pem"), None);
        // a bare filesystem / home root is too broad to honor
        assert_eq!(translate_read_pattern("//**"), None);
        assert_eq!(translate_read_pattern("~/**"), None);
        assert_eq!(translate_read_pattern("~/"), None);
        assert_eq!(translate_read_pattern(""), None);
    }

    #[test]
    fn claude_read_grant_paths_extracts_only_read_allow_rules() {
        let paths = claude_read_grant_paths(
            r#"{"permissions":{"allow":[
                "Bash(ls)","Edit(~/x/**)","Write(~/y/**)","Read(~/z/**)","WebFetch"
            ]}}"#,
        );
        assert_eq!(paths, vec!["~/z/".to_string()]);
        // malformed / missing structure → empty, never a panic
        assert!(claude_read_grant_paths("not json").is_empty());
        assert!(claude_read_grant_paths("{}").is_empty());
        assert!(claude_read_grant_paths(r#"{"permissions":{}}"#).is_empty());
        // deny/ask rules are not allow-grants
        assert!(claude_read_grant_paths(r#"{"permissions":{"deny":["Read(~/z/**)"]}}"#).is_empty());
    }

    #[test]
    fn claude_read_rule_admits_read_but_never_write() {
        let paths = claude_read_grant_paths(
            r#"{"permissions":{"allow":["Read(~/.local/share/mise/**)","Edit(~/.local/share/mise/**)"]}}"#,
        );
        assert_eq!(paths, vec!["~/.local/share/mise/".to_string()]);
        let grants: Vec<(&str, bool, bool)> = paths.iter().map(|p| (p.as_str(), true, false)).collect();
        with_grants(&grants, || {
            let r = classify_region("~/.local/share/mise/installs/python/bin/python");
            assert!(r.read_locus <= LocalLocus::WorktreeTrusted, "read admitted");
            assert!(r.write_locus > LocalLocus::Worktree, "the Edit() rule is ignored — write stays denied");
        });
    }

    #[test]
    fn a_claude_read_grant_still_respects_the_dot_rule_and_shields() {
        let paths = claude_read_grant_paths(r#"{"permissions":{"allow":["Read(~/work/**)"]}}"#);
        assert_eq!(paths, vec!["~/work/".to_string()]);
        let grants: Vec<(&str, bool, bool)> = paths.iter().map(|p| (p.as_str(), true, false)).collect();
        with_grants(&grants, || {
            assert!(classify_region("~/work/notes.txt").read_locus <= LocalLocus::WorktreeTrusted, "granted read admitted");
            // a hidden credential swept under the grant is still not widened
            assert_eq!(classify_region("~/work/.ssh/id_rsa").read_locus, LocalLocus::Machine, "hidden cred not widened");
        });
    }

    #[test]
    fn claude_settings_read_grants_reads_home_settings_only() {
        let home = tempfile::tempdir().unwrap();
        let claude = home.path().join(".claude");
        std::fs::create_dir_all(&claude).unwrap();
        std::fs::write(
            claude.join("settings.json"),
            r#"{"permissions":{"allow":["Read(~/.gem/**)","Edit(~/.gem/**)"]}}"#,
        )
        .unwrap();
        let grants = claude_settings_read_grants(home.path());
        assert!(!grants.is_empty());
        assert!(grants.iter().all(|g| g.read && !g.write), "Read() rules are read-only");
        // no settings file → empty, no panic
        let empty = tempfile::tempdir().unwrap();
        assert!(claude_settings_read_grants(empty.path()).is_empty());
    }


    /// A sample path inside `node`, the grant that NAMES it, and the grant on its PARENT.
    ///
    /// `StringPrefix` is skipped (returns `None`): a `/dev/sd*` style matcher has no component
    /// boundary, so "the parent" is not well defined. No credential store uses one today, and the
    /// caller asserts the set it covered is non-empty so this cannot silently empty the guard.
    #[cfg(test)]
    fn naming_probe(matcher: &Matcher) -> Option<(String, String, String)> {
        let parent_of = |s: &str| {
            let t = s.trim_end_matches('/');
            match t.rfind('/') {
                Some(0) => "/".to_string(),
                Some(i) => format!("{}/", &t[..i]),
                None => return None,
            }
            .into()
        };
        match matcher {
            Matcher::Exact(s) => Some((s.clone(), s.clone(), parent_of(s)?)),
            Matcher::Prefix(s) => {
                Some((format!("{s}probe"), s.clone(), parent_of(s)?))
            }
            // BOTH spellings. Probing a segment only as `~/.ssh/probe` left the absolute form
            // unexercised, and that is precisely where the root was being computed wrongly.
            Matcher::Segment(seg) => Some((format!("~/{seg}/probe"), format!("~/{seg}"), "~/".to_string())),
            Matcher::StringPrefix(_) => None,
        }
    }

    /// The absolute-form probe for a `Segment` node, which `naming_probe` gives in `~/` form.
    #[cfg(test)]
    fn absolute_segment_probe(matcher: &Matcher) -> Option<(String, String, String)> {
        let Matcher::Segment(seg) = matcher else { return None };
        Some((format!("/opt/app/{seg}/probe"), format!("/opt/app/{seg}"), "/opt/app/".to_string()))
    }

    /// Enumerated over the REAL region table, so a credential store added later is covered without
    /// anyone remembering to extend this.
    ///
    /// The rule: a grant covers what it names. Naming a store reaches it; granting the parent does
    /// not. The second half is the one with teeth, because most stores are NOT dot-prefixed
    /// (`/etc/shadow`, `/root`, `~/Library/Keychains`, `~/Library/Messages`, the browser profiles
    /// under `Application Support`) so the hidden-component rule never fires for them, and without
    /// the naming test a `~/Library/` grant would silently unlock Keychains and Messages.
    /// A grant written without a trailing slash covers the subtree, because that is what anyone
    /// writing it means. Before this, `path = "~/projects"` matched only the directory entry and
    /// nothing inside it, so a grant written the natural way was very nearly inert.
    /// The naming test must compare against the DEEPEST matching secret node, not the first one
    /// declared. `/root/.ssh/id_rsa` matches two: the `/root/` prefix (root `/root`) and the `.ssh`
    /// segment (root `/root/.ssh`). If the shallow one governed, a grant on `/root` would name the
    /// `.ssh` nested inside it. Today the segment nodes are declared first and this holds by
    /// accident of file order, which is exactly why it is pinned here.
    #[test]
    fn the_naming_test_uses_the_deepest_matching_secret_node() {
        with_os("linux", || {
            assert_eq!(secret_node_root("/root/.ssh/id_rsa").as_deref(), Some("/root/.ssh"));
            assert_eq!(secret_node_root("/root/notes.txt").as_deref(), Some("/root"));
            with_grants(&[("/root", true, true)], || {
                assert_eq!(
                    classify_region("/root/.ssh/id_rsa").read_locus,
                    LocalLocus::Machine,
                    "a grant on /root must not name the .ssh nested inside it"
                );
            });
        });
    }

    /// Freezing a trust FILE is not enough if its DIRECTORY can be replaced.
    ///
    /// With a grant on `~/.config`, every step of this was auto-approved:
    ///   rm -rf ~/.config  &&  ln -s /tmp/evil ~/.config
    /// after which safe-chains read its grants, and its `level` ceiling, out of a directory the
    /// agent controls. safe-chains classifies a path by its literal spelling and does not follow
    /// symlinks (AGENTS.md §0.2), which is right for classification and is exactly why the
    /// relocation has to be stopped at the point the directory is replaced.
    ///
    /// Derived from the trust files rather than from the directory nodes, so deleting those nodes
    /// makes this FAIL. The frozen-face guard cannot do that job: it enumerates declared nodes, so
    /// removing one leaves nothing to enumerate and it passes vacuously.
    #[test]
    fn a_trust_files_directory_cannot_be_destroyed_or_replaced() {
        let mut checked = 0usize;
        for node in REGIONS.nodes.iter().filter(|n| n.applies_here() && n.role.frozen == Frozen::Write) {
            let Matcher::Exact(path) = &node.matcher else { continue };
            let Some((parent, _)) = path.rsplit_once('/') else { continue };
            // `~` is DELIBERATELY not frozen (decided 2026-08-02), so it is skipped here rather
            // than passing quietly. Granting all of `~` write access is an intentional act by the
            // user, and one the docs already advise against; having done it, `rm -rf ~` is the
            // access they asked for. safe-chains does not second-guess a grant that broad. The
            // consequence to be aware of is that such a grant also permits relocating `~` itself,
            // and with it the trust root beneath it.
            if parent == "~" {
                continue;
            }
            with_grants(&[(parent, true, true)], || {
                for line in [format!("rm -rf {parent}"), format!("ln -s /tmp/evil {parent}")] {
                    assert!(
                        !crate::is_safe_command(&line),
                        "`{line}` relocates the trust root holding {path}"
                    );
                }
                // The other half of the trade-off: freezing the directory must not freeze what is
                // INSIDE it, or a `~/.config` grant would stop being useful for every other tool.
                assert!(
                    crate::is_safe_command(&format!("touch {parent}/ordinary.toml")),
                    "{parent}: freezing the directory must not freeze its contents"
                );
            });
            checked += 1;
        }
        assert!(checked >= 2, "only {checked} trust files probed — the guard is vacuous");
    }

    /// The rebind split, end to end: writing INTO a trust-root directory works, replacing it does not.
    ///
    /// The first freeze of these directories used the write face, which stopped the relocation but
    /// also denied `cp x ~/.config` — an ordinary thing to do in a directory you granted. Writing
    /// through a name and changing what the name refers to are different acts, and the region model
    /// now has a face for each. Both halves are asserted, because a fix that only tightened would
    /// pass a one-sided guard while leaving the over-deny in place.
    #[test]
    fn a_trust_root_directory_is_writable_into_but_not_replaceable() {
        for dir in ["~/.config", "~/.claude"] {
            with_grants(&[(dir, true, true)], || {
                for allowed in [format!("cp a.toml {dir}"), format!("mv a.toml {dir}"), format!("touch {dir}/x")] {
                    assert!(crate::is_safe_command(&allowed), "{allowed} must stay allowed");
                }
                for refused in [
                    format!("rm -rf {dir}"),
                    format!("rmdir {dir}"),
                    format!("ln -s /tmp/evil {dir}"),
                    format!("mv {dir} {dir}.bak"),
                ] {
                    assert!(!crate::is_safe_command(&refused), "{refused} relocates the trust root");
                }
            });
        }
    }

    /// A `~/`-anchored node must protect the ABSOLUTE spelling of the same directory too.
    ///
    /// `resolve` deliberately does not fold `/Users/you/x` to `~/x`, and grants cover both spellings
    /// only because `grant_matchers` generates both. Region nodes had no such treatment, so every
    /// `~/`-anchored node — the credential stores under `~/Library`, `~/.config/gh`, the trust files
    /// and their directories — matched one spelling only. Without a grant that failed safe, since
    /// the absolute form fell through to `unknown`, which is why it went unnoticed. WITH a grant it
    /// did not: the grant's own absolute matcher applied to a path the shield never claimed.
    #[test]
    fn a_home_anchored_node_protects_the_absolute_spelling_too() {
        let Some(home) = std::env::var_os("HOME").and_then(|h| h.into_string().ok()) else {
            return;
        };
        let cases = [
            ("~/.config", "the trust-root directory"),
            ("~/.config/safe-chains.toml", "safe-chains' own config"),
            ("~/.claude/settings.json", "the harness settings file"),
            ("~/Library/Keychains/login.keychain", "a credential store"),
        ];
        for (tilde, what) in cases {
            let absolute = tilde.replacen('~', &home, 1);
            // The same directory, so the same classification, whichever way it is spelled — and
            // asserted BOTH with and without a grant, since the un-granted case failed safe on its
            // own and hid the divergence until a grant was present.
            let compare = |when: &str| {
                let t = classify_region(tilde);
                let a = classify_region(&absolute);
                assert_eq!(a.reads_secret, t.reads_secret, "{absolute} ({when}): {what} loses its secret bit");
                assert_eq!(a.write_locus, t.write_locus, "{absolute} ({when}): {what} write face diverges");
                assert_eq!(a.rebind_locus, t.rebind_locus, "{absolute} ({when}): {what} rebind face diverges");
            };
            compare("no grant");
            with_grants(&[(tilde, true, true)], || compare("granted"));
        }
    }

    /// A node claiming the whole filesystem could never be NAMED by anything.
    #[test]
    fn nothing_names_a_root_that_claims_everything() {
        assert!(!at_or_below("/etc/shadow", ""), "an empty node root must not admit an absolute grant");
        assert!(!at_or_below("~/.ssh", ""));
        assert!(at_or_below("~/.ssh/known_hosts", "~/.ssh"), "a path inside the store still names it");
        assert!(!at_or_below("~/.sshfoo", "~/.ssh"), "a name-prefix neighbour does not name it");
    }

    #[test]
    fn a_grant_covers_the_subtree_however_the_path_is_spelled() {
        for spelling in ["~/projects", "~/projects/"] {
            with_grants(&[(spelling, true, true)], || {
                assert_eq!(
                    classify_region("~/projects/sibling/notes.txt").write_locus,
                    LocalLocus::Worktree,
                    "{spelling} must cover its contents"
                );
                assert_eq!(classify_region("~/projects").write_locus, LocalLocus::Worktree, "{spelling} covers the dir itself");
            });
        }
        // The component boundary still holds: a neighbour sharing a name prefix is not covered.
        with_grants(&[("~/projects", true, true)], || {
            assert_eq!(classify_region("~/projectsX/secret.txt").write_locus, LocalLocus::Machine, "~/projectsX is a different directory");
        });
    }

    #[test]
    fn a_grant_reaches_a_credential_store_only_when_it_names_it() {
        for os in ["macos", "linux"] {
            with_os(os, || {
                let mut covered = 0;
                for node in REGIONS.nodes.iter().filter(|n| n.applies_here() && n.role.reads_secret) {
                    let probes: Vec<_> = [naming_probe(&node.matcher), absolute_segment_probe(&node.matcher)]
                        .into_iter()
                        .flatten()
                        .collect();
                    if probes.is_empty() {
                        continue;
                    }
                    for (path, naming, parent) in probes {
                    assert!(
                        base_region(&path).reads_secret,
                        "{os}: probe {path} does not reach the secret node it was built from"
                    );
                    with_grants(&[(naming.as_str(), true, true)], || {
                        assert_eq!(
                            classify_region(&path).read_locus,
                            LocalLocus::WorktreeTrusted,
                            "{os}: a grant naming {naming} must reach {path}"
                        );
                    });
                    with_grants(&[(parent.as_str(), true, true)], || {
                        assert_eq!(
                            classify_region(&path).read_locus,
                            LocalLocus::Machine,
                            "{os}: a grant on the parent {parent} must NOT reach the secret at {path}"
                        );
                    });
                    covered += 1;
                    }
                }
                assert!(covered > 10, "{os}: only {covered} credential stores probed - the guard has gone vacuous");
            });
        }
    }

    /// Grants borrowed from another tool's config never name a credential store, however specific
    /// they are. A `Read(~/.ssh/**)` rule answers Claude's permission prompt; it does not say the
    /// user wants every command touching `~/.ssh` auto-approved here.
    #[test]
    fn a_derived_grant_never_names_a_credential_store() {
        for os in ["macos", "linux"] {
            with_os(os, || {
                let mut covered = 0;
                for node in REGIONS.nodes.iter().filter(|n| n.applies_here() && n.role.reads_secret) {
                    let Some((path, naming, _)) = naming_probe(&node.matcher) else {
                        continue;
                    };
                    with_derived_grants(&[(naming.as_str(), true, false)], || {
                        assert_eq!(
                            classify_region(&path).read_locus,
                            LocalLocus::Machine,
                            "{os}: a derived grant on {naming} must not reach {path}"
                        );
                    });
                    // The same grant written by the user in safe-chains' own config DOES reach it,
                    // so this guard is testing the source and not merely re-testing the naming test.
                    with_grants(&[(naming.as_str(), true, false)], || {
                        assert_eq!(classify_region(&path).read_locus, LocalLocus::WorktreeTrusted);
                    });
                    covered += 1;
                }
                assert!(covered > 10, "{os}: only {covered} stores probed - the guard has gone vacuous");
            });
        }
    }

    /// Every frozen face, enumerated over the region table, each checked on the face it froze.
    ///
    /// Granting these away forfeits the ability to enforce anything afterwards, so no grant opens
    /// them however it is spelled. Which FACE that means differs by role, and conflating the two
    /// was the over-deny this split exists to fix: `frozen = "write"` (a trust file) must refuse
    /// every write, while `frozen = "rebind"` (the directory it lives in) must still ACCEPT an
    /// ordinary write and refuse only the replacement. Reads are never frozen: `/etc/passwd` is
    /// world-readable and safe-chains' own config is readable already.
    #[test]
    fn no_grant_opens_a_frozen_face() {
        for os in ["macos", "linux"] {
            with_os(os, || {
                let (mut covered, mut rebind_only) = (0, 0);
                for node in REGIONS.nodes.iter().filter(|n| n.applies_here()) {
                    let role = node.role;
                    let system = role.write_locus >= LocalLocus::SystemIntegrity;
                    if role.frozen == Frozen::Nothing && !system {
                        continue;
                    }
                    let Some((path, naming, parent)) = naming_probe(&node.matcher) else {
                        continue;
                    };
                    for grant in [naming.as_str(), parent.as_str(), "~/", "/"] {
                        with_grants(&[(grant, true, true)], || {
                            let r = classify_region(&path);
                            // The rebind face is frozen for BOTH kinds: a write freeze implies it.
                            assert!(
                                r.rebind_locus > LocalLocus::Worktree,
                                "{os}: grant {grant} opens the frozen rebind at {path}"
                            );
                            if role.frozen == Frozen::Write || system {
                                assert!(
                                    r.write_locus > LocalLocus::Worktree,
                                    "{os}: grant {grant} opens the frozen write at {path}"
                                );
                            }
                        });
                    }
                    // The other half of the split, asserted only for the grant that actually
                    // REACHES the node. A broad `~/` grant does not reach `~/.config` at all — the
                    // dotfile rule stops it — so it proves nothing about the freeze either way.
                    if role.frozen == Frozen::Rebind {
                        with_grants(&[(naming.as_str(), true, true)], || {
                            assert!(
                                classify_region(&path).write_locus <= LocalLocus::Worktree,
                                "{os}: a rebind-only freeze must still allow writing into {path}"
                            );
                        });
                    }
                    covered += 1;
                    if role.frozen == Frozen::Rebind {
                        rebind_only += 1;
                    }
                }
                assert!(covered > 5, "{os}: only {covered} frozen nodes probed - the guard has gone vacuous");
                assert!(rebind_only >= 2, "{os}: {rebind_only} rebind-only nodes - the split is untested");
            });
        }
    }

    #[test]
    fn shields_fold_case_on_macos_so_a_case_variant_cannot_evade_them() {
        // On APFS (case-insensitive) a case-variant names the SAME file, so every protection —
        // credential stores AND the `.git`/`.envrc` write-freeze — must fire on the variant.
        with_os("macos", || {
            assert!(classify_region("~/.AWS/credentials").reads_secret, ".AWS folds to the .aws secret");
            assert!(classify_region("~/.SSH/id_rsa").reads_secret, ".SSH folds to the .ssh secret");
            assert_eq!(classify_region("~/.AWS/credentials").read_locus, LocalLocus::Machine);
            assert_eq!(classify_region("/etc/Master.Passwd").read_locus, LocalLocus::Machine, "system secret folds");
            // the agent-injectable one: a case-variant .git/.envrc WRITE stays frozen
            assert!(classify_region(".GIT/hooks/pre-commit").write_locus > LocalLocus::Worktree, ".GIT write frozen");
            assert!(classify_region(".Git/hooks/pre-commit").write_locus > LocalLocus::Worktree, "mixed-case .Git frozen");
            assert!(classify_region(".ENVRC").write_locus > LocalLocus::Worktree, ".ENVRC write frozen");
            // A grant NAMING the folded spelling reaches it: on this filesystem `~/.AWS` is the
            // very same directory as `~/.aws`, so someone who granted one granted the other.
            with_grants(&[("~/.AWS/", true, false)], || {
                assert_eq!(classify_region("~/.AWS/credentials").read_locus, LocalLocus::WorktreeTrusted, "a grant naming the folded secret reaches it");
            });
            // A grant that does NOT name it still cannot reach through the fold.
            with_grants(&[("~/", true, false)], || {
                assert_eq!(classify_region("~/.AWS/credentials").read_locus, LocalLocus::Machine, "a broad grant cannot reach a folded secret");
            });
        });
    }

    #[test]
    fn case_folding_is_macos_only_so_linux_keeps_distinct_paths() {
        // On a case-sensitive fs `.GIT` and `~/.AWS` are DIFFERENT files, not the shielded ones —
        // folding there would be a false-deny. The canonical spelling is shielded on every OS.
        with_os("linux", || {
            assert_eq!(classify_region(".GIT/hooks/pre-commit").write_locus, LocalLocus::Worktree, "linux: .GIT is an ordinary worktree path");
            assert!(!classify_region("~/.AWS/credentials").reads_secret, "linux: .AWS is not the .aws secret");
        });
        for os in ["macos", "linux"] {
            assert!(with_os(os, || classify_region("~/.aws/credentials").reads_secret), "{os}: canonical .aws shielded");
            assert!(with_os(os, || classify_region(".git/hooks/pre-commit").write_locus > LocalLocus::Worktree), "{os}: canonical .git frozen");
        }
    }

    #[test]
    fn admit_nodes_never_fold_so_a_case_variant_is_not_widened() {
        // Folding an ADMIT would be fail-OPEN on a case-sensitive volume (`/TMP` ≠ `/tmp`). So even
        // on macOS `/TMP` is NOT admitted as scratch — it fails closed to unknown.
        with_os("macos", || {
            assert!(classify_region("/tmp/x").write_locus <= LocalLocus::Worktree, "/tmp is scratch (admitted)");
            assert_eq!(classify_region("/TMP/x").write_locus, LocalLocus::Machine, "/TMP is not folded into the scratch admit");
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
    fn a_broad_grant_never_reaches_a_secret_carveout() {
        with_grants(&[("~/", true, true)], || {
            let r = classify_region("~/.ssh/id_rsa");
            assert_eq!(r.read_locus, LocalLocus::Machine, "a ~/ grant does not name ~/.ssh, so it does not reach it");
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
