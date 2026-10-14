use crate::is_safe_command;

fn check(cmd: &str) -> bool {
    is_safe_command(cmd)
}

safe! {
    jj_commit_message_from_variable: "jj commit -m \"$MSG\"",
    jj_describe_long_message_from_variable: "jj describe --message \"$MSG\"",
    jj_log_revset_from_variable: "jj log -r \"$REV\"",
    jj_log_template_from_variable: "jj log --no-graph -T \"$T\"",
    jj_rebase_onto_from_variable: "jj rebase -r @ -o \"$DEST\"",
    jj_wrapper_repository_value_from_variable: "jj --at-op \"$OP\" log",
    jj_diff_after_double_dash_variable: "jj diff -- $X",
}

denied! {
    jj_diff_config_formatter_denied: "jj diff --config ui.diff-formatter=/tmp/evil.sh",
    jj_log_config_file_denied: "jj log --config-file /tmp/evil.toml",
    jj_log_config_toml_denied: "jj log --config-toml x=1",
    jj_diff_tool_denied: "jj diff --tool /tmp/evil.sh",
    jj_show_tool_denied: "jj show --tool=/tmp/evil.sh",
    jj_config_after_sub_denied: "jj new --config=ui.editor=/tmp/evil.sh",
    jj_diff_unknown_word_denied: "jj diff $X",
    jj_new_unknown_word_denied: "jj new $X",
    jj_log_unknown_word_after_value_denied: "jj log -r main $X",
    jj_log_variable_after_switch_denied: "jj log --no-graph \"$X\" -p",
    jj_diff_variable_after_summary_denied: "jj diff --summary \"$X\"",
    jj_diff_variable_after_name_only_denied: "jj diff --name-only \"$X\"",
    jj_describe_variable_after_no_edit_denied: "jj describe -m x --no-edit \"$X\"",
    jj_diff_variable_after_ambiguous_short_denied: "jj diff -s \"$X\"",
    jj_variable_after_glued_value_denied: "jj log --color=never \"$X\"",
    jj_config_env_foreign_denied: "JJ_CONFIG=/tmp/x jj log",
    jj_variable_after_glued_short_template_denied: "jj log -n1 -Tauthor \"$X\" -p",
    jj_variable_after_glued_short_revision_denied: "jj log -rmaster \"$X\" -p",
    jj_variable_after_glued_short_from_denied: "jj diff -fmaster \"$X\"",
    jj_variable_after_glued_short_message_denied: "jj describe -mfoo \"$X\"",
    jj_variable_after_one_letter_glued_value_denied: "jj log -Tr \"$X\"",
    jj_variable_after_valued_letter_glued_to_valued_letter_denied: "jj log -rr \"$X\"",
    jj_variable_after_cluster_ending_in_valued_letter_denied: "jj log -pr \"$X\"",
    jj_editor_env_denied: "JJ_EDITOR=/tmp/x jj describe",
}
