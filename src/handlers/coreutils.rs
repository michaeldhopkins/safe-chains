use crate::parse::{Segment, Token, WordSet, has_flag};
use crate::policy::{self, FlagPolicy};

static FIND_DANGEROUS_FLAGS: WordSet = WordSet::new(&[
    "-delete",
    "-fls",
    "-fprint",
    "-fprint0",
    "-fprintf",
    "-ok",
    "-okdir",
]);

pub fn is_safe_find(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    let mut i = 1;
    while i < tokens.len() {
        if FIND_DANGEROUS_FLAGS.contains(&tokens[i]) {
            return false;
        }
        if tokens[i] == "-exec" || tokens[i] == "-execdir" {
            let cmd_start = i + 1;
            let cmd_end = tokens[cmd_start..]
                .iter()
                .position(|t| *t == ";" || *t == "+")
                .map(|p| cmd_start + p)
                .unwrap_or(tokens.len());
            if cmd_start >= cmd_end {
                return false;
            }
            let exec_cmd = Segment::from_tokens_replacing(&tokens[cmd_start..cmd_end], "{}", "file");
            if !is_safe(&exec_cmd) {
                return false;
            }
            i = cmd_end + 1;
            continue;
        }
        i += 1;
    }
    true
}

fn expr_has_exec(token: &Token) -> bool {
    let bytes = token.as_bytes();
    if bytes == b"e"
        || (bytes.last() == Some(&b'e')
            && bytes.len() >= 2
            && matches!(bytes[bytes.len() - 2], b'0'..=b'9' | b'/' | b'$'))
    {
        return true;
    }
    if bytes.len() < 4 || bytes[0] != b's' {
        return false;
    }
    let delim = bytes[1];
    let mut count = 0;
    let mut escaped = false;
    let mut flags_start = None;
    for (i, &b) in bytes[2..].iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if b == b'\\' {
            escaped = true;
            continue;
        }
        if b == delim {
            count += 1;
            if count == 2 {
                flags_start = Some(i + 3);
                break;
            }
        }
    }
    flags_start.is_some_and(|start| start < bytes.len() && bytes[start..].contains(&b'e'))
}

fn sed_has_exec_modifier(tokens: &[Token]) -> bool {
    let mut i = 1;
    let mut saw_script = false;

    while i < tokens.len() {
        let token = &tokens[i];

        if *token == "-e" || *token == "--expression" {
            if tokens.get(i + 1).is_some_and(expr_has_exec) {
                return true;
            }
            saw_script = true;
            i += 2;
            continue;
        }

        if token.starts_with("-") {
            i += 1;
            continue;
        }

        if !saw_script {
            if expr_has_exec(token) {
                return true;
            }
            saw_script = true;
        }
        i += 1;
    }
    false
}

pub fn is_safe_sed(tokens: &[Token]) -> bool {
    !has_flag(tokens, Some("-i"), Some("--in-place")) && !sed_has_exec_modifier(tokens)
}

pub fn is_safe_sort(tokens: &[Token]) -> bool {
    !has_flag(tokens, Some("-o"), Some("--output"))
        && !has_flag(tokens, None, Some("--compress-program"))
}

pub fn is_safe_yq(tokens: &[Token]) -> bool {
    !has_flag(tokens, Some("-i"), Some("--inplace"))
}

pub fn is_safe_xmllint(tokens: &[Token]) -> bool {
    !has_flag(tokens, None, Some("--output"))
}

fn awk_has_dangerous_construct(token: &Token) -> bool {
    let code = token.content_outside_double_quotes();
    code.contains("system") || code.contains("getline") || code.contains('|') || code.contains('>')
}

pub fn is_safe_awk(tokens: &[Token]) -> bool {
    if has_flag(tokens, Some("-f"), None) {
        return false;
    }
    for token in &tokens[1..] {
        if token.starts_with("-") {
            continue;
        }
        if awk_has_dangerous_construct(token) {
            return false;
        }
    }
    true
}

pub fn is_safe_arch(tokens: &[Token]) -> bool {
    tokens.len() == 1
}

static HOSTNAME_DISPLAY: WordSet = WordSet::new(&["-A", "-I", "-d", "-f", "-i", "-s"]);

pub fn is_safe_command_builtin(tokens: &[Token]) -> bool {
    tokens.len() >= 3
        && (tokens[1] == "-v" || tokens[1] == "-V")
}

pub fn is_safe_hostname(tokens: &[Token]) -> bool {
    if tokens.len() == 1 {
        return true;
    }
    tokens[1..].iter().all(|t| HOSTNAME_DISPLAY.contains(t))
}

static GREP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--basic-regexp", "--binary", "--byte-offset", "--color", "--colour",
        "--count", "--dereference-recursive", "--extended-regexp",
        "--files-with-matches", "--files-without-match", "--fixed-strings",
        "--ignore-case", "--initial-tab", "--invert-match", "--line-buffered",
        "--line-number", "--line-regexp", "--no-filename", "--no-messages",
        "--null", "--null-data", "--only-matching", "--perl-regexp", "--quiet",
        "--recursive", "--silent", "--text", "--with-filename", "--word-regexp",
        "-E", "-F", "-G", "-H", "-I", "-J", "-L", "-P", "-R", "-S",
        "-T", "-U", "-V", "-Z",
        "-a", "-b", "-c", "-h", "-i", "-l", "-n", "-o", "-p", "-q",
        "-r", "-s", "-v", "-w", "-x", "-z",
    ]),
    standalone_short: b"EFGHIJLPRSTUVZabchilnopqrsvwxz",
    valued: WordSet::new(&[
        "--after-context", "--before-context", "--binary-files", "--color",
        "--colour", "--context", "--devices", "--directories", "--exclude",
        "--exclude-dir", "--exclude-from", "--file", "--group-separator",
        "--include", "--label", "--max-count", "--regexp",
        "-A", "-B", "-C", "-D", "-d", "-e", "-f", "-m",
    ]),
    valued_short: b"ABCDdefm",
    bare: false,
    max_positional: None,
};

pub fn is_safe_grep(tokens: &[Token]) -> bool {
    policy::check(tokens, &GREP_POLICY)
}

static RG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--binary", "--block-buffered", "--byte-offset", "--case-sensitive",
        "--column", "--count", "--count-matches", "--crlf", "--debug",
        "--files", "--files-with-matches", "--files-without-match",
        "--fixed-strings", "--follow", "--glob-case-insensitive", "--heading",
        "--hidden", "--ignore-case", "--ignore-file-case-insensitive",
        "--include-zero", "--invert-match", "--json", "--line-buffered",
        "--line-number", "--line-regexp", "--max-columns-preview", "--mmap",
        "--multiline", "--multiline-dotall", "--no-config", "--no-filename",
        "--no-heading", "--no-ignore", "--no-ignore-dot", "--no-ignore-exclude",
        "--no-ignore-files", "--no-ignore-global", "--no-ignore-messages",
        "--no-ignore-parent", "--no-ignore-vcs", "--no-line-number",
        "--no-messages", "--no-mmap", "--no-pcre2-unicode", "--no-require-git",
        "--no-unicode", "--null", "--null-data", "--one-file-system",
        "--only-matching", "--passthru", "--pcre2", "--pcre2-version",
        "--pretty", "--quiet", "--search-zip", "--smart-case", "--sort-files",
        "--stats", "--text", "--trim", "--type-list", "--unicode",
        "--unrestricted", "--vimgrep", "--with-filename", "--word-regexp",
        "-F", "-H", "-I", "-L", "-N", "-P", "-S", "-U", "-V",
        "-a", "-b", "-c", "-h", "-i", "-l", "-n", "-o", "-p", "-q",
        "-s", "-u", "-v", "-w", "-x", "-z",
    ]),
    standalone_short: b"FHILNPSUVabchilnopqsuvwxz",
    valued: WordSet::new(&[
        "--after-context", "--before-context", "--color", "--colors",
        "--context", "--context-separator", "--dfa-size-limit", "--encoding",
        "--engine", "--field-context-separator", "--field-match-separator",
        "--file", "--glob", "--iglob", "--ignore-file", "--max-columns",
        "--max-count", "--max-depth", "--max-filesize", "--path-separator",
        "--regex-size-limit", "--regexp", "--replace", "--sort", "--sortr",
        "--threads", "--type", "--type-add", "--type-clear", "--type-not",
        "-A", "-B", "-C", "-E", "-M", "-T",
        "-e", "-f", "-g", "-j", "-m", "-r", "-t",
    ]),
    valued_short: b"ABCEMTefgjmrt",
    bare: false,
    max_positional: None,
};

pub fn is_safe_rg(tokens: &[Token]) -> bool {
    policy::check(tokens, &RG_POLICY)
}

static CAT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--number", "--number-nonblank", "--show-all", "--show-ends",
        "--show-nonprinting", "--show-tabs", "--squeeze-blank",
        "-A", "-E", "-T",
        "-b", "-e", "-l", "-n", "-s", "-t", "-u", "-v",
    ]),
    standalone_short: b"AETbelnstuv",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_cat(tokens: &[Token]) -> bool {
    policy::check(tokens, &CAT_POLICY)
}

static HEAD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--quiet", "--silent", "--verbose", "--zero-terminated",
        "-q", "-v", "-z",
    ]),
    standalone_short: b"0123456789qvz",
    valued: WordSet::new(&[
        "--bytes", "--lines",
        "-c", "-n",
    ]),
    valued_short: b"cn",
    bare: true,
    max_positional: None,
};

pub fn is_safe_head(tokens: &[Token]) -> bool {
    policy::check(tokens, &HEAD_POLICY)
}

static TAIL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--follow", "--quiet", "--retry", "--silent", "--verbose",
        "--zero-terminated",
        "-F", "-f", "-q", "-r", "-v", "-z",
    ]),
    standalone_short: b"0123456789Ffqrvz",
    valued: WordSet::new(&[
        "--bytes", "--lines", "--max-unchanged-stats", "--pid",
        "--sleep-interval",
        "-b", "-c", "-n",
    ]),
    valued_short: b"bcn",
    bare: true,
    max_positional: None,
};

pub fn is_safe_tail(tokens: &[Token]) -> bool {
    policy::check(tokens, &TAIL_POLICY)
}

static WC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--bytes", "--chars", "--lines", "--max-line-length", "--words",
        "--zero-terminated",
        "-L", "-c", "-l", "-m", "-w",
    ]),
    standalone_short: b"Lclmw",
    valued: WordSet::new(&["--files0-from"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_wc(tokens: &[Token]) -> bool {
    policy::check(tokens, &WC_POLICY)
}

static CUT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--complement", "--only-delimited", "--zero-terminated",
        "-n", "-s", "-w", "-z",
    ]),
    standalone_short: b"nswz",
    valued: WordSet::new(&[
        "--bytes", "--characters", "--delimiter", "--fields",
        "--output-delimiter",
        "-b", "-c", "-d", "-f",
    ]),
    valued_short: b"bcdf",
    bare: false,
    max_positional: None,
};

pub fn is_safe_cut(tokens: &[Token]) -> bool {
    policy::check(tokens, &CUT_POLICY)
}

static TR_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--complement", "--delete", "--squeeze-repeats", "--truncate-set1",
        "-C", "-c", "-d", "-s",
    ]),
    standalone_short: b"Ccds",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

pub fn is_safe_tr(tokens: &[Token]) -> bool {
    policy::check(tokens, &TR_POLICY)
}

static UNIQ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--count", "--ignore-case", "--repeated", "--unique",
        "--zero-terminated",
        "-D", "-c", "-d", "-i", "-u", "-z",
    ]),
    standalone_short: b"Dcdiuz",
    valued: WordSet::new(&[
        "--all-repeated", "--check-chars", "--group", "--skip-chars",
        "--skip-fields",
        "-f", "-s", "-w",
    ]),
    valued_short: b"fsw",
    bare: true,
    max_positional: Some(1),
};

pub fn is_safe_uniq(tokens: &[Token]) -> bool {
    policy::check(tokens, &UNIQ_POLICY)
}

static DIFF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--brief", "--ed", "--expand-tabs", "--ignore-all-space",
        "--ignore-blank-lines", "--ignore-case", "--ignore-space-change",
        "--ignore-tab-expansion", "--left-column", "--minimal",
        "--new-file", "--no-dereference", "--no-ignore-file-name-case",
        "--normal", "--paginate", "--rcs", "--recursive",
        "--report-identical-files", "--show-c-function", "--side-by-side",
        "--speed-large-files", "--strip-trailing-cr",
        "--suppress-blank-empty", "--suppress-common-lines", "--text",
        "--unidirectional-new-file",
        "-B", "-E", "-N", "-P", "-T",
        "-a", "-b", "-c", "-d", "-e", "-f", "-i", "-l", "-n", "-p",
        "-q", "-r", "-s", "-t", "-u", "-w", "-y",
    ]),
    standalone_short: b"BENPTabcdefilnpqrstuwy",
    valued: WordSet::new(&[
        "--changed-group-format", "--color", "--context", "--exclude",
        "--exclude-from", "--from-file", "--ifdef", "--ignore-matching-lines",
        "--label", "--line-format", "--new-group-format", "--new-line-format",
        "--old-group-format", "--old-line-format", "--show-function-line",
        "--starting-file", "--tabsize", "--to-file", "--unchanged-group-format",
        "--unchanged-line-format", "--unified", "--width",
        "-C", "-D", "-F", "-I", "-L", "-S", "-U", "-W", "-X", "-x",
    ]),
    valued_short: b"CDFILSUWXx",
    bare: false,
    max_positional: None,
};

pub fn is_safe_diff(tokens: &[Token]) -> bool {
    policy::check(tokens, &DIFF_POLICY)
}

static COMM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--check-order", "--nocheck-order", "--total", "--zero-terminated",
        "-1", "-2", "-3", "-i", "-z",
    ]),
    standalone_short: b"123iz",
    valued: WordSet::new(&["--output-delimiter"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

pub fn is_safe_comm(tokens: &[Token]) -> bool {
    policy::check(tokens, &COMM_POLICY)
}

static PASTE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--serial", "--zero-terminated",
        "-s", "-z",
    ]),
    standalone_short: b"sz",
    valued: WordSet::new(&[
        "--delimiters",
        "-d",
    ]),
    valued_short: b"d",
    bare: true,
    max_positional: None,
};

pub fn is_safe_paste(tokens: &[Token]) -> bool {
    policy::check(tokens, &PASTE_POLICY)
}

static TAC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--before", "--regex",
        "-b", "-r",
    ]),
    standalone_short: b"br",
    valued: WordSet::new(&[
        "--separator",
        "-s",
    ]),
    valued_short: b"s",
    bare: true,
    max_positional: None,
};

pub fn is_safe_tac(tokens: &[Token]) -> bool {
    policy::check(tokens, &TAC_POLICY)
}

static REV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_rev(tokens: &[Token]) -> bool {
    policy::check(tokens, &REV_POLICY)
}

static NL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--no-renumber",
        "-p",
    ]),
    standalone_short: b"p",
    valued: WordSet::new(&[
        "--body-numbering", "--footer-numbering", "--header-numbering",
        "--join-blank-lines", "--line-increment", "--number-format",
        "--number-separator", "--number-width", "--section-delimiter",
        "--starting-line-number",
        "-b", "-d", "-f", "-h", "-i", "-l", "-n", "-s", "-v", "-w",
    ]),
    valued_short: b"bdfhilnsvw",
    bare: true,
    max_positional: None,
};

pub fn is_safe_nl(tokens: &[Token]) -> bool {
    policy::check(tokens, &NL_POLICY)
}

static EXPAND_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--initial",
        "-i",
    ]),
    standalone_short: b"i",
    valued: WordSet::new(&[
        "--tabs",
        "-t",
    ]),
    valued_short: b"t",
    bare: true,
    max_positional: None,
};

pub fn is_safe_expand(tokens: &[Token]) -> bool {
    policy::check(tokens, &EXPAND_POLICY)
}

static UNEXPAND_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--first-only",
        "-a",
    ]),
    standalone_short: b"a",
    valued: WordSet::new(&[
        "--tabs",
        "-t",
    ]),
    valued_short: b"t",
    bare: true,
    max_positional: None,
};

pub fn is_safe_unexpand(tokens: &[Token]) -> bool {
    policy::check(tokens, &UNEXPAND_POLICY)
}

static FOLD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--bytes", "--spaces",
        "-b", "-s",
    ]),
    standalone_short: b"bs",
    valued: WordSet::new(&[
        "--width",
        "-w",
    ]),
    valued_short: b"w",
    bare: true,
    max_positional: None,
};

pub fn is_safe_fold(tokens: &[Token]) -> bool {
    policy::check(tokens, &FOLD_POLICY)
}

static FMT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--crown-margin", "--split-only", "--tagged-paragraph",
        "--uniform-spacing",
        "-c", "-m", "-n", "-s", "-u",
    ]),
    standalone_short: b"cmnsu",
    valued: WordSet::new(&[
        "--goal", "--prefix", "--width",
        "-d", "-g", "-l", "-p", "-t", "-w",
    ]),
    valued_short: b"dglptw",
    bare: true,
    max_positional: None,
};

pub fn is_safe_fmt(tokens: &[Token]) -> bool {
    policy::check(tokens, &FMT_POLICY)
}

static COLUMN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--fillrows", "--json", "--keep-empty-lines", "--table",
        "--table-noextreme", "--table-noheadings", "--table-right-all",
        "-J", "-L", "-R", "-e", "-n", "-t", "-x",
    ]),
    standalone_short: b"JLRentx",
    valued: WordSet::new(&[
        "--output-separator", "--separator", "--table-columns",
        "--table-empty-lines", "--table-hide", "--table-name",
        "--table-order", "--table-right", "--table-truncate", "--table-wrap",
        "-E", "-H", "-O", "-W", "-c", "-d", "-o", "-r", "-s",
    ]),
    valued_short: b"EHOWcdors",
    bare: true,
    max_positional: None,
};

pub fn is_safe_column(tokens: &[Token]) -> bool {
    policy::check(tokens, &COLUMN_POLICY)
}

static ICONV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--list", "--silent",
        "-c", "-l", "-s",
    ]),
    standalone_short: b"cls",
    valued: WordSet::new(&[
        "--from-code", "--to-code",
        "-f", "-t",
    ]),
    valued_short: b"ft",
    bare: false,
    max_positional: None,
};

pub fn is_safe_iconv(tokens: &[Token]) -> bool {
    policy::check(tokens, &ICONV_POLICY)
}

static NROFF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-S", "-c", "-h", "-i", "-k", "-p", "-q", "-t",
    ]),
    standalone_short: b"Schikpqt",
    valued: WordSet::new(&[
        "-M", "-P", "-T", "-d", "-m", "-n", "-o", "-r", "-w",
    ]),
    valued_short: b"MPTdmnorw",
    bare: false,
    max_positional: None,
};

pub fn is_safe_nroff(tokens: &[Token]) -> bool {
    policy::check(tokens, &NROFF_POLICY)
}

static ECHO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-E", "-e", "-n",
    ]),
    standalone_short: b"Een",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_echo(tokens: &[Token]) -> bool {
    policy::check(tokens, &ECHO_POLICY)
}

static PRINTF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

pub fn is_safe_printf(tokens: &[Token]) -> bool {
    policy::check(tokens, &PRINTF_POLICY)
}

static SEQ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--equal-width",
        "-w",
    ]),
    standalone_short: b"w",
    valued: WordSet::new(&[
        "--format", "--separator",
        "-f", "-s", "-t",
    ]),
    valued_short: b"fst",
    bare: false,
    max_positional: None,
};

pub fn is_safe_seq(tokens: &[Token]) -> bool {
    policy::check(tokens, &SEQ_POLICY)
}

pub fn is_safe_test(tokens: &[Token]) -> bool {
    !tokens.is_empty()
}

pub fn is_safe_expr(tokens: &[Token]) -> bool {
    tokens.len() >= 2
}

static BC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--digit-clamp", "--global-stacks", "--interactive", "--mathlib",
        "--no-digit-clamp", "--no-line-length", "--no-prompt",
        "--no-read-prompt", "--quiet", "--standard", "--warn",
        "-C", "-P", "-R",
        "-c", "-g", "-i", "-l", "-q", "-s", "-w",
    ]),
    standalone_short: b"CPRcgilqsw",
    valued: WordSet::new(&[
        "--expression", "--file", "--ibase", "--obase", "--redefine",
        "--scale", "--seed",
        "-E", "-I", "-O", "-S",
        "-e", "-f", "-r",
    ]),
    valued_short: b"EIOSefr",
    bare: true,
    max_positional: None,
};

pub fn is_safe_bc(tokens: &[Token]) -> bool {
    policy::check(tokens, &BC_POLICY)
}

static FACTOR_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--exponents",
        "-h",
    ]),
    standalone_short: b"h",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_factor(tokens: &[Token]) -> bool {
    policy::check(tokens, &FACTOR_POLICY)
}

static BAT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--diff", "--list-languages", "--list-themes", "--no-config",
        "--number", "--plain", "--show-all",
        "-A", "-P", "-d", "-n", "-p", "-u",
    ]),
    standalone_short: b"APdnpu",
    valued: WordSet::new(&[
        "--color", "--decorations", "--diff-context", "--file-name",
        "--highlight-line", "--italic-text", "--language", "--line-range",
        "--map-syntax", "--paging", "--style", "--tabs",
        "--terminal-width", "--theme", "--wrap",
        "-H", "-l", "-m", "-r",
    ]),
    valued_short: b"Hlmr",
    bare: true,
    max_positional: None,
};

pub fn is_safe_bat(tokens: &[Token]) -> bool {
    policy::check(tokens, &BAT_POLICY)
}

static FD_EXEC_LONG: WordSet = WordSet::new(&["--exec", "--exec-batch"]);

pub fn is_safe_fd(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    for t in &tokens[1..] {
        if FD_EXEC_LONG.contains(t) {
            return false;
        }
        if t.starts_with('-')
            && !t.starts_with("--")
            && t.as_bytes()[1..].iter().any(|&b| b == b'x' || b == b'X')
        {
            return false;
        }
    }
    true
}

pub fn is_safe_tree(tokens: &[Token]) -> bool {
    !has_flag(tokens, Some("-o"), None)
}

pub fn is_safe_file_cmd(tokens: &[Token]) -> bool {
    !has_flag(tokens, Some("-C"), Some("--compile"))
}

pub fn is_safe_date(tokens: &[Token]) -> bool {
    !has_flag(tokens, Some("-s"), Some("--set"))
}

static ROUTE_SAFE_FLAGS: WordSet = WordSet::new(&["-4", "-6", "-n", "-v"]);

static ROUTE_SAFE_SUBCMDS: WordSet = WordSet::new(&["get", "monitor", "print", "show"]);

pub fn is_safe_route(tokens: &[Token]) -> bool {
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        if ROUTE_SAFE_FLAGS.contains(t) {
            i += 1;
            continue;
        }
        if ROUTE_SAFE_SUBCMDS.contains(t) {
            return true;
        }
        return false;
    }
    true
}

static IFCONFIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-L", "-a", "-l", "-s", "-v"]),
    standalone_short: b"Lalsv",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(1),
};

pub fn is_safe_ifconfig(tokens: &[Token]) -> bool {
    policy::check(tokens, &IFCONFIG_POLICY)
}

static BARE_ONLY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(0),
};

static LS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--almost-all", "--author", "--classify",
        "--context", "--dereference", "--dereference-command-line",
        "--dereference-command-line-symlink-to-dir", "--directory",
        "--escape", "--file-type", "--full-time",
        "--group-directories-first", "--hide-control-chars",
        "--human-readable", "--indicator-style",
        "--inode", "--kibibytes", "--literal", "--no-group",
        "--numeric-uid-gid", "--quote-name", "--recursive",
        "--reverse", "--show-control-chars", "--si", "--size",
        "-1", "-A", "-B", "-C", "-F", "-G", "-H", "-L",
        "-N", "-Q", "-R", "-S", "-U", "-X", "-Z",
        "-a", "-c", "-d", "-f", "-g", "-h", "-i", "-k",
        "-l", "-m", "-n", "-o", "-p", "-q", "-r", "-s",
        "-t", "-u", "-v", "-x",
    ]),
    standalone_short: b"1ABCFGHLNQRSUXZacdfghiklmnopqrstuvx",
    valued: WordSet::new(&[
        "--block-size", "--color", "--format", "--hide",
        "--hyperlink", "--ignore",
        "--quoting-style", "--sort", "--tabsize", "--time",
        "--time-style", "--width",
        "-I", "-T", "-w",
    ]),
    valued_short: b"ITw",
    bare: true,
    max_positional: None,
};

static EZA_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--accessed", "--all", "--binary", "--blocks", "--blocksize",
        "--bytes", "--changed", "--classify", "--color-scale", "--color-scale-mode",
        "--context", "--created", "--dereference", "--extended", "--flags",
        "--follow-symlinks", "--git", "--git-ignore", "--git-repos", "--git-repos-no-status",
        "--group", "--group-directories-first", "--header", "--hyperlink", "--icons",
        "--inode", "--links", "--list-dirs", "--long", "--modified",
        "--mounts", "--no-filesize", "--no-git", "--no-icons", "--no-permissions",
        "--no-quotes", "--no-time", "--no-user", "--numeric", "--octal-permissions",
        "--oneline", "--only-dirs", "--only-files", "--recurse", "--reverse",
        "--tree", "-1", "-@", "-A", "-B",
        "-D", "-F", "-G", "-H", "-I",
        "-M", "-R", "-S", "-T", "-U",
        "-Z", "-a", "-b", "-d", "-f",
        "-g", "-h", "-i", "-l", "-m",
        "-r", "-s", "-u", "-x",
    ]),
    standalone_short: b"1@ABDFGHIMRSTUZabdfghilmrsux",
    valued: WordSet::new(&[
        "--color", "--colour", "--git-ignore-glob", "--grid-columns",
        "--group-directories-first-dirs", "--ignore-glob", "--level",
        "--smart-group", "--sort", "--time", "--time-style",
        "--total-size", "--width",
        "-L", "-X", "-t", "-w",
    ]),
    valued_short: b"LXtw",
    bare: true,
    max_positional: None,
};

static DELTA_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--blame-code-style", "--blame-palette",
        "--color-only", "--dark", "--diff-highlight",
        "--diff-so-fancy", "--hyperlinks", "--keep-plus-minus-markers",
        "--light", "--line-numbers", "--list-languages",
        "--list-syntax-themes", "--navigate", "--no-gitconfig",
        "--raw", "--relative-paths", "--show-config",
        "--show-syntax-themes", "--side-by-side",
        "-n", "-s",
    ]),
    standalone_short: b"ns",
    valued: WordSet::new(&[
        "--commit-decoration-style", "--commit-style", "--config",
        "--diff-stat-align-width", "--features", "--file-added-label",
        "--file-decoration-style", "--file-modified-label",
        "--file-removed-label", "--file-renamed-label",
        "--file-style", "--file-transformation",
        "--hunk-header-decoration-style", "--hunk-header-file-style",
        "--hunk-header-line-number-style", "--hunk-header-style",
        "--hunk-label", "--inline-hint-style",
        "--inspect-raw-lines", "--line-buffer-size",
        "--line-fill-method", "--line-numbers-left-format",
        "--line-numbers-left-style", "--line-numbers-minus-style",
        "--line-numbers-plus-style", "--line-numbers-right-format",
        "--line-numbers-right-style", "--line-numbers-zero-style",
        "--map-styles", "--max-line-distance", "--max-line-length",
        "--merge-conflict-begin-symbol", "--merge-conflict-end-symbol",
        "--merge-conflict-ours-diff-header-decoration-style",
        "--merge-conflict-ours-diff-header-style",
        "--merge-conflict-theirs-diff-header-decoration-style",
        "--merge-conflict-theirs-diff-header-style",
        "--minus-emph-style", "--minus-empty-line-marker-style",
        "--minus-non-emph-style", "--minus-style",
        "--paging", "--plus-emph-style",
        "--plus-empty-line-marker-style", "--plus-non-emph-style",
        "--plus-style", "--syntax-theme", "--tabs",
        "--true-color", "--whitespace-error-style", "--width",
        "-w",
    ]),
    valued_short: b"w",
    bare: true,
    max_positional: None,
};

static COLORDIFF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--brief", "--ed", "--expand-tabs", "--initial-tab",
        "--left-column", "--minimal", "--normal",
        "--paginate", "--rcs", "--report-identical-files",
        "--side-by-side", "--speed-large-files",
        "--strip-trailing-cr", "--suppress-blank-empty",
        "--suppress-common-lines", "--text",
        "-B", "-E", "-N", "-P", "-T", "-Z",
        "-a", "-b", "-c", "-d", "-e", "-i", "-l", "-n",
        "-p", "-q", "-r", "-s", "-t", "-u", "-v", "-w", "-y",
    ]),
    standalone_short: b"BENPTZabcdefilnpqrstuvwy",
    valued: WordSet::new(&[
        "--changed-group-format", "--color", "--context",
        "--from-file", "--horizon-lines", "--ifdef",
        "--ignore-matching-lines", "--label", "--line-format",
        "--new-group-format", "--new-line-format",
        "--old-group-format", "--old-line-format",
        "--show-function-line", "--starting-file",
        "--tabsize", "--to-file", "--unchanged-group-format",
        "--unchanged-line-format", "--unified", "--width",
        "-C", "-D", "-F", "-I", "-L", "-S", "-U", "-W",
    ]),
    valued_short: b"CDFILSUW",
    bare: false,
    max_positional: None,
};

static DIRNAME_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--zero", "-z"]),
    standalone_short: b"z",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static BASENAME_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--multiple", "--zero", "-a", "-z"]),
    standalone_short: b"az",
    valued: WordSet::new(&["--suffix", "-s"]),
    valued_short: b"s",
    bare: false,
    max_positional: None,
};

static REALPATH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--canonicalize-existing", "--canonicalize-missing",
        "--logical", "--no-symlinks", "--physical", "--quiet",
        "--strip", "--zero",
        "-L", "-P", "-e", "-m", "-q", "-s", "-z",
    ]),
    standalone_short: b"LPemqsz",
    valued: WordSet::new(&["--relative-base", "--relative-to"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static READLINK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--canonicalize", "--canonicalize-existing",
        "--canonicalize-missing", "--no-newline", "--verbose", "--zero",
        "-e", "-f", "-m", "-n", "-v", "-z",
    ]),
    standalone_short: b"efmnvz",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static STAT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--dereference", "--file-system", "--terse",
        "-F", "-L", "-l", "-n", "-q", "-r", "-s", "-x",
    ]),
    standalone_short: b"FLlnqrsx",
    valued: WordSet::new(&[
        "--format", "--printf",
        "-c", "-f", "-t",
    ]),
    valued_short: b"cft",
    bare: false,
    max_positional: None,
};

static DU_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--apparent-size", "--bytes", "--count-links",
        "--dereference", "--dereference-args", "--human-readable",
        "--inodes", "--no-dereference", "--null",
        "--one-file-system", "--separate-dirs", "--si",
        "--summarize", "--total",
        "-0", "-D", "-H", "-L", "-P", "-S", "-a", "-b",
        "-c", "-h", "-k", "-l", "-m", "-s", "-x",
    ]),
    standalone_short: b"0DHLPSabchklmsx",
    valued: WordSet::new(&[
        "--block-size", "--exclude", "--files0-from",
        "--max-depth", "--threshold", "--time",
        "--time-style",
        "-B", "-d", "-t",
    ]),
    valued_short: b"Bdt",
    bare: true,
    max_positional: None,
};

static DF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--human-readable", "--inodes", "--local",
        "--no-sync", "--portability", "--print-type",
        "--si", "--sync", "--total",
        "-H", "-P", "-T", "-a", "-h", "-i", "-k", "-l",
    ]),
    standalone_short: b"HPTahikl",
    valued: WordSet::new(&[
        "--block-size", "--exclude-type", "--output", "--type",
        "-B", "-t", "-x",
    ]),
    valued_short: b"Btx",
    bare: true,
    max_positional: None,
};

static PWD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-L", "-P"]),
    standalone_short: b"LP",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(0),
};

static CD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-L", "-P", "-e"]),
    standalone_short: b"LPe",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(1),
};

static UNSET_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-f", "-n", "-v"]),
    standalone_short: b"fnv",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static PRINTENV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--null", "-0"]),
    standalone_short: b"0",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static TYPE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-P", "-a", "-f", "-p", "-t"]),
    standalone_short: b"Pafpt",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static WHEREIS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-b", "-l", "-m", "-s", "-u"]),
    standalone_short: b"blmsu",
    valued: WordSet::new(&["-B", "-M", "-S", "-f"]),
    valued_short: b"BMSf",
    bare: false,
    max_positional: None,
};

static WHICH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--all", "-a", "-s"]),
    standalone_short: b"as",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static UNAME_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--kernel-name", "--kernel-release",
        "--kernel-version", "--machine", "--nodename",
        "--operating-system", "--processor",
        "-a", "-m", "-n", "-o", "-p", "-r", "-s", "-v",
    ]),
    standalone_short: b"amnoprsv",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(0),
};

static NPROC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--all"]),
    standalone_short: b"",
    valued: WordSet::new(&["--ignore"]),
    valued_short: b"",
    bare: true,
    max_positional: Some(0),
};

static UPTIME_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--pretty", "--since", "-p", "-s"]),
    standalone_short: b"ps",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(0),
};

static ID_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--context", "--group", "--groups", "--name",
        "--real", "--user", "--zero",
        "-G", "-Z", "-g", "-n", "-p", "-r", "-u", "-z",
    ]),
    standalone_short: b"GZgnpruz",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(1),
};

static GROUPS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static TTY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--quiet", "--silent", "-s"]),
    standalone_short: b"s",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(0),
};

static LOCALE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all-locales", "--category-name", "--charmaps",
        "--keyword-name", "--verbose",
        "-a", "-c", "-k", "-m", "-v",
    ]),
    standalone_short: b"ackmv",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static CAL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--monday", "--sunday", "--three", "--year",
        "-1", "-3", "-h", "-j", "-m", "-s", "-w", "-y",
    ]),
    standalone_short: b"13hjmswy",
    valued: WordSet::new(&[
        "-A", "-B", "-d", "-n",
    ]),
    valued_short: b"ABdn",
    bare: true,
    max_positional: Some(2),
};

static SLEEP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static WHO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--boot", "--count", "--dead", "--heading",
        "--login", "--lookup", "--mesg", "--message", "--process",
        "--runlevel", "--short", "--time", "--users", "--writable",
        "-H", "-T", "-a", "-b", "-d",
        "-l", "-m", "-p", "-q", "-r",
        "-s", "-t", "-u", "-w",
    ]),
    standalone_short: b"HTSabdlmpqrstuw",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(2),
};

static W_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--from", "--ip-addr", "--no-current", "--no-header",
        "--old-style", "--short",
        "-f", "-h", "-i", "-o", "-s", "-u",
    ]),
    standalone_short: b"fhiosu",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(1),
};

static LAST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--dns", "--fullnames", "--fulltimes", "--hostlast",
        "--ip", "--nohostname", "--system", "--time-format",
        "-F", "-R", "-a", "-d", "-i", "-w", "-x",
    ]),
    standalone_short: b"0123456789FRadiwx",
    valued: WordSet::new(&[
        "--limit", "--present", "--since", "--time-format", "--until",
        "-f", "-n", "-p", "-s", "-t",
    ]),
    valued_short: b"fnpst",
    bare: true,
    max_positional: None,
};

static LASTLOG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&["--before", "--time", "--user", "-b", "-t", "-u"]),
    valued_short: b"btu",
    bare: true,
    max_positional: Some(0),
};

static PS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--cumulative", "--deselect", "--forest", "--headers", "--info",
        "--no-headers", "-A", "-C", "-H", "-L",
        "-M", "-N", "-S", "-T", "-Z",
        "-a", "-c", "-d", "-e", "-f",
        "-j", "-l", "-m", "-r", "-v",
        "-w", "-x",
    ]),
    standalone_short: b"ACHLMNSTZacdefjlmrvwx",
    valued: WordSet::new(&[
        "--cols", "--columns", "--format", "--group", "--pid",
        "--ppid", "--rows", "--sid", "--sort", "--tty", "--user",
        "--width",
        "-G", "-O", "-U", "-g", "-n", "-o", "-p", "-s",
        "-t", "-u",
    ]),
    valued_short: b"GOUnopstug",
    bare: true,
    max_positional: None,
};

static TOP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-1", "-B", "-E", "-H", "-S", "-b", "-c", "-e",
        "-i",
    ]),
    standalone_short: b"1BEHSbcei",
    valued: WordSet::new(&[
        "-F", "-O", "-U", "-d", "-f",
        "-l", "-n", "-o", "-p", "-s", "-u", "-w",
    ]),
    valued_short: b"FOUdflnopsuw",
    bare: true,
    max_positional: None,
};

static HTOP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--no-color", "--no-mouse", "--no-unicode", "--tree",
        "-C", "-H", "-M", "-t",
    ]),
    standalone_short: b"CHMt",
    valued: WordSet::new(&[
        "--delay", "--filter", "--highlight-changes",
        "--pid", "--sort-key", "--user",
        "-F", "-d", "-p", "-s", "-u",
    ]),
    valued_short: b"Fdpsu",
    bare: true,
    max_positional: Some(0),
};

static IOTOP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--accumulated", "--batch", "--kilobytes", "--only",
        "--processes", "--quiet",
        "-P", "-a", "-b", "-k", "-o", "-q", "-t",
    ]),
    standalone_short: b"Pabkoqt",
    valued: WordSet::new(&[
        "--delay", "--iter", "--pid", "--user",
        "-d", "-n", "-p", "-u",
    ]),
    valued_short: b"dnpu",
    bare: true,
    max_positional: Some(0),
};

static PROCS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--no-header", "--or", "--tree", "--watch-interval",
        "-l", "-t",
    ]),
    standalone_short: b"lt",
    valued: WordSet::new(&[
        "--color", "--completion", "--config", "--gen-completion",
        "--insert", "--only", "--pager", "--sorta", "--sortd",
        "--theme",
        "-i", "-w",
    ]),
    valued_short: b"iw",
    bare: true,
    max_positional: None,
};

static DUST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--bars-on-right", "--files0-from", "--ignore-all-in-file", "--invert-filter", "--no-colors",
        "--no-percent-bars", "--only-dir", "--only-file", "--skip-total", "-D",
        "-F", "-H", "-P", "-R", "-S",
        "-b", "-c", "-f", "-i", "-p",
        "-r", "-s",
    ]),
    standalone_short: b"DFHPbcfiprRsS",
    valued: WordSet::new(&[
        "--depth", "--exclude", "--filter", "--terminal_width",
        "-M", "-X", "-d", "-e", "-n", "-t", "-v", "-w", "-z",
    ]),
    valued_short: b"MXdentvwz",
    bare: true,
    max_positional: None,
};

static LSOF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-C", "-G", "-M", "-N", "-O", "-P", "-R",
        "-U", "-V", "-X", "-b", "-h",
        "-l", "-n", "-t", "-w", "-x",
    ]),
    standalone_short: b"CGMNOPRUVXbhlntwx",
    valued: WordSet::new(&[
        "-F", "-S", "-T", "-a", "-c", "-d", "-g",
        "-i", "-k", "-o", "-p", "-r", "-s", "-u",
    ]),
    valued_short: b"FSTacdgikoprsug",
    bare: true,
    max_positional: None,
};

static PGREP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--count", "--delimiter", "--full", "--inverse",
        "--lightweight", "--list-full", "--list-name",
        "--newest", "--oldest",
        "-L", "-a", "-c", "-f", "-i", "-l", "-n",
        "-o", "-v", "-w", "-x",
    ]),
    standalone_short: b"Lacfilnovwx",
    valued: WordSet::new(&[
        "--euid", "--group", "--parent", "--pgroup", "--pidfile",
        "--session", "--terminal", "--uid", "-F", "-G",
        "-P", "-U", "-d", "-g", "-s",
        "-t", "-u",
    ]),
    valued_short: b"FGPdgstUu",
    bare: false,
    max_positional: None,
};

static JQ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--ascii-output", "--color-output", "--compact-output", "--exit-status", "--join-output",
        "--monochrome-output", "--null-input", "--raw-input", "--raw-output", "--raw-output0",
        "--seq", "--slurp", "--sort-keys", "--tab", "-C",
        "-M", "-R", "-S", "-c", "-e",
        "-j", "-n", "-r", "-s",
    ]),
    standalone_short: b"CMRScegjnrs",
    valued: WordSet::new(&[
        "--arg", "--argjson", "--args", "--from-file",
        "--indent", "--jsonargs", "--rawfile",
        "--slurpfile", "-f",
    ]),
    valued_short: b"f",
    bare: true,
    max_positional: None,
};

static BASE64_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--decode", "--ignore-garbage",
        "-D", "-d", "-i",
    ]),
    standalone_short: b"Ddi",
    valued: WordSet::new(&["--wrap", "-b", "-w"]),
    valued_short: b"bw",
    bare: true,
    max_positional: None,
};

static XXD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--autoskip", "--bits", "--capitalize", "--decimal",
        "--ebcdic", "--include", "--little-endian", "--plain",
        "--postscript", "--revert", "--uppercase",
        "-C", "-E", "-a", "-b", "-d", "-e", "-i", "-p",
        "-r", "-u",
    ]),
    standalone_short: b"CEabdeipru",
    valued: WordSet::new(&[
        "--color", "--cols", "--groupsize", "--len",
        "--name", "--offset", "--seek",
        "-R", "-c", "-g", "-l", "-n", "-o", "-s",
    ]),
    valued_short: b"Rcglnos",
    bare: true,
    max_positional: None,
};

static GETCONF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-a"]),
    standalone_short: b"a",
    valued: WordSet::new(&["-v"]),
    valued_short: b"v",
    bare: true,
    max_positional: None,
};

static UUIDGEN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--random", "--time", "-r", "-t"]),
    standalone_short: b"rt",
    valued: WordSet::new(&[
        "--md5", "--name", "--namespace", "--sha1", "-N",
        "-m", "-n", "-s",
    ]),
    valued_short: b"mnNs",
    bare: true,
    max_positional: Some(0),
};

static GNU_HASH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--binary", "--check", "--ignore-missing", "--quiet",
        "--status", "--strict", "--tag", "--text", "--warn",
        "--zero",
        "-b", "-c", "-t", "-w", "-z",
    ]),
    standalone_short: b"bctwz",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static MD5_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-n", "-p", "-q", "-r", "-t"]),
    standalone_short: b"npqrt",
    valued: WordSet::new(&["-s"]),
    valued_short: b"s",
    bare: true,
    max_positional: None,
};

static SHASUM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--binary", "--check", "--portable", "--status",
        "--strict", "--tag", "--text", "--warn",
        "-0", "-b", "-c", "-p", "-s", "-t",
    ]),
    standalone_short: b"0bcpst",
    valued: WordSet::new(&["--algorithm", "-a"]),
    valued_short: b"a",
    bare: true,
    max_positional: None,
};

static CKSUM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--base64", "--check", "--raw", "--strict",
        "--tag", "--untagged", "--warn", "--zero",
        "-c", "-w", "-z",
    ]),
    standalone_short: b"cwz",
    valued: WordSet::new(&["--algorithm", "--length", "-a", "-l"]),
    valued_short: b"al",
    bare: true,
    max_positional: None,
};

static B2SUM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--binary", "--check", "--ignore-missing", "--quiet",
        "--status", "--strict", "--tag", "--text", "--warn",
        "--zero",
        "-b", "-c", "-t", "-w", "-z",
    ]),
    standalone_short: b"bctwz",
    valued: WordSet::new(&["--length", "-l"]),
    valued_short: b"l",
    bare: true,
    max_positional: None,
};

static SUM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--sysv", "-r", "-s"]),
    standalone_short: b"rs",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static STRINGS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--include-all-whitespace", "--print-file-name",
        "-a", "-f", "-w",
    ]),
    standalone_short: b"afw",
    valued: WordSet::new(&[
        "--bytes", "--encoding", "--output-separator",
        "--radix", "--target",
        "-T", "-e", "-n", "-o", "-s", "-t",
    ]),
    valued_short: b"Tenost",
    bare: false,
    max_positional: None,
};

static HEXDUMP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-C", "-b", "-c", "-d", "-o", "-v", "-x",
    ]),
    standalone_short: b"Cbcdovx",
    valued: WordSet::new(&[
        "-L", "-e", "-f", "-n", "-s",
    ]),
    valued_short: b"Lefns",
    bare: true,
    max_positional: None,
};

static OD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--output-duplicates", "--traditional",
        "-b", "-c", "-d", "-f", "-i", "-l", "-o",
        "-s", "-v", "-x",
    ]),
    standalone_short: b"bcdfilosvx",
    valued: WordSet::new(&[
        "--address-radix", "--endian", "--format",
        "--read-bytes", "--skip-bytes", "--strings",
        "--width",
        "-A", "-N", "-S", "-j", "-t", "-w",
    ]),
    valued_short: b"ANSjtw",
    bare: true,
    max_positional: None,
};

static SIZE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--common", "--totals",
        "-A", "-B", "-G", "-d", "-o", "-t", "-x",
    ]),
    standalone_short: b"ABGdotx",
    valued: WordSet::new(&[
        "--format", "--radix", "--target",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static SW_VERS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--buildVersion", "--productName",
        "--productVersion", "--productVersionExtra",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(0),
};

static MDLS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--raw", "-r"]),
    standalone_short: b"r",
    valued: WordSet::new(&["--name", "--nullMarker", "-n"]),
    valued_short: b"n",
    bare: false,
    max_positional: None,
};

static OTOOL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-D", "-I", "-L", "-V", "-X", "-a", "-c", "-d",
        "-f", "-h", "-l", "-o", "-r", "-t", "-v", "-x",
    ]),
    standalone_short: b"DILVXacdfhlortvx",
    valued: WordSet::new(&["-p", "-s"]),
    valued_short: b"ps",
    bare: false,
    max_positional: None,
};

static NM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--debug-syms", "--defined-only", "--demangle",
        "--dynamic", "--extern-only", "--line-numbers",
        "--no-demangle", "--no-llvm-bc", "--no-sort",
        "--numeric-sort", "--portability", "--print-armap",
        "--print-file-name", "--print-size", "--reverse-sort",
        "--special-syms", "--undefined-only",
        "-A", "-B", "-C", "-D", "-P", "-S",
        "-a", "-g", "-j", "-l", "-m", "-n", "-o",
        "-p", "-r", "-s", "-u", "-v", "-x",
    ]),
    standalone_short: b"ABCDPSagjlmnoprsuvx",
    valued: WordSet::new(&[
        "--format", "--radix", "--size-sort", "--target",
        "-f", "-t",
    ]),
    valued_short: b"ft",
    bare: false,
    max_positional: None,
};

static SYSTEM_PROFILER_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--json", "--xml", "-json", "-listDataTypes",
        "-nospinner", "-xml",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&["-detailLevel", "-timeout"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static IOREG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-S", "-a", "-b", "-f", "-i", "-l", "-r",
        "-t", "-x",
    ]),
    standalone_short: b"Sabfilrtx",
    valued: WordSet::new(&[
        "-c", "-d", "-k", "-n", "-p", "-w",
    ]),
    valued_short: b"cdknpw",
    bare: true,
    max_positional: Some(0),
};

static VM_STAT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&["-c"]),
    valued_short: b"c",
    bare: true,
    max_positional: Some(1),
};

static MDFIND_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-0", "-count", "-interpret", "-literal", "-live",
    ]),
    standalone_short: b"0",
    valued: WordSet::new(&["-attr", "-name", "-onlyin", "-s"]),
    valued_short: b"s",
    bare: false,
    max_positional: None,
};

static DIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-4", "-6", "-m", "-r", "-u", "-v",
    ]),
    standalone_short: b"46mruv",
    valued: WordSet::new(&[
        "-b", "-c", "-f", "-k", "-p", "-q", "-t", "-x", "-y",
    ]),
    valued_short: b"bcfkpqtxy",
    bare: true,
    max_positional: None,
};

pub fn is_safe_nslookup(tokens: &[Token]) -> bool {
    for t in &tokens[1..] {
        let s = t.as_str();
        if !s.starts_with('-') {
            continue;
        }
        if s == "-debug" || s == "-nodebug" || s == "-d2" {
            continue;
        }
        if s.starts_with("-type=")
            || s.starts_with("-query=")
            || s.starts_with("-port=")
            || s.starts_with("-timeout=")
            || s.starts_with("-retry=")
            || s.starts_with("-class=")
            || s.starts_with("-domain=")
            || s.starts_with("-querytype=")
        {
            continue;
        }
        return false;
    }
    true
}

static HOST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-4", "-6", "-C", "-a", "-c", "-d", "-l",
        "-r", "-s", "-v",
    ]),
    standalone_short: b"46Cacdlrsv",
    valued: WordSet::new(&[
        "-D", "-N", "-R", "-T", "-W", "-i", "-m", "-t",
    ]),
    valued_short: b"DNRTWimt",
    bare: false,
    max_positional: None,
};

static WHOIS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-A", "-B", "-G", "-H", "-I", "-K", "-L",
        "-M", "-Q", "-R", "-S", "-a", "-b", "-c",
        "-d", "-f", "-g", "-l", "-m", "-r", "-x",
    ]),
    standalone_short: b"ABGHIKLMQRSabcdfglmrx",
    valued: WordSet::new(&[
        "-T", "-V", "-h", "-i", "-p", "-s", "-t",
    ]),
    valued_short: b"TVhipst",
    bare: false,
    max_positional: None,
};

static NETSTAT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--continuous", "--extend", "--groups",
        "--interfaces", "--listening", "--masquerade",
        "--numeric", "--numeric-hosts", "--numeric-ports",
        "--numeric-users", "--program", "--route",
        "--statistics", "--symbolic", "--tcp", "--timers",
        "--udp", "--unix", "--verbose", "--wide",
        "-A", "-C", "-L", "-M", "-N", "-R", "-S", "-W",
        "-Z",
        "-a", "-b", "-c", "-d", "-e", "-f", "-g", "-i",
        "-l", "-m", "-n", "-o", "-p", "-q", "-r",
        "-s", "-t", "-u", "-v", "-w", "-x",
    ]),
    standalone_short: b"ACLMNRSWZabcdefgilmnopqrstuvwx",
    valued: WordSet::new(&[
        "-I",
    ]),
    valued_short: b"I",
    bare: true,
    max_positional: None,
};

static SS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--dccp", "--extended", "--family", "--help",
        "--info", "--ipv4", "--ipv6", "--listening", "--memory",
        "--no-header", "--numeric", "--oneline", "--options",
        "--packet", "--processes", "--raw", "--resolve",
        "--sctp", "--summary", "--tcp", "--tipc", "--udp",
        "--unix", "--version", "--vsock",
        "-0", "-4", "-6", "-E", "-H", "-O", "-V",
        "-a", "-e", "-i", "-l", "-m", "-n", "-o",
        "-p", "-r", "-s", "-t", "-u", "-w", "-x",
    ]),
    standalone_short: b"046EHOVaeilmnoprstuwx",
    valued: WordSet::new(&[
        "--filter", "--query",
        "-A", "-F", "-f",
    ]),
    valued_short: b"AFf",
    bare: true,
    max_positional: None,
};

pub fn is_safe_ss(tokens: &[Token]) -> bool {
    if has_flag(tokens, Some("-K"), Some("--kill")) {
        return false;
    }
    policy::check(tokens, &SS_POLICY)
}

static IDENTIFY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--verbose", "-ping", "-quiet", "-regard-warnings",
        "-verbose",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "-channel", "-define", "-density", "-depth",
        "-features", "-format", "-fuzz", "-interlace",
        "-limit", "-list", "-log", "-moments",
        "-monitor", "-precision", "-seed", "-set",
        "-size", "-strip", "-unique",
        "-virtual-pixel",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static SHELLCHECK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--color", "--external-sources", "--list-optional",
        "--norc", "--severity", "--wiki-link-count",
        "-C", "-a", "-x",
    ]),
    standalone_short: b"Cax",
    valued: WordSet::new(&[
        "--enable", "--exclude", "--format", "--include",
        "--rcfile", "--severity", "--shell", "--source-path",
        "--wiki-link-count",
        "-P", "-S", "-W", "-e", "-f", "-i", "-o", "-s",
    ]),
    valued_short: b"PSWefios",
    bare: false,
    max_positional: None,
};

static CLOC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--3", "--autoconf", "--by-file", "--by-file-by-lang", "--by-percent",
        "--categorized", "--counted", "--diff", "--diff-list-file", "--docstring-as-code",
        "--follow-links", "--force-lang-def", "--found-langs", "--git", "--hide-rate",
        "--ignored", "--include-content", "--json", "--md", "--no-autogen",
        "--no3", "--opt-match-d", "--opt-match-f", "--opt-not-match-d", "--opt-not-match-f",
        "--original-dir", "--progress-rate", "--quiet", "--sdir", "--show-ext",
        "--show-lang", "--show-os", "--show-stored-lang", "--skip-uniqueness", "--sql-append",
        "--strip-comments", "--sum-one", "--sum-reports", "--unicode", "--use-sloccount",
        "--v", "--vcs", "--xml", "--yaml",
    ]),
    standalone_short: b"v",
    valued: WordSet::new(&[
        "--config", "--csv-delimiter", "--diff-alignment",
        "--diff-timeout", "--exclude-content",
        "--exclude-dir", "--exclude-ext",
        "--exclude-lang", "--exclude-list-file",
        "--force-lang", "--fullpath",
        "--include-ext", "--include-lang",
        "--lang-no-ext", "--list-file", "--match-d",
        "--match-f", "--not-match-d", "--not-match-f",
        "--out", "--read-binary-files", "--read-lang-def",
        "--report-file", "--script-lang", "--skip-archive",
        "--sql", "--sql-project", "--sql-style",
        "--timeout", "--write-lang-def",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
};

static TOKEI_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--compact", "--files", "--hidden", "--no-ignore",
        "--no-ignore-dot", "--no-ignore-parent",
        "--no-ignore-vcs", "--verbose",
        "-C", "-V", "-f",
    ]),
    standalone_short: b"CVf",
    valued: WordSet::new(&[
        "--columns", "--exclude", "--input",
        "--languages", "--num-format", "--output",
        "--sort", "--type",
        "-c", "-e", "-i", "-l", "-o", "-s", "-t",
    ]),
    valued_short: b"ceilost",
    bare: true,
    max_positional: None,
};

static CUCUMBER_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--backtrace", "--color", "--dry-run", "--expand",
        "--guess", "--i18n-keywords", "--i18n-languages",
        "--init", "--no-color", "--no-diff", "--no-multiline",
        "--no-snippets", "--no-source", "--no-strict",
        "--publish", "--publish-quiet", "--quiet",
        "--retry", "--snippets", "--strict", "--verbose",
        "--wip",
        "-b", "-d", "-e", "-q",
    ]),
    standalone_short: b"bdeq",
    valued: WordSet::new(&[
        "--ci-environment", "--format", "--format-options",
        "--language", "--lines", "--name", "--order",
        "--out", "--profile", "--require",
        "--require-module", "--retry", "--tags",
        "-f", "-i", "-l", "-n", "-o", "-p", "-r", "-t",
    ]),
    valued_short: b"filnoprt",
    bare: true,
    max_positional: None,
};

fn dispatch_policy(cmd: &str, tokens: &[Token]) -> Option<bool> {
    match cmd {
        "ls" => Some(policy::check(tokens, &LS_POLICY)),
        "eza" | "exa" => Some(policy::check(tokens, &EZA_POLICY)),
        "delta" => Some(policy::check(tokens, &DELTA_POLICY)),
        "colordiff" => Some(policy::check(tokens, &COLORDIFF_POLICY)),
        "dirname" => Some(policy::check(tokens, &DIRNAME_POLICY)),
        "basename" => Some(policy::check(tokens, &BASENAME_POLICY)),
        "realpath" => Some(policy::check(tokens, &REALPATH_POLICY)),
        "readlink" => Some(policy::check(tokens, &READLINK_POLICY)),
        "stat" => Some(policy::check(tokens, &STAT_POLICY)),
        "du" => Some(policy::check(tokens, &DU_POLICY)),
        "df" => Some(policy::check(tokens, &DF_POLICY)),
        "true" | "false" | "whoami" | "branchdiff" => {
            Some(policy::check(tokens, &BARE_ONLY))
        }
        "printenv" => Some(policy::check(tokens, &PRINTENV_POLICY)),
        "type" => Some(policy::check(tokens, &TYPE_POLICY)),
        "whereis" => Some(policy::check(tokens, &WHEREIS_POLICY)),
        "which" => Some(policy::check(tokens, &WHICH_POLICY)),
        "pwd" => Some(policy::check(tokens, &PWD_POLICY)),
        "cd" => Some(policy::check(tokens, &CD_POLICY)),
        "unset" => Some(policy::check(tokens, &UNSET_POLICY)),
        "uname" => Some(policy::check(tokens, &UNAME_POLICY)),
        "nproc" => Some(policy::check(tokens, &NPROC_POLICY)),
        "uptime" => Some(policy::check(tokens, &UPTIME_POLICY)),
        "id" => Some(policy::check(tokens, &ID_POLICY)),
        "groups" => Some(policy::check(tokens, &GROUPS_POLICY)),
        "tty" => Some(policy::check(tokens, &TTY_POLICY)),
        "locale" => Some(policy::check(tokens, &LOCALE_POLICY)),
        "cal" => Some(policy::check(tokens, &CAL_POLICY)),
        "sleep" => Some(policy::check(tokens, &SLEEP_POLICY)),
        "who" => Some(policy::check(tokens, &WHO_POLICY)),
        "w" => Some(policy::check(tokens, &W_POLICY)),
        "last" => Some(policy::check(tokens, &LAST_POLICY)),
        "lastlog" => Some(policy::check(tokens, &LASTLOG_POLICY)),
        "ps" => Some(policy::check(tokens, &PS_POLICY)),
        "top" => Some(policy::check(tokens, &TOP_POLICY)),
        "htop" => Some(policy::check(tokens, &HTOP_POLICY)),
        "iotop" => Some(policy::check(tokens, &IOTOP_POLICY)),
        "procs" => Some(policy::check(tokens, &PROCS_POLICY)),
        "dust" => Some(policy::check(tokens, &DUST_POLICY)),
        "lsof" => Some(policy::check(tokens, &LSOF_POLICY)),
        "pgrep" => Some(policy::check(tokens, &PGREP_POLICY)),
        "jq" => Some(policy::check(tokens, &JQ_POLICY)),
        "base64" => Some(policy::check(tokens, &BASE64_POLICY)),
        "xxd" => Some(policy::check(tokens, &XXD_POLICY)),
        "getconf" => Some(policy::check(tokens, &GETCONF_POLICY)),
        "uuidgen" => Some(policy::check(tokens, &UUIDGEN_POLICY)),
        "md5sum" | "sha256sum" | "sha1sum" | "sha512sum" => {
            Some(policy::check(tokens, &GNU_HASH_POLICY))
        }
        "md5" => Some(policy::check(tokens, &MD5_POLICY)),
        "shasum" => Some(policy::check(tokens, &SHASUM_POLICY)),
        "cksum" => Some(policy::check(tokens, &CKSUM_POLICY)),
        "b2sum" => Some(policy::check(tokens, &B2SUM_POLICY)),
        "sum" => Some(policy::check(tokens, &SUM_POLICY)),
        "strings" => Some(policy::check(tokens, &STRINGS_POLICY)),
        "hexdump" => Some(policy::check(tokens, &HEXDUMP_POLICY)),
        "od" => Some(policy::check(tokens, &OD_POLICY)),
        "size" => Some(policy::check(tokens, &SIZE_POLICY)),
        "sw_vers" => Some(policy::check(tokens, &SW_VERS_POLICY)),
        "mdls" => Some(policy::check(tokens, &MDLS_POLICY)),
        "otool" => Some(policy::check(tokens, &OTOOL_POLICY)),
        "nm" => Some(policy::check(tokens, &NM_POLICY)),
        "system_profiler" => Some(policy::check(tokens, &SYSTEM_PROFILER_POLICY)),
        "ioreg" => Some(policy::check(tokens, &IOREG_POLICY)),
        "vm_stat" => Some(policy::check(tokens, &VM_STAT_POLICY)),
        "mdfind" => Some(policy::check(tokens, &MDFIND_POLICY)),
        "dig" => Some(policy::check(tokens, &DIG_POLICY)),
        "host" => Some(policy::check(tokens, &HOST_POLICY)),
        "whois" => Some(policy::check(tokens, &WHOIS_POLICY)),
        "netstat" => Some(policy::check(tokens, &NETSTAT_POLICY)),
        "identify" => Some(policy::check(tokens, &IDENTIFY_POLICY)),
        "shellcheck" => Some(policy::check(tokens, &SHELLCHECK_POLICY)),
        "cloc" => Some(policy::check(tokens, &CLOC_POLICY)),
        "tokei" => Some(policy::check(tokens, &TOKEI_POLICY)),
        "cucumber" => Some(policy::check(tokens, &CUCUMBER_POLICY)),
        _ => None,
    }
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    if let result @ Some(_) = dispatch_policy(cmd, tokens) {
        return result;
    }
    match cmd {
        "fd" => Some(is_safe_fd(tokens)),
        "tree" => Some(is_safe_tree(tokens)),
        "file" => Some(is_safe_file_cmd(tokens)),
        "date" => Some(is_safe_date(tokens)),
        "nslookup" => Some(is_safe_nslookup(tokens)),
        "ss" => Some(is_safe_ss(tokens)),
        "ifconfig" => Some(is_safe_ifconfig(tokens)),
        "route" => Some(is_safe_route(tokens)),
        "safe-chains" => Some(true),
        "grep" | "egrep" | "fgrep" => Some(is_safe_grep(tokens)),
        "rg" => Some(is_safe_rg(tokens)),
        "cat" => Some(is_safe_cat(tokens)),
        "head" => Some(is_safe_head(tokens)),
        "tail" => Some(is_safe_tail(tokens)),
        "wc" => Some(is_safe_wc(tokens)),
        "cut" => Some(is_safe_cut(tokens)),
        "tr" => Some(is_safe_tr(tokens)),
        "uniq" => Some(is_safe_uniq(tokens)),
        "diff" => Some(is_safe_diff(tokens)),
        "comm" => Some(is_safe_comm(tokens)),
        "paste" => Some(is_safe_paste(tokens)),
        "tac" => Some(is_safe_tac(tokens)),
        "rev" => Some(is_safe_rev(tokens)),
        "nl" => Some(is_safe_nl(tokens)),
        "expand" => Some(is_safe_expand(tokens)),
        "unexpand" => Some(is_safe_unexpand(tokens)),
        "fold" => Some(is_safe_fold(tokens)),
        "fmt" => Some(is_safe_fmt(tokens)),
        "column" => Some(is_safe_column(tokens)),
        "iconv" => Some(is_safe_iconv(tokens)),
        "nroff" => Some(is_safe_nroff(tokens)),
        "echo" => Some(is_safe_echo(tokens)),
        "printf" => Some(is_safe_printf(tokens)),
        "seq" => Some(is_safe_seq(tokens)),
        "test" => Some(is_safe_test(tokens)),
        "expr" => Some(is_safe_expr(tokens)),
        "bc" => Some(is_safe_bc(tokens)),
        "factor" => Some(is_safe_factor(tokens)),
        "bat" => Some(is_safe_bat(tokens)),
        "arch" => Some(is_safe_arch(tokens)),
        "command" => Some(is_safe_command_builtin(tokens)),
        "hostname" => Some(is_safe_hostname(tokens)),
        "find" => Some(is_safe_find(tokens, is_safe)),
        "sed" => Some(is_safe_sed(tokens)),
        "sort" => Some(is_safe_sort(tokens)),
        "yq" => Some(is_safe_yq(tokens)),
        "xmllint" => Some(is_safe_xmllint(tokens)),
        "awk" | "gawk" | "mawk" | "nawk" => Some(is_safe_awk(tokens)),
        _ => None,
    }
}

fn policy_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("ls", LS_POLICY.describe()),
        CommandDoc::handler("eza / exa", EZA_POLICY.describe()),
        CommandDoc::handler("delta", DELTA_POLICY.describe()),
        CommandDoc::handler("colordiff", COLORDIFF_POLICY.describe()),
        CommandDoc::handler("dirname", DIRNAME_POLICY.describe()),
        CommandDoc::handler("basename", BASENAME_POLICY.describe()),
        CommandDoc::handler("realpath", REALPATH_POLICY.describe()),
        CommandDoc::handler("readlink", READLINK_POLICY.describe()),
        CommandDoc::handler("stat", STAT_POLICY.describe()),
        CommandDoc::handler("du", DU_POLICY.describe()),
        CommandDoc::handler("df", DF_POLICY.describe()),
        CommandDoc::handler("true / false",
            "Bare invocation only. No flags or arguments allowed."),
        CommandDoc::handler("printenv", PRINTENV_POLICY.describe()),
        CommandDoc::handler("type", TYPE_POLICY.describe()),
        CommandDoc::handler("whereis", WHEREIS_POLICY.describe()),
        CommandDoc::handler("which", WHICH_POLICY.describe()),
        CommandDoc::handler("whoami",
            "Bare invocation only. No flags or arguments allowed."),
        CommandDoc::handler("pwd", PWD_POLICY.describe()),
        CommandDoc::handler("cd", CD_POLICY.describe()),
        CommandDoc::handler("unset", UNSET_POLICY.describe()),
        CommandDoc::handler("uname", UNAME_POLICY.describe()),
        CommandDoc::handler("nproc", NPROC_POLICY.describe()),
        CommandDoc::handler("uptime", UPTIME_POLICY.describe()),
        CommandDoc::handler("id", ID_POLICY.describe()),
        CommandDoc::handler("groups",
            "Any positional arguments (usernames). No flags."),
        CommandDoc::handler("tty", TTY_POLICY.describe()),
        CommandDoc::handler("locale", LOCALE_POLICY.describe()),
        CommandDoc::handler("cal", CAL_POLICY.describe()),
        CommandDoc::handler("sleep",
            "Positional duration arguments only. No flags."),
        CommandDoc::handler("who", WHO_POLICY.describe()),
        CommandDoc::handler("w", W_POLICY.describe()),
        CommandDoc::handler("last", LAST_POLICY.describe()),
        CommandDoc::handler("lastlog", format!("{} Denied: -C/--clear, -S/--set.", LASTLOG_POLICY.describe())),
        CommandDoc::handler("ps", PS_POLICY.describe()),
        CommandDoc::handler("top", TOP_POLICY.describe()),
        CommandDoc::handler("htop", HTOP_POLICY.describe()),
        CommandDoc::handler("iotop", IOTOP_POLICY.describe()),
        CommandDoc::handler("procs", PROCS_POLICY.describe()),
        CommandDoc::handler("dust", DUST_POLICY.describe()),
        CommandDoc::handler("lsof", LSOF_POLICY.describe()),
        CommandDoc::handler("pgrep", PGREP_POLICY.describe()),
        CommandDoc::handler("jq", JQ_POLICY.describe()),
        CommandDoc::handler("base64", BASE64_POLICY.describe()),
        CommandDoc::handler("xxd", XXD_POLICY.describe()),
        CommandDoc::handler("getconf", GETCONF_POLICY.describe()),
        CommandDoc::handler("uuidgen", UUIDGEN_POLICY.describe()),
        CommandDoc::handler("md5sum / sha256sum / sha1sum / sha512sum", GNU_HASH_POLICY.describe()),
        CommandDoc::handler("md5", MD5_POLICY.describe()),
        CommandDoc::handler("shasum", SHASUM_POLICY.describe()),
        CommandDoc::handler("cksum", CKSUM_POLICY.describe()),
        CommandDoc::handler("b2sum", B2SUM_POLICY.describe()),
        CommandDoc::handler("sum", SUM_POLICY.describe()),
        CommandDoc::handler("strings", STRINGS_POLICY.describe()),
        CommandDoc::handler("hexdump", HEXDUMP_POLICY.describe()),
        CommandDoc::handler("od", OD_POLICY.describe()),
        CommandDoc::handler("size", SIZE_POLICY.describe()),
        CommandDoc::handler("sw_vers", SW_VERS_POLICY.describe()),
        CommandDoc::handler("mdls", MDLS_POLICY.describe()),
        CommandDoc::handler("otool", OTOOL_POLICY.describe()),
        CommandDoc::handler("nm", NM_POLICY.describe()),
        CommandDoc::handler("system_profiler", SYSTEM_PROFILER_POLICY.describe()),
        CommandDoc::handler("ioreg", IOREG_POLICY.describe()),
        CommandDoc::handler("vm_stat", VM_STAT_POLICY.describe()),
        CommandDoc::handler("mdfind", MDFIND_POLICY.describe()),
        CommandDoc::handler("dig", DIG_POLICY.describe()),
        CommandDoc::handler("nslookup",
            "Allowed: positional args, -debug, -nodebug, -d2, and valued options (-type=, -query=, -port=, -timeout=, -retry=, -class=, -domain=, -querytype=)."),
        CommandDoc::handler("host", HOST_POLICY.describe()),
        CommandDoc::handler("whois", WHOIS_POLICY.describe()),
        CommandDoc::handler("netstat", NETSTAT_POLICY.describe()),
        CommandDoc::handler("ss", format!("{} Denied: -K/--kill, -D/--diag.", SS_POLICY.describe())),
        CommandDoc::handler("identify", IDENTIFY_POLICY.describe()),
        CommandDoc::handler("shellcheck", SHELLCHECK_POLICY.describe()),
        CommandDoc::handler("cloc", CLOC_POLICY.describe()),
        CommandDoc::handler("tokei", TOKEI_POLICY.describe()),
        CommandDoc::handler("cucumber", CUCUMBER_POLICY.describe()),
        CommandDoc::handler("branchdiff",
            "Bare invocation only. No flags or arguments allowed."),
        CommandDoc::handler("safe-chains",
            "Any arguments allowed (safe-chains is this tool)."),
    ]
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    let mut docs = vec![
        CommandDoc::handler("arch",
            "Allowed: bare `arch` only (prints machine architecture). Flags denied (can execute commands under different architectures)."),
        CommandDoc::handler("cat", CAT_POLICY.describe()),
        CommandDoc::handler("comm", COMM_POLICY.describe()),
        CommandDoc::handler("command",
            "Allowed: -v, -V (check if command exists). Bare `command` and execution of other commands denied."),
        CommandDoc::handler("cut", CUT_POLICY.describe()),
        CommandDoc::handler("diff", DIFF_POLICY.describe()),
        CommandDoc::handler("find",
            "Safe unless dangerous flags: -delete, -ok, -okdir, -fls, -fprint, -fprint0, -fprintf. \
             -exec/-execdir allowed when the executed command is itself safe."),
        CommandDoc::handler("grep", GREP_POLICY.describe()),
        CommandDoc::handler("head", HEAD_POLICY.describe()),
        CommandDoc::wordset("hostname", &HOSTNAME_DISPLAY),
        CommandDoc::handler("nl", NL_POLICY.describe()),
        CommandDoc::handler("paste", PASTE_POLICY.describe()),
        CommandDoc::handler("rev", REV_POLICY.describe()),
        CommandDoc::handler("rg", RG_POLICY.describe()),
        CommandDoc::handler("sed",
            "Safe unless -i/--in-place flag or 'e' modifier on substitutions (executes replacement as shell command)."),
        CommandDoc::handler("sort",
            "Safe unless -o/--output or --compress-program flag."),
        CommandDoc::handler("tac", TAC_POLICY.describe()),
        CommandDoc::handler("tail", TAIL_POLICY.describe()),
        CommandDoc::handler("tr", TR_POLICY.describe()),
        CommandDoc::handler("uniq", format!("{} Max 1 positional arg (second would be output file).", UNIQ_POLICY.describe())),
        CommandDoc::handler("wc", WC_POLICY.describe()),
        CommandDoc::handler("yq",
            "Safe unless -i/--inplace flag."),
        CommandDoc::handler("awk / gawk / mawk / nawk",
            "Safe unless program contains system, getline, |, >, >>, or -f flag (file-based program)."),
        CommandDoc::handler("xmllint",
            "Safe unless --output flag."),
        CommandDoc::handler("expand", EXPAND_POLICY.describe()),
        CommandDoc::handler("unexpand", UNEXPAND_POLICY.describe()),
        CommandDoc::handler("fold", FOLD_POLICY.describe()),
        CommandDoc::handler("fmt", FMT_POLICY.describe()),
        CommandDoc::handler("column", COLUMN_POLICY.describe()),
        CommandDoc::handler("iconv", ICONV_POLICY.describe()),
        CommandDoc::handler("nroff", NROFF_POLICY.describe()),
        CommandDoc::handler("echo", ECHO_POLICY.describe()),
        CommandDoc::handler("printf", PRINTF_POLICY.describe()),
        CommandDoc::handler("seq", SEQ_POLICY.describe()),
        CommandDoc::handler("test",
            "Allowed: any arguments (test uses operators like -f, -d as conditionals, not flags)."),
        CommandDoc::handler("expr",
            "Allowed: any arguments (expr uses operators as expressions, not flags). Requires at least one argument."),
        CommandDoc::handler("bc", BC_POLICY.describe()),
        CommandDoc::handler("factor", FACTOR_POLICY.describe()),
        CommandDoc::handler("bat", BAT_POLICY.describe()),
        CommandDoc::handler("fd",
            "Safe unless --exec/-x or --exec-batch/-X flags (execute arbitrary commands)."),
        CommandDoc::handler("tree",
            "Safe unless -o flag (write output to file)."),
        CommandDoc::handler("file",
            "Safe unless -C/--compile flag (write compiled magic file)."),
        CommandDoc::handler("date",
            "Safe unless -s/--set flag (set system date)."),
        CommandDoc::handler("route",
            "Allowed subcommands: get, monitor, print, show. Allowed flags: -4, -6, -n, -v. Bare invocation allowed."),
        CommandDoc::handler("ifconfig", IFCONFIG_POLICY.describe()),
    ];
    docs.extend(policy_docs());
    docs
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        grep_pattern: "grep pattern file.txt",
        grep_recursive: "grep -rn pattern .",
        grep_combined: "grep -inl pattern .",
        grep_context: "grep -A 3 -B 3 pattern file",
        grep_extended: "grep -E 'foo|bar' file",
        grep_fixed: "grep -F exact file",
        grep_count: "grep -c pattern file",
        grep_file_pattern: "grep -f patterns.txt file",
        grep_exclude: "grep --exclude='*.o' pattern .",
        grep_color: "grep --color pattern file",
        grep_color_eq: "grep --color=always pattern file",
        grep_max_count: "grep --max-count=5 pattern file",
        grep_null: "grep --null -l pattern .",
        grep_perl: "grep -P '\\d+' file",
        egrep_safe: "egrep 'foo|bar' file",
        fgrep_safe: "fgrep exact file",

        rg_pattern: "rg pattern",
        rg_fixed: "rg -F literal .",
        rg_context: "rg -A 5 -B 5 pattern",
        rg_type: "rg -t rust pattern",
        rg_glob: "rg -g '*.rs' pattern",
        rg_max_count: "rg -m 10 pattern",
        rg_replace: "rg -r replacement pattern",
        rg_json: "rg --json pattern",
        rg_multiline: "rg -U pattern",
        rg_files: "rg --files",
        rg_type_list: "rg --type-list",
        rg_combined: "rg -inl pattern .",
        rg_color: "rg --color always pattern",
        rg_threads: "rg -j 4 pattern",

        cat_file: "cat file.txt",
        cat_number: "cat -n file.txt",
        cat_bare: "cat",
        cat_show_all: "cat -A file.txt",
        cat_combined: "cat -bns file.txt",

        head_default: "head file.txt",
        head_lines: "head -n 20 file.txt",
        head_bytes: "head -c 100 file.txt",
        head_numeric: "head -5 file.txt",
        head_bare: "head",
        head_quiet: "head -q file1 file2",

        tail_default: "tail file.txt",
        tail_lines: "tail -n 20 file.txt",
        tail_follow: "tail -f logfile",
        tail_follow_upper: "tail -F logfile",
        tail_numeric: "tail -20 logfile",
        tail_bare: "tail",

        wc_default: "wc file.txt",
        wc_lines: "wc -l file.txt",
        wc_words: "wc -w file.txt",
        wc_chars: "wc -m file.txt",
        wc_combined: "wc -lw file.txt",
        wc_bare: "wc",

        cut_fields: "cut -f 1 file.txt",
        cut_delim: "cut -d: -f1 /etc/passwd",
        cut_bytes: "cut -b 1-10 file",
        cut_complement: "cut --complement -f 1 file",

        tr_lower: "tr A-Z a-z",
        tr_delete: "tr -d '\\n'",
        tr_squeeze: "tr -s ' '",

        uniq_bare: "uniq",
        uniq_input: "uniq sorted.txt",
        uniq_count: "uniq -c sorted.txt",
        uniq_skip: "uniq -f 1 sorted.txt",
        uniq_ignore_case: "uniq -i sorted.txt",
        uniq_combined: "uniq -cu sorted.txt",

        diff_files: "diff file1.txt file2.txt",
        diff_unified: "diff -u file1 file2",
        diff_context: "diff -C 3 file1 file2",
        diff_recursive: "diff -r dir1 dir2",
        diff_brief: "diff --brief dir1 dir2",
        diff_side: "diff -y file1 file2",
        diff_color: "diff --color file1 file2",

        comm_default: "comm file1 file2",
        comm_suppress: "comm -23 file1 file2",
        comm_combined: "comm -12 file1 file2",

        paste_files: "paste file1 file2",
        paste_serial: "paste -s file",
        paste_delim: "paste -d, file1 file2",
        paste_bare: "paste",

        tac_file: "tac file.txt",
        tac_bare: "tac",
        tac_separator: "tac -s '---' file",
        tac_before: "tac -b file",

        rev_file: "rev file.txt",
        rev_bare: "rev",

        nl_file: "nl file.txt",
        nl_bare: "nl",
        nl_body: "nl -b a file.txt",
        nl_format: "nl -n rz file.txt",
        nl_width: "nl -w 4 file.txt",

        arch_bare: "arch",
        arch_help: "arch --help",
        arch_version: "arch --version",
        command_help: "command --help",
        command_version: "command --version",
        command_v: "command -v git",
        command_v_upper: "command -V git",
        command_v_path: "command -v /usr/bin/git",
        hostname_help: "hostname --help",
        hostname_version: "hostname --version",
        hostname_bare: "hostname",
        hostname_fqdn: "hostname -f",
        hostname_short: "hostname -s",
        hostname_domain: "hostname -d",
        hostname_ip: "hostname -i",
        hostname_all_ip: "hostname -I",
        hostname_all_addr: "hostname -A",
        find_name: "find . -name '*.rb'",
        find_type_name: "find . -type f -name '*.py'",
        find_maxdepth: "find /tmp -maxdepth 2",
        find_print: "find . -name '*.log' -print",
        find_print0: "find . -name '*.log' -print0",
        find_exec_grep_l: "find . -name '*.rb' -exec grep -l pattern {} \\;",
        find_exec_grep_l_plus: "find . -name '*.rb' -exec grep -l pattern {} +",
        find_exec_cat: "find . -exec cat {} \\;",
        find_execdir_cat: "find . -execdir cat {} \\;",
        find_execdir_grep: "find . -execdir grep pattern {} \\;",
        find_exec_grep_safe: "find . -name '*.py' -exec grep pattern {} +",
        find_exec_nested_bash_safe: "find . -exec bash -c 'git status' \\;",
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
        sort_basic: "sort file.txt",
        sort_reverse: "sort -r file.txt",
        sort_n_u: "sort -n -u file.txt",
        sort_field: "sort -t: -k2 /etc/passwd",
        yq_read: "yq '.key' file.yaml",
        yq_eval: "yq eval '.metadata.name' deployment.yaml",
        xmllint_read: "xmllint --xpath '//name' file.xml",
        xmllint_format: "xmllint --format file.xml",
        awk_print_field: "awk '{print $1}' file.txt",
        awk_print_multiple_fields: "awk '{print $1, $3}' file.txt",
        awk_field_separator: "awk -F: '{print $1}' /etc/passwd",
        awk_pattern: "awk '/error/ {print $0}' log.txt",
        awk_nr: "awk 'NR==5' file.txt",
        awk_begin_end_safe: "awk 'BEGIN{n=0} {n++} END{print n}' file.txt",
        gawk_safe: "gawk '{print $2}' file.txt",
        awk_netstat_pipeline: "awk '{print $6}'",
        awk_string_literal_system: "awk 'BEGIN{print \"system failed\"}'",
        awk_string_literal_redirect: "awk '{print \">\"}'",
        awk_string_literal_pipe: "awk '{print \"a | b\"}'",
        awk_string_literal_getline: "awk 'BEGIN{print \"getline is a keyword\"}'",
    }

    denied! {
        grep_unknown_flag_denied: "grep --output=file pattern",
        grep_unknown_short_denied: "grep -Y pattern file",

        rg_pre_denied: "rg --pre cat pattern",
        rg_pre_glob_denied: "rg --pre cat --pre-glob '*.pdf' pattern",
        rg_unknown_denied: "rg --unknown-flag pattern",

        cat_unknown_denied: "cat --unknown file",

        head_unknown_denied: "head --output file.txt",
        head_unknown_short_denied: "head -X file",

        tail_unknown_denied: "tail --unknown file",

        wc_unknown_denied: "wc --unknown file",

        cut_unknown_denied: "cut --unknown file",

        tr_unknown_denied: "tr --unknown a b",

        uniq_output_file_denied: "uniq input.txt output.txt",
        uniq_unknown_denied: "uniq --unknown sorted.txt",

        diff_unknown_denied: "diff --unknown file1 file2",

        comm_unknown_denied: "comm --unknown file1 file2",

        paste_unknown_denied: "paste --unknown file",

        tac_unknown_denied: "tac --unknown file",

        rev_unknown_denied: "rev --unknown file",
        rev_unknown_short_denied: "rev -x file",

        nl_unknown_denied: "nl --unknown file",

        arch_exec_denied: "arch -x86_64 rm -rf /",
        arch_flag_denied: "arch -arm64 echo hello",
        arch_any_flag_denied: "arch -x86_64",
        command_bare_denied: "command",
        command_exec_denied: "command git status",
        command_exec_rm_denied: "command rm -rf /",
        command_only_flag_denied: "command -v",
        hostname_set_denied: "hostname evil",
        hostname_set_fqdn_denied: "hostname new.example.com",
        hostname_flag_with_name_denied: "hostname -f evil",
        find_delete_denied: "find . -name '*.tmp' -delete",
        find_exec_rm: "find . -exec rm {} \\;",
        find_exec_rm_rf: "find . -exec rm -rf {} +",
        find_execdir_unsafe_denied: "find . -execdir rm {} \\;",
        find_ok_denied: "find . -ok rm {} \\;",
        find_okdir_denied: "find . -okdir rm {} \\;",
        find_exec_nested_bash_chain_denied: "find . -exec bash -c 'ls && rm -rf /' \\;",
        find_type_delete_denied: "find . -type f -name '*.bak' -delete",
        find_fprint_denied: "find . -fprint /tmp/list.txt",
        find_fprint0_denied: "find . -fprint0 /tmp/list.txt",
        find_fls_denied: "find . -fls /tmp/list.txt",
        find_fprintf_denied: "find . -fprintf /tmp/list.txt '%p'",
        sed_inplace_denied: "sed -i 's/foo/bar/' file.txt",
        sed_in_place_long_denied: "sed --in-place 's/foo/bar/' file.txt",
        sed_inplace_backup_denied: "sed -i.bak 's/foo/bar/' file.txt",
        sed_ni_combined_denied: "sed -ni 's/foo/bar/p' file.txt",
        sed_in_combined_denied: "sed -in 's/foo/bar/' file.txt",
        sed_in_place_eq_denied: "sed --in-place=.bak 's/foo/bar/' file.txt",
        sed_exec_modifier_denied: "sed 's/test/touch \\/tmp\\/pwned/e'",
        sed_exec_with_global_denied: "sed 's/foo/bar/ge'",
        sed_exec_alternate_delim_denied: "sed 's|test|touch /tmp/pwned|e'",
        sed_exec_via_e_flag_denied: "sed -e 's/test/touch tmp/e'",
        sed_exec_with_w_flag_denied: "sed 's/test/cmd/we'",
        sed_standalone_e_command_denied: "sed e",
        sed_address_e_command_denied: "sed 1e",
        sed_regex_address_e_denied: "sed '/pattern/e'",
        sed_range_address_e_denied: "sed '1,5e'",
        sed_dollar_address_e_denied: "sed '$e'",
        sed_e_via_flag_denied: "sed -e e",
        sed_expression_flag_exec_denied: "sed -e 's/foo/bar/e'",
        sed_multiple_expressions_exec_denied: "sed -e 's/foo/bar/' -e 's/x/y/e'",
        sort_output_denied: "sort -o output.txt file.txt",
        sort_output_long_denied: "sort --output=result.txt file.txt",
        sort_output_long_space_denied: "sort --output result.txt file.txt",
        sort_rno_combined_denied: "sort -rno sorted.txt file.txt",
        sort_compress_program_denied: "sort --compress-program sh file.txt",
        sort_compress_program_eq_denied: "sort --compress-program=gzip file.txt",
        yq_inplace_denied: "yq -i '.key = \"value\"' file.yaml",
        yq_inplace_long_denied: "yq --inplace '.key = \"value\"' file.yaml",
        xmllint_output_denied: "xmllint --output result.xml file.xml",
        xmllint_output_eq_denied: "xmllint --output=result.xml file.xml",
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
        sed_inplace_trailing_help_denied: "sed -i 's/foo/bar/' file --help",
        sed_inplace_trailing_version_denied: "sed -i 's/foo/bar/' file --version",
        sort_output_trailing_help_denied: "sort -o output.txt file --help",
        sort_output_trailing_version_denied: "sort -o output.txt file --version",
    }

    safe! {
        expand_file: "expand file.txt",
        expand_initial: "expand -i file.txt",
        expand_tabs: "expand -t 4 file.txt",
        expand_bare: "expand",

        unexpand_file: "unexpand file.txt",
        unexpand_all: "unexpand -a file.txt",
        unexpand_tabs: "unexpand --tabs 8 file.txt",
        unexpand_bare: "unexpand",

        fold_file: "fold file.txt",
        fold_width: "fold -w 80 file.txt",
        fold_bytes: "fold -b file.txt",
        fold_spaces: "fold -s file.txt",
        fold_bare: "fold",

        fmt_file: "fmt file.txt",
        fmt_width: "fmt -w 72 file.txt",
        fmt_split: "fmt -s file.txt",
        fmt_bare: "fmt",

        column_file: "column file.txt",
        column_table: "column -t file.txt",
        column_separator: "column -s, file.txt",
        column_json: "column -J file.txt",
        column_bare: "column",

        iconv_convert: "iconv -f UTF-8 -t ASCII file.txt",
        iconv_list: "iconv -l",
        iconv_silent: "iconv -s -f LATIN1 -t UTF-8 file",

        nroff_file: "nroff -man page.1",
        nroff_macro: "nroff -m mandoc page.1",
        nroff_term: "nroff -T ascii page.1",

        echo_hello: "echo hello world",
        echo_no_newline: "echo -n hello",
        echo_escape: "echo -e 'hello\\nworld'",
        echo_bare: "echo",

        printf_format: "printf '%s\\n' hello",
        printf_number: "printf '%d' 42",

        seq_range: "seq 1 10",
        seq_step: "seq 1 2 10",
        seq_format: "seq -f '%.2f' 1 0.5 5",
        seq_separator: "seq -s, 1 5",
        seq_equal_width: "seq -w 1 10",

        test_file: "test -f file.txt",
        test_dir: "test -d /tmp",
        test_eq: "test 1 -eq 1",
        test_bare: "test",

        expr_add: "expr 1 + 2",
        expr_match: "expr hello : 'h.*'",
        expr_length: "expr length hello",

        bc_bare: "bc",
        bc_mathlib: "bc -l",
        bc_quiet: "bc -q",
        bc_file: "bc -l calc.bc",

        factor_number: "factor 42",
        factor_multiple: "factor 42 100",
        factor_bare: "factor",

        bat_file: "bat file.txt",
        bat_plain: "bat -p file.txt",
        bat_language: "bat -l rust file.txt",
        bat_line_range: "bat -r 10:20 file.txt",
        bat_theme: "bat --theme=gruvbox file.txt",
        bat_number: "bat -n file.txt",
        bat_bare: "bat",
    }

    denied! {
        expand_unknown_denied: "expand --unknown file",
        unexpand_unknown_denied: "unexpand --unknown file",
        fold_unknown_denied: "fold --unknown file",
        fmt_unknown_denied: "fmt --unknown file",
        column_unknown_denied: "column --unknown file",
        iconv_output_denied: "iconv -o output.txt file",
        iconv_unknown_denied: "iconv --unknown file",
        nroff_unknown_denied: "nroff --unknown file",
        echo_unknown_denied: "echo --unknown hello",
        printf_bare_denied: "printf",
        seq_unknown_denied: "seq --unknown 1 10",
        bc_unknown_denied: "bc --unknown",
        factor_unknown_denied: "factor --unknown",
        bat_pager_denied: "bat --pager 'rm -rf /' file",
        bat_unknown_denied: "bat --unknown file",
    }

    safe! {
        fd_pattern: "fd pattern",
        fd_hidden: "fd -H pattern",
        fd_type: "fd -t f pattern",
        fd_extension: "fd -e rs pattern",
        fd_glob: "fd -g '*.rs'",
        fd_follow: "fd -L pattern",
        fd_absolute: "fd -a pattern",
        fd_color: "fd --color auto pattern",
        fd_max_depth: "fd --max-depth 3 pattern",

        tree_basic: "tree",
        tree_depth: "tree -L 3",
        tree_dirs: "tree -d",
        tree_all: "tree -a",
        tree_pattern: "tree -P '*.rs'",
        tree_json: "tree -J",

        file_basic: "file README.md",
        file_brief: "file -b README.md",
        file_mime: "file --mime README.md",
        file_multiple: "file *.txt",
        file_dereference: "file -L symlink",

        ls_basic: "ls",
        ls_all: "ls -la",
        ls_recursive: "ls -R /tmp",
        eza_basic: "eza --long",
        exa_tree: "exa --tree",
        delta_diff: "delta file1 file2",
        colordiff_files: "colordiff file1 file2",
        dirname_path: "dirname /usr/bin/ls",
        basename_path: "basename /usr/bin/ls",
        realpath_path: "realpath ./relative",
        readlink_path: "readlink /usr/bin/python",
        stat_file: "stat file.txt",
        du_dir: "du -sh /tmp",
        df_all: "df -h",
    }

    denied! {
        fd_exec_denied: "fd pattern --exec rm",
        fd_exec_short_denied: "fd pattern -x rm",
        fd_exec_batch_denied: "fd -t f pattern --exec-batch rm",
        fd_exec_batch_short_denied: "fd pattern -X rm",
        fd_exec_combined_denied: "fd -xH pattern",
        fd_exec_batch_combined_denied: "fd -HX pattern",
        fd_bare_denied: "fd",
        tree_output_denied: "tree -o tree.txt",
        file_compile_denied: "file -C",
        file_compile_long_denied: "file --compile",
    }

    safe! {
        date_bare: "date",
        date_format: "date '+%Y-%m-%d'",
        date_utc: "date -u",
        date_reference: "date -r file.txt",
    }

    denied! {
        date_set_denied: "date -s '2025-01-01'",
        date_set_long_denied: "date --set='2025-01-01'",
        date_set_long_space_denied: "date --set '2025-01-01'",
    }

    safe! {
        route_bare: "route",
        route_n: "route -n",
        route_print: "route print",
        route_get: "route get 8.8.8.8",
        route_show: "route show",
        route_monitor: "route monitor",
        route_dash_v: "route -v",
        route_n_get: "route -n get 8.8.8.8",
        route_4_get: "route -4 get 8.8.8.8",
        ifconfig_bare: "ifconfig",
        ifconfig_iface: "ifconfig eth0",
        ifconfig_lo: "ifconfig lo0",
        ifconfig_all: "ifconfig -a",
        ifconfig_short: "ifconfig -s",
        ifconfig_verbose: "ifconfig -v",
        ifconfig_list: "ifconfig -l",
    }

    denied! {
        route_add_denied: "route add default 192.168.1.1",
        route_del_denied: "route del default",
        route_delete_denied: "route delete default",
        route_change_denied: "route change default 192.168.1.1",
        route_flush_denied: "route flush",
        route_replace_denied: "route replace default via 192.168.1.1",
        route_n_add_denied: "route -n add default 192.168.1.1",
        route_unknown_subcmd_denied: "route foo",
        ifconfig_up_denied: "ifconfig eth0 up",
        ifconfig_down_denied: "ifconfig eth0 down",
        ifconfig_set_ip_denied: "ifconfig eth0 192.168.1.1",
        ifconfig_netmask_denied: "ifconfig eth0 192.168.1.1 netmask 255.255.255.0",
        ifconfig_mtu_denied: "ifconfig eth0 mtu 1500",
        ifconfig_promisc_denied: "ifconfig eth0 promisc",
        ifconfig_unknown_flag_denied: "ifconfig --unknown",
    }

    safe! {
        ls_long: "ls -la",
        ls_human: "ls -lh /tmp",
        ls_color: "ls --color=auto",
        ls_recursive_bare: "ls -R",
        ls_sort: "ls --sort=size",
        ls_bare: "ls",
        eza_long: "eza --long --git",
        eza_tree: "eza --tree",
        delta_files: "delta file1 file2",
        colordiff_unified: "colordiff -u file1 file2",
        dirname_zero: "dirname -z /usr/bin/ls",
        basename_suffix: "basename -s .rs file.rs",
        readlink_canon: "readlink -f /usr/bin/python",
        stat_format: "stat -c '%s' file.txt",
        du_human: "du -sh /tmp",
        du_depth: "du -d 1 .",
        df_human: "df -h",
        df_type: "df -t ext4",
        true_bare: "true",
        false_bare: "false",
        printenv_bare: "printenv",
        printenv_var: "printenv HOME",
        printenv_null: "printenv -0",
        type_cmd: "type ls",
        type_all: "type -a ls",
        whereis_cmd: "whereis ls",
        which_cmd: "which ls",
        which_all: "which -a ls",
        whoami_bare: "whoami",
        pwd_bare: "pwd",
        pwd_logical: "pwd -L",
        cd_dir: "cd /tmp",
        cd_bare: "cd",
        unset_var: "unset FOO",
        unset_func: "unset -f myfunc",
        uname_all: "uname -a",
        uname_machine: "uname -m",
        uname_bare: "uname",
        nproc_bare: "nproc",
        nproc_all: "nproc --all",
        uptime_bare: "uptime",
        uptime_pretty: "uptime -p",
        id_bare: "id",
        id_user: "id -u",
        id_name: "id -un",
        groups_bare: "groups",
        groups_user: "groups root",
        tty_bare: "tty",
        tty_silent: "tty -s",
        locale_bare: "locale",
        locale_all: "locale -a",
        cal_bare: "cal",
        cal_year: "cal -y",
        cal_three: "cal -3",
        sleep_duration: "sleep 1",
        sleep_multiple: "sleep 1s 2s",
        who_bare: "who",
        who_all: "who -a",
        who_am_i: "who am i",
        w_bare: "w",
        w_short: "w -s",
        last_bare: "last",
        last_n: "last -n 5",
        last_numeric: "last -5",
        last_file: "last -f /var/log/wtmp",
        lastlog_bare: "lastlog",
        lastlog_user: "lastlog -u root",
        ps_bare: "ps",
        ps_aux: "ps aux",
        ps_ef: "ps -ef",
        top_batch: "top -bn1",
        htop_bare: "htop",
        iotop_batch: "iotop -b -n 1",
        procs_bare: "procs",
        dust_bare: "dust",
        dust_depth: "dust -d 2",
        lsof_bare: "lsof",
        lsof_port: "lsof -i :8080",
        pgrep_name: "pgrep firefox",
        pgrep_full: "pgrep -f 'python.*server'",
        jq_filter: "jq '.name' file.json",
        jq_compact: "jq -c . file.json",
        jq_raw: "jq -r '.url' file.json",
        jq_slurp: "jq -s '.[0]' file.json",
        base64_decode: "base64 -d file.txt",
        base64_encode: "base64 file.txt",
        xxd_file: "xxd file.bin",
        xxd_bits: "xxd -b file.bin",
        xxd_revert: "xxd -r file.hex",
        getconf_bare: "getconf",
        getconf_var: "getconf PAGE_SIZE",
        uuidgen_bare: "uuidgen",
        uuidgen_random: "uuidgen -r",
        md5sum_file: "md5sum file.txt",
        md5sum_check: "md5sum -c checksums.md5",
        sha256sum_file: "sha256sum file.txt",
        sha1sum_file: "sha1sum file.txt",
        sha512sum_file: "sha512sum file.txt",
        md5_file: "md5 file.txt",
        md5_string: "md5 -s hello",
        shasum_file: "shasum file.txt",
        shasum_algo: "shasum -a 256 file.txt",
        cksum_file: "cksum file.txt",
        b2sum_file: "b2sum file.txt",
        sum_file: "sum file.txt",
        strings_file: "strings binary.exe",
        strings_bytes: "strings -n 8 binary.exe",
        hexdump_file: "hexdump -C file.bin",
        od_file: "od -x file.bin",
        size_file: "size binary.o",
        sw_vers_bare: "sw_vers",
        sw_vers_name: "sw_vers --productName",
        mdls_file: "mdls file.txt",
        mdls_name: "mdls -name kMDItemContentType file.txt",
        otool_headers: "otool -h binary",
        otool_libs: "otool -L binary",
        nm_file: "nm binary.o",
        nm_extern: "nm -g binary.o",
        system_profiler_bare: "system_profiler",
        system_profiler_hw: "system_profiler SPHardwareDataType",
        ioreg_bare: "ioreg",
        ioreg_tree: "ioreg -t",
        vm_stat_bare: "vm_stat",
        vm_stat_interval: "vm_stat 5",
        mdfind_query: "mdfind 'kMDItemContentType == public.image'",
        mdfind_name: "mdfind -name README",
        dig_domain: "dig example.com",
        dig_type: "dig -t MX example.com",
        dig_at_server: "dig @8.8.8.8 example.com",
        nslookup_domain: "nslookup example.com",
        nslookup_server: "nslookup example.com 8.8.8.8",
        nslookup_type: "nslookup -type=MX example.com",
        host_domain: "host example.com",
        host_type: "host -t AAAA example.com",
        whois_domain: "whois example.com",
        netstat_bare: "netstat",
        netstat_listen: "netstat -tlnp",
        netstat_all: "netstat -an",
        ss_bare: "ss",
        ss_listen: "ss -tlnp",
        identify_file: "identify image.png",
        shellcheck_file: "shellcheck script.sh",
        shellcheck_format: "shellcheck -f json script.sh",
        cloc_dir: "cloc src/",
        tokei_bare: "tokei",
        tokei_sort: "tokei -s lines",
        branchdiff_bare: "branchdiff",
    }

    denied! {
        true_with_args_denied: "true --extra",
        false_with_args_denied: "false something",
        whoami_flag_denied: "whoami --unknown",
        ls_unknown_flag_denied: "ls --execute-cmd",
        branchdiff_flag_denied: "branchdiff --unknown",
        ss_kill_denied: "ss --kill",
        ss_kill_short_denied: "ss -K",
        ss_diag_denied: "ss -D /tmp/dump",
        ss_diag_long_denied: "ss --diag=/tmp/dump",
        lastlog_clear_denied: "lastlog -C",
        lastlog_set_denied: "lastlog -S",
        lastlog_clear_long_denied: "lastlog --clear",
        lastlog_set_long_denied: "lastlog --set",
        nslookup_unknown_denied: "nslookup -unknown example.com",
        mdls_plist_denied: "mdls -plist output.plist file.txt",
        sleep_bare_denied: "sleep",
    }
}
