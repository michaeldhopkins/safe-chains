//! The spellings that print the environment rather than set it.
//!
//! The environment carries credentials (`OPENAI_API_KEY`, `GITHUB_TOKEN`), and what a command
//! prints reaches the agent. `export`, `declare` and `typeset` set variables, which is fine, but
//! with no names or with `-p` they list every exported variable with its value, and zsh prints a
//! name given without a value or an attribute flag. A lone `-` or `+` ends the options, so zsh's
//! `export -` lists too, and zsh's `typeset -m` treats each name as a pattern and lists every match,
//! even beside an assignment. `ps` shows each process's environment with the BSD `e` or `E` modifier.
use crate::parse::Token;

pub(crate) fn prints_the_environment(tokens: &[Token]) -> bool {
    let Some(first) = tokens.first() else {
        return false;
    };
    match crate::registry::canonical_name(first.command_name()) {
        "export" => lists_variables(&tokens[1..], false),
        "declare" => lists_variables(&tokens[1..], true),
        "ps" => ps_shows_the_environment(&tokens[1..]),
        "mise" => tokens.get(1).is_some_and(|t| *t == "set") && mise_set_shows_values(&tokens[2..]),
        _ => false,
    }
}

fn mise_set_shows_values(arguments: &[Token]) -> bool {
    // clap prints the help and stops at the first `--help` or `-h`, before reading any name.
    if arguments.first().is_some_and(|t| *t == "--help" || *t == "-h") {
        return false;
    }
    // `--remove NAME` drops that one name. A removal on its own prints nothing; a name left beside
    // it is printed, including when `--remove=NAME` glues the removed name and leaves the next word.
    let valued = crate::registry::sub_valued_flags("mise", "set").unwrap_or_default();
    let mut names = Vec::new();
    let mut removed = false;
    let mut after_options = false;
    let mut words = arguments.iter();
    while let Some(word) = words.next() {
        if after_options {
            names.push(word.as_str());
            continue;
        }
        if *word == "--" {
            after_options = true;
            continue;
        }
        if *word == "--remove" {
            removed = true;
            words.next();
            continue;
        }
        if valued.iter().any(|flag| flag == word.as_str()) {
            words.next();
            continue;
        }
        if !word.starts_with('-') {
            names.push(word.as_str());
        }
    }
    if names.is_empty() {
        return !removed;
    }
    names.iter().any(|name| !name.contains('='))
}

fn ps_shows_the_environment(arguments: &[Token]) -> bool {
    let valued = crate::registry::command_valued_flags("ps").unwrap_or_default();
    let mut words = arguments.iter().map(Token::as_str);
    while let Some(word) = words.next() {
        if valued.iter().any(|v| v == word) {
            words.next();
        } else if word.starts_with('-') {
            continue;
        } else if bsd_options_show_the_environment(word) {
            return true;
        } else if word.ends_with(['o', 'O']) && word.chars().all(|c| c.is_ascii_alphabetic()) {
            words.next();
        }
    }
    false
}

fn lists_variables(arguments: &[Token], a_bare_name_prints: bool) -> bool {
    let mut letters = String::new();
    let mut named = false;
    let mut bare_name = false;
    let mut after_separator = false;
    for argument in arguments {
        let word = argument.as_str();
        if !after_separator && matches!(word, "--" | "-" | "+") {
            after_separator = true;
        } else if !after_separator && word.len() > 1 && word.starts_with(['-', '+']) {
            if !only_known_attributes(&word[1..]) {
                return true;
            }
            letters.push_str(&word[1..]);
        } else {
            named = true;
            bare_name |= !word.contains('=');
        }
    }
    if letters.contains(['p', 'm']) {
        return true;
    }
    if !named {
        return !(a_bare_name_prints && !letters.is_empty() && letters.chars().all(|c| matches!(c, 'f' | 'F')));
    }
    a_bare_name_prints && bare_name && letters.is_empty()
}

/// Whether every letter is an attribute flag bash or zsh documents, with digits only as the width,
/// precision or base that follows `-E`, `-F`, `-L`, `-R`, `-Z` or `-i`. zsh reads a bare digit
/// option (`typeset -1 X`) as a request to print, and an unknown letter may be one a later shell
/// adds, so either counts as listing. `p` and `m` are known but list, and the caller judges them.
fn only_known_attributes(letters: &str) -> bool {
    let mut takes_digits = false;
    letters.chars().all(|c| {
        if c.is_ascii_digit() {
            return takes_digits;
        }
        takes_digits = matches!(c, 'E' | 'F' | 'L' | 'R' | 'Z' | 'i');
        "AEFHILRTUZafghilmnprtux".contains(c)
    })
}

/// A word without a dash that macOS ps may read as BSD options: letters with a pid glued on, or a
/// word whose text is not known until the shell expands it.
fn bsd_options_show_the_environment(word: &str) -> bool {
    if word.contains(['$', '`']) || word.contains(crate::cst::eval::TAGGED_PREFIX) {
        return true;
    }
    let letters: String = word.chars().filter(|c| !c.is_ascii_digit()).collect();
    letters.chars().all(|c| c.is_ascii_alphabetic()) && letters.contains(['e', 'E'])
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    use proptest::prelude::*;

    #[test]
    fn listing_the_environment_is_never_approved() {
        for cmd in [
            "export", "export -p", "export -n", "export -p FOO", "export -- ", "declare", "declare -p", "declare -p PATH", "declare -x",
            "declare -xp", "declare -r", "declare OPENAI_API_KEY", "typeset", "typeset -x", "typeset -p", "typeset GITHUB_TOKEN",
            "typeset +x", "ps eww", "ps e", "ps auxe", "ps a e", "ps aux e", "ps ax eww", "ps -o command e", "mise env",
            "mise env -s bash", "mise env --json", "mise set", "mise set OPENAI_API_KEY", "mise set -g",
            "mise set --remove X OPENAI_API_KEY", "mise set FAKEKEY --remove X", "mise set --remove=X FAKEKEY", "launchctl print gui/501",
            "launchctl print system", "launchctl dumpstate", "export -", "export +", "declare -", "declare +", "typeset -", "typeset +",
            "typeset -m '*'", "typeset -m 'OPENAI*'", "declare -m 'A*'", "typeset -xm 'A*'", "typeset -m FOO", "ps E", "ps Eww", "ps axE",
            "ps E1", "ps axE1", "ps auxE123", "ps wwE1234", "ps e1", "ps wwE$$", "ps E12345", "ps Eww12345", "ps \"E$$\"", "ps $(echo e)",
            "ps $(echo -E)", "ps \"$(echo a)e\"", "ps a$(echo e)", "ps `echo e`", "ps -o pid $(echo e)", "ps 1e2", "ps axo pid e",
            "env -i \"X=$OPENAI_API_KEY\"", "env -i \"X=${OPENAI_API_KEY}\"", "env -i X=\"`cat k`\"", "COMMAND_MODE=legacy ps -ef",
            "env COMMAND_MODE=legacy ps -e", "ps wwE$$", "ps E12345", "ps Eww12345", "ps \"E$$\"", "ps $(echo e)", "ps $(echo -E)",
            "ps \"$(echo a)e\"", "ps a$(echo e)", "ps `echo e`", "ps -o pid $(echo e)", "ps 1e2", "typeset -m 'OPENAI*' X=1",
            "typeset -m 'OPENAI*' X=", "declare -m X=1 'OPENAI*'", "typeset -mx 'O*' X=1", "typeset +m 'O*' X=1", "typeset -m X=1",
            "typeset -1 X", "typeset -1x X", "typeset -9 X", "typeset +9 X", "declare -0 X", "declare +1 X", "typeset -1 Y=2 X",
            "typeset -x1 X", "export -1 X", "export -0 X", "typeset -Q X", "declare -1i X", "typeset +1x X", "mise set -E staging",
            "mise set --file ./mise.toml", "mise set -f ./mise.toml OPENAI_API_KEY", "mise set -E staging OPENAI_API_KEY",
            "mise set --env=staging OPENAI_API_KEY",
        ] {
            assert!(!is_safe_command(cmd), "{cmd} must not be approved");
        }
    }

    #[test]
    fn setting_variables_and_listing_processes_are_unaffected() {
        for cmd in [
            "export FOO=bar", "export FOO", "export -n FOO", "export PATH_EXTRA=./bin", "declare -i count=5", "declare -x FOO=bar",
            "declare -ix FOO=42", "declare -a list", "declare FOO=bar", "declare -f", "declare -F", "declare -f my_function",
            "typeset -x FOO=bar", "ps", "ps aux", "ps -ef", "ps -o pid,command -p 123", "ps 123", "ps -o user,pid", "ps -o user -p 123",
            "ps -U nobody", "ps axo pid,user", "ps -p 123 -o etime", "env -i FOO=bar", "mise set FOO=bar", "mise set -g FOO=bar",
            "mise set --global FOO=bar", "mise set --remove FOO", "mise set --remove FOO BAR=1", "mise env --help", "launchctl list",
            "launchctl print-disabled gui/501", "export - FOO=bar", "ps axww", "ps -e", "typeset -U path", "typeset -L10 X",
            "typeset -Z2 X=5", "typeset -F3 X=1", "typeset -i16 X=255", "typeset -gx FOO=bar", "mise set --file ./mise.toml FOO=bar",
            "mise set -E staging FOO=bar", "mise set -f ./mise.toml FOO=bar", "mise set --env=staging FOO=bar",
        ] {
            assert!(is_safe_command(cmd), "{cmd} should be approved");
        }
    }

    proptest! {
        /// However the attribute flags and assignments are spelled, a `-p` anywhere lists, and a
        /// line made only of assignments never does. (`-i` is left out: an integer attribute reads a
        /// non-numeric value as arithmetic, which the arithmetic rules judge on their own.)
        #[test]
        fn a_p_flag_lists_and_assignments_alone_do_not(
            cmd in prop::sample::select(vec!["export", "declare", "typeset"]),
            flags in prop::collection::vec(prop::sample::select(vec!["-x", "-r", "-a", "-g", "-l", "-u"]), 0..3),
            values in prop::collection::vec("FOO[A-Z0-9_]{0,6}=[a-z0-9]{0,6}", 1..4),
            p_at in 0usize..4,
        ) {
            let flags: Vec<&str> = if cmd == "export" { Vec::new() } else { flags };
            let line = format!("{cmd} {} {}", flags.join(" "), values.join(" "));
            prop_assert!(is_safe_command(&line), "{}", line);
            let mut words: Vec<String> = flags.iter().map(|f| (*f).to_string()).chain(values.iter().cloned()).collect();
            words.insert(p_at.min(words.len()), "-p".to_string());
            let listing = format!("{cmd} {}", words.join(" "));
            prop_assert!(!is_safe_command(&listing), "{}", listing);
            let ended = format!("{cmd} {} -", flags.join(" "));
            prop_assert!(!is_safe_command(&ended), "{}", ended);
            let ended_then_set = format!("{cmd} {} - {}", flags.join(" "), values.join(" "));
            prop_assert!(is_safe_command(&ended_then_set), "{}", ended_then_set);
            if cmd != "export" {
                let names: Vec<&str> = values.iter().filter_map(|v| v.split('=').next()).collect();
                let matched = format!("{cmd} -m {} {}", flags.join(" "), names.join(" "));
                prop_assert!(!is_safe_command(&matched), "{}", matched);
                let matched_and_set = format!("{cmd} -m {} {} {}", flags.join(" "), names.join(" "), values.join(" "));
                prop_assert!(!is_safe_command(&matched_and_set), "{}", matched_and_set);
            }
        }

        /// A name left after `--remove` is printed, wherever the removal sits. A removal alone is not,
        /// and `mise set NAME=VALUE` is approved exactly when the environment rules allow that
        /// assignment: a name the rules list (`LD_`, `PATH`, `BASH_ENV`) is judged by its value.
        #[test]
        fn a_name_beside_remove_is_printed_and_a_removal_alone_is_not(
            name in "[A-Z][A-Z0-9_]{0,12}",
            removed in "[A-Z][A-Z0-9_]{0,8}",
            spelling in 0usize..3,
        ) {
            let line = match spelling {
                0 => format!("mise set --remove {removed} {name}"),
                1 => format!("mise set {name} --remove {removed}"),
                _ => format!("mise set --remove={removed} {name}"),
            };
            prop_assert!(!is_safe_command(&line), "{}", line);
            prop_assert!(is_safe_command(&format!("mise set --remove {removed}")), "mise set --remove {}", removed);
            let set = format!("mise set {name}={removed}");
            let expected = crate::envvars::statement_assignment_verdict(&name, &removed, true).is_allowed();
            prop_assert_eq!(is_safe_command(&set), expected, "{}", set);
        }
    }
}
