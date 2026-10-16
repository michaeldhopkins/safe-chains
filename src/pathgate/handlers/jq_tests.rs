use super::{JQ, PY_YQ, YQ, filter_words, jq_reads_beyond_operands, yq_reads_beyond_operands};
use crate::is_safe_command;
use proptest::prelude::*;

const JQ_FAMILY: &[&str] = &["jq", "gojq", "jaq", "xq", "tomlq", "yq"];

fn jq(args: &[&str]) -> bool {
    jq_reads_beyond_operands(args, &filter_words(args, &JQ, false))
}

fn yq(args: &[&str]) -> bool {
    yq_reads_beyond_operands(args, &filter_words(args, &YQ, true))
}

#[test]
fn the_filter_is_the_first_operand_after_the_options_take_their_values() {
    let cases: &[(&[&str], &[&str])] = &[
        (&[".", "./f"], &["."]),
        (&["-nr", ".a", "./f"], &[".a"]),
        (&["--arg", "k", "v", ".", "./f"], &["."]),
        (&["--rawfile", "k", "./f", "--argjson", "j", "1", ".x"], &[".x"]),
        (&["--indent", "2", "."], &["."]),
        (&["--indent=2", "."], &["."]),
        (&["--", "-1"], &["-1"]),
        (&["-L", "dir", "."], &["."]),
        (&["-Ldir", "."], &["."]),
        (&["-nL", "dir", "."], &["."]),
        (&["--args", ".", "a", "b"], &["."]),
    ];
    for (args, want) in cases {
        assert_eq!(filter_words(args, &JQ, false), *want, "{args:?}");
    }
    assert_eq!(filter_words(&["--xml-root", "r", "--xml-dtd", ".", "./f"], &PY_YQ, false), ["."]);
    assert_eq!(filter_words(&["eval", ".a", "./f"], &YQ, true), ["eval", ".a"]);
    assert_eq!(filter_words(&["-I", "2", "e", ".a"], &YQ, true), ["e", ".a"]);
    assert_eq!(filter_words(&["--expression", "load(1)", "./f"], &YQ, true), ["load(1)", "./f"]);
    assert_eq!(filter_words(&["./f", "--expression", "load(1)"], &YQ, true), ["load(1)", "./f"]);
    assert_eq!(filter_words(&["--expression=load(1)", "./f"], &YQ, true), ["load(1)", "./f"]);
    assert_eq!(filter_words(&["e", "./f", "--expression", "load(1)"], &YQ, true), ["load(1)", "e", "./f"]);
    assert_eq!(filter_words(&[".", "--arg", "k", "include", "./f"], &JQ, false), ["."]);
}

/// The filter search and the grammars agree on every option: a valued one has an arity, and a
/// switch has none. Otherwise the search would take a value for the filter, or the filter for a
/// value, and leave the real filter unread.
#[test]
fn every_valued_option_in_the_grammars_has_a_known_arity() {
    for (name, arities) in [("jq", &JQ), ("gojq", &JQ), ("jaq", &JQ), ("xq", &PY_YQ), ("tomlq", &PY_YQ), ("yq", &YQ)] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("commands/data/{name}.toml"));
        let doc: toml::Value = toml::from_str(&std::fs::read_to_string(&path).expect("read grammar")).expect("parse grammar");
        let command = &doc["command"][0];
        assert_eq!(command["name"].as_str(), Some(name));
        let valued = command.get("valued").and_then(toml::Value::as_array).expect("a valued list");
        assert!(!valued.is_empty(), "{name}: no valued options read");
        for flag in valued.iter().filter_map(toml::Value::as_str) {
            let known = arities.two.contains(&flag) || arities.one.contains(&flag) || arities.program.contains(&flag);
            assert!(known, "{name}: {flag} has no arity in the filter search");
        }
        let standalone = command.get("standalone").and_then(toml::Value::as_array).expect("a standalone list");
        for flag in standalone.iter().filter_map(toml::Value::as_str) {
            let takes = arities.two.contains(&flag) || arities.one.contains(&flag) || arities.program.contains(&flag);
            assert!(!takes, "{name}: {flag} is a switch in the grammar but takes a value in the filter search");
        }
    }
}

#[test]
fn an_unknown_word_where_the_filter_goes_is_refused() {
    for line in [
        "for p in 'include \"m\"; f'; do jq -n \"$p\"; done",
        "for p in 'load(\"/etc/passwd\")'; do yq \"$p\" ./a.yml; done",
        "for p in ./*.jq; do jq -n \"$p\"; done",
        "echo 'include \"m\"; f' | while read -r p; do jq -n \"$p\"; done",
        "jq -n \"$P\"",
    ] {
        assert!(!is_safe_command(line), "{line} must not approve");
    }
}

#[test]
fn an_unknown_word_where_a_file_or_a_value_goes_still_approves() {
    for line in [
        "for f in ./a.json ./b.json; do jq . \"$f\"; done",
        "for f in ./a.json ./b.json; do jq --arg k v -r .name \"$f\"; done",
        "for v in a b; do jq --arg k \"$v\" -r .name ./d.json; done",
        "for f in ./a.yml ./b.yml; do yq .a \"$f\"; done",
    ] {
        assert!(is_safe_command(line), "{line} must still approve");
    }
}

#[test]
fn a_program_file_is_refused_wherever_it_points() {
    for cmd in JQ_FAMILY {
        for form in ["-f ./p.jq", "--from-file ./p.jq", "--from-file=./p.jq", "-n -f ./p.jq", "-nf ./p.jq", "-fn ./p.jq", "-rf ./p.jq"] {
            let line = format!("{cmd} {form} ./d.json");
            assert!(!is_safe_command(&line), "{line} must not approve");
        }
    }
}

#[test]
fn an_importing_program_file_is_refused() {
    assert!(!is_safe_command("jq -n -f ./imports-config.jq"));
    assert!(!is_safe_command("jq -f ./p.jq ./data.json"));
}

#[test]
fn inline_module_directives_are_refused() {
    for cmd in JQ_FAMILY {
        for program in [
            "import \"config\" as c; c::f",
            "import \"config\" as $c; $c",
            "import \"config\" as $c {search: \"/any/dir\"}; $c",
            "include \"m\"; f",
            "include \"m\" {search: \"/etc\"}; f",
            "import\"x\"as d;d::f",
            "include\"m\";f",
            "module {}; import \"x\" as d; d::f",
            "# comment\ninclude \"m\"; f",
            "\"m\" | modulemeta",
            "\"m\"|modulemeta",
            "1/modulemeta",
            "INCLUDE \"m\"; f",
        ] {
            let line = format!("{cmd} -n {}", shell_words::quote(program));
            assert!(!is_safe_command(&line), "{line} must not approve");
        }
    }
}

#[test]
fn a_directive_spelled_through_ansi_c_quoting_is_refused() {
    assert!(!is_safe_command("jq $'\\x69nclude \"m\"; f' ./f.json"));
    assert!(!is_safe_command("jq -n $'import \"x\" as d; d::f'"));
}

#[test]
fn a_directive_after_the_program_separator_is_refused() {
    assert!(!is_safe_command("jq -n -- 'include \"m\"; f'"));
    assert!(!is_safe_command("jq --arg k v 'import \"x\" as d; d::f' ./d.json"));
}

#[test]
fn module_path_flags_are_refused() {
    for cmd in JQ_FAMILY {
        for form in ["-L . -n .", "-L/ -n .", "-Lmods -n .", "--library-path . -n .", "--library-path=. -n .", "-nL . ."] {
            let line = format!("{cmd} {form}");
            assert!(!is_safe_command(&line), "{line} must not approve");
        }
    }
}

#[test]
fn yq_load_operators_and_eval_are_refused() {
    for program in [
        "load(\"/etc/passwd\")", "load(\"./x.yml\")", "load_str(\"x\")", "strload(\"x\")", "str_load(\"x\")", "load_xml(\"x\")",
        "loadxml(\"x\")", "xml_load(\"x\")", "load_props(\"x\")", "load_base64(\"x\")", ".a = load(\"x\")", "..|load(\"x\")",
        "eval(\"x\")", "eval(strenv(Q))", "LOAD(\"x\")",
    ] {
        let line = format!("yq {} ./a.yml", shell_words::quote(program));
        assert!(!is_safe_command(&line), "{line} must not approve");
        let line = format!("yq eval {} ./a.yml", shell_words::quote(program));
        assert!(!is_safe_command(&line), "{line} must not approve");
    }
    assert!(!is_safe_command("yq --from-file ./p.yq ./a.yml"));
}

#[test]
fn a_yq_expression_is_read_wherever_and_however_it_is_given() {
    for line in [
        "yq --expression 'load(\"/etc/passwd\")' ./a.yml",
        "yq ./a.yml --expression 'load(\"/etc/passwd\")'",
        "yq --expression='load(\"/etc/passwd\")' ./a.yml",
        "yq e ./a.yml --expression 'load(\"/etc/passwd\")'",
        "yq ./a.yml --expression='eval(strenv(Q))'",
    ] {
        assert!(!is_safe_command(line), "{line} must not approve");
    }
    assert!(is_safe_command("yq ./a.yml --expression .a"));
}

#[test]
fn file_variables_are_gated_as_operands() {
    for cmd in ["jq", "gojq", "jaq"] {
        for flag in ["--rawfile", "--slurpfile"] {
            assert!(!is_safe_command(&format!("{cmd} {flag} k ~/.ssh/id_rsa -n null")));
            assert!(!is_safe_command(&format!("{cmd} {flag} k ~/.docker/config.json -n null")));
            assert!(is_safe_command(&format!("{cmd} {flag} k ./data.json -n null")));
        }
    }
}

#[test]
fn ordinary_filters_still_approve() {
    for line in [
        "jq . ./data.json",
        "jq -r .name ./package.json",
        "jq -r '.compilerOptions.include' ./tsconfig.json",
        "jq '.include[]' ./tsconfig.json",
        "jq '.imports | keys' ./deno.json",
        "jq -c '.items[] | select(.ok)' ./data.json",
        "jq . ./include/a.json",
        "jq . ./my-include.json",
        "jq -n env",
        "jq -n 'env.HOME'",
        "jq -n 'env | getpath([\"HOME\"])'",
        "jq input_filename ./d.json",
        "jq -r '@sh' ./d.json",
        "jq -n 'get_search_list'",
        "jq --stream -c . ./data.json",
        "jq --args -n null a b",
        "gojq . ./data.json",
        "jaq . ./data.json",
        "yq '.a' ./a.yml",
        "yq eval '.a' ./a.yml",
        "yq eval-all '.a' ./a.yml",
        "yq '.payload' ./a.yml",
        "yq '.downloads.loaded' ./a.yml",
        "yq 'env(HOME)' ./a.yml",
        "gh api repos/o/r/contents/f --jq '.content'",
    ] {
        assert!(is_safe_command(line), "{line} must still approve");
    }
}

fn shell_safe_word() -> impl Strategy<Value = String> {
    "[a-z.\\[\\]|]{0,12}"
}

proptest! {
    #[test]
    fn a_directive_at_any_directive_position_is_seen(
        kw in prop::sample::select(vec!["import", "include", "IMPORT", "Include"]),
        lead in prop::sample::select(vec!["", " ", "\n", "\t", "\r\n", ";", "; ", "# c\n", "module {};", "\u{0b}"]),
        tail in prop::sample::select(vec!["\"m\";", " \"m\";", "\t\"m\" as d;", "@text \"m\" as $d;", "#c\n\"m\";"]),
        rest in shell_safe_word(),
    ) {
        let program = format!("{lead}{kw}{tail}{rest}");
        prop_assert!(jq(&[&program]), "missed: {program:?}");
        prop_assert!(jq(&["-n", "--arg", "k", "v", &program, "./d.json"]));
    }

    #[test]
    fn modulemeta_after_any_operator_is_seen(
        op in prop::sample::select(vec!["|", " | ", "(", "/", "-", "+", ",", "[", "//", " and ", "?//", "\n"]),
        prefix in shell_safe_word(),
    ) {
        let program = format!("\"m\"{op}{prefix}{op}modulemeta");
        prop_assert!(jq(&[&program]), "missed: {program:?}");
    }

    #[test]
    fn a_field_or_a_longer_name_never_reads_as_a_directive(
        kw in prop::sample::select(vec!["import", "include", "modulemeta"]),
        before in prop::sample::select(vec![".", "x", "_", "$", "a.", "re"]),
        after in prop::sample::select(vec!["", "s", "_x", "ed", "9"]),
    ) {
        let program = format!("{before}{kw}{after}");
        prop_assert!(!jq(&[&program]), "false alarm: {program:?}");
    }

    #[test]
    fn every_short_cluster_holding_f_or_l_is_seen(
        others in "[nrcsSaejMCR]{0,4}",
        flag in prop::sample::select(vec!['f', 'L']),
        split in 0usize..5,
    ) {
        let at = split.min(others.len());
        let word = format!("-{}{flag}{}", &others[..at], &others[at..]);
        prop_assert!(jq(&[&word]), "missed: {word}");
    }

    #[test]
    fn every_yq_loader_spelling_is_seen_outside_a_field(
        name in prop::sample::select(vec!["load", "load_str", "loadstr", "strload", "str_load", "load_xml", "loadxml", "xml_load", "xmlload", "load_props", "loadprops", "load_base64", "loadbase64", "eval"]),
        lead in prop::sample::select(vec!["", " ", "|", "| ", "(", ".a = ", "..|", ",", "[", "\n"]),
    ) {
        let program = format!("{lead}{name}(\"x\")");
        prop_assert!(yq(&[&program]), "missed: {program:?}");
        prop_assert!(yq(&["eval", &program]), "missed after eval: {program:?}");
    }

    /// Whatever options come first, each taking its own values (which may look like flags or
    /// filters), the filter is the word after them, and a directive in it is seen.
    #[test]
    fn the_filter_is_found_behind_any_options(
        options in prop::collection::vec(prop::sample::select(vec![
            vec!["-n"], vec!["-r"], vec!["-nr"], vec!["--tab"], vec!["--args"], vec!["--indent=2"],
            vec!["--arg", "k", "include"], vec!["--argjson", "k", "1"], vec!["--slurpfile", "k", "./f"],
            vec!["--rawfile", "k", "-x"], vec!["--indent", "2"], vec!["--arg", "-n", "."],
        ]), 0..5),
        program in prop::sample::select(vec!["include \"m\"; f", ".a", "\"m\" | modulemeta", "."]),
        trailing in prop::collection::vec(prop::sample::select(vec![
            vec!["-r"], vec!["--arg", "k", "include"], vec!["--indent", "2"], vec!["--", "-x"],
        ]), 0..3),
    ) {
        let mut args: Vec<&str> = options.concat();
        args.push(program);
        args.push("./d.json");
        args.extend(trailing.concat());
        prop_assert_eq!(filter_words(&args, &JQ, false), vec![program]);
        prop_assert_eq!(jq(&args), program.contains("include") || program.contains("modulemeta"), "{:?}", args);
    }

    #[test]
    fn checking_never_panics(words in prop::collection::vec(any::<String>(), 0..4)) {
        let refs: Vec<&str> = words.iter().map(String::as_str).collect();
        let _ = jq(&refs);
        let _ = yq(&refs);
    }
}
