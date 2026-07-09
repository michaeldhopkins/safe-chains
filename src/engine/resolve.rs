//! The profile resolver — turning a parsed command into its behavior profile
//! (annex `behavioral-taxonomy-engine`). Dormant: nothing here is wired into the
//! live classifier yet.
//!
//! This module starts with argument classification — the reusable predicates that
//! read a facet off an argument value. `classify_locus` refines the existing
//! `is_safe_write_target` branch order (`src/cst/check.rs`, a 2-bucket boolean) into
//! the full [`LocalLocus`] ladder (v1.4 §2.2).

use super::facet::{Capability, DisclosureAudience, LocalLocus, Operation, Profile, Scale};
use crate::parse::{Token, has_flag};

/// Resolve a command's leaf tokens to its behavior profile, or `None` if the command
/// has no resolver yet (the caller then worst-cases / falls back to the legacy
/// classifier — §0 fail-closed). Redirects, substitutions, and chain semantics are the
/// surrounding CST's job, not this leaf's (annex `…-engine` §1).
pub fn resolve(tokens: &[Token]) -> Option<Profile> {
    match tokens.first()?.command_name() {
        "echo" => Some(resolve_echo(tokens)),
        "cat" => Some(resolve_cat(tokens)),
        "grep" => Some(resolve_grep(tokens)),
        _ => None,
    }
}

/// `echo` — the reference *structural* certification (§0): every facet is positively
/// safe by the command's form. `echo` writes its literal arguments to stdout and does
/// nothing else — no filesystem, network, execution, secret, or state change — and its
/// only flags (`-n`/`-e`/`-E`) format the output. (A redirect like `echo x > f` or a
/// substitution like `echo "$SECRET"` is a *separate* capability the enclosing CST
/// resolves; this leaf is `echo`'s intrinsic behavior.)
fn resolve_echo(_tokens: &[Token]) -> Profile {
    let mut c = Capability::new(Operation::Observe);
    c.disclosure.audience = DisclosureAudience::LocalProcess; // its output reaches the model
    c.because = "echo prints its arguments to stdout; no fs/net/exec/secret".to_string();
    Profile::of(vec![c])
}

/// `cat FILE…` — reads each file's content to stdout (→ the model). Positive
/// certification (§0): `operation = observe`, `secret = none` (a byte-reader extracts no
/// credential — the sensitivity of the *content* is carried by `locus` + `disclosure`,
/// not by detecting a secret path), no network/execution. `locus` per file is
/// `classify_locus` (fail-closed: `$VAR`/`..` → `machine`). `cat`'s flags (`-n`/`-A`/…)
/// only format output and take no values.
fn resolve_cat(tokens: &[Token]) -> Profile {
    let files = positionals(tokens, |_| false);
    Profile::of(reads_to_model(&files, Scale::Single))
}

/// One `observe · content-to-model` capability per path (empty list = reads stdin). A
/// `-` operand is stdin (process-scoped); every other path is placed by `classify_locus`.
fn reads_to_model(paths: &[&str], scale: Scale) -> Vec<Capability> {
    if paths.is_empty() {
        return vec![reads_content(LocalLocus::Process, scale, "reads stdin")];
    }
    paths
        .iter()
        .map(|p| {
            if *p == "-" {
                reads_content(LocalLocus::Process, scale, "reads stdin (-)")
            } else {
                reads_content(classify_locus(p), scale, "reads file content to the model")
            }
        })
        .collect()
}

fn reads_content(locus: LocalLocus, scale: Scale, because: &str) -> Capability {
    let mut c = Capability::new(Operation::Observe);
    c.locus.local = locus;
    c.scale = scale;
    c.disclosure.audience = DisclosureAudience::LocalProcess; // content → the model
    c.because = because.to_string();
    c
}

/// `grep PATTERN FILE…` — searches files and prints matching lines (file content) to the
/// model. Like `cat` for its file operands, with three grep-specific twists: the first
/// positional is the *pattern* (not a file) unless `-e`/`-f` supplied it; `-f FILE` names
/// a pattern file grep also *reads*; and `-r`/`-R` searches recursively (`scale =
/// unbounded`). Same positive certification as `cat` (observe, `secret = none`, no
/// net/exec); `locus` per read is `classify_locus`.
fn resolve_grep(tokens: &[Token]) -> Profile {
    let recursive = has_flag(tokens, Some("-r"), Some("--recursive"))
        || has_flag(tokens, Some("-R"), None);
    let scale = if recursive { Scale::Unbounded } else { Scale::Single };

    let mut files = Vec::new(); // positional file operands
    let mut pattern_files = Vec::new(); // -f/--file pattern files grep reads
    let mut pattern_from_flag = false;
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
            } else {
                // --regexp=… (pattern) or any other long flag; inline values stay in-token
                pattern_from_flag |= t.starts_with("--regexp=");
                i += 1;
            }
        } else {
            let (pattern_file, from_flag, consumes_next) = grep_short_cluster(t, next);
            pattern_files.extend(pattern_file);
            pattern_from_flag |= from_flag;
            i += if consumes_next { 2 } else { 1 };
        }
    }

    if !pattern_from_flag && !files.is_empty() {
        files.remove(0); // the first positional is the PATTERN, not a file
    }
    if recursive && files.is_empty() {
        files.push("."); // grep -r with no path searches the cwd
    }

    let mut caps: Vec<Capability> = pattern_files
        .iter()
        .map(|f| reads_content(classify_locus(f), Scale::Single, "reads a grep -f pattern file"))
        .collect();
    caps.extend(reads_to_model(&files, scale));
    Profile::of(caps)
}

/// Parse a grep short-option cluster (e.g. `-ifpatterns`), honoring GNU semantics that a
/// value-taking short consumes the rest of its cluster (glued) or the next token.
/// Returns `(pattern_file, pattern_from_flag, consumes_next_token)`:
/// `-f` → a pattern *file* read; `-e` → a pattern *string*; `-m`/`-A`/`-B`/`-C`/`-d` → a
/// count/action value to skip; other chars are standalone.
fn grep_short_cluster<'a>(cluster: &'a str, next: Option<&'a str>) -> (Option<&'a str>, bool, bool) {
    let bytes = cluster.as_bytes();
    let mut k = 1;
    while k < bytes.len() {
        let glued = &cluster[k + 1..];
        let has_glued = !glued.is_empty();
        match bytes[k] {
            b'f' => return (if has_glued { Some(glued) } else { next }, true, !has_glued),
            b'e' => return (None, true, !has_glued),
            b'm' | b'A' | b'B' | b'C' | b'd' => return (None, false, !has_glued),
            _ => k += 1,
        }
    }
    (None, false, false)
}

/// The positional (non-flag) operands of an invocation: skips `tokens[0]` (the command),
/// flags, and — for flags `takes_value` reports true and that are not inline `--x=y` —
/// their following value. `--` ends flag parsing; a bare `-` is a positional (stdin).
fn positionals(tokens: &[Token], takes_value: impl Fn(&str) -> bool) -> Vec<&str> {
    let mut out = Vec::new();
    let mut flags_done = false;
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if !flags_done && t == "--" {
            flags_done = true;
        } else if !flags_done && t.starts_with('-') && t != "-" {
            if takes_value(t) && !t.contains('=') {
                i += 1; // also skip this flag's value
            }
        } else {
            out.push(t);
        }
        i += 1;
    }
    out
}

/// The filesystem rung a path reaches (v1.4 §2.2). A value that cannot be pinned —
/// a `$VAR` expansion or a `..` parent-escape — takes the worst-case fs rung
/// (`machine`), matching the allowlist floor `is_safe_write_target` already enforces
/// by denying such targets.
///
/// The same classifier serves reads and writes; the *level* draws the line
/// (`read-local` admits `<= user`, `write-local` admits `<= worktree`), which is the
/// refinement the facet model buys over the old single boolean.
pub fn classify_locus(path: &str) -> LocalLocus {
    // Unpinnable FIRST (§0 fail-closed): a `$VAR` expansion or a `..` escape could name
    // anything, so no positive (lower) classification is sound — not even a `/tmp/`
    // prefix, since `/tmp/$X` can expand through `..` to anywhere. Worst-case to
    // `machine` (the top fs rung; raw devices need an explicit /dev/ match).
    if path.contains('$') || is_parent_escape(path) {
        return LocalLocus::Machine;
    }
    // Standard streams — no real filesystem is touched.
    if matches!(path, "/dev/null" | "/dev/stdout" | "/dev/stderr" | "/dev/tty")
        || path.starts_with("/dev/fd/")
    {
        return LocalLocus::Process;
    }
    // Raw block/char devices — beneath the filesystem (dd of=/dev/rdisk0, mount).
    if is_block_device(path) {
        return LocalLocus::Device;
    }
    // Temp — process-scoped scratch.
    if path.starts_with("/tmp/")
        || path.starts_with("/private/tmp/")
        || path.starts_with("/var/tmp/")
    {
        return LocalLocus::Temp;
    }
    // Files another tool auto-executes or trusts (.git/ hooks & config, .envrc).
    if has_trusted_segment(path) {
        return LocalLocus::WorktreeTrusted;
    }
    // The user's own home / keychain scope.
    if path.starts_with('~') {
        return LocalLocus::User;
    }
    // Any other absolute path — /etc, /usr, services, another user's home.
    if path.starts_with('/') {
        return LocalLocus::Machine;
    }
    // A plain relative path inside the working tree.
    LocalLocus::Worktree
}

/// A raw block/char device node (not a standard stream — those are handled first).
/// The prefix list is curated and conservative; unmatched `/dev/*` nodes fall through
/// to the general `machine` rule.
fn is_block_device(path: &str) -> bool {
    const DEVICE_PREFIXES: &[&str] = &[
        "/dev/disk",
        "/dev/rdisk",
        "/dev/sd",
        "/dev/nvme",
        "/dev/hd",
        "/dev/vd",
        "/dev/mmcblk",
        "/dev/loop",
    ];
    DEVICE_PREFIXES.iter().any(|p| path.starts_with(p))
}

fn is_parent_escape(path: &str) -> bool {
    path == ".." || path.starts_with("../") || path.contains("/../") || path.ends_with("/..")
}

/// Whether any path segment is a directory a tool auto-executes or trusts. Matches
/// today's `is_safe_write_target` (`.git`, `.envrc`); CI-config trees (`.github/`,
/// `.gitlab-ci.yml`) are a future refinement of this set.
fn has_trusted_segment(path: &str) -> bool {
    path.split('/').any(|seg| seg == ".git" || seg == ".envrc")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_streams_are_process_scoped() {
        assert_eq!(classify_locus("/dev/null"), LocalLocus::Process);
        assert_eq!(classify_locus("/dev/stdout"), LocalLocus::Process);
        assert_eq!(classify_locus("/dev/fd/3"), LocalLocus::Process);
    }

    #[test]
    fn raw_devices_are_device_rung() {
        assert_eq!(classify_locus("/dev/rdisk0"), LocalLocus::Device);
        assert_eq!(classify_locus("/dev/sda1"), LocalLocus::Device);
        assert_eq!(classify_locus("/dev/nvme0n1"), LocalLocus::Device);
    }

    #[test]
    fn temp_paths_are_temp() {
        assert_eq!(classify_locus("/tmp/scratch"), LocalLocus::Temp);
        assert_eq!(classify_locus("/private/tmp/x"), LocalLocus::Temp);
        assert_eq!(classify_locus("/var/tmp/y"), LocalLocus::Temp);
    }

    #[test]
    fn plain_relative_paths_are_worktree() {
        assert_eq!(classify_locus("notes.md"), LocalLocus::Worktree);
        assert_eq!(classify_locus("src/engine/mod.rs"), LocalLocus::Worktree);
        assert_eq!(classify_locus("build/out"), LocalLocus::Worktree);
    }

    #[test]
    fn trusted_dotdirs_are_worktree_trusted() {
        assert_eq!(classify_locus(".git/hooks/pre-commit"), LocalLocus::WorktreeTrusted);
        assert_eq!(classify_locus(".git/config"), LocalLocus::WorktreeTrusted);
        assert_eq!(classify_locus(".envrc"), LocalLocus::WorktreeTrusted);
        assert_eq!(classify_locus("nested/.git/x"), LocalLocus::WorktreeTrusted);
    }

    #[test]
    fn home_paths_are_user() {
        assert_eq!(classify_locus("~/.ssh/id_rsa"), LocalLocus::User);
        assert_eq!(classify_locus("~/.config/foo"), LocalLocus::User);
        assert_eq!(classify_locus("~"), LocalLocus::User);
    }

    #[test]
    fn other_absolute_paths_are_machine() {
        assert_eq!(classify_locus("/etc/hosts"), LocalLocus::Machine);
        assert_eq!(classify_locus("/usr/local/bin/x"), LocalLocus::Machine);
        assert_eq!(classify_locus("/Users/someone/notes"), LocalLocus::Machine);
    }

    #[test]
    fn unresolvable_paths_worst_case_to_machine() {
        assert_eq!(classify_locus("$HOME/.ssh/id_rsa"), LocalLocus::Machine);
        assert_eq!(classify_locus("$OUT/file"), LocalLocus::Machine);
        assert_eq!(classify_locus("../secret"), LocalLocus::Machine);
        assert_eq!(classify_locus("a/../../etc/passwd"), LocalLocus::Machine);
        assert_eq!(classify_locus("dir/.."), LocalLocus::Machine);
    }

    #[test]
    fn an_unpinnable_marker_dominates_every_otherwise_safe_prefix() {
        // conservative: an unpinnable segment can't be trusted, even under a safe prefix
        assert_eq!(classify_locus("build/$ARTIFACT"), LocalLocus::Machine);
        assert_eq!(classify_locus("/tmp/$X"), LocalLocus::Machine, "$ beats the /tmp prefix");
        assert_eq!(classify_locus("/tmp/a/../../etc/passwd"), LocalLocus::Machine, ".. escapes /tmp");
        assert_eq!(classify_locus("/dev/null$"), LocalLocus::Machine, "$ beats /dev/null");
    }

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
        level("inert")
    }

    fn read_local() -> &'static crate::engine::level::Level {
        level("read-local")
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
        assert!(resolve(&toks(&["rm", "-rf", "/"])).is_none(), "unresearched → caller worst-cases");
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
        for path in ["~/.ssh/id_rsa", "/etc/hosts", "$SECRET", "../outside"] {
            let p = resolve(&toks(&["cat", path])).expect("cat");
            assert!(!read_local().admits(&p), "cat {path} is above read-local by locus");
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
    fn grep_stdin_and_standalone_flags() {
        assert!(inert().admits(&resolve(&toks(&["grep", "foo"])).expect("grep")), "no file → stdin");
        let p = resolve(&toks(&["grep", "-i", "-n", "foo", "file.txt"])).expect("grep");
        assert_eq!(p.capabilities.len(), 1, "-i -n standalone; foo pattern; file.txt file");
        assert!(read_local().admits(&p));
    }

    use proptest::prelude::*;

    proptest! {
        /// Fail-closed (§0): a `$` anywhere forces the worst rung, whatever the rest
        /// looks like — the classifier can never be talked below `machine` by a
        /// safe-looking prefix wrapped around an unpinnable expansion.
        #[test]
        fn a_dollar_anywhere_forces_machine(s in ".{0,30}") {
            prop_assert_eq!(classify_locus(&format!("{s}$")), LocalLocus::Machine);
        }

        /// Fail-closed: a `..` parent-escape forces the worst rung.
        #[test]
        fn a_parent_escape_forces_machine(s in "[a-zA-Z0-9/_]{0,20}") {
            prop_assert_eq!(classify_locus(&format!("{s}/../x")), LocalLocus::Machine);
            prop_assert_eq!(classify_locus(&format!("../{s}")), LocalLocus::Machine);
        }
    }
}
