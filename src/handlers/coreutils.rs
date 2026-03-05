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

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "fd" => Some(is_safe_fd(tokens)),
        "tree" => Some(is_safe_tree(tokens)),
        "file" => Some(is_safe_file_cmd(tokens)),
        "ls" | "eza" | "exa" | "delta" | "colordiff" => Some(true),
        "dirname" | "basename" | "realpath" | "readlink" => Some(true),
        "stat" | "du" | "df" => Some(true),
        "date" => Some(is_safe_date(tokens)),
        "true" | "false" => Some(true),
        "printenv" | "type" | "whereis" | "which" | "whoami" => Some(true),
        "pwd" | "cd" | "unset" => Some(true),
        "uname" | "nproc" | "uptime" | "id" | "groups" => Some(true),
        "tty" | "locale" | "cal" | "sleep" => Some(true),
        "who" | "w" | "last" | "lastlog" => Some(true),
        "ps" | "top" | "htop" | "iotop" | "procs" | "dust" => Some(true),
        "lsof" | "pgrep" => Some(true),
        "jq" | "base64" | "xxd" | "getconf" | "uuidgen" => Some(true),
        "md5sum" | "md5" | "sha256sum" | "shasum" | "sha1sum" => Some(true),
        "sha512sum" | "cksum" | "b2sum" | "sum" => Some(true),
        "strings" | "hexdump" | "od" | "size" => Some(true),
        "sw_vers" | "mdls" | "otool" | "nm" => Some(true),
        "system_profiler" | "ioreg" | "vm_stat" | "mdfind" => Some(true),
        "dig" | "nslookup" | "host" | "whois" => Some(true),
        "netstat" | "ss" => Some(true),
        "ifconfig" => Some(is_safe_ifconfig(tokens)),
        "route" => Some(is_safe_route(tokens)),
        "identify" | "shellcheck" | "cloc" | "tokei" => Some(true),
        "cucumber" | "branchdiff" | "safe-chains" => Some(true),
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

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
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
    ]
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
}
