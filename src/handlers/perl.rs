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
        if bytes[i] == b'$' || bytes[i] == b'@' || bytes[i] == b'%' {
            i += 1;
            if i < bytes.len() && (bytes[i] == b'_' || bytes[i].is_ascii_alphabetic()) {
                while i < bytes.len() && (bytes[i] == b'_' || bytes[i].is_ascii_alphanumeric()) {
                    i += 1;
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
                        Some(code) if perl_code_is_safe(&code) => {}
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
    if has_code && code_all_safe {
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
    }
}
