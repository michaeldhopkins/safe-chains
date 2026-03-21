use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static JJ_GLOBAL_STANDALONE: WordSet = WordSet::new(&[
    "--debug", "--ignore-immutable", "--ignore-working-copy",
    "--no-pager", "--quiet", "--verbose",
]);

static JJ_GLOBAL_VALUED: WordSet =
    WordSet::new(&["--at-op", "--at-operation", "--color", "--repository", "-R"]);

static JJ_READ_ONLY: WordSet =
    WordSet::new(&["--help", "--version", "-h", "cat", "diff", "help", "log", "root", "show", "st", "status", "version"]);

static JJ_MULTI: &[(&str, WordSet)] = &[
    ("bookmark", WordSet::new(&["list"])),
    ("config", WordSet::new(&["get", "list"])),
    ("file", WordSet::new(&["list", "show"])),
    ("git", WordSet::new(&["fetch"])),
    ("op", WordSet::new(&["log"])),
    ("resolve", WordSet::new(&["--list"])),
    ("tag", WordSet::new(&["list"])),
    ("workspace", WordSet::new(&["list"])),
];

static JJ_TRIPLE: &[(&str, &str, WordSet)] =
    &[("git", "remote", WordSet::new(&["list"]))];

pub fn is_safe_jj(tokens: &[Token]) -> Verdict {
    let mut args = &tokens[1..];
    loop {
        if args.is_empty() {
            return Verdict::Denied;
        }
        if JJ_GLOBAL_STANDALONE.contains(&args[0]) {
            args = &args[1..];
        } else if JJ_GLOBAL_VALUED.contains(&args[0]) {
            if args.len() < 2 {
                return Verdict::Denied;
            }
            args = &args[2..];
        } else if let Some(prefix) = args[0].split_value("=") {
            let flag = args[0].as_str().split_once('=').map_or(args[0].as_str(), |(k, _)| k);
            if JJ_GLOBAL_VALUED.contains(flag) && !prefix.is_empty() {
                args = &args[1..];
            } else {
                break;
            }
        } else {
            break;
        }
    }
    if args.is_empty() {
        return Verdict::Denied;
    }
    if JJ_READ_ONLY.contains(&args[0]) {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    for (prefix, actions) in JJ_MULTI.iter() {
        if args[0] == *prefix && args.get(1).is_some_and(|a| actions.contains(a) || a == "--help" || a == "-h") {
            return Verdict::Allowed(SafetyLevel::Inert);
        }
    }
    for (first, second, actions) in JJ_TRIPLE.iter() {
        if args[0] == *first && args.get(1).is_some_and(|t| t == *second) {
            return if args.get(2).is_some_and(|a| actions.contains(a)) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
        }
    }
    Verdict::Denied

}

pub(in crate::handlers::vcs) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "jj" => Some(is_safe_jj(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::vcs) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, doc_multi, wordset_items};
    vec![
        CommandDoc::handler("jj",
            "https://jj-vcs.github.io/jj/latest/cli-reference/",
            doc_multi(&JJ_READ_ONLY, JJ_MULTI)
                .triple_word(JJ_TRIPLE)
                .section(format!(
                    "Skips global flags: standalone ({}), valued ({}).",
                    wordset_items(&JJ_GLOBAL_STANDALONE),
                    wordset_items(&JJ_GLOBAL_VALUED),
                ))
                .build()),
    ]
}

#[cfg(test)]
pub(in crate::handlers::vcs) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Positional { cmd: "jj" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        jj_log: "jj log",
        jj_diff: "jj diff --stat",
        jj_show: "jj show abc123",
        jj_status: "jj status",
        jj_st: "jj st",
        jj_help: "jj help",
        jj_version: "jj --version",
        jj_version_subcmd: "jj version",
        jj_op_log: "jj op log",
        jj_file_show: "jj file show some/path",
        jj_config_get: "jj config get user.name",
        jj_config_list: "jj config list",
        jj_bookmark_list: "jj bookmark list",
        jj_git_remote_list: "jj git remote list",
        jj_ignore_working_copy_diff: "jj --ignore-working-copy diff --from 'trunk()' --to '@' --summary",
        jj_no_pager_log: "jj --no-pager log",
        jj_repository_flag: "jj -R /some/repo status",
        jj_color_valued: "jj --color auto log",
        jj_color_eq: "jj --color=auto log",
        jj_at_op: "jj --at-op @- diff",
        jj_multiple_global_flags: "jj --no-pager --ignore-working-copy --color=auto diff",
        jj_global_flag_multi_word: "jj --no-pager bookmark list",
        jj_git_fetch: "jj git fetch",
        jj_git_fetch_with_global_flags: "jj --no-pager git fetch",
        jj_root: "jj root",
        jj_file_list: "jj file list",
        jj_file_list_with_flags: "jj --no-pager file list",
        jj_workspace_list: "jj workspace list",
        jj_workspace_list_with_flags: "jj --no-pager workspace list",
        jj_resolve_list: "jj resolve --list",
        jj_resolve_list_file: "jj resolve --list somefile",
        jj_tag_list: "jj tag list",
        jj_tag_list_with_flags: "jj --no-pager tag list",
        jj_cat: "jj cat some/file.rb",
        jj_cat_revision: "jj cat -r master some/file.rb",
        jj_cat_pipe_standardrb: "jj cat -r master spec/foo_spec.rb | bundle exec standardrb --stdin spec/foo_spec.rb 2>&1",
        jj_workspace_help: "jj workspace --help",
        jj_workspace_help_h: "jj workspace -h",
    }

    denied! {
        jj_global_flag_no_subcommand_denied: "jj --ignore-working-copy",
        jj_global_flag_mutating_denied: "jj --ignore-working-copy new master",
        jj_new_denied: "jj new master",
        jj_edit_denied: "jj edit abc123",
        jj_squash_denied: "jj squash",
        jj_describe_denied: "jj describe -m 'test'",
        jj_bookmark_denied: "jj bookmark set my-branch",
        jj_git_push_denied: "jj git push",
        jj_rebase_denied: "jj rebase -d master",
        jj_restore_denied: "jj restore file.rb",
        jj_abandon_denied: "jj abandon",
        jj_config_set_denied: "jj config set user.name foo",
        jj_workspace_add_denied: "jj workspace add ../new",
        jj_workspace_forget_denied: "jj workspace forget default",
        jj_file_annotate_denied: "jj file annotate",
        jj_resolve_denied: "jj resolve somefile",
        jj_tag_create_denied: "jj tag create v1.0",
        jj_help_bypass_denied: "jj new -- --help",
        bare_jj_denied: "jj",
    }
}
