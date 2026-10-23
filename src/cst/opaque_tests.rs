//! Words whose value is unknown, and bash's `$'…'` / `$"…"` quotes.
//!
//! The invariant: a word the classifier cannot read is never more permissive than any word it
//! could be. If a command refuses a flag at some position, it refuses every unknown word there.

use crate::is_safe_command;
use proptest::prelude::*;

/// Spellings of a word whose value is not known statically. `{F}` is not one of them: it stands
/// for the flag itself, ANSI-C quoted, which bash decodes back to the flag.
const OPAQUE: &[&str] =
    &["$X", "\"$X\"", "${X}", "\"${X:-x}\"", "$(cat ./x)", "\"$(cat ./x)\"", "`cat ./x`", "*", "?", "[ab]", "$X-x", "\"$@\"", "$1"];

const GLOBS: &[&str] = &["*", "?", "[ab]"];

/// Flags each refused by some command somewhere: a write, an exec, a delete, a config or input
/// file, an override. The property only uses those a given command refuses.
const FLAGS: &[&str] = &[
    "-delete", "-exec", "--exec=sh", "-rf", "-o", "--output=/tmp/x", "-f", "--files0-from=/etc/shadow", "-i", "--pre=sh", "-e", "-c",
    "--config=/etc/x", "-x", "--force", "-F", "--upload-pack=sh", "-p", "--amend", "-1", "-9",
];

/// Commands whose grammars differ in how they take flags: getopt clusters, find's single-dash
/// primaries, a pattern operand, subcommands, a script operand.
const COMMANDS: &[&str] = &[
    "find / -name x", "find . -type f", "cat ./a", "ls ./a", "sort ./f", "grep foo ./f", "rg foo ./f", "sed s/a/b/ ./f",
    "git log --oneline", "git diff HEAD", "git status", "head -n 5 ./f", "wc -l ./f", "timeout 5 find .", "echo hi", "cp ./a ./b",
    "awk '{print}' ./f", "tar -tf ./a.tar", "jq . ./f.json",
];

fn words(cmd: &str) -> Vec<String> {
    shell_words::split(cmd).unwrap_or_default()
}

/// `cmd` with `raw` (shell text, inserted as typed) before its `at`-th word.
fn insert(cmd: &str, at: usize, raw: &str) -> String {
    let w = words(cmd);
    let at = at.clamp(1, w.len());
    format!("{} {raw} {}", shell_words::join(&w[..at]), shell_words::join(&w[at..]))
        .trim_end()
        .to_string()
}

/// `\xHH` for every byte: the spelling of `s` that shares no character with it.
fn hex_escaped(s: &str) -> String {
    s.bytes().map(|b| format!("\\x{b:02x}")).collect()
}

/// Every refused flag at `at` must also refuse every opaque word there, and the ANSI-C quoted
/// flag must classify exactly as the flag. Returns the violations.
fn violations(cmd: &str, at: usize) -> Vec<String> {
    let mut out = Vec::new();
    for flag in FLAGS {
        let literal = insert(cmd, at, flag);
        let refused = !is_safe_command(&literal);
        let ansi = insert(cmd, at, &format!("$'{}'", hex_escaped(flag)));
        if is_safe_command(&ansi) != !refused {
            out.push(format!("`{ansi}` and `{literal}` differ"));
        }
        if !refused {
            continue;
        }
        let numeric = flag[1..].bytes().all(|b| b.is_ascii_digit());
        for opaque in OPAQUE.iter().chain(numeric.then_some(&"$(($N))")) {
            // A glob matches names in one directory, and a name holds no `/`.
            if GLOBS.contains(opaque) && flag.contains('/') {
                continue;
            }
            let probe = insert(cmd, at, opaque);
            if is_safe_command(&probe) {
                out.push(format!("`{probe}` approved, `{literal}` refused"));
            }
        }
    }
    out
}

#[test]
fn an_unknown_word_never_approves_what_a_refused_flag_would() {
    let mut bad = Vec::new();
    for cmd in COMMANDS {
        assert!(is_safe_command(cmd), "precondition: `{cmd}` is approved");
        for at in 1..=words(cmd).len() {
            bad.extend(violations(cmd, at));
        }
    }
    assert!(bad.is_empty(), "unknown words more permissive than a flag:\n  {}", bad.join("\n  "));
}

/// Commands that read their next word as a program, and programs each would refuse.
const PROGRAM_SLOTS: &[&str] = &["awk", "awk -F,", "sed", "sed -n", "jq -n", "gojq -n", "jaq -n", "xq -n", "yq", "sh -c", "bash -c"];
const PROGRAMS: &[&str] = &["BEGIN{system(\"x\")}", "1e", "w ./x", "include \"m\"; .", "\"m\" | modulemeta", "load(\"x\")", "rm -rf ./x"];

/// Ways to put a word nobody has seen where `{}` stands: a variable, a loop over files, piped and
/// found file names.
const UNSEEN: &[&str] = &[
    "{} \"$X\"",
    "for p in $LIST; do {} \"$p\"; done",
    "for p in ./*; do {} \"$p\"; done",
    "find ./src | while read -r p; do {} \"$p\"; done",
    "find ./src -print0 | xargs -0 {}",
    "find ./src | xargs -I@ {} @",
    "find ./src -exec {} '{}' ';'",
    "fd . ./src -x {} '{}'",
];

proptest! {
    /// An unknown word never approves where a program the command refuses could stand: it could be
    /// that program. The flag property above asks the same of options.
    #[test]
    fn an_unknown_word_never_approves_what_a_refused_program_would(
        slot in prop::sample::select(PROGRAM_SLOTS.to_vec()),
        program in prop::sample::select(PROGRAMS.to_vec()),
        form in prop::sample::select(UNSEEN.to_vec()),
    ) {
        let literal = format!("{slot} {}", shell_words::quote(program));
        prop_assume!(!is_safe_command(&literal));
        let unseen = form.replacen("{}", slot, 1);
        prop_assert!(!is_safe_command(&unseen), "`{}` approved, `{}` refused", unseen, literal);
    }
}

#[test]
fn a_seen_word_in_a_program_slot_is_judged_as_written() {
    assert!(!is_safe_command("for p in 'BEGIN{system(1)}'; do awk \"$p\"; done"));
    assert!(!is_safe_command("for p in 'include \"m\"; f'; do jq -n \"$p\"; done"));
    assert!(is_safe_command("for f in ./a ./b; do awk '{print}' \"$f\"; done"));
    assert!(is_safe_command("find ./src -print0 | xargs -0 grep -l foo"));
    assert!(is_safe_command("find ./src -exec wc -l '{}' ';'"));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// The same over every approved `examples_safe` invocation in the registry, so a new command
    /// or handler is covered the day it lands. An insertion right after a flag is skipped: there
    /// the word may be that flag's value, which the grammar takes whatever it holds (see
    /// `opaque::probe_is_value`) while it refuses a literal value that looks like a flag.
    #[test]
    fn registry_commands_never_trust_an_unknown_word(pick in any::<prop::sample::Index>(), at in 1usize..8) {
        let approved: Vec<&String> = crate::registry::corpus_examples()
            .into_iter()
            .flat_map(|(_, safe, _)| safe.iter())
            .filter(|c| !c.contains(['$', '`', '*', '?', '[', '{', '\'', '"', '\\', '\n', '|', ';', '&', '<', '>', '(']))
            .collect();
        let cmd = pick.get(&approved).as_str();
        let w = words(cmd);
        prop_assume!(!w.is_empty() && is_safe_command(cmd));
        let at = at.min(w.len());
        prop_assume!(at == 1 || !w[at - 1].starts_with('-'));
        // After `--` every word is an operand: a literal flag there is refused only by a
        // grammar stricter than the tool.
        prop_assume!(!w[..at].iter().any(|t| t == "--"));
        let bad = violations(cmd, at);
        prop_assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    /// Bash's ANSI-C quote is decoded byte for byte: any string, every byte spelled `\xHH`, is the
    /// same word as the string single-quoted.
    #[test]
    fn ansi_c_hex_spelling_is_the_same_word(s in "[^'\\x00]{0,24}") {
        prop_assert_eq!(super::ansi_c::decode(&hex_escaped(&s)), s.clone());
        let octal: String = s.bytes().map(|b| format!("\\{b:03o}")).collect();
        prop_assert_eq!(super::ansi_c::decode(&octal), s.clone());
        for template in ["cat {}", "find / {}", "git {} log", "rm ./a {}"] {
            let quoted = template.replace("{}", &format!("'{s}'"));
            let ansi = template.replace("{}", &format!("$'{}'", hex_escaped(&s)));
            prop_assert_eq!(is_safe_command(&quoted), is_safe_command(&ansi), "`{}` vs `{}`", quoted, ansi);
        }
    }

    /// A variable a builtin may have set again is never read as the value it had before.
    #[test]
    fn a_rebound_variable_is_not_read_stale(
        builtin in prop::sample::select(vec!["export", "declare", "typeset", "readonly", "read", "unset", "local", "source ./env.sh;", "printf -v X x;", "mapfile", "getopts ab"]),
        value in prop::sample::select(vec!["-delete", "/etc/shadow", "~/.ssh/id_rsa"]),
    ) {
        let set = if builtin.ends_with(';') { builtin.to_string() } else { format!("{builtin} X={value}") };
        for reader in ["find / $X", "cat \"$X\"", "rm $X"] {
            let cmd = format!("X=./a; {set}; {reader}");
            prop_assert!(!is_safe_command(&cmd), "`{}` approved", cmd);
        }
    }

    /// `$"…"` is the double-quoted string (no message catalog is installed).
    #[test]
    fn locale_quote_is_the_double_quoted_string(s in "[a-z/ .-]{0,16}") {
        for template in ["cat {}", "find / {}", "git log {}"] {
            let dq = template.replace("{}", &format!("\"{s}\""));
            let locale = template.replace("{}", &format!("$\"{s}\""));
            prop_assert_eq!(is_safe_command(&dq), is_safe_command(&locale), "`{}` vs `{}`", dq, locale);
        }
    }
}

macro_rules! cases {
    ($verdict:expr, $($name:ident: $cmd:expr,)*) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(is_safe_command($cmd), $verdict, "{}", $cmd);
            }
        )*
    };
}

cases! { false,
    ansi_c_delete: "find / $'-delete'",
    ansi_c_exec: "find ~ $'-exec' rm {} +",
    ansi_c_octal_delete: "find / $'\\055delete'",
    ansi_c_hex_delete: "find / $'\\x2ddelete'",
    ansi_c_unicode_delete: "find / $'\\u002ddelete'",
    ansi_c_glued_to_literal: "find / -$'delete'",
    locale_delete: "find / $\"-delete\"",
    cmdsub_delete: "find / $(printf -- -delete)",
    backtick_delete: "find / `printf -- -delete`",
    var_unquoted: "find / $X",
    var_quoted: "find / \"$X\"",
    var_braced: "find / ${X}",
    var_default_flag: "find / \"${X:--delete}\"",
    var_empty_then_flag: "find / $X-delete",
    assigned_flag: "X=-delete; find / $X",
    assigned_flag_quoted: "X=-delete; find / \"$X\"",
    assigned_empty_then_flag: "E=; find / $E-delete",
    assigned_cmdsub: "X=$(printf -- -delete); find / $X",
    function_arg_flag: "f(){ find / \"$1\"; }; f -delete",
    function_arg_ansi_c: "f(){ find / \"$1\"; }; f $'-delete'",
    positional_unbound: "find / $1",
    all_args: "find / \"$@\"",
    glob_leading: "find / *",
    glob_question: "find / ?",
    glob_bracket: "find / [-]delete",
    brace_sequence_negative: "find / {-1..-1}",
    brace_alternative_flag: "find / {-delete,x}",
    arithmetic_negative: "find / $((-1))",
    arithmetic_variable: "sleep $((n))",
    loop_literal_flag: "for f in -delete; do find / $f; done",
    loop_literal_flag_quoted: "for f in -delete; do find / \"$f\"; done",
    loop_glob: "for f in *; do find / \"$f\"; done",
    loop_unquoted_sub: "for f in $(find .); do cat \"$f\"; done",
    read_from_unknown: "echo -delete | while read f; do find / \"$f\"; done",
    xargs_piped_flag: "echo -delete | xargs find /",
    xargs_split_names: "find . | xargs cat",
    xargs_unknown_stdin: "xargs find /",
    unquoted_value_splits: "find . -name $X",
    unquoted_message_splits: "git commit -m $MSG",
    after_standalone_flag: "git commit -v \"$X\"",
    after_glued_message_value: "git commit -mfix \"$X\"",
    after_cluster_with_glued_value: "git commit -amfix \"$X\"",
    sh_c_inner_var: "sh -c 'find / $X'",
    valued_slot_then_more: "sort -o \"$X\" ./f",
    ifs_assigned: "IFS=_; X=a_-delete; find / $X",
    ifs_exported: "export IFS=_; X=a_-delete; find / $X",
    ifs_read: "read IFS; X=a_-delete; find / $X",
    ifs_by_computed_name: "A=I; B=FS; export $A$B=_; X=a_-delete; find / $X",
    rebound_by_export: "X=./a; export X=-delete; find / $X",
    rebound_by_declare: "X=./a; declare X=/etc/shadow; cat $X",
    rebound_by_read: "X=./a; read X; find / \"$X\"",
    rebound_by_unset: "X=./a; unset X; cat $X",
    rebound_by_arithmetic: "X=./a; : $((X=1)); cat $X",
    find_exec_output_is_not_paths: "find . -exec echo -delete ';' | xargs -I{} find / {}",
    find_exec_output_locus: "find . -exec echo /etc/shadow ';' | xargs -I{} cat {}",
    perl_unknown_switch_after_code: "perl -pe 1 -x ./f",
    glob_in_assigned_value: "X='*'; find / $X",
    glob_in_loop_item: "for f in '*'; do find / $f; done",
}

cases! { true,
    ansi_c_decodes_name: "$'git' log",
    ansi_c_escape_value: "echo $'a\\tb'",
    locale_plain: "echo $\"hi\"",
    quoted_value_slot: "find . -name \"$X\"",
    message_slot: "git commit -m \"$MSG\"",
    message_slot_heredoc: "git commit -m \"$(cat <<'EOF'\nfix: a thing\nEOF\n)\"",
    message_slot_clustered: "git commit -am \"$MSG\"",
    count_slot: "head -n \"$N\" ./f",
    after_end_of_options: "ls -- *",
    prefixed_glob: "cat ./*.txt",
    trusted_home: "ls $HOME",
    assigned_plain: "X=./a; cat $X",
    constant_arithmetic: "sleep $((1+1))",
    loop_literal_words: "for f in a b c; do git show $f; done",
    loop_prefixed_glob_quoted: "for f in ./*.txt; do cat \"$f\"; done",
    loop_pwd: "for f in $(pwd); do cat $f; done",
    read_from_find: "find . | while read f; do cat \"$f\"; done",
    xargs_null_separated: "find . -print0 | xargs -0 cat",
    xargs_replace: "find . | xargs -I{} cat {}",
    echo_any_word: "echo $X",
    exit_status: "exit $?",
    cd_to_pwd: "cd $(pwd) && cat f",
    perl_switch_cluster: "perl -lne print ./f",
    prefixed_glob_in_value: "X='./*'; cat $X",
    perl_switch_arguments: "perl -F: -0777 -CSD -lane print ./f",
}

cases! { true,
    declaration_assignment: "export PATH=/opt/homebrew/bin:$PATH",
    declaration_substitution: "export X=$(id -u)",
    numeric_id: "launchctl print gui/$(id -u)/x",
    which_path: "ls -la \"$(which docker)\"",
}

cases! { false,
    declaration_without_assignment: "export $X",
    perl_module_dir_unknown: "PERL_USE_UNSAFE_INC=1 perl -MEvil -e 1",
}

const GLUED_PROGRAMS: &[&str] = &[
    "awk '{print \"{}\"}' f", "awk 'BEGIN{x=\"{}\"}'", "awk '/{}/' f", "perl -ne 'print \"{}\"' f", "sed 's/{}/b/' f", "sed -n '/{}/p' f",
    "sed 'y/{}/b/' f",
];

const BREAKOUT_ITEMS: &[&str] =
    &["x\"; system(\"id\"); \"", "/ {system(\"id\")} /", "@{[system q(id)]}", "h/y/w /tmp/o\n;s/q", "a/b/w ~/.zshrc"];

#[test]
fn an_item_glued_into_a_program_is_refused() {
    for program in GLUED_PROGRAMS {
        for line in [
            format!("xargs -I{{}} {program} < list"),
            format!("ls | xargs -I{{}} {program}"),
            format!("find . -exec {program} \\;"),
            format!("fd -x {program}"),
        ] {
            assert!(!is_safe_command(&line), "{line} must not approve");
        }
    }
    for line in ["find . -name '*.rs' -exec cp {} {}.bak \\;", "fd -e txt -x cat {.}.md", "ls | xargs -I{} cat ./{}"] {
        assert!(is_safe_command(line), "{line} must still approve");
    }
}

proptest! {
    /// An item xargs, find or fd glues into a word can be any text, so a command that carries one
    /// is approved only where every text it could be is approved written in place.
    #[test]
    fn a_glued_item_never_approves_what_its_text_would_not(
        program in prop::sample::select(GLUED_PROGRAMS.to_vec()),
        item in prop::sample::select(BREAKOUT_ITEMS.to_vec()),
        runner in prop::sample::select(vec!["xargs -I{} ", "find . -exec ", "fd -x "]),
    ) {
        let tail = if runner.starts_with("find") { " \\;" } else { "" };
        let glued = format!("ls | {runner}{program}{tail}");
        let direct = program.replace("{}", item);
        if is_safe_command(&glued) {
            prop_assert!(is_safe_command(&direct), "{glued} approves but {direct} does not");
        }
    }
}

#[test]
fn a_variable_a_declaring_builtin_sets_without_naming_it_is_no_longer_trusted() {
    for cmd in [
        "REPLY=./ok; read; cat $REPLY", "REPLY=./ok; read -r; cat $REPLY", "REPLY=./ok; read x; cat $REPLY",
        "MAPFILE=./ok; mapfile; cat $MAPFILE",
    ] {
        assert!(!is_safe_command(cmd), "{cmd} reads a variable the builtin may have set");
    }
}

#[test]
fn an_option_cluster_names_an_array_only_for_read() {
    assert!(is_safe_command("x=./ok; declare -ax foo; cat $x"), "`x` there is an option letter of `declare`, not an array");
    assert!(!is_safe_command("x=./ok; read -ax; cat $x"), "`read -ax` names the array `x`");
}

#[test]
fn a_quoted_name_is_one_literal_text() {
    for cmd in ["read \"x\"", "read \"x\"; ls", "export \"FOO\"=bar"] {
        assert!(is_safe_command(cmd), "{cmd} names its variable in plain sight");
    }
}
