use crate::command::{CheckFn, CommandDef, SubDef};
use crate::parse::{Segment, Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static GIT_LOG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--abbrev-commit", "--all", "--ancestry-path",
        "--author-date-order", "--bisect", "--boundary",
        "--branches", "--cherry", "--cherry-mark", "--cherry-pick",
        "--children", "--clear-decorations",
        "--compact-summary", "--cumulative",
        "--date-order",
        "--dense", "--do-walk",
        "--early-output",
        "--first-parent", "--follow", "--full-diff", "--full-history",
        "--graph",
        "--ignore-missing",
        "--left-only", "--left-right", "--log-size",
        "--mailmap",
        "--merges", "--minimal",
        "--name-only", "--name-status",
        "--no-abbrev-commit",
        "--no-color", "--no-decorate",
        "--no-expand-tabs", "--no-ext-diff", "--no-merges",
        "--no-notes", "--no-patch", "--no-prefix",
        "--no-renames", "--no-walk",
        "--numstat",
        "--oneline",
        "--parents", "--patch", "--patch-with-raw",
        "--patch-with-stat", "--patience",
        "--raw", "--reflog", "--relative-date", "--remotes",
        "--reverse",
        "--shortstat", "--show-linear-break", "--show-notes",
        "--show-pulls", "--show-signature", "--simplify-by-decoration",
        "--simplify-merges", "--source", "--sparse", "--stat",
        "--stdin", "--summary",
        "--tags", "--text", "--topo-order",
        "--use-mailmap",
        "-p", "-q", "-u",
    ]),
    standalone_short: b"0123456789pqu",
    valued: WordSet::flags(&[
        "--abbrev", "--after", "--author", "--before",
        "--color", "--committer", "--date",
        "--decorate", "--decorate-refs", "--decorate-refs-exclude",
        "--diff-algorithm", "--diff-filter",
        "--encoding", "--exclude",
        "--format", "--glob", "--grep",
        "--max-count", "--max-parents", "--min-parents",
        "--pretty",
        "--since", "--skip", "--until",
        "-L", "-n",
    ]),
    valued_short: b"Ln",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_DIFF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--cached", "--check", "--compact-summary", "--cumulative",
        "--dirstat-by-file",
        "--exit-code",
        "--find-copies-harder", "--full-index",
        "--ignore-all-space", "--ignore-blank-lines",
        "--ignore-cr-at-eol", "--ignore-space-at-eol",
        "--ignore-space-change",
        "--merge-base", "--minimal",
        "--name-only", "--name-status", "--no-color",
        "--no-ext-diff", "--no-index", "--no-patch",
        "--no-prefix", "--no-renames",
        "--numstat",
        "--patch", "--patch-with-raw", "--patch-with-stat",
        "--patience", "--pickaxe-all",
        "--raw",
        "--shortstat", "--staged", "--stat", "--summary",
        "--text",
        "-B", "-C", "-M", "-R",
        "-a", "-b", "-p", "-u", "-w", "-z",
    ]),
    standalone_short: b"BCMRabpuwz",
    valued: WordSet::flags(&[
        "--abbrev", "--color", "--diff-algorithm", "--diff-filter",
        "--dirstat", "--dst-prefix",
        "--inter-hunk-context",
        "--line-prefix",
        "--output-indicator-new", "--output-indicator-old",
        "--relative", "--src-prefix",
        "--stat-width",
        "--unified", "--word-diff", "--word-diff-regex",
        "-G", "-O", "-S", "-U",
    ]),
    valued_short: b"GOSU",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--abbrev-commit", "--compact-summary", "--cumulative",
        "--expand-tabs", "--full-index",
        "--ignore-all-space", "--ignore-blank-lines",
        "--ignore-space-at-eol", "--ignore-space-change",
        "--mailmap", "--minimal",
        "--name-only", "--name-status", "--no-color",
        "--no-ext-diff", "--no-notes", "--no-patch",
        "--no-prefix", "--no-renames",
        "--numstat",
        "--patch", "--patch-with-raw", "--patch-with-stat",
        "--patience", "--raw",
        "--shortstat", "--show-notes", "--show-signature",
        "--source", "--stat", "--summary",
        "--text", "--use-mailmap",
        "-p", "-q", "-u", "-w",
    ]),
    standalone_short: b"pquw",
    valued: WordSet::flags(&[
        "--abbrev", "--color",
        "--decorate", "--decorate-refs", "--decorate-refs-exclude",
        "--diff-algorithm", "--diff-filter",
        "--encoding", "--format",
        "--notes", "--pretty",
        "-O",
    ]),
    valued_short: b"O",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_STATUS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--ahead-behind", "--branch",
        "--ignore-submodules",
        "--long", "--no-ahead-behind",
        "--no-renames", "--null",
        "--renames",
        "--short", "--show-stash",
        "--verbose",
        "-b", "-s", "-v", "-z",
    ]),
    standalone_short: b"bsvz",
    valued: WordSet::flags(&[
        "--column", "--find-renames",
        "--ignored",
        "--porcelain",
        "--untracked-files",
        "-M", "-u",
    ]),
    valued_short: b"Mu",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_BLAME_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--color-by-age", "--color-lines",
        "--incremental",
        "--line-porcelain",
        "--minimal",
        "--porcelain", "--progress",
        "--root",
        "--show-email", "--show-name", "--show-number",
        "--show-stats",
        "-b", "-c", "-e", "-f", "-l", "-n", "-p", "-s", "-t", "-w",
    ]),
    standalone_short: b"bcefklnpstw",
    valued: WordSet::flags(&[
        "--abbrev",
        "--contents",
        "--ignore-rev", "--ignore-revs-file",
        "-C", "-L", "-M", "-S",
    ]),
    valued_short: b"CLMS",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_GREP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all-match", "--and",
        "--basic-regexp", "--break",
        "--cached", "--column", "--count",
        "--exclude-standard", "--extended-regexp",
        "--files-with-matches", "--files-without-match",
        "--fixed-strings", "--full-name", "--function-context",
        "--heading",
        "--ignore-case", "--index", "--invert-match",
        "--line-number",
        "--name-only", "--no-color", "--no-index", "--null",
        "--only-matching",
        "--perl-regexp",
        "--quiet",
        "--recurse-submodules", "--recursive",
        "--show-function",
        "--text", "--textconv",
        "--untracked",
        "--word-regexp",
        "-E", "-F", "-G", "-H", "-I", "-L", "-P", "-W",
        "-a", "-c", "-h", "-i", "-l", "-n", "-o",
        "-p", "-q", "-r", "-v", "-w", "-z",
    ]),
    standalone_short: b"EFGHILPWachilnopqrvwz",
    valued: WordSet::flags(&[
        "--after-context", "--before-context",
        "--color", "--context",
        "--max-count", "--max-depth",
        "--open-files-in-pager", "--threads",
        "-A", "-B", "-C", "-O",
        "-e", "-f", "-m",
    ]),
    valued_short: b"ABCOefm",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_FETCH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--append", "--atomic",
        "--dry-run",
        "--force",
        "--ipv4", "--ipv6",
        "--keep",
        "--multiple",
        "--negotiate-only", "--no-auto-gc", "--no-auto-maintenance",
        "--no-show-forced-updates", "--no-tags", "--no-write-fetch-head",
        "--porcelain", "--prefetch", "--progress",
        "--prune", "--prune-tags",
        "--quiet",
        "--refetch",
        "--set-upstream", "--show-forced-updates", "--stdin",
        "--tags",
        "--unshallow", "--update-head-ok", "--update-shallow",
        "--verbose", "--write-commit-graph", "--write-fetch-head",
        "-4", "-6",
        "-P", "-a", "-f", "-k", "-m", "-n", "-p", "-q", "-t", "-u", "-v",
    ]),
    standalone_short: b"46Pafkmnpqtuv",
    valued: WordSet::flags(&[
        "--deepen", "--depth",
        "--filter",
        "--jobs", "--negotiation-tip",
        "--recurse-submodules", "--refmap",
        "--server-option",
        "--shallow-exclude", "--shallow-since",
        "-j", "-o",
    ]),
    valued_short: b"jo",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_SHORTLOG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--committer", "--email", "--numbered", "--summary",
        "-c", "-e", "-n", "-s",
    ]),
    standalone_short: b"cens",
    valued: WordSet::flags(&[
        "--format", "--group",
    ]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_LS_FILES_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--cached", "--debug", "--deduplicate", "--deleted",
        "--directory", "--empty-directory", "--eol",
        "--error-unmatch", "--exclude-standard",
        "--full-name",
        "--ignored",
        "--killed",
        "--modified",
        "--no-empty-directory",
        "--others",
        "--recurse-submodules", "--resolve-undo",
        "--sparse", "--stage",
        "--unmerged",
        "-c", "-d", "-f", "-i", "-k", "-m", "-o", "-s", "-t", "-u", "-v", "-z",
    ]),
    standalone_short: b"cdfikmorstuvz",
    valued: WordSet::flags(&[
        "--abbrev",
        "--exclude", "--exclude-from", "--exclude-per-directory",
        "--format",
        "--with-tree",
        "-X", "-x",
    ]),
    valued_short: b"Xx",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_LS_REMOTE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--branches",
        "--exit-code",
        "--get-url",
        "--quiet",
        "--refs",
        "--symref",
        "--tags",
        "-b", "-q", "-t",
    ]),
    standalone_short: b"bqt",
    valued: WordSet::flags(&[
        "--server-option", "--sort",
        "-o",
    ]),
    valued_short: b"o",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_LS_TREE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--full-name", "--full-tree",
        "--long",
        "--name-only", "--name-status",
        "--object-only",
        "-d", "-l", "-r", "-t", "-z",
    ]),
    standalone_short: b"dlrtz",
    valued: WordSet::flags(&[
        "--abbrev", "--format",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_CAT_FILE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--batch-all-objects", "--buffer",
        "--filters", "--follow-symlinks",
        "--mailmap",
        "--textconv", "--unordered", "--use-mailmap",
        "-Z", "-e", "-p", "-s", "-t",
    ]),
    standalone_short: b"Zepst",
    valued: WordSet::flags(&[
        "--batch", "--batch-check", "--batch-command",
        "--filter", "--path",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_DESCRIBE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--always", "--contains",
        "--debug",
        "--exact-match", "--first-parent",
        "--long",
        "--tags",
    ]),
    standalone_short: b"",
    valued: WordSet::flags(&[
        "--abbrev", "--broken",
        "--candidates", "--dirty",
        "--exclude", "--match",
    ]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_MERGE_BASE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--fork-point",
        "--independent", "--is-ancestor",
        "--octopus",
        "-a",
    ]),
    standalone_short: b"a",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_FOR_EACH_REF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--ignore-case", "--include-root-refs",
        "--omit-empty",
        "--perl", "--python",
        "--shell", "--stdin",
        "--tcl",
        "-p", "-s",
    ]),
    standalone_short: b"ps",
    valued: WordSet::flags(&[
        "--color", "--contains", "--count",
        "--exclude", "--format",
        "--merged", "--no-contains", "--no-merged",
        "--points-at", "--sort",
    ]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_DIFF_TREE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--cc", "--combined-all-paths",
        "--find-copies-harder", "--full-index",
        "--ignore-all-space", "--ignore-space-at-eol",
        "--ignore-space-change",
        "--merge-base", "--minimal",
        "--name-only", "--name-status", "--no-commit-id",
        "--no-ext-diff", "--no-patch", "--no-renames",
        "--numstat",
        "--patch", "--patch-with-raw", "--patch-with-stat",
        "--pickaxe-all",
        "--raw", "--root",
        "--shortstat", "--stat", "--stdin", "--summary",
        "--text",
        "-B", "-C", "-M", "-R",
        "-a", "-c", "-m", "-p", "-r", "-s", "-t", "-u", "-v", "-z",
    ]),
    standalone_short: b"BCMRacmprstuvz",
    valued: WordSet::flags(&[
        "--abbrev", "--diff-algorithm", "--diff-filter",
        "--pretty",
        "-O", "-S",
    ]),
    valued_short: b"OS",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_NAME_REV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--always", "--annotate-stdin",
        "--name-only", "--tags", "--undefined",
    ]),
    standalone_short: b"",
    valued: WordSet::flags(&[
        "--exclude", "--refs",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_COUNT_OBJECTS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--human-readable", "--verbose",
        "-H", "-v",
    ]),
    standalone_short: b"Hv",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_CHECK_IGNORE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--no-index", "--non-matching",
        "--quiet", "--stdin", "--verbose",
        "-n", "-q", "-v", "-z",
    ]),
    standalone_short: b"nqvz",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_MERGE_TREE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--allow-unrelated-histories",
        "--messages", "--name-only",
        "--quiet",
        "--stdin", "--trivial-merge", "--write-tree",
        "-z",
    ]),
    standalone_short: b"z",
    valued: WordSet::flags(&[
        "--merge-base",
        "-X",
    ]),
    valued_short: b"X",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_VERIFY_COMMIT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--raw", "--verbose",
        "-v",
    ]),
    standalone_short: b"v",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_VERIFY_TAG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--raw", "--verbose",
        "-v",
    ]),
    standalone_short: b"v",
    valued: WordSet::flags(&[
        "--format",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GIT_REV_PARSE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--absolute-git-dir",
        "--all",
        "--branches",
        "--git-common-dir", "--git-dir", "--git-path",
        "--is-bare-repository", "--is-inside-git-dir",
        "--is-inside-work-tree", "--is-shallow-repository",
        "--local-env-vars",
        "--quiet",
        "--remotes",
        "--shared-index-path", "--show-cdup", "--show-prefix",
        "--show-superproject-working-tree", "--show-toplevel",
        "--symbolic", "--symbolic-full-name",
        "--tags", "--verify",
        "-q",
    ]),
    standalone_short: b"q",
    valued: WordSet::flags(&[
        "--abbrev-ref", "--after", "--before",
        "--default", "--exclude",
        "--glob", "--prefix",
        "--resolve-git-dir", "--short",
        "--since", "--until",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Positional,
};

static GIT_REMOTE_MUTATING: WordSet = WordSet::new(&[
    "add", "prune", "remove", "rename", "set-branches", "set-url",
]);

static GIT_BRANCH_MUTATING: WordSet = WordSet::new(&[
    "--copy", "--delete", "--edit-description", "--move",
    "--set-upstream-to", "--unset-upstream",
    "-C", "-D", "-M", "-c", "-d", "-m", "-u",
]);

static GIT_STASH_SAFE: WordSet =
    WordSet::new(&["list", "show"]);

static GIT_TAG_MUTATING: WordSet = WordSet::new(&[
    "--annotate", "--delete", "--force", "--sign",
    "-a", "-d", "-f", "-s",
]);

static GIT_CONFIG_SAFE: WordSet =
    WordSet::new(&["--get", "--get-all", "--get-regexp", "--list", "-l"]);

static GIT_NOTES_SAFE: WordSet =
    WordSet::new(&["list", "show"]);

fn check_git_help(_tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    true
}

fn check_git_remote(tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    tokens.get(1).is_none_or(|a| !GIT_REMOTE_MUTATING.contains(a))
}

fn check_git_branch(tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    tokens[1..].iter().all(|a| {
        !GIT_BRANCH_MUTATING.contains(a)
            && !GIT_BRANCH_MUTATING
                .iter()
                .any(|f| f.starts_with("--") && a.starts_with(&format!("{f}=")))
    })
}

fn check_git_stash(tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    tokens.get(1).is_some_and(|a| GIT_STASH_SAFE.contains(a))
}

fn check_git_tag(tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    if tokens.len() == 1 {
        return true;
    }
    tokens[1..].iter().all(|a| !GIT_TAG_MUTATING.contains(a))
}

fn check_git_config(tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    tokens.get(1).is_some_and(|a| GIT_CONFIG_SAFE.contains(a))
        && tokens[2..].iter().all(|a| !a.starts_with('-'))
}

fn check_git_worktree(tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    tokens.get(1).is_some_and(|a| a == "list")
        && tokens[2..].iter().all(|a| {
            !a.starts_with('-')
                || *a == "--porcelain"
                || *a == "--verbose"
                || *a == "-v"
                || *a == "-z"
        })
}

fn check_git_notes(tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    tokens.get(1).is_some_and(|a| GIT_NOTES_SAFE.contains(a))
        && tokens[2..].iter().all(|a| !a.starts_with('-'))
}

fn check_git_sub(args: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    GIT_SUBS
        .iter()
        .find(|s| s.name() == args[0].as_str())
        .is_some_and(|s| s.check(args, is_safe))
}

static GIT_SUBS: &[SubDef] = &[
    SubDef::Policy { name: "blame", policy: &GIT_BLAME_POLICY },
    SubDef::Custom { name: "branch", check: check_git_branch as CheckFn, doc: "branch (read-only flags).", test_suffix: None },
    SubDef::Policy { name: "cat-file", policy: &GIT_CAT_FILE_POLICY },
    SubDef::Policy { name: "check-ignore", policy: &GIT_CHECK_IGNORE_POLICY },
    SubDef::Custom { name: "config", check: check_git_config as CheckFn, doc: "config (--get, --get-all, --get-regexp, --list, -l only).", test_suffix: Some("--list") },
    SubDef::Policy { name: "count-objects", policy: &GIT_COUNT_OBJECTS_POLICY },
    SubDef::Policy { name: "describe", policy: &GIT_DESCRIBE_POLICY },
    SubDef::Policy { name: "diff", policy: &GIT_DIFF_POLICY },
    SubDef::Policy { name: "diff-tree", policy: &GIT_DIFF_TREE_POLICY },
    SubDef::Policy { name: "fetch", policy: &GIT_FETCH_POLICY },
    SubDef::Policy { name: "for-each-ref", policy: &GIT_FOR_EACH_REF_POLICY },
    SubDef::Policy { name: "grep", policy: &GIT_GREP_POLICY },
    SubDef::Custom { name: "help", check: check_git_help as CheckFn, doc: "", test_suffix: None },
    SubDef::Policy { name: "log", policy: &GIT_LOG_POLICY },
    SubDef::Policy { name: "ls-files", policy: &GIT_LS_FILES_POLICY },
    SubDef::Policy { name: "ls-remote", policy: &GIT_LS_REMOTE_POLICY },
    SubDef::Policy { name: "ls-tree", policy: &GIT_LS_TREE_POLICY },
    SubDef::Policy { name: "merge-base", policy: &GIT_MERGE_BASE_POLICY },
    SubDef::Policy { name: "merge-tree", policy: &GIT_MERGE_TREE_POLICY },
    SubDef::Policy { name: "name-rev", policy: &GIT_NAME_REV_POLICY },
    SubDef::Custom { name: "notes", check: check_git_notes as CheckFn, doc: "notes (list, show only).", test_suffix: Some("list") },
    SubDef::Policy { name: "reflog", policy: &GIT_LOG_POLICY },
    SubDef::Custom { name: "remote", check: check_git_remote as CheckFn, doc: "remote (read-only actions).", test_suffix: None },
    SubDef::Policy { name: "rev-parse", policy: &GIT_REV_PARSE_POLICY },
    SubDef::Policy { name: "shortlog", policy: &GIT_SHORTLOG_POLICY },
    SubDef::Policy { name: "show", policy: &GIT_SHOW_POLICY },
    SubDef::Custom { name: "stash", check: check_git_stash as CheckFn, doc: "stash (list, show only).", test_suffix: None },
    SubDef::Policy { name: "status", policy: &GIT_STATUS_POLICY },
    SubDef::Custom { name: "tag", check: check_git_tag as CheckFn, doc: "tag (list only).", test_suffix: None },
    SubDef::Policy { name: "verify-commit", policy: &GIT_VERIFY_COMMIT_POLICY },
    SubDef::Policy { name: "verify-tag", policy: &GIT_VERIFY_TAG_POLICY },
    SubDef::Custom { name: "worktree", check: check_git_worktree as CheckFn, doc: "worktree (list only).", test_suffix: Some("list") },
];

pub(crate) static GIT: CommandDef = CommandDef {
    name: "git",
    subs: GIT_SUBS,
    bare_flags: &[],
    help_eligible: true,
    url: "https://git-scm.com/docs",
};

pub(in crate::handlers::vcs) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "git" => {
            if tokens.last().is_some_and(|t| *t == "-h")
                && !tokens.iter().any(|t| *t == "--")
            {
                return Some(true);
            }
            let mut args = &tokens[1..];
            while args.len() >= 2 && args[0] == "-C" {
                args = &args[2..];
            }
            if args.is_empty() {
                return Some(false);
            }
            if matches!(args[0].as_str(), "--version" | "-V" | "--help" | "-h") {
                return Some(true);
            }
            Some(check_git_sub(args, is_safe))
        }
        _ => None,
    }
}

pub(in crate::handlers::vcs) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    let mut git_doc = GIT.to_doc();
    git_doc.description.push_str("\n\nSupports `-C <dir>` prefix.");
    vec![git_doc]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        git_log: "git log --oneline -5",
        git_log_graph: "git log --graph --all --oneline",
        git_log_author: "git log --author=foo --since=2024-01-01",
        git_log_format: "git log --format='%H %s' -n 10",
        git_log_stat: "git log --stat --no-merges",
        git_log_bare: "git log",
        git_diff: "git diff --stat",
        git_diff_cached: "git diff --cached --name-only",
        git_diff_algorithm: "git diff --diff-algorithm=patience",
        git_show: "git show HEAD:some/file.rb",
        git_show_stat: "git show --stat HEAD",
        git_status: "git status --porcelain",
        git_status_short: "git status -sb",
        git_fetch: "git fetch origin master",
        git_fetch_all: "git fetch --all --prune",
        git_ls_tree: "git ls-tree HEAD",
        git_ls_tree_r: "git ls-tree -r --name-only HEAD",
        git_grep: "git grep pattern",
        git_grep_flags: "git grep -n -i --count pattern",
        git_rev_parse: "git rev-parse HEAD",
        git_rev_parse_toplevel: "git rev-parse --show-toplevel",
        git_merge_base: "git merge-base master HEAD",
        git_merge_tree: "git merge-tree HEAD~1 HEAD master",
        git_version: "git --version",
        git_help: "git help log",
        git_shortlog: "git shortlog -s",
        git_shortlog_ne: "git shortlog -sne",
        git_describe: "git describe --tags",
        git_describe_always: "git describe --always --abbrev=7",
        git_blame: "git blame file.rb",
        git_blame_flags: "git blame -L 10,20 -w file.rb",
        git_reflog: "git reflog",
        git_reflog_n: "git reflog -n 10",
        git_ls_files: "git ls-files",
        git_ls_files_others: "git ls-files --others --exclude-standard",
        git_ls_remote: "git ls-remote origin",
        git_ls_remote_tags: "git ls-remote --tags origin",
        git_diff_tree: "git diff-tree --no-commit-id -r HEAD",
        git_cat_file: "git cat-file -p HEAD",
        git_cat_file_t: "git cat-file -t HEAD",
        git_check_ignore: "git check-ignore .jj/",
        git_check_ignore_v: "git check-ignore -v .gitignore",
        git_name_rev: "git name-rev HEAD",
        git_for_each_ref: "git for-each-ref refs/heads",
        git_for_each_ref_format: "git for-each-ref --format='%(refname)' --sort=-committerdate",
        git_count_objects: "git count-objects -v",
        git_verify_commit: "git verify-commit HEAD",
        git_verify_tag: "git verify-tag v1.0",
        git_merge_base_all: "git merge-base --all HEAD main",
        git_c_flag: "git -C /some/repo diff --stat",
        git_c_nested: "git -C /some/repo -C nested log",
        git_remote_bare: "git remote",
        git_remote_v: "git remote -v",
        git_remote_get_url: "git remote get-url origin",
        git_remote_show: "git remote show origin",
        git_branch_list: "git branch",
        git_branch_list_all: "git branch -a",
        git_branch_list_verbose: "git branch -v",
        git_branch_contains: "git branch --contains abc123",
        git_stash_list: "git stash list",
        git_stash_show: "git stash show -p",
        git_tag_list: "git tag",
        git_tag_list_pattern: "git tag -l 'v1.*'",
        git_tag_list_long: "git tag --list",
        git_config_list: "git config --list",
        git_config_get: "git config --get user.name",
        git_config_get_all: "git config --get-all remote.origin.url",
        git_config_get_regexp: "git config --get-regexp 'remote.*'",
        git_config_l: "git config -l",
        git_worktree_list: "git worktree list",
        git_worktree_list_porcelain: "git worktree list --porcelain",
        git_worktree_list_verbose: "git worktree list -v",
        git_notes_show: "git notes show HEAD",
        git_notes_list: "git notes list",
        git_worktree_help: "git worktree --help",
        git_worktree_help_h: "git worktree -h",
    }

    denied! {
        git_rebase_help_denied: "git rebase --help",
        git_push_help_denied: "git push --help",
        git_fetch_upload_pack_denied: "git fetch --upload-pack=malicious origin",
        git_ls_remote_upload_pack_denied: "git ls-remote --upload-pack malicious origin",
        git_tag_delete_denied: "git tag -d v1.0",
        git_tag_annotate_denied: "git tag -a v1.0 -m 'release'",
        git_tag_sign_denied: "git tag -s v1.0",
        git_tag_force_denied: "git tag -f v1.0",
        git_config_set_denied: "git config user.name foo",
        git_config_unset_denied: "git config --unset user.name",
        git_config_list_trailing_flag_denied: "git config --list --evil",
        git_config_get_trailing_flag_denied: "git config --get user.name --evil",
        git_worktree_add_denied: "git worktree add ../new-branch",
        git_worktree_list_trailing_denied: "git worktree list --evil",
        git_notes_list_trailing_flag_denied: "git notes list --evil",
        git_branch_d_denied: "git branch -D feature",
        git_branch_delete_denied: "git branch --delete feature",
        git_branch_move_denied: "git branch -m old new",
        git_branch_copy_denied: "git branch -c old new",
        git_branch_set_upstream_denied: "git branch --set-upstream-to=origin/main",
        git_remote_add_denied: "git remote add upstream https://github.com/foo/bar",
        git_remote_remove_denied: "git remote remove upstream",
        git_remote_rename_denied: "git remote rename origin upstream",
        git_config_flag_denied: "git -c user.name=foo log",
        git_help_bypass_denied: "git push -- --help",
    }
}
