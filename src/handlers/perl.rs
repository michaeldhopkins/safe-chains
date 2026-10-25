use super::perl_regex::{has_substitution_eval, strip_inert_text};
use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static SAFE_PERL_WORDS: WordSet = WordSet::new(&[
    "ARGV", "BEGIN", "END", "STDERR", "STDIN", "STDOUT", "abs", "and", "atan2", "chomp", "chop", "chr", "close", "cmp", "cos", "defined",
    "delete", "die", "each", "else", "elsif", "eof", "eq", "exists", "exp", "for", "foreach", "ge", "grep", "gt", "hex", "if", "int",
    "join", "keys", "last", "lc", "lcfirst", "le", "length", "local", "log", "lt", "map", "my", "ne", "next", "no", "not", "oct", "or",
    "ord", "our", "pack", "pop", "pos", "print", "printf", "push", "qq", "qr", "qw", "ref", "return", "reverse", "rindex", "say", "scalar",
    "shift", "sin", "sort", "splice", "split", "sprintf", "sqrt", "substr", "tell", "tr", "uc", "ucfirst", "undef", "unless", "unpack",
    "unshift", "until", "values", "wantarray", "warn", "while",
]);

pub(super) fn special_scalar_at(bytes: &[u8], i: usize) -> Option<usize> {
    let mut j = i + 1;
    while bytes.get(j).is_some_and(u8::is_ascii_whitespace) {
        j += 1;
    }
    if bytes.get(j) == Some(&b'{') {
        let close = j + 1 + bytes.get(j + 1..)?.iter().position(|&b| b == b'}' || b == b'{')?;
        let inner = bytes[j + 1..close].trim_ascii();
        return (bytes[close] == b'}' && matches!(inner, b"^I" | b"/" | b"\\" | b",")).then_some(close + 1);
    }
    if bytes.get(j..j + 2) == Some(b"^I") {
        return Some(j + 2);
    }
    matches!(bytes.get(j), Some(b'/' | b'\\' | b',')).then_some(j + 1)
}

pub(super) fn is_interp_punct_var(b: Option<u8>) -> bool {
    b.is_some_and(|c| c.is_ascii_punctuation() && !matches!(c, b'(' | b')' | b'[' | b']' | b'{' | b'}' | b'<' | b'>' | b'\\' | b'|' | b'"'))
}

pub(super) fn skip_interp_var_head(bytes: &[u8], mut i: usize) -> Option<usize> {
    if bytes.get(i) == Some(&b'^') {
        return Some(i + 1 + usize::from(bytes.get(i + 1).is_some_and(u8::is_ascii_alphabetic)));
    }
    if bytes.get(i).is_some_and(|b| b.is_ascii_punctuation() && !matches!(b, b'"' | b'\\' | b'_')) {
        return Some(i + 1);
    }
    let name = i;
    while i < bytes.len() && (bytes[i] == b'_' || bytes[i] == b':' || bytes[i].is_ascii_alphanumeric()) {
        i += 1;
    }
    (!bytes[name..i].starts_with(b"CORE::")).then_some(i)
}

fn is_assignment_eq(bytes: &[u8], k: usize) -> bool {
    if bytes.get(k) != Some(&b'=') || matches!(bytes.get(k + 1), Some(b'=' | b'~' | b'>')) {
        return false;
    }
    if k == 0 {
        return true;
    }
    match bytes[k - 1] {
        b'=' | b'!' => false,
        b'<' | b'>' => k >= 2 && bytes[k - 2] == bytes[k - 1],
        _ => true,
    }
}

fn mutates_argv(bytes: &[u8], start: usize, end: usize) -> bool {
    let mut j = end;
    while bytes.get(j).is_some_and(u8::is_ascii_whitespace) {
        j += 1;
    }
    if bytes.get(j) == Some(&b'}') {
        j += 1;
    }
    while j < bytes.len() && !matches!(bytes[j], b';' | b'{' | b'}') {
        if is_assignment_eq(bytes, j) || matches!(bytes.get(j..j + 2), Some(b"++" | b"--" | b"=~" | b"!~")) {
            return true;
        }
        j += 1;
    }
    let mut p = start;
    while p > 0
        && (bytes[p - 1].is_ascii_alphanumeric()
            || matches!(bytes[p - 1], b'_' | b':' | b' ' | b'\t' | b'\n' | b'$' | b'@' | b'%' | b'#' | b'*' | b'{' | b'('))
    {
        if bytes[p - 1].is_ascii_alphanumeric() || bytes[p - 1] == b'_' {
            let mut w = p;
            while w > 0 && (bytes[w - 1].is_ascii_alphanumeric() || bytes[w - 1] == b'_') {
                w -= 1;
            }
            if matches!(
                &bytes[w..p],
                b"local"
                    | b"our"
                    | b"my"
                    | b"undef"
                    | b"push"
                    | b"unshift"
                    | b"splice"
                    | b"pop"
                    | b"shift"
                    | b"delete"
                    | b"chomp"
                    | b"chop"
                    | b"substr"
                    | b"for"
                    | b"foreach"
            ) {
                return true;
            }
            if !bytes[p..start].starts_with(b"::") {
                break;
            }
            p = w;
        } else {
            p -= 1;
        }
    }
    p > 0 && (bytes[p - 1] == b'\\' || bytes[p - 1] == b'}' || (p >= 2 && matches!(&bytes[p - 2..p], b"++" | b"--")))
}

pub(crate) fn perl_code_is_safe(token: &Token) -> bool {
    if crate::parse::has_unknown_content(token) || has_substitution_eval(token.as_str()) {
        return false;
    }
    let Some(stripped) = strip_inert_text(token.as_str()) else { return false };
    if stripped.contains('`') {
        return false;
    }
    let bytes = stripped.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if bytes.get(i + 1) == Some(&b'&') {
                i += 2;
                continue;
            }
            let mut j = i + 1;
            while bytes.get(j).is_some_and(u8::is_ascii_whitespace) {
                j += 1;
            }
            if bytes.get(j).is_some_and(|b| matches!(b, b'{' | b'$' | b'_' | b':') || b.is_ascii_alphabetic()) {
                return false;
            }
            i += 1;
            continue;
        }
        if bytes.get(i..i + 2) == Some(b"->") || bytes[i] == b']' {
            let mut j = i + if bytes[i] == b']' { 1 } else { 2 };
            while bytes.get(j).is_some_and(u8::is_ascii_whitespace) {
                j += 1;
            }
            if bytes.get(j) == Some(&b'(') {
                return false;
            }
        }
        if bytes[i] == b'$' || bytes[i] == b'@' || bytes[i] == b'%' {
            i += 1;
            while matches!(bytes.get(i), Some(b'#' | b' ' | b'\t')) {
                i += 1;
            }
            if bytes.get(i) == Some(&b'$') {
                return false;
            }
            if bytes.get(i) == Some(&b'{') {
                let start = i + 1;
                let mut end = start;
                while bytes.get(end).is_some_and(|b| *b == b'_' || b.is_ascii_alphanumeric()) {
                    end += 1;
                }
                if end == start || bytes.get(end) != Some(&b'}') || (&bytes[start..end] == b"ARGV" && mutates_argv(bytes, start, end)) {
                    return false;
                }
                i = end + 1;
                continue;
            }
            if i < bytes.len() && (bytes[i] == b'_' || bytes[i].is_ascii_alphabetic()) {
                let start = i;
                while i < bytes.len() && (bytes[i] == b'_' || bytes[i].is_ascii_alphanumeric()) {
                    i += 1;
                }
                if &bytes[start..i] == b"ARGV" && mutates_argv(bytes, start, i) {
                    return false;
                }
            }
            continue;
        }
        if bytes[i] == b'_' || bytes[i].is_ascii_alphabetic() {
            let start = i;
            while i < bytes.len() && (bytes[i] == b'_' || bytes[i].is_ascii_alphanumeric()) {
                i += 1;
            }
            let word = &stripped[start..i];
            if word == "ARGV" && mutates_argv(bytes, start, i) {
                return false;
            }
            if word.len() > 1 && !SAFE_PERL_WORDS.contains(word) {
                return false;
            }
            continue;
        }
        i += 1;
    }
    true
}

/// What a `perl` invocation asks for, once its flag grammar is walked: whether every `-e`/`-E`
/// one-liner passed the identifier gate, whether `-i` turns the operands into in-place rewrites,
/// and which operands are the FILES. The engine (`resolve_perl`) gates those files by locus; this
/// scan only reports them, because "which token is a file" is perl-grammar knowledge and "may this
/// path be read/written" is engine knowledge.
pub(crate) struct PerlScan {
    /// Where the code came from and whether it could be read.
    pub code: PerlCode,
    /// `-i[SUFFIX]` — the file operands are rewritten in place rather than read.
    pub in_place: bool,
    /// Operands left after the flag walk. With `-e` these are input files; without it the first
    /// would be a SCRIPT file, which [`PerlCode::Opaque`] already refuses.
    pub files: Vec<String>,
    /// Directories `-I` and `-Mlib=` put on the module path, whose modules a `-M` then runs.
    pub load_paths: Vec<String>,
}

/// What perl was asked to run.
#[derive(PartialEq, Eq)]
pub(crate) enum PerlCode {
    /// `--version` / `--help` and friends — reports on perl itself, running nothing.
    None,
    /// A `-e`/`-E` one-liner was present AND every one passed [`perl_code_is_safe`].
    Inspectable,
    /// No readable one-liner: either the code is a script FILE operand, or it used identifiers
    /// outside the modeled vocabulary. Both are arbitrary execution.
    Opaque,
}

/// Walk `perl`'s flag grammar. `None` when a token shape isn't modeled, so the caller worst-cases
/// rather than guessing which operands were files.
pub(crate) fn scan_perl(tokens: &[Token]) -> Option<PerlScan> {
    let mut scan = PerlScan { code: PerlCode::Opaque, in_place: false, files: Vec::new(), load_paths: Vec::new() };
    if tokens.len() == 2 && tokens[1].is_one_of(&["--version", "--help", "-v", "-V"]) {
        scan.code = PerlCode::None;
        return Some(scan);
    }

    let (mut has_code, mut code_all_safe) = (false, true);
    let mut pieces = Vec::new();
    let mut flags_done = false;
    let mut i = 1;
    while i < tokens.len() {
        let token = &tokens[i];
        if flags_done || !token.starts_with("-") || *token == "-" {
            scan.files.push(token.as_str().to_string());
            i += 1;
            continue;
        }
        if *token == "--" {
            flags_done = true;
            i += 1;
            continue;
        }
        if token.starts_with("--") {
            return None;
        }
        let flags = &token.as_str()[1..];
        if let Some(used) = super::perl_load::load_flag(tokens, i, &mut scan.load_paths)? {
            i += used;
            continue;
        }
        // Scan the cluster LEFT TO RIGHT, because both `-e` and `-i` swallow the rest of it and
        // which one wins is decided by which comes first. `-eprint` is `-e` with the code `print`;
        // `-i.bake` is `-i` with the backup suffix `.bake`; `-pie` is `-p -i` with the suffix `e`,
        // which is why it carries no code and cannot be an in-place edit of an inspected one-liner.
        // Reading the cluster as a set instead made `-i.bake` unparseable and `-eprint` opaque.
        let mut consumed_next = false;
        for (at, c) in flags.char_indices() {
            match c {
                'e' | 'E' => {
                    has_code = true;
                    let glued = &flags[at + 1..];
                    let code = if glued.is_empty() {
                        consumed_next = true;
                        tokens.get(i + 1).cloned()
                    } else {
                        Some(Token::from_raw(glued.to_string()))
                    };
                    match code {
                        Some(code) if perl_code_is_safe(&code) => pieces.push(code.as_str().to_string()),
                        _ => code_all_safe = false,
                    }
                    break;
                }
                'i' => {
                    scan.in_place = true;
                    break;
                }
                'F' | 'C' | '0' => break,
                'a' | 'c' | 'l' | 'n' | 'p' | 't' | 'T' | 'w' | 'W' | 'X' | '1'..='9' => {}
                _ => return None,
            }
        }
        i += 1 + usize::from(consumed_next);
    }
    if has_code && code_all_safe && (pieces.len() <= 1 || perl_code_is_safe(&Token::from_raw(pieces.join("\n")))) {
        scan.code = PerlCode::Inspectable;
    }
    Some(scan)
}

/// The LEGACY verdict, kept at its historical answer: code-gated, `-i` refused outright, operands
/// unexamined. It is dead for `perl` itself — `commands/perl/perl.toml` declares a behavior, so
/// `engine_verdict(…).unwrap_or(legacy)` always resolves through the hook — and it stays frozen
/// precisely so the never-looser corpus gate has a stable floor to compare the engine against.
/// The engine is the one that reads operands, because it is the layer that owns locus.
pub fn is_safe_perl(tokens: &[Token]) -> bool {
    match scan_perl(tokens) {
        Some(scan) => match scan.code {
            PerlCode::None => true,
            PerlCode::Inspectable => !scan.in_place,
            PerlCode::Opaque => false,
        },
        None => false,
    }
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "perl" => Some(if is_safe_perl(tokens) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        perl_version: "perl --version",
        perl_help: "perl --help",
        perl_v: "perl -v",
        perl_big_v: "perl -V",
        perl_print_hello: "perl -e 'print \"hello\\n\"'",
        perl_say: "perl -E 'say \"hello\"'",
        perl_ne_grep: "perl -ne 'print if /pattern/' file.txt",
        perl_pe_substitute: "perl -pe 's/foo/bar/g' file.txt",
        perl_lane_field: "perl -lane 'print $F[0]' file.txt",
        perl_chomp_split_join: "perl -ne 'chomp; print join(\",\", split(/\\t/)), \"\\n\"'",
        perl_tr_transliterate: "perl -pe 'tr/a-z/A-Z/' file.txt",
        perl_begin_end_count: "perl -ne 'BEGIN{$n=0} $n++; END{print $n}'",
        perl_ne_word_pattern: "perl -ne 'print if /\\berror\\b/' log.txt",
        perl_my_variable: "perl -e 'my $x = 1; print $x'",
        perl_keys_values: "perl -e 'my %h; print keys %h'",
        perl_string_containing_system: "perl -e 'print \"system is down\\n\"'",
        perl_substitute_alternate_delim: "perl -pe 's{error_count}{warning_count}g' file.txt",
        perl_match_with_if: "perl -ne 'print if /TODO/' file.txt",
        perl_match_after_unless: "perl -ne 'print unless /^#/' file.txt",
        perl_module_flag: "perl -MList::Util -e 'print length \"test\"'",
        perl_include_flag: "perl -Ilib -e 'print \"ok\\n\"'",
        perl_inplace_worktree: "perl -i -pe 's/foo/bar/' file.txt",
        perl_inplace_backup_worktree: "perl -i.bak -pe 's/foo/bar/' file.txt",
        perl_inplace_pi_worktree: "perl -pi -e 's/foo/bar/' ./app/x.rb",
        // `-i` swallows the REST of its cluster as the backup suffix, so a suffix that happens to
        // contain an `e` is still just a suffix. Reading the cluster as a set of letters instead
        // made this unparseable.
        perl_inplace_suffix_containing_e: "perl -i.bake -pe 's/foo/bar/' file.txt",
        perl_inplace_suffix_orig: "perl -i.orig -pe 's/foo/bar/' file.txt",
        // `-e` likewise swallows the rest of its cluster, so glued code is the same request as
        // separated code and must classify the same way.
        perl_glued_code: "perl -eprint file.txt",
        perl_glued_code_with_p: "perl -peprint file.txt",
    }

    denied! {
        perl_script_file_denied: "perl script.pl",
        perl_no_e_flag_denied: "perl -n file.txt",
        perl_inplace_system_denied: "perl -i -pe 's/foo/bar/' /etc/hosts",
        perl_inplace_home_denied: "perl -i.bak -pe 's/foo/bar/' ~/.bashrc",
        perl_read_secret_denied: "perl -pe 's/foo/bar/' ~/.ssh/id_rsa",
        perl_read_system_denied: "perl -ne 'print' /etc/shadow",
        // Glued code goes through the same identifier gate as separated code.
        perl_glued_code_unsafe_denied: "perl -esystem(\"rm\") file.txt",
        // `-pie` is `-p -i` with the backup SUFFIX `e`, not `-p -i -e`. So it carries no
        // inspectable one-liner at all, and the token after it is a file rather than code.
        perl_pie_inplace_denied: "perl -pie 's/foo/bar/' file.txt",
        perl_system_denied: "perl -e 'system(\"rm -rf /\")'",
        perl_exec_denied: "perl -e 'exec(\"bad\")'",
        perl_backtick_denied: "perl -e 'print `ls`'",
        perl_qx_denied: "perl -e 'qx(ls)'",
        perl_eval_denied: "perl -e 'eval(\"bad code\")'",
        perl_open_denied: "perl -e 'open(FH, \">file\")'",
        perl_unlink_denied: "perl -e 'unlink(\"file\")'",
        perl_rename_denied: "perl -e 'rename(\"a\", \"b\")'",
        perl_mkdir_denied: "perl -e 'mkdir(\"dir\")'",
        perl_rmdir_denied: "perl -e 'rmdir(\"dir\")'",
        perl_chmod_denied: "perl -e 'chmod(0755, \"file\")'",
        perl_truncate_denied: "perl -e 'truncate(\"file\", 0)'",
        perl_substitution_eval_denied: "perl -pe 's/foo/bar/e' file.txt",
        perl_substitution_eval_global_denied: "perl -pe 's/foo/bar/ge' file.txt",
        perl_use_denied: "perl -e 'use POSIX'",
        perl_require_denied: "perl -e 'require POSIX'",
        perl_fork_denied: "perl -e 'fork()'",
        perl_socket_denied: "perl -e 'socket(S, 2, 1, 0)'",
        perl_system_trailing_help_denied: "perl -e 'system(\"rm\")' --help",
        perl_system_trailing_version_denied: "perl -e 'system(\"rm\")' --version",
        // Double-quote INTERPOLATION executes code — the string-stripping bypass class.
        perl_interp_arrayref_system: "perl -e 'print \"@{[system(q(id))]}\"'",
        perl_interp_scalarref_system: "perl -e 'print \"${\\ system(q(id))}\"'",
        perl_interp_backtick: "perl -e 'print \"@{[`id`]}\"'",
        perl_interp_hash_subscript: "perl -e 'print \"$h{`id`}\"'",
        perl_interp_array_subscript: "perl -e 'print \"$a[`id`]\"'",
        perl_subref_block_string_denied: "perl -e '&{\"system\"}(\"id\")'",
        perl_subref_block_var_denied: "perl -e '&{$f}(\"id\")'",
        perl_subref_block_spaced_denied: "perl -e '& { $f }(\"id\")'",
        perl_subref_scalar_denied: "perl -e '&$x(\"id\")'",
        perl_subref_scalar_bare_denied: "perl -e '&$x'",
        perl_subref_scalar_spaced_denied: "perl -e '& $x(\"id\")'",
        perl_ampersand_named_sub_denied: "perl -e '&foo(\"id\")'",
        perl_ampersand_safe_word_sub_denied: "perl -e '&print(\"hi\")'",
        perl_ampersand_single_char_sub_denied: "perl -e '&f(\"id\")'",
        perl_ampersand_root_pkg_sub_denied: "perl -e '&::print(\"hi\")'",
        perl_arrow_call_denied: "perl -e '$x->(\"id\")'",
        perl_arrow_call_empty_denied: "perl -e '$x->()'",
        perl_arrow_call_spaced_denied: "perl -e '$x -> (\"id\")'",
        perl_subscript_arrow_call_denied: "perl -e '$a[0]->(\"id\")'",
        perl_subscript_implicit_arrow_call_denied: "perl -e '$a[0](\"id\")'",
        perl_core_call_denied: "perl -e 'CORE::system(\"id\")'",
        perl_core_safe_word_call_denied: "perl -e 'CORE::print(\"hi\")'",
        perl_core_global_call_denied: "perl -e 'CORE::GLOBAL::system(\"id\")'",
        perl_core_global_safe_word_call_denied: "perl -e 'CORE::GLOBAL::print(\"hi\")'",
        perl_interp_subref_block_denied: "perl -e 'print \"@{[&{q(system)}(q(id))]}\"'",
        perl_interp_subref_scalar_denied: "perl -e 'print \"@{[&$x]}\"'",
        perl_interp_ampersand_name_denied: "perl -e 'print \"@{[&print(1)]}\"'",
        perl_interp_arrow_call_denied: "perl -e 'print \"@{[$x->()]}\"'",
        perl_interp_core_call_denied: "perl -e 'print \"@{[CORE::print(1)]}\"'",
        perl_interp_core_global_call_denied: "perl -e 'print \"@{[CORE::GLOBAL::print(1)]}\"'",
        perl_interp_subscript_core_call_denied: "perl -e 'print \"$a[CORE::shift]\"'",
        perl_interp_punct_subscript_ampersand_denied: "perl -e 'print \"$+{&print}\"'",
        perl_interp_braced_arrow_subscript_denied: "perl -e 'print \"${a}->[CORE::shift]\"'",
        perl_qq_interp_core_denied: "perl -e 'print qq{${CORE::shift}}'",
        perl_qq_interp_punct_subscript_denied: "perl -e 'print qq{$+{&print}}'",
        perl_assign_argv_denied: "perl -ne 'BEGIN { @ARGV = (\"/etc/passwd\") } print' file.txt",
        perl_assign_argv_element_denied: "perl -ne 'BEGIN { $ARGV[0] = \"/etc/passwd\" } print' file.txt",
        perl_assign_argv_last_index_denied: "perl -ne 'BEGIN { $#ARGV = 0 } print' file.txt",
        perl_push_argv_denied: "perl -ne 'BEGIN { push @ARGV, \"/etc/passwd\" } print' file.txt",
        perl_assign_inplace_var_denied: "perl -pe 'BEGIN { $^I = \".bak\" } s/a/b/' file.txt",
        perl_assign_inplace_var_braced_denied: "perl -pe 'BEGIN { ${^I} = \"\" } s/a/b/' file.txt",
        perl_assign_input_record_sep_denied: "perl -ne 'BEGIN { $/ = undef } print' file.txt",
        perl_local_input_record_sep_denied: "perl -ne 'BEGIN { local $/ } print' file.txt",
        perl_assign_output_record_sep_denied: "perl -ne 'BEGIN { $\\ = \"\\n\" } print' file.txt",
        perl_assign_output_field_sep_denied: "perl -ne 'BEGIN { $, = \",\" } print' file.txt",
        perl_assign_list_special_vars_denied: "perl -ne 'BEGIN { ($/, $\\, $,) = (\"a\", \"b\", \"c\") } print' file.txt",
        perl_split_e_assign_inplace_denied: "perl -pe '$^I' -e '= \".bak\"' file.txt",
        perl_assign_inplace_glued_op_denied: "perl -pe 'BEGIN { $^Ix=1 } s/a/b/' file.txt",
        perl_symref_inplace_denied: "perl -pe 'BEGIN { ${\"^I\"} = \".bak\" } s/a/b/' file.txt",
        perl_symref_argv_denied: "perl -ne 'BEGIN { @{\"ARGV\"} = (\"/etc/passwd\") } print' file.txt",
    }

    use crate::parse::Token;
    use proptest::prelude::*;

    proptest::proptest! {
        // IDEMPOTENCE (identity-on-non-paired). `closing_delimiter` is the identity on every byte
        // that is NOT a paired opener — unpaired punctuation, control bytes, and the CLOSING half
        // of a pair like `)` or `>` all round-trip unchanged. On a paired opener the function
        // returns a closer that is itself NOT a paired opener (so calling it again would be a
        // no-op, not a recursion). Together this is the full byte-wise characterization of the
        // function across all 256 values, which `skip_delimited` relies on: closing what is not
        // already a paired opener would pair up a delimiter the caller intended as plain text.
        #[test]
        fn closing_delimiter_is_identity_exactly_on_non_paired_bytes(
            o in any::<u8>(),
        ) {
            if super::super::perl_regex::is_paired_delimiter(o) {
                let c = super::super::perl_regex::closing_delimiter(o);
                proptest::prop_assert_ne!(c, o,
                    "paired opener 0x{:02x} must map to a different closer", o);
                proptest::prop_assert!(!super::super::perl_regex::is_paired_delimiter(c),
                    "the closer of paired opener 0x{:02x} must itself not be a paired opener", o);
                proptest::prop_assert_eq!(super::super::perl_regex::closing_delimiter(c), c,
                    "the closer of a paired opener must be a fixed point of closing_delimiter");
            } else {
                proptest::prop_assert_eq!(super::super::perl_regex::closing_delimiter(o), o,
                    "non-paired byte 0x{:02x} must be the identity under closing_delimiter", o);
            }
        }

        // INVARIANT (long-flag bail-out). The flag walker models short flags and the `--` sentinel,
        // but anything else starting with `--` is unknown — the handler returns `None` so the engine
        // can worst-case instead of guessing which operands matter. The two informational forms
        // (`--version`, `--help`) short-circuit BEFORE this rule and only when tokens.len() == 2, so
        // excluding them keeps this property pinning the bail-out path specifically.
        #[test]
        fn scan_perl_with_unknown_long_flag_returns_none(
            flag in "--[a-z][a-z0-9]{1,8}",
            tail in proptest::collection::vec("[a-z.]{0,6}", 0..=4),
        ) {
            proptest::prop_assume!(flag != "--version");
            proptest::prop_assume!(flag != "--help");
            let mut raw: Vec<String> = vec!["perl".to_string(), flag];
            raw.extend(tail);
            let tokens: Vec<Token> = raw.iter().map(|s| Token::from_test(s)).collect();
            proptest::prop_assert!(
                super::scan_perl(&tokens).is_none(),
                "unknown long flag must return None, got tokens {:?}",
                raw,
            );
        }

        // EQUIVALENCE (single-substitution /e detection). `has_substitution_eval` walks the code
        // looking for `s<delim>...<delim>...<delim><flags>` and reports whether `e` is among the
        // flags. With ONE isolated substitution whose bodies contain no `s`, the scan cannot find
        // any other `s` to trigger on, and the only `s` present is the literal starting the
        // substitution — so the answer is exactly whether `e` appears in the flags. Bodies come
        // from a body-friendly alphabet that does not include the chosen delimiter, so no stray
        // match inside either body can short-circuit the walker.
        #[test]
        fn has_substitution_eval_matches_e_flag_for_isolated_substitution(
            delim in proptest::sample::select(&['/', '|', '!', '?', ',', ';', ':', '^', '#'][..]),
            body1 in "[a-z0-9]{0,4}",
            body2 in "[a-z0-9]{0,4}",
            flags in "[a-z]{0,5}",
        ) {
            proptest::prop_assume!(!body1.contains('s'));
            proptest::prop_assume!(!body2.contains('s'));
            let code = format!("s{delim}{body1}{delim}{body2}{delim}{flags}");
            let expected = flags.contains('e');
            proptest::prop_assert_eq!(
                super::has_substitution_eval(&code),
                expected,
                "isolated substitution {:?} (expected e={})",
                code,
                expected,
            );
        }

        #[test]
        fn a_sub_call_or_special_variable_assignment_is_never_approved(
            benign in proptest::sample::select(&[
                "print if /x/",
                "my $n = 1; print $n",
                "chomp; print",
                "print $/; print $\"",
            ][..]),
            flag in proptest::sample::select(&["-e", "-E", "-ne", "-pe"][..]),
            call in proptest::sample::select(&[
                "&{\"system\"}(\"id\")",
                "&{$f} (\"id\")",
                "& { $f }()",
                "&$x(\"id\")",
                "&$x",
                "& $x()",
                "&foo(\"id\")",
                "&print(\"hi\")",
                "&f()",
                "&::print(1)",
                "$x->(\"id\")",
                "$x->()",
                "$x -> ()",
                "$a[0]->(\"id\")",
                "$a[0](\"id\")",
                "CORE::system(\"id\")",
                "CORE::print(1)",
                "CORE::GLOBAL::system(\"id\")",
                "CORE::GLOBAL::print(1)",
            ][..]),
            interp_wrap in proptest::sample::select(&[
                "print \"@{[ EXPR ]}\"",
                "print \"${\\ EXPR }\"",
                "print \"${ EXPR }\"",
                "print \"$a[EXPR]\"",
                "print \"$h{EXPR}\"",
                "print \"$+{EXPR}\"",
                "print \"${a}->[EXPR]\"",
                "print qq{@{[ EXPR ]}}",
                "print qq{$+{EXPR}}",
            ][..]),
            assign in proptest::sample::select(&[
                "@ARGV = (\"x\")",
                "$ARGV[0] = \"x\"",
                "$#ARGV = 0",
                "@{ARGV} = (\"x\")",
                "push @ARGV, \"x\"",
                "unshift @ARGV, \"x\"",
                "$^I = \".bak\"",
                "${^I} = \"\"",
                "$ ^I = \"\"",
                "$/ = undef",
                "${/} = \"\"",
                "$ / = \"\"",
                "local $/",
                "$\\ = \"\\n\"",
                "${\\} = \"\\n\"",
                "$ \\ = \"\\n\"",
                "$, = \",\"",
                "${,} = \",\"",
                "$ , = \",\"",
                "($/, $\\, $,) = (\"a\", \"b\", \"c\")",
            ][..]),
        ) {
            let safe_line = format!("perl {flag} {} file.txt", shell_words::quote(benign));
            proptest::prop_assert!(check(&safe_line), "benign one-liner refused: {safe_line}");

            let direct_call = format!("perl {flag} {} file.txt", shell_words::quote(&format!("{benign}; {call}")));
            proptest::prop_assert!(!check(&direct_call), "sub call approved: {direct_call}");

            let interpolated = interp_wrap.replacen("EXPR", call, 1);
            let interp_line = format!("perl {flag} {} file.txt", shell_words::quote(&format!("{benign}; {interpolated}")));
            proptest::prop_assert!(!check(&interp_line), "interpolated sub call approved: {interp_line}");

            let assign_line = format!("perl {flag} {} file.txt", shell_words::quote(&format!("{benign}; {assign}")));
            proptest::prop_assert!(!check(&assign_line), "special variable assignment approved: {assign_line}");
        }
    }
}
