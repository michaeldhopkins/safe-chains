//! How much a write depends on the folder it runs in, for a harness that does not say which folder
//! that is (docs/design/unknown-folder-writes.md §4–§6).
//!
//! Pure: a path and a level in, an answer out. The ambient state (which level is in force, what a
//! command wrote) lives in `folder`.

/// How much one write depends on the folder the command runs in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    /// The target is pinned without the folder: absolute, `~`, a stream; or, declared on a command
    /// (`writes_cwd = "none"`), whatever it writes is somewhere fixed (`pkill`, `rustup target add`).
    Free,
    /// A relative path that stays below the folder and names nothing sensitive.
    RelativePlain,
    /// A relative path whose name means something wherever it lands: a hidden segment, a git hook,
    /// a credential file name, a region the table names under the workspace or under `~`.
    RelativeSensitive,
    /// A relative path that climbs out of the folder, or whose spelling cannot be read (`$VAR`, a
    /// glob, a substitution).
    RelativeUnplaced,
    /// The tool writes, without naming a path, only into a subtree of the folder it owns and
    /// regenerates (`cargo build` → `target/`).
    ImplicitOutput,
    /// The tool rewrites existing files of its own kind of project in the folder (`cargo fmt`,
    /// `git commit -am x`).
    ImplicitSource,
    /// The command runs code the folder holds (`cargo run`, `swift build`).
    RunsFolderCode,
    /// Declared on a command (`writes_cwd = "named"`): it writes only the paths it names, each
    /// judged by its own anchor value (`touch`, `tee`, `sed -i`). Not a decompressor, whose output
    /// name is derived from its input, nor a sync that brings the source's names.
    NamesItsWrites,
}

impl Anchor {
    pub fn name(self) -> &'static str {
        match self {
            Anchor::Free => "anchor-free",
            Anchor::RelativePlain => "relative-plain",
            Anchor::RelativeSensitive => "relative-sensitive",
            Anchor::RelativeUnplaced => "relative-unplaced",
            Anchor::ImplicitOutput => "implicit-output",
            Anchor::ImplicitSource => "implicit-source",
            Anchor::RunsFolderCode => "runs-folder-code",
            Anchor::NamesItsWrites => "names-its-writes",
        }
    }
}

/// How far writes are approved when the folder is unknown. A second dial beside `--level`: a
/// command passes when it passes both, and this one never loosens what `--level` refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FolderLevel {
    /// Reads only; every write goes to the harness's prompt.
    Reads,
    /// What a developer approves in a known project, except deletion and paths that climb out.
    Developer,
    /// The folder is treated as the workspace: deletion and `../sibling` paths too.
    Workspace,
}

impl FolderLevel {
    pub const DEFAULT: FolderLevel = FolderLevel::Developer;
    pub const NAMES: [&'static str; 3] = ["reads", "developer", "workspace"];

    pub fn parse(name: &str) -> Option<FolderLevel> {
        match name {
            "reads" => Some(FolderLevel::Reads),
            "developer" => Some(FolderLevel::Developer),
            "workspace" => Some(FolderLevel::Workspace),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            FolderLevel::Reads => "reads",
            FolderLevel::Developer => "developer",
            FolderLevel::Workspace => "workspace",
        }
    }

    /// Whether a write the command makes without naming a path is approved at this level.
    pub fn admits_implicit(self, anchor: Anchor) -> bool {
        self >= FolderLevel::Developer
            && matches!(anchor, Anchor::Free | Anchor::ImplicitOutput | Anchor::ImplicitSource | Anchor::RunsFolderCode)
    }
}

/// What a resolved path is used for, which is also the face of a region role it reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    Read,
    Write,
    /// Changes what the NAME refers to, rather than the bytes underneath it: `rm` unbinds it, `ln`
    /// points it elsewhere, `mv` takes it away. Identical to `Write` for almost every path; the two
    /// diverge where a role says a directory may be written INTO but not replaced, and in an
    /// unknown folder, where it is the deletion the `developer` level leaves to the prompt.
    Rebind,
}

impl Use {
    /// Whether this use changes state, which is what `expand_vars` needs to pick a loop variable's
    /// representative item. A rebind is a write for that purpose.
    pub(crate) fn mutates(self) -> bool {
        self != Use::Read
    }
}

/// The anchor value of `path` as written, relative to the folder the command runs in.
pub fn of_path(path: &str) -> Anchor {
    if path.starts_with('/') || path.starts_with('~') {
        return Anchor::Free;
    }
    match normalize(path) {
        Some(segments) if !is_sensitive(&segments) => Anchor::RelativePlain,
        Some(_) => Anchor::RelativeSensitive,
        None => Anchor::RelativeUnplaced,
    }
}

/// Where `path`, relative to the unknown folder, is classified for `use_` at `level`: a path
/// relative to the workspace root, which the ordinary classifier then judges, or `None` to leave it
/// in the unknown folder, where no write lands inside anything.
///
/// A read of a relative path is placed at every level, `reads` included: if the agent runs it in
/// `~`, reading there is the user's choice (the owner, 2026-10-08). A sensitive name, a climb out of
/// the folder, and a glob anywhere but the last segment are still not placed. `developer` places a
/// plain path for a write, never a rebind. `workspace` also places a rebind (but not of the folder
/// itself) and a path that climbs exactly one level into a sibling.
pub fn placement(path: &str, use_: Use, level: FolderLevel) -> Option<String> {
    if path.starts_with('/') || path.starts_with('~') {
        return None;
    }
    if use_ == Use::Read {
        return read_placement(path);
    }
    if level == FolderLevel::Reads {
        return None;
    }
    if let Some(segments) = normalize(path) {
        if is_sensitive(&segments) {
            return None;
        }
        let rebinds_the_folder = segments.is_empty();
        let allowed = use_ == Use::Write || (level == FolderLevel::Workspace && !rebinds_the_folder);
        return allowed.then(|| if segments.is_empty() { ".".to_string() } else { segments.join("/") });
    }
    if level < FolderLevel::Workspace {
        return None;
    }
    let rest = sibling_hop(path)?;
    if rest.len() < 2 && use_ == Use::Rebind {
        return None;
    }
    Some(format!("../{}", rest.join("/")))
}

/// A read's placement. A glob is allowed in the last segment only (`tests/*.rs`): there it names
/// files in one directory it shows, and a shell glob never matches a leading dot. One that could
/// match a sensitive file name (`id_*`, `hooks/pre-*`) is not placed.
fn read_placement(path: &str) -> Option<String> {
    let (dir, last) = path.rsplit_once('/').unwrap_or(("", path));
    let globbed = last.contains(['*', '?', '[']);
    let literal = if globbed { dir } else { path };
    let mut segments = if literal.is_empty() { Vec::new() } else { normalize(literal)? };
    if read_is_secret(&segments) {
        return None;
    }
    if globbed {
        let pattern = last.replace('[', "?");
        if last.starts_with('.') || unreadable(&last.replace(['*', '?', '[', ']'], "")) || last.contains("..") {
            return None;
        }
        if KEY_FILES.iter().copied().chain(pinned_names(true)).any(|name| glob_matches(&pattern, name)) {
            return None;
        }
        segments.push(last);
    }
    Some(if segments.is_empty() { ".".to_string() } else { segments.join("/") })
}

/// Whether `pattern` (`*` any run, `?` one character, case-folded) matches `name` in full.
fn glob_matches(pattern: &str, name: &str) -> bool {
    let (p, n): (Vec<char>, Vec<char>) = (pattern.to_ascii_lowercase().chars().collect(), name.to_ascii_lowercase().chars().collect());
    let (mut pi, mut ni, mut star, mut mark) = (0, 0, None, 0);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}

/// The segments of `path` once `.` and empty segments are dropped and `..` folded, or `None` when
/// it climbs above the folder or cannot be read off the command line.
fn normalize(path: &str) -> Option<Vec<&str>> {
    if path.is_empty() || unreadable(path) {
        return None;
    }
    let mut out = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                out.pop()?;
            }
            s => out.push(s),
        }
    }
    Some(out)
}

/// `../name/…` that climbs exactly one level and then stays down: the segments below the parent.
/// A sibling is a plain, non-sensitive name; the folder's own name is unknown, so `../same` is as
/// much a sibling as anything else.
fn sibling_hop(path: &str) -> Option<Vec<&str>> {
    if unreadable(path) {
        return None;
    }
    let rest = path.trim_start_matches("./").strip_prefix("../")?;
    let segments = normalize(rest)?;
    (!segments.is_empty() && !is_sensitive(&segments)).then_some(segments)
}

fn unreadable(path: &str) -> bool {
    path.contains(['$', '*', '?', '[', '{', '}', '`', '\\']) || path.contains("__SAFE_CHAINS_") || path.contains("://")
}

/// Whether the segments name something sensitive under any folder: a hidden segment anywhere, a git
/// hook, a credential file name, or a region the table names as written or placed under `~`.
fn is_sensitive(segments: &[&str]) -> bool {
    if segments.is_empty() {
        return false;
    }
    if segments.iter().any(|s| s.starts_with('.')) {
        return true;
    }
    let same = |a: &str, b: &str| a.eq_ignore_ascii_case(b);
    if segments.windows(2).any(|w| same(w[0], "hooks") && GIT_HOOKS.iter().any(|h| same(h, w[1]))) {
        return true;
    }
    let last = segments[segments.len() - 1];
    if NAMED_FILES.iter().any(|n| same(n, last))
        || pinned_names(false).any(|n| same(n, last))
        || starts_with_the_end_of_a_home_node(segments, false)
    {
        return true;
    }
    let joined = segments.join("/");
    use crate::engine::resolve::regions::names_a_region;
    names_a_region(&joined) || names_a_region(&format!("~/{joined}"))
}

/// Whether a READ of the segments could be a secret under any folder: a node of the credential
/// shield as written or under `~`, the end of one (`Keychains/login.keychain-db` from
/// `~/Library`), a key file (`id_rsa`) or a file a shield node pins (`credentials`). Reads of other
/// sensitive names (`.gitignore`, `.github/`) are ordinary, so only these stay refused.
pub fn read_is_secret(segments: &[&str]) -> bool {
    use crate::engine::resolve::regions::names_a_secret;
    let same = |a: &str, b: &str| a.eq_ignore_ascii_case(b);
    let Some(last) = segments.last() else { return false };
    let joined = segments.join("/");
    KEY_FILES.iter().any(|n| same(n, last))
        || pinned_names(true).any(|n| same(n, last))
        || starts_with_the_end_of_a_home_node(segments, true)
        || names_a_secret(&joined)
        || names_a_secret(&format!("~/{joined}"))
}

/// Private keys, by OpenSSH's default names (ssh(1) FILES).
const KEY_FILES: &[&str] = &["id_rsa", "id_dsa", "id_ecdsa", "id_ecdsa_sk", "id_ed25519", "id_ed25519_sk"];

/// The file names exact home nodes pin (`credentials` from `~/.cargo/credentials`), hidden ones left
/// out because a hidden segment is sensitive on its own.
fn pinned_names(secret_only: bool) -> impl Iterator<Item = &'static str> {
    crate::engine::resolve::regions::home_nodes()
        .iter()
        .filter(move |n| n.exact && (n.secret || !secret_only))
        .filter_map(|n| n.segments.last().map(String::as_str))
        .filter(|name| !name.starts_with('.'))
}

/// Whether the segments start with the end of a home node, so they can be that place whichever
/// folder under `~` the command runs in.
fn starts_with_the_end_of_a_home_node(segments: &[&str], secret_only: bool) -> bool {
    let same = |a: &str, b: &str| a.eq_ignore_ascii_case(b);
    crate::engine::resolve::regions::home_nodes()
        .iter()
        .filter(|n| n.secret || !secret_only)
        .any(|node| {
            let node = &node.segments;
            (1..node.len()).any(|k| node[k..].len() <= segments.len() && node[k..].iter().zip(segments).all(|(a, b)| same(a, b)))
        })
}

/// Files whose name means the same thing in whichever folder they sit, beyond what the region table
/// pins: OpenSSH's per-user files (ssh(1) and sshd(8) FILES, OpenSSH 10.2, researched 2026-10-08)
/// other than `config`, GnuPG's (gpg(1) FILES, 2.4), fish's startup file, and the hook and
/// permission files Claude Code and Codex read from their own folders (`hooks.json`,
/// `settings.local.json`, Codex's `rules/default.rules`). `config` and `config.toml` are left out on
/// purpose: they are the accepted risk §6 names, since names that common cannot be refused
/// everywhere.
pub const NAMED_FILES: &[&str] = &[
    "authorized_keys", "authorized_keys2", "known_hosts", "id_rsa", "id_dsa", "id_ecdsa", "id_ecdsa_sk", "id_ed25519", "id_ed25519_sk",
    "rc", "environment", "gpg.conf", "gpg-agent.conf", "dirmngr.conf", "pubring.kbx", "trustdb.gpg", "config.fish", "hooks.json",
    "settings.local.json", "default.rules",
];

/// githooks(5) as of Git 2.51 (researched 2026-10-08). Matched only below a `hooks` segment, so the
/// command runs in `.git` and writes `hooks/pre-commit`.
pub const GIT_HOOKS: &[&str] = &[
    "applypatch-msg", "pre-applypatch", "post-applypatch", "pre-commit", "pre-merge-commit", "prepare-commit-msg", "commit-msg",
    "post-commit", "pre-rebase", "post-checkout", "post-merge", "pre-push", "pre-receive", "update", "proc-receive", "post-receive",
    "post-update", "reference-transaction", "push-to-checkout", "pre-auto-gc", "post-rewrite", "sendemail-validate", "fsmonitor-watchman",
    "p4-changelist", "p4-prepare-changelist", "p4-post-changelist", "p4-pre-submit", "post-index-change",
];

#[cfg(test)]
#[path = "anchor_tests.rs"]
mod tests;
