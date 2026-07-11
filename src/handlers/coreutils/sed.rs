use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::{Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagTolerance};

/// What scanning sed's inline script(s) found. `exec` = an `e` command/flag (executes a shell
/// command — RCE). `unknown` = a command letter we don't model (fail closed). `writes`/`reads` are
/// the filenames the script's file commands touch — `w`/`W`/`s///w` write, `r`/`R` read — which the
/// caller path-gates by locus (a local write is fine; `/etc/cron.d/x` is not).
// Independent findings of one scan — not mutually exclusive (a `-f` invocation can also carry an
// inline `-e 'e'`), so they stay separate flags rather than an enum.
#[allow(clippy::struct_excessive_bools)]
#[derive(Default)]
pub(crate) struct SedScan {
    pub exec: bool,
    pub unknown: bool,
    /// A `-f`/`--file` script came from a file we can't read — its `e`/`w`/`r` commands are invisible,
    /// so the whole invocation is not auto-approved (like `awk -f`, `bash script.sh`, mlr `--load`).
    pub script_file: bool,
    pub writes: Vec<String>,
    pub reads: Vec<String>,
}

/// Scan every inline script of a `sed` invocation: each `-e`/`--expression` value (in ALL forms —
/// separate `-e S`, glued `-eS`, cluster `-neS`, and `--expression=S`), plus the first positional
/// when neither `-e` nor `-f` supplied the script. It must recognise every script-supplying form the
/// engine's flag parser does, so it neither misses a script nor mistakes an input FILE for one (a
/// file scanned as a script trips the unknown-command deny — the `sed -eS file` regression this
/// replaced). `-f FILE` scripts live in a file we can't see; the caller reads it as an operand.
pub(crate) fn scan_sed(tokens: &[Token]) -> SedScan {
    let mut scan = SedScan::default();
    let mut have_script = false; // a script came from -e/-f → positionals are input files
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if let Some(long) = t.strip_prefix("--") {
            let (name, glued) = match long.split_once('=') {
                Some((n, v)) => (n, Some(v)),
                None => (long, None),
            };
            match name {
                "expression" => {
                    have_script = true;
                    match glued {
                        Some(v) => {
                            scan_script(v.as_bytes(), &mut scan);
                            i += 1;
                        }
                        None => {
                            if let Some(v) = tokens.get(i + 1) {
                                scan_script(v.as_bytes(), &mut scan);
                            }
                            i += 2;
                        }
                    }
                }
                "file" => {
                    have_script = true;
                    scan.script_file = true;
                    i += if glued.is_some() { 1 } else { 2 };
                }
                "line-length" => i += if glued.is_some() { 1 } else { 2 },
                _ => i += 1, // other long flags are validated by the flag parser
            }
        } else if t.starts_with('-') && t != "-" {
            let bytes = t.as_bytes();
            let mut k = 1;
            let mut consumes_next = false;
            while k < bytes.len() {
                if !bytes[k].is_ascii() {
                    break; // a non-ASCII byte isn't a flag; the flag parser worst-cases it
                }
                match bytes[k] {
                    // -e/-f/-l take the rest of the cluster as the value, else the next token.
                    b'e' => {
                        have_script = true;
                        let glued = &t[k + 1..];
                        if glued.is_empty() {
                            if let Some(v) = tokens.get(i + 1) {
                                scan_script(v.as_bytes(), &mut scan);
                            }
                            consumes_next = true;
                        } else {
                            scan_script(glued.as_bytes(), &mut scan);
                        }
                        break;
                    }
                    b'f' => {
                        have_script = true;
                        scan.script_file = true;
                        consumes_next = t[k + 1..].is_empty();
                        break;
                    }
                    b'l' => {
                        consumes_next = t[k + 1..].is_empty();
                        break;
                    }
                    b'i' => break, // -i[SUFFIX]: the rest of the cluster is the backup suffix
                    _ => k += 1,   // a bool short flag (or one the flag parser will reject)
                }
            }
            i += 1 + usize::from(consumes_next);
        } else {
            if !have_script {
                scan_script(tokens[i].as_bytes(), &mut scan);
                have_script = true;
            }
            i += 1;
        }
    }
    scan
}

// A sed script is a sequence of `[addr[,addr]][!] command [args]`, separated by `;`/newline/blocks.
// We walk command by command, skipping addresses and the delimited bodies of `s///`/`y///` and
// `/regex/` addresses (so a `w`/`r`/`e` inside a regex or replacement is NOT mistaken for a command),
// and record the file/exec commands. Unknown command letters fail closed.
fn scan_script(b: &[u8], scan: &mut SedScan) {
    let n = b.len();
    let mut i = 0;
    while i < n {
        while i < n && matches!(b[i], b' ' | b'\t' | b'\n' | b';' | b'{' | b'}') {
            i += 1;
        }
        if i >= n {
            break;
        }
        if b[i] == b'#' {
            while i < n && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        i = skip_addresses(b, i);
        while i < n && matches!(b[i], b'!' | b' ' | b'\t') {
            i += 1;
        }
        if i >= n {
            break;
        }
        let cmd = b[i];
        i += 1;
        match cmd {
            b's' => i = scan_s_command(b, i, scan),
            b'y' => i = skip_two_delims(b, i),
            b'w' | b'W' => {
                let (f, ni) = read_filename(b, i);
                scan.writes.push(f);
                i = ni;
            }
            b'r' | b'R' => {
                let (f, ni) = read_filename(b, i);
                scan.reads.push(f);
                i = ni;
            }
            b'e' => {
                scan.exec = true;
                while i < n && b[i] != b'\n' {
                    i += 1;
                }
            }
            // a/i/c take text to end of line; b/t/T/: take a label; l/L/q/Q an optional number;
            // v a version string. None can hide a file/exec command (their args run to EOL).
            b'a' | b'i' | b'c' => {
                while i < n && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'b' | b't' | b'T' | b':' | b'v' => {
                while i < n && !matches!(b[i], b';' | b'\n') {
                    i += 1;
                }
            }
            b'l' | b'L' | b'q' | b'Q' => {
                while i < n && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            b'p' | b'P' | b'd' | b'D' | b'n' | b'N' | b'g' | b'G' | b'h' | b'H' | b'x' | b'='
            | b'z' | b'F' | b'{' | b'}' => {}
            _ => {
                scan.unknown = true;
                return;
            }
        }
    }
}

fn skip_addresses(b: &[u8], mut i: usize) -> usize {
    i = skip_one_address(b, i);
    while i < b.len() && matches!(b[i], b' ' | b'\t') {
        i += 1;
    }
    if i < b.len() && b[i] == b',' {
        i += 1;
        while i < b.len() && matches!(b[i], b' ' | b'\t') {
            i += 1;
        }
        i = skip_one_address(b, i);
    }
    i
}

fn skip_one_address(b: &[u8], mut i: usize) -> usize {
    let n = b.len();
    if i >= n {
        return i;
    }
    match b[i] {
        b'$' => i + 1,
        b'0'..=b'9' => {
            while i < n && (b[i].is_ascii_digit() || b[i] == b'~') {
                i += 1;
            }
            i
        }
        b'+' | b'~' => {
            i += 1;
            while i < n && b[i].is_ascii_digit() {
                i += 1;
            }
            i
        }
        b'/' => skip_regex(b, i + 1, b'/'),
        b'\\' if i + 1 < n => skip_regex(b, i + 2, b[i + 1]),
        _ => i,
    }
}

fn skip_regex(b: &[u8], mut i: usize, delim: u8) -> usize {
    let n = b.len();
    while i < n {
        if b[i] == b'\\' {
            i += 2;
            continue;
        }
        if b[i] == delim {
            i += 1;
            break;
        }
        i += 1;
    }
    while i < n && matches!(b[i], b'I' | b'M') {
        i += 1;
    }
    i
}

fn scan_s_command(b: &[u8], i: usize, scan: &mut SedScan) -> usize {
    let n = b.len();
    if i >= n {
        return i;
    }
    let delim = b[i];
    let mut j = skip_delim_body(b, i + 1, delim); // past the regex's closing delim
    j = skip_delim_body(b, j, delim); // past the replacement's closing delim
    while j < n {
        match b[j] {
            b'e' => {
                scan.exec = true;
                j += 1;
            }
            b'w' => {
                // The `w filename` flag is greedy to end of line — it must be the last flag.
                let (f, nj) = read_filename(b, j + 1);
                scan.writes.push(f);
                return nj;
            }
            b'g' | b'p' | b'i' | b'I' | b'm' | b'M' | b'0'..=b'9' => j += 1,
            _ => break,
        }
    }
    j
}

fn skip_delim_body(b: &[u8], mut i: usize, delim: u8) -> usize {
    let n = b.len();
    while i < n {
        if b[i] == b'\\' {
            i += 2;
            continue;
        }
        if b[i] == delim {
            return i + 1;
        }
        i += 1;
    }
    i
}

fn skip_two_delims(b: &[u8], i: usize) -> usize {
    let n = b.len();
    if i >= n {
        return i;
    }
    let delim = b[i];
    let j = skip_delim_body(b, i + 1, delim);
    skip_delim_body(b, j, delim)
}

fn read_filename(b: &[u8], mut i: usize) -> (String, usize) {
    let n = b.len();
    while i < n && matches!(b[i], b' ' | b'\t') {
        i += 1;
    }
    let start = i;
    while i < n && b[i] != b'\n' {
        i += 1;
    }
    (String::from_utf8_lossy(&b[start..i]).into_owned(), i)
}

static SED_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--debug", "--help", "--posix", "--quiet", "--sandbox",
        "--silent", "--unbuffered", "--version",
        "-E", "-V", "-h", "-n", "-r", "-u", "-z",
    ]),
    valued: WordSet::flags(&[
        "--expression", "--file", "--line-length",
        "-e", "-f", "-l",
    ]),
    bare: false,
    max_positional: None,
    tolerance: FlagTolerance::strict(),
};

fn sed_verdict(tokens: &[Token]) -> Verdict {
    if !policy::check(tokens, &SED_POLICY) {
        return Verdict::Denied;
    }
    let scan = scan_sed(tokens);
    if scan.exec || scan.unknown || scan.script_file {
        return Verdict::Denied;
    }
    // Path-gate the script's file commands by locus — identical to the engine resolver, which is
    // authoritative. A local `w`/`r` is a SafeWrite/read; a system/home/out-of-worktree target denies.
    let mut level = SafetyLevel::Inert;
    for w in &scan.writes {
        if !crate::engine::resolve::write_target_verdict(&crate::pathctx::resolve(w)).is_allowed() {
            return Verdict::Denied;
        }
        level = SafetyLevel::SafeWrite;
    }
    for r in &scan.reads {
        if !crate::engine::resolve::read_content_verdict(&crate::pathctx::resolve(r)).is_allowed() {
            return Verdict::Denied;
        }
    }
    Verdict::Allowed(level)
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "sed" => Some(sed_verdict(tokens)),
        _ => None,
    }
}


#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "sed", valid_prefix: Some("sed 's/a/b/'") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        sed_substitute: "sed 's/foo/bar/'",
        sed_n_flag: "sed -n 's/foo/bar/p'",
        sed_e_flag: "sed -e 's/foo/bar/' -e 's/baz/qux/'",
        sed_extended: "sed -E 's/[0-9]+/NUM/g'",
        sed_filename_starting_with_e_allowed: "sed 's/foo/bar/' error.log",
        sed_filename_ending_with_e_allowed: "sed 's/foo/bar/' Makefile",
        sed_no_exec_allowed: "sed 's/foo/bar/g'",
        sed_no_exec_print_allowed: "sed 's/foo/bar/gp'",
        sed_filename_1e_after_script: "sed 's/foo/bar/' 1e",
        sed_expression_flag_with_filename: "sed -e 's/foo/bar/' filename",
        sed_expression_flag_then_safe_filename: "sed -e 's/foo/bar/' 1e 2e",
        // engine-authoritative: sed -i editing a WORKTREE file in place is admitted at
        // write-local (a recoverable, bounded mutation). Editing a system/home file still
        // denies by locus; the exec-modifier RCE vector below still denies.
        sed_inplace: "sed -i 's/foo/bar/' file.txt",
        sed_in_place_long: "sed --in-place 's/foo/bar/' file.txt",
        sed_inplace_backup: "sed -i.bak 's/foo/bar/' file.txt",
        sed_ni_combined: "sed -ni 's/foo/bar/p' file.txt",
        sed_in_combined: "sed -in 's/foo/bar/' file.txt",
        sed_in_place_eq: "sed --in-place=.bak 's/foo/bar/' file.txt",
        sed_inplace_trailing_help: "sed -i 's/foo/bar/' file --help",
        sed_inplace_trailing_version: "sed -i 's/foo/bar/' file --version",
        // A `w`/`r` command targeting a LOCAL (worktree) file is a bounded write/read.
        sed_w_local_file_allowed: "sed 'w out.txt' input.txt",
        sed_r_local_file_allowed: "sed 'r data.txt' input.txt",
        sed_s_w_local_file_allowed: "sed 's/a/b/w out.txt' input.txt",
        // False-positive guards: `w`/`r`/`e` inside a regex/replacement are NOT commands.
        sed_w_inside_address_regex_allowed: "sed '/w /d' input.txt",
        sed_r_inside_replacement_allowed: "sed 's/x/r /' input.txt",
        sed_transliterate_allowed: "sed 'y/abcde/wxyzr/' input.txt",
        // Every -e form must recognize the SCRIPT (and not scan the input file as one).
        sed_glued_e_allowed: "sed -es/a/b/ input.txt",
        sed_glued_e_quoted_allowed: "sed -e's/a/b/' input.txt",
        sed_cluster_ne_allowed: "sed -nes/a/b/p input.txt",
        sed_expression_equals_allowed: "sed --expression=s/a/b/ input.txt",
    }

    denied! {
        sed_exec_modifier_denied: "sed 's/test/touch \\/tmp\\/pwned/e'",
        sed_exec_with_global_denied: "sed 's/foo/bar/ge'",
        sed_exec_alternate_delim_denied: "sed 's|test|touch /tmp/pwned|e'",
        sed_exec_via_e_flag_denied: "sed -e 's/test/touch tmp/e'",
        sed_standalone_e_command_denied: "sed e",
        sed_address_e_command_denied: "sed 1e",
        sed_regex_address_e_denied: "sed '/pattern/e'",
        sed_range_address_e_denied: "sed '1,5e'",
        sed_dollar_address_e_denied: "sed '$e'",
        sed_e_command_with_argument_denied: "sed '1e id'",
        sed_e_after_separator_denied: "sed 'p;e cat /etc/passwd'",
        sed_e_via_flag_denied: "sed -e e",
        sed_expression_flag_exec_denied: "sed -e 's/foo/bar/e'",
        sed_multiple_expressions_exec_denied: "sed -e 's/foo/bar/' -e 's/x/y/e'",
        // Script file commands writing/reading OUTSIDE the workspace.
        sed_w_system_file_denied: "sed 'w /etc/passwd' input.txt",
        sed_w_cron_file_denied: "sed '1w /etc/cron.d/x' input.txt",
        sed_r_secret_denied: "sed 'r /etc/shadow' input.txt",
        sed_s_w_system_file_denied: "sed 's/a/b/w /etc/passwd' input.txt",
        sed_w_home_file_denied: "sed 'w ~/.bashrc' input.txt",
        // The exec/file vectors must be caught in the glued and equals -e forms too.
        sed_glued_e_write_denied: "sed -e'w /etc/passwd' input.txt",
        sed_equals_expression_exec_denied: "sed --expression=e input.txt",
        sed_cluster_e_write_denied: "sed -ne'w /etc/passwd' input.txt",
        // -f runs a script file we can't inspect (its e/w/r commands are invisible) → denied,
        // in every form, like awk -f / bash script.sh / mlr --load.
        sed_f_script_file_denied: "sed -f script.sed input.txt",
        sed_file_long_denied: "sed --file script.sed input.txt",
        sed_f_glued_denied: "sed -fscript.sed input.txt",
        sed_file_equals_denied: "sed --file=script.sed input.txt",
    }
}
