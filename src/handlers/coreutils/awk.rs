use crate::parse::{Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagTolerance};
use crate::verdict::{SafetyLevel, Verdict};

fn strip_regex_literals(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' {
            result.push(b' ');
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'/' {
                    i += 1;
                    break;
                }
                i += 1;
            }
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(result).unwrap_or_default()
}

fn has_shell_pipe(code: &str) -> bool {
    let collapsed = code.replace("||", "  ");
    collapsed.contains('|')
}

fn has_redirect(code: &str) -> bool {
    let bytes = code.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'>' && !(i + 1 < bytes.len() && bytes[i + 1] == b'=') {
            if b == b'>' && i > 0 && bytes[i - 1] == b'>' {
                return true;
            }
            let stmt_start = bytes[..i].iter().rposition(|&c| c == b';' || c == b'{').map_or(0, |p| p + 1);
            let before = &code[stmt_start..i];
            if before.contains("printf") || before.contains("print") {
                return true;
            }
        }
    }
    false
}

fn has_dangerous_getline(code: &str) -> bool {
    let mut search = code;
    while let Some(pos) = search.find("getline") {
        let after = &search[pos + 7..];
        let after_trimmed = after.trim_start();
        let skip_var = if !after_trimmed.is_empty() && after_trimmed.as_bytes()[0].is_ascii_alphabetic() {
            let var_end = after_trimmed.find(|c: char| !c.is_ascii_alphanumeric() && c != '_').unwrap_or(after_trimmed.len());
            after_trimmed[var_end..].trim_start()
        } else {
            after_trimmed
        };
        if skip_var.starts_with('<') {
            return true;
        }
        let before = &code[..search.as_ptr() as usize - code.as_ptr() as usize + pos];
        let before_trimmed = before.trim_end();
        if before_trimmed.ends_with('|') {
            return true;
        }
        search = &search[pos + 7..];
    }
    false
}

fn awk_has_dangerous_construct(token: &Token) -> bool {
    let code = token.content_outside_double_quotes();
    if code.contains("system") {
        return true;
    }
    if has_dangerous_getline(&code) {
        return true;
    }
    let stripped = strip_regex_literals(&code);
    has_shell_pipe(&stripped) || has_redirect(&stripped)
}

/// Whether a `/` at this point opens a regex literal rather than dividing. awk decides by what came
/// before: an operand ends with a name, a number, `)`, `]`, a literal or a postfix `++`/`--`, and
/// after one of those a `/` divides. Every case not recognised as a regex is read as division,
/// which leaves the text after it in view of the checks.
fn regex_may_start(out: &[u8], last: Option<usize>, after_literal: bool) -> bool {
    if after_literal {
        return false;
    }
    let Some(end) = last else {
        return true;
    };
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    match out[end] {
        c @ (b'+' | b'-') => end == 0 || out[end - 1] != c,
        b'(' | b',' | b'{' | b'}' | b';' | b'!' | b'~' | b'&' | b'|' | b'=' | b'<' | b'>' | b'?' | b':' | b'[' | b'\n' | b'*' | b'%'
        | b'^' | b'/' => true,
        c if ident(c) => ["print", "printf", "return", "case"].iter().any(|k| {
            let k = k.as_bytes();
            end + 1 >= k.len() && &out[end + 1 - k.len()..=end] == k && (end + 1 == k.len() || !ident(out[end - k.len()]))
        }),
        _ => false,
    }
}

/// The index just past the string or regex literal opening at `start`. A literal cannot span a
/// line, so an unterminated one ends at the newline and the next line is code again. With
/// `brackets`, a `/` inside a regex bracket expression (`/[/]/`) does not close the regex, as in
/// gawk; without, it does, as in other awks.
fn skip_literal(b: &[u8], start: usize, brackets: bool) -> usize {
    let close = b[start];
    let mut in_bracket = false;
    let mut i = start + 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            b'\n' => return i,
            b'[' if brackets && !in_bracket => {
                in_bracket = true;
                i += 1;
                if b.get(i) == Some(&b'^') {
                    i += 1;
                }
                if b.get(i) != Some(&b']') {
                    continue;
                }
            }
            b'[' if in_bracket && matches!(b.get(i + 1), Some(b':' | b'.' | b'=')) => {
                let delim = b[i + 1];
                i += 2;
                while i < b.len() && !(b[i] == delim && b.get(i + 1) == Some(&b']')) && b[i] != b'\n' {
                    i += 1;
                }
                if b.get(i) != Some(&delim) {
                    return i;
                }
                i += 1;
            }
            b']' if in_bracket => in_bracket = false,
            c if c == close && !in_bracket => return i + 1,
            _ => {}
        }
        i += 1;
    }
    b.len()
}

/// An awk program with every string and regex literal blanked to one space, leaving the code.
fn awk_code(src: &str, brackets: bool) -> String {
    let b = src.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut last: Option<usize> = None;
    let mut after_literal = false;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'"' || (c == b'/' && regex_may_start(&out, last, after_literal)) {
            i = skip_literal(b, i, brackets && c == b'/');
            out.push(b' ');
            after_literal = true;
            continue;
        }
        if !matches!(c, b' ' | b'\t') {
            after_literal = false;
            last = Some(out.len());
        }
        out.push(c);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Whether an awk program can run a command, write a file or load code: `system`, a pipe, a
/// redirect, `getline` from a file or command, or any `@` construct. gawk's `@f()` calls the
/// function NAMED by `f`, `system` included, so the name never appears in the program; `@load`
/// loads a shared library and `@include` reads another program (`@include "inplace"` rewrites the
/// input files). Literals are blanked first, under both readings of a `/` in a bracket
/// expression, so text inside a string or regex never hides code from the checks and code is never
/// mistaken for a literal under either reading.
///
/// `program` is false for a file operand, where an `@` is part of a file name (`icon@2x.txt`);
/// the other checks still apply, so an operand mistaken for a file is not waved through.
fn awk_program_runs_code(src: &str, program: bool) -> bool {
    [false, true].into_iter().any(|brackets| {
        let code = awk_code(src, brackets);
        (program && code.contains('@'))
            || code.contains("system")
            || has_dangerous_getline(&code)
            || has_shell_pipe(&code)
            || has_redirect(&code)
    })
}

/// Whether an option token takes the NEXT token as its value: `-F`/`-v` alone, the long forms, or a
/// short cluster ending in one of them (`-bF :`), as the flag policy reads a cluster.
fn takes_next_token(s: &str) -> bool {
    if matches!(s, "--assign" | "--field-separator") {
        return true;
    }
    if s.starts_with("--") {
        return false;
    }
    let cluster = s.as_bytes().get(1..).unwrap_or_default();
    cluster.iter().position(|&c| matches!(c, b'F' | b'v')).is_some_and(|p| p + 1 == cluster.len())
}

static AWK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--characters-as-bytes", "--copyright", "--gen-pot", "--lint", "--no-optimize", "--optimize", "--posix", "--re-interval",
        "--sandbox", "--traditional", "--use-lc-numeric", "--version", "-C", "-N", "-O", "-P", "-S", "-V", "-b", "-c", "-g", "-r", "-s",
        "-t",
    ]),
    valued: WordSet::flags(&["--assign", "--field-separator", "-F", "-v"]),
    bare: false,
    max_positional: None,
    tolerance: FlagTolerance::strict(),
};

/// awk's operands: the inline program, then the files (`awk 'prog' files…`). Everything after `--`
/// is an operand, a program starting with `-` included. (`-f progfile` is already denied by the
/// flag policy, so the first operand is always the program.)
fn awk_operands(tokens: &[Token]) -> Vec<&str> {
    let mut positionals = Vec::new();
    let mut options_done = false;
    let mut i = 1;
    while i < tokens.len() {
        let s = tokens[i].as_str();
        if !options_done && s == "--" {
            options_done = true;
            i += 1;
            continue;
        }
        if !options_done && s.starts_with('-') && takes_next_token(s) {
            i += 2; // valued flag + its separate value
            continue;
        }
        if !options_done && s.starts_with('-') && s != "-" {
            i += 1; // standalone flag, or a glued valued flag (`-F:`, `-vx=1`)
            continue;
        }
        positionals.push(s);
        i += 1;
    }
    positionals
}

fn is_safe_awk(tokens: &[Token]) -> bool {
    for token in &tokens[1..] {
        if !token.starts_with("-") && awk_has_dangerous_construct(token) {
            return false;
        }
    }
    let operands = awk_operands(tokens);
    if operands.iter().enumerate().any(|(n, o)| awk_program_runs_code(o, n == 0)) {
        return false;
    }
    if !policy::check(tokens, &AWK_POLICY) {
        return false;
    }
    // Gate the FILE operands by read locus, so `awk '{print}' /etc/shadow` denies (audit fix). The
    // program is skipped: it is a regex/action, not a path, and would false-positive the gate.
    !operands
        .iter()
        .skip(1)
        .any(|f| crate::policy::looks_like_path(f) && crate::engine::resolve::read_content_verdict(f) == Verdict::Denied)
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "awk" | "gawk" | "mawk" | "nawk" => Some(if is_safe_awk(tokens) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![crate::docs::CommandDoc::handler(
        "awk / gawk / mawk / nawk",
        "https://www.gnu.org/software/gawk/manual/gawk.html",
        format!("- Program validated: system, getline, |, > constructs checked\n{}", AWK_POLICY.describe()),
        "text",
    )]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "awk", valid_prefix: Some("awk '{print}'") },
    crate::handlers::CommandEntry::Custom { cmd: "gawk", valid_prefix: Some("gawk '{print}'") },
    crate::handlers::CommandEntry::Custom { cmd: "mawk", valid_prefix: Some("mawk '{print}'") },
    crate::handlers::CommandEntry::Custom { cmd: "nawk", valid_prefix: Some("nawk '{print}'") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        awk_print_field: "awk '{print $1}' file.txt",
        awk_print_multiple_fields: "awk '{print $1, $3}' file.txt",
        awk_field_separator: "awk -F: '{print $1}' ./data.txt",
        awk_pattern: "awk '/error/ {print $0}' log.txt",
        awk_nr: "awk 'NR==5' file.txt",
        awk_begin_end_safe: "awk 'BEGIN{n=0} {n++} END{print n}' file.txt",
        gawk_safe: "gawk '{print $2}' file.txt",
        awk_netstat_pipeline: "awk '{print $6}'",
        awk_comparison_gte: "awk 'NR>=10 {print}' file.txt",
        awk_comparison_gte_complex: "awk '{if(length($0)>=80) print NR\": \"$0}' file.txt",
        awk_multiple_comparisons: "awk 'NR>=5 && NR<=20' file.txt",
        awk_logical_or: "awk 'NF==1 || NR<5' file.txt",
        awk_logical_or_no_spaces: "awk 'NF==1||NR<5' file.txt",
        awk_logical_or_with_regex: "awk 'NF==1 || /^[A-Z]+ [0-9]+$/' file.txt",
        awk_logical_or_assign: "awk 'BEGIN{ok = a || b; print ok}'",
        awk_logical_or_three_terms: "awk 'NR==1 || NR==5 || NR==10' file.txt",
        awk_division: "awk '{print $1/100}' file.txt",
        awk_multiple_divisions: "awk '{avg=$1/10; pct=avg/total*100; print pct}' file.txt",
        awk_modulo_and_division: "awk '{print $1%10, $1/10}' file.txt",

        awk_string_literal_system: "awk 'BEGIN{print \"system failed\"}'",
        awk_string_literal_redirect: "awk '{print \">\"}'",
        awk_string_literal_pipe: "awk '{print \"a | b\"}'",
        awk_string_literal_getline: "awk 'BEGIN{print \"getline is a keyword\"}'",

        awk_regex_alternation: "awk '/foo|bar/ {print}' file.txt",
        awk_regex_multi_alt: "awk '/^def |^class |^end/ {print}' file.rb",
        awk_regex_redirect_char: "awk '/a>b/ {print}' file.txt",
        awk_regex_complex: "awk '/^  def /{m=$0; l=NR} NR-l>=10 && /^  def |^class |^end/{print}' file.rb",
        awk_regex_single_char: "awk '/^#/ {print}' file.txt",
        awk_regex_escaped_slash: "awk '/path\\/to/ {print}' file.txt",
        awk_regex_pipe_and_gte: "awk '/error|warning/ && NR>=10 {print}' log.txt",
        awk_regex_empty: "awk '/^$/ {print NR}' file.txt",
        awk_regex_pipe_in_match: "awk '$0 ~ /foo|bar/ {print}' file.txt",
        awk_regex_multiple_patterns: "awk '/start/,/end/ {print}' file.txt",
        awk_regex_redirect_in_char_class: "awk '/[><=]/ {print}' file.txt",
        awk_regex_pipe_in_char_class: "awk '/[|&]/ {print}' file.txt",
        awk_regex_mixed_with_math: "awk '/error|warn/ {c++} END{print c>=0 ? c : 0}' log.txt",
        awk_no_program_just_flag: "awk --version",
        awk_getline_plain: "awk '/pattern/{getline; print}' file",
        awk_getline_var: "awk '/pattern/{getline line; print line}' file",
        awk_getline_next_record: "awk 'NR%2==1{first=$0; getline; print first, $0}' file",
        awk_getline_print_nr: "awk '/pattern/{getline; print NR\": \"$0}' file",
        awk_comparison_gt: "awk 'length > 80' file.txt",
        awk_comparison_gt_print: "awk 'length > 80 {print}' file.txt",
        awk_comparison_nr_gt: "awk 'NR > 5' file.txt",
        awk_comparison_field_gt: "awk '$1 > 100 {print $0}' file.txt",
        awk_comparison_gt_conditional: "awk '{if(length($0) > 80) print NR}' file.txt",
        awk_comparison_gt_multi: "awk 'NR > 5 && NR < 20' file.txt",
        awk_regex_with_at: "awk '/user@example.com/ {print}' file.txt",
        awk_string_with_at: "awk '{print \"a@b\"}' file.txt",
        awk_dashdash_program: "awk -- '{print $1}' file.txt",
        awk_file_name_with_at: "awk '{print}' icon@2x.txt",
    }

    denied! {
        awk_system_denied: "awk 'BEGIN{system(\"rm -rf /\")}'",
        awk_getline_denied: "awk '{getline line < \"/etc/shadow\"; print line}'",
        awk_pipe_output_denied: "awk '{print $0 | \"mail user@host\"}'",
        awk_redirect_denied: "awk '{print $0 > \"output.txt\"}'",
        awk_append_denied: "awk '{print $0 >> \"output.txt\"}'",
        awk_file_program_denied: "awk -f script.awk data.txt",
        gawk_system_denied: "gawk 'BEGIN{system(\"rm\")}'",
        awk_system_call_denied: "awk 'BEGIN{system(\"rm\")}'",
        awk_system_space_paren_denied: "awk 'BEGIN{system (\"rm\")}'",
        awk_pipe_outside_string_denied: "awk '{print $0 | \"cmd\"}'",
        awk_redirect_outside_string_denied: "awk '{print $0 > \"file\"}'",
        awk_system_trailing_help_denied: "awk 'BEGIN{system(\"rm\")}' --help",
        awk_system_trailing_version_denied: "awk 'BEGIN{system(\"rm\")}' --version",
        awk_system_between_division_denied: "awk '{x=1/2;system(\"rm\");y=3/4}' file",
        awk_getline_from_file_denied: "awk '{getline line < \"input.txt\"; print line}' file",
        awk_getline_from_cmd_denied: "awk 'BEGIN{cmd=\"date\"; cmd | getline d; print d}'",
        awk_pipe_bare_denied: "awk '{cmd=\"sort\"; print $0 | cmd}' file",
        awk_redirect_bare_var_denied: "awk '{f=\"out.txt\"; print $0 > f}' file",
        awk_system_in_function_denied: "awk 'function run(){system(\"rm\")} BEGIN{run()}'",
        awk_getline_from_pipe_denied: "awk 'BEGIN{\"date\" | getline d; print d}'",
        awk_append_bare_denied: "awk '{print >> \"log.txt\"}' file",
        awk_redirect_no_space_denied: "awk '{print >\"out\"}' file",
        awk_pipe_no_space_denied: "awk '{print|\"cmd\"}' file",
        gawk_indirect_call_denied: "gawk 'BEGIN{f=\"sys\" \"tem\"; @f(\"id\")}'",
        gawk_load_denied: "gawk '@load \"filefuncs\"; BEGIN{print 1}'",
        gawk_include_inplace_denied: "gawk '@include \"inplace\"; {print \"x\"}' file.txt",
        awk_pipe_between_divisions_denied: "awk '{x=1/2; print $0 | \"sh\"; y=3/4}' file.txt",
        awk_quote_in_regex_hides_pipe_denied: "awk '/\"/ {print $0 | \"sh\"}' file.txt",
        awk_dashdash_program_denied: "awk -- '-1 {system(\"id\")}' file.txt",
        awk_dashdash_program_file_locus_denied: "awk -- '-1 {print}' /etc/shadow",
        gawk_cluster_value_then_indirect_call_denied: "gawk -bv n=0 'BEGIN{@f()}' file.txt",
    }
}

#[cfg(test)]
mod properties {
    use crate::is_safe_command;
    use proptest::prelude::*;

    /// Statements that run nothing, chosen so that a literal or a division in them could throw a
    /// literal scanner out of step: a `/` that divides, a regex holding `@`, `/` or `"`, a string
    /// holding every character the checks look for.
    const BENIGN: &[&str] = &[
        "x = 1/2", "y = NF / 3", "z = n++ / 2", "t = a[1] / 4", "u = (NR) / 5", "if ($0 ~ /a@b\\/c/) n++", "if ($0 ~ /[/]x/) n++",
        "if ($0 ~ /[[:alpha:]]@/) n++", "s = \"a/b \\\"q\\\" @ | > sys\" \"tem\"", "print $1, NR",
    ];

    /// Benign under the awk grammar, but a quote or a slash inside a regex that a scanner reading
    /// strings and regexes in separate passes gets wrong.
    const DISGUISES: &[&str] = &["if (/x\"y/) n++", "if (/[/\"]/) n++", "w = 1 / 2 / 3", "v = 6/ 2; q = /\"/"];

    /// Statements that run a command, write a file, read one, or call a function by name.
    const STATEMENTS: &[&str] = &[
        "f = \"sys\" \"tem\"; @f(\"id\")", "@g()", "system(\"id\")", "print $0 | \"sh\"", "print $0 |& \"sh\"", "print $0 > \"out\"",
        "printf \"x\" >> \"out\"", "\"date\" | getline d", "getline line < \"/etc/passwd\"",
    ];

    /// Top-level gawk directives that load code or another program.
    const DIRECTIVES: &[&str] =
        &["@load \"filefuncs\"", "@load \"./evil\"", "@include \"inplace\"", "@include \"x.awk\"", "@namespace \"n\""];

    const COMMANDS: &[&str] = &["awk", "gawk", "mawk", "nawk"];

    /// How the program reaches awk, `{p}` standing for it.
    const SHAPES: &[&str] = &[
        "{c} '{p}' f.txt", "{c} -bF : '{p}' f.txt", "{c} -bv n=0 '{p}' icon@2x.txt", "{c} -F: '{p}' f.txt", "{c} -v n=0 -- '{p}' f.txt",
        "{c} '{p}'",
    ];

    fn programs(pre: &str, post: &str, stmt: &str, directive: &str) -> Vec<String> {
        vec![
            format!("BEGIN {{ {pre}; {stmt}; {post} }}"),
            format!("/x/ {{ {pre}; {stmt}; {post} }}"),
            format!("{directive}\nBEGIN {{ {pre}; {post} }}"),
            format!("{directive}; BEGIN {{ {pre}; {post} }}"),
        ]
    }

    proptest! {
        /// No awk program that can run a command, write or read a file, call a function by name or
        /// load code is approved, whatever surrounds it and however it reaches awk. The
        /// surroundings are each approved on their own, so the construct is what is refused.
        #[test]
        fn an_awk_program_that_runs_code_is_never_approved(
            pre in proptest::sample::select([BENIGN, DISGUISES].concat()),
            post in proptest::sample::select([BENIGN, DISGUISES].concat()),
            stmt in proptest::sample::select(STATEMENTS.to_vec()),
            directive in proptest::sample::select(DIRECTIVES.to_vec()),
            cmd in proptest::sample::select(COMMANDS.to_vec()),
            shape in proptest::sample::select(SHAPES.to_vec()),
        ) {
            for program in programs(pre, post, stmt, directive) {
                let line = shape.replace("{c}", cmd).replace("{p}", &program);
                prop_assert!(!is_safe_command(&line), "awk program that runs code was approved: `{}`", line);
            }
            let dashed = format!("{cmd} -- '-1 {{ {pre}; {stmt}; {post} }}' f.txt");
            prop_assert!(!is_safe_command(&dashed), "awk program that runs code was approved: `{}`", dashed);
        }

        #[test]
        fn the_surroundings_alone_are_approved(
            pre in proptest::sample::select(BENIGN.to_vec()),
            post in proptest::sample::select(BENIGN.to_vec()),
            cmd in proptest::sample::select(COMMANDS.to_vec()),
            shape in proptest::sample::select(SHAPES.to_vec()),
        ) {
            let line = shape.replace("{c}", cmd).replace("{p}", &format!("BEGIN {{ {pre}; {post} }}"));
            prop_assert!(is_safe_command(&line), "benign awk program was refused: `{}`", line);
        }
    }
}
