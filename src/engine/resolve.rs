//! The profile resolver — turning a parsed command into its behavior profile
//! (annex `behavioral-taxonomy-engine`). Runs via `engine::bridge` behind
//! `SAFE_CHAINS_ENGINE` (default `legacy`, so it is not authoritative by default).
//!
//! Argument classification is the reusable core: `classify_locus` refines the existing
//! `is_safe_write_target` branch order (`src/cst/check.rs`, a 2-bucket boolean) into
//! the full [`LocalLocus`] ladder (v1.4 §2.2); the per-command resolvers build on it.

use super::facet::*;
use crate::parse::{Token, has_flag};

/// Resolve a command's leaf tokens to its behavior profile, or `None` if the command
/// has no resolver yet (the caller then worst-cases / falls back to the legacy
/// classifier — §0 fail-closed). Redirects, substitutions, and chain semantics are the
/// surrounding CST's job, not this leaf's (annex `…-engine` §1).
pub fn resolve(tokens: &[Token]) -> Option<Profile> {
    let arg0 = tokens.first()?;
    let resolver: fn(&[Token]) -> Profile = match arg0.command_name() {
        "echo" => resolve_echo,
        "cat" => resolve_cat,
        "grep" => resolve_grep,
        "rm" => resolve_rm,
        "mkdir" => resolve_mkdir,
        "touch" => resolve_touch,
        "cp" => resolve_cp,
        _ => return None,
    };
    // A resolvable basename reached via a NON-STANDARD path (`./cat`, `/tmp/cat`,
    // `~/bin/grep`) is not necessarily the real tool — a planted binary named `cat` would
    // be certified as safe coreutils. Don't certify it; worst-case (§0). Bare names and
    // standard bin paths are trusted. (Legacy classifies purely by basename and inherits
    // the spoof; the engine is stricter here, which keeps it never-looser.)
    if !trusted_command_path(arg0.as_str()) {
        return Some(Profile::of(vec![Capability::worst(
            "resolvable name invoked from a non-standard path — possible spoof (§0)",
        )]));
    }
    Some(resolver(tokens))
}

/// A command name with no resolver and no plausible future one — the stable stand-in for
/// "unresearched" across engine tests. Using a real tool here is a trap: when `rm` gained
/// a resolver, three tests that used `rm` as their unresearched example silently broke.
/// A name that will never be a real tool can never be silently repurposed.
#[cfg(test)]
pub(crate) const UNRESOLVED_CMD: &[&str] = &["safe-chains-unresolved-sentinel"];

/// Whether `arg0` is a trusted way to invoke a standard tool: a bare name (found via
/// `$PATH`) or an absolute path under a standard system bin directory. A path elsewhere
/// (`./x`, `/tmp/x`, `~/bin/x`) may be an impostor.
fn trusted_command_path(arg0: &str) -> bool {
    const STD_BINS: &[&str] =
        &["/usr/bin/", "/bin/", "/usr/local/bin/", "/opt/homebrew/bin/", "/sbin/", "/usr/sbin/"];
    !arg0.contains('/') || STD_BINS.iter().any(|p| arg0.starts_with(p))
}

/// `echo` — the reference *structural* certification (§0): every facet is positively
/// safe by the command's form. `echo` writes its literal arguments to stdout and does
/// nothing else — no filesystem, network, execution, secret, or state change — and its
/// only flags (`-n`/`-e`/`-E`) format the output. (A redirect like `echo x > f` or a
/// substitution like `echo "$SECRET"` is a *separate* capability the enclosing CST
/// resolves; this leaf is `echo`'s intrinsic behavior.)
fn resolve_echo(_tokens: &[Token]) -> Profile {
    let mut c = Capability::new(Operation::Observe);
    c.disclosure.audience = DisclosureAudience::LocalProcess; // its output reaches the model
    c.because = "echo prints its arguments to stdout; no fs/net/exec/secret".to_string();
    Profile::of(vec![c])
}

/// `cat FILE…` — reads each file's content to stdout (→ the model). Positive
/// certification (§0): `operation = observe`, `secret = none` (a byte-reader extracts no
/// credential — the sensitivity of the *content* is carried by `locus` + `disclosure`,
/// not by detecting a secret path), no network/execution. `locus` per file is
/// `classify_locus` (fail-closed: `$VAR`/`..` → `machine`). `cat`'s flags (`-n`/`-A`/…)
/// only format output and take no values.
fn resolve_cat(tokens: &[Token]) -> Profile {
    const CAT: Flags = Flags {
        short: b"AbeEnstTuv",
        valued_short: &[],
        long: &[
            "--number", "--number-nonblank", "--show-all", "--show-ends", "--show-nonprinting",
            "--show-tabs", "--squeeze-blank", "--help", "--version",
        ],
        valued_long: &[],
    };
    let Some(files) = CAT.positionals(tokens) else {
        return Profile::of(vec![Capability::worst("cat: unrecognized flag — worst-cased (§0)")]);
    };
    Profile::of(reads_to_model(&files, Scale::Single))
}

/// One `observe · content-to-model` capability per path (empty list = reads stdin). A
/// `-` operand is stdin (process-scoped); every other path is placed by `classify_locus`.
fn reads_to_model(paths: &[&str], scale: Scale) -> Vec<Capability> {
    if paths.is_empty() {
        return vec![reads_content(LocalLocus::Process, scale, "reads stdin")];
    }
    paths
        .iter()
        .map(|p| {
            if *p == "-" {
                reads_content(LocalLocus::Process, scale, "reads stdin (-)")
            } else {
                reads_content(classify_locus(p), scale, "reads file content to the model")
            }
        })
        .collect()
}

fn reads_content(locus: LocalLocus, scale: Scale, because: &str) -> Capability {
    let mut c = Capability::new(Operation::Observe);
    c.locus.local = locus;
    c.scale = scale;
    c.disclosure.audience = DisclosureAudience::LocalProcess; // content → the model
    c.because = because.to_string();
    c
}

/// `grep PATTERN FILE…` — searches files and prints matching lines (file content) to the
/// model. Like `cat` for its file operands, with three grep-specific twists: the first
/// positional is the *pattern* (not a file) unless `-e`/`-f` supplied it; `-f FILE` names
/// a pattern file grep also *reads*; and `-r`/`-R` searches recursively (`scale =
/// unbounded`). Same positive certification as `cat` (observe, `secret = none`, no
/// net/exec); `locus` per read is `classify_locus`.
fn resolve_grep(tokens: &[Token]) -> Profile {
    // `-r` (or --recursive); `-R`/--dereference-recursive is not benign and worst-cases
    // in the walk below, so it needn't be detected here.
    let recursive = has_flag(tokens, Some("-r"), Some("--recursive"));
    let scale = if recursive { Scale::Unbounded } else { Scale::Single };

    let mut files = Vec::new(); // positional file operands
    let mut pattern_files = Vec::new(); // -f/--file pattern files grep reads
    let mut pattern_from_flag = false;
    let mut unknown_flag = false;
    let mut flags_done = false;
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        let next = tokens.get(i + 1).map(Token::as_str);
        if !flags_done && t == "--" {
            flags_done = true;
            i += 1;
        } else if flags_done || !t.starts_with('-') || t == "-" {
            files.push(t);
            i += 1;
        } else if t.starts_with("--") {
            if let Some(v) = t.strip_prefix("--file=") {
                pattern_from_flag = true;
                pattern_files.push(v);
                i += 1;
            } else if t == "--file" {
                pattern_from_flag = true;
                pattern_files.extend(next);
                i += 2;
            } else if t == "--regexp" {
                pattern_from_flag = true;
                i += 2;
            } else if t.starts_with("--regexp=") {
                pattern_from_flag = true;
                i += 1;
            } else {
                unknown_flag |= !grep_long_known(t);
                i += 1;
            }
        } else {
            match grep_short_cluster(t, next) {
                GrepShort::Unrecognized => {
                    unknown_flag = true;
                    i += 1;
                }
                GrepShort::Standalone => i += 1,
                GrepShort::Pattern { file, consumes_next } => {
                    pattern_files.extend(file);
                    pattern_from_flag = true;
                    i += if consumes_next { 2 } else { 1 };
                }
                GrepShort::SkipValue { consumes_next } => i += if consumes_next { 2 } else { 1 },
            }
        }
    }

    if unknown_flag {
        return Profile::of(vec![Capability::worst("grep: unrecognized flag — worst-cased (§0)")]);
    }
    if files.is_empty() {
        // No positional operand → grep has no pattern (a `-e`/`-f` pattern still needs a
        // search target). This is a usage error; the legacy classifier denies it, so the
        // engine must not be looser — worst-case (§0).
        return Profile::of(vec![Capability::worst("grep: no pattern operand — worst-cased (§0)")]);
    }

    if !pattern_from_flag {
        files.remove(0); // the first positional is the PATTERN, not a file
    }
    if recursive && files.is_empty() {
        files.push("."); // grep -r with no path searches the cwd
    }

    let mut caps: Vec<Capability> = pattern_files
        .iter()
        .map(|f| reads_content(classify_locus(f), Scale::Single, "reads a grep -f pattern file"))
        .collect();
    caps.extend(reads_to_model(&files, scale));
    Profile::of(caps)
}

/// The outcome of parsing one grep short-option cluster.
enum GrepShort<'a> {
    /// An unrecognized short (e.g. `-P`, code-executing PCRE) → the caller worst-cases.
    Unrecognized,
    /// All chars benign; no value taken.
    Standalone,
    /// `-e`/`-f` supplied the pattern (so positionals are files); `-f`'s value, if any,
    /// is a pattern file grep reads.
    Pattern { file: Option<&'a str>, consumes_next: bool },
    /// `-m`/`-A`/`-B`/`-C`/`-d` — a count/action value to skip.
    SkipValue { consumes_next: bool },
}

/// Parse a grep short-option cluster (e.g. `-ifpatterns`), honoring GNU semantics that a
/// value-taking short consumes the rest of its cluster (glued) or the next token.
fn grep_short_cluster<'a>(cluster: &'a str, next: Option<&'a str>) -> GrepShort<'a> {
    // NB: `r` (recursive) is benign, but `R` (--dereference-recursive) follows symlinks
    // and can escape the classified locus, so it is NOT benign — it worst-cases.
    const BENIGN: &[u8] = b"ivnclLoqswxHhaIrzZEFGbU";
    let bytes = cluster.as_bytes();
    let mut k = 1;
    while k < bytes.len() {
        let glued = &cluster[k + 1..];
        let has = !glued.is_empty();
        match bytes[k] {
            b'f' => {
                let file = if has { Some(glued) } else { next };
                return GrepShort::Pattern { file, consumes_next: !has };
            }
            b'e' => return GrepShort::Pattern { file: None, consumes_next: !has },
            b'm' | b'A' | b'B' | b'C' | b'd' => return GrepShort::SkipValue { consumes_next: !has },
            b if BENIGN.contains(&b) => k += 1,
            _ => return GrepShort::Unrecognized,
        }
    }
    GrepShort::Standalone
}

/// Whether a grep long flag (its `--name`, ignoring any `=value`) is recognized-benign.
/// `--perl-regexp` and anything unlisted are not → worst-case (§0).
fn grep_long_known(flag: &str) -> bool {
    const KNOWN: &[&str] = &[
        "--recursive", "--ignore-case", "--invert-match", // NB: --dereference-recursive
        // (symlink-following) is intentionally absent → worst-case (M2)
        "--line-number", "--count", "--files-with-matches", "--files-without-match",
        "--only-matching", "--word-regexp", "--line-regexp", "--fixed-strings",
        "--extended-regexp", "--basic-regexp", "--with-filename", "--no-filename",
        "--quiet", "--silent", "--no-messages", "--null", "--byte-offset", "--text",
        "--color", "--colour", "--help", "--version", "--after-context", "--before-context",
        "--context", "--max-count", "--include", "--exclude", "--exclude-dir",
        "--include-dir", "--binary-files", "--devices", "--directories",
    ];
    let name = flag.split('=').next().unwrap_or(flag);
    KNOWN.contains(&name)
}

/// `rm FILE…` — deletes files. Positive certification (§0): `operation = destroy`, no
/// network/execution/secret. `locus` per operand is `classify_locus`; `-r`/`-R` recurse
/// (`scale = unbounded`). `reversibility = effortful` — a delete is recoverable only from
/// out-of-band backups; we do NOT assume VCS/trash (fail-closed). NB: `-f` (--force) only
/// suppresses prompts — it does NOT raise reversibility for `rm` (the danger of `rm -rf`
/// is the *recursion*, not the force), so the generic "--force → irreversible" modifier
/// is command-specific, not universal.
fn resolve_rm(tokens: &[Token]) -> Profile {
    // `--no-preserve-root` (which enables `rm -rf /`) is intentionally absent → worst-case.
    const RM: Flags = Flags {
        short: b"fiIrRdv",
        valued_short: &[],
        long: &[
            "--force", "--interactive", "--recursive", "--dir", "--verbose",
            "--one-file-system", "--preserve-root", "--help", "--version",
        ],
        valued_long: &[],
    };
    let Some(operands) = RM.positionals(tokens) else {
        return Profile::of(vec![Capability::worst("rm: unrecognized/dangerous flag — worst-cased (§0)")]);
    };
    if operands.is_empty() {
        return Profile::of(vec![Capability::worst("rm: no operand — worst-cased (§0)")]);
    }
    let recursive = has_flag(tokens, Some("-r"), Some("--recursive")) || has_flag(tokens, Some("-R"), None);
    let scale = breadth_scale(&operands, recursive);
    Profile::of(operands.iter().map(|p| destroys(classify_locus(p), scale)).collect())
}

fn destroys(locus: LocalLocus, scale: Scale) -> Capability {
    writes(
        Operation::Destroy,
        locus,
        scale,
        Reversibility::Effortful,        // recoverable only from backups (fail-closed)
        PersistenceLevel::Transient,     // a delete leaves nothing behind
        "rm deletes files (recoverable only from out-of-band backups)",
    )
}

/// A local write capability (the create/mutate/destroy family), placed at `locus`. The
/// caller supplies the reversibility and persistence that fit the operation — `trivial` +
/// `data` for a fresh mkdir/touch, `effortful` + `transient` for a delete, `effortful` +
/// `data` for a clobbering copy.
fn writes(
    op: Operation,
    locus: LocalLocus,
    scale: Scale,
    reversibility: Reversibility,
    persistence: PersistenceLevel,
    because: &str,
) -> Capability {
    let mut c = Capability::new(op);
    c.locus.local = locus;
    c.scale = scale;
    c.reversibility = reversibility;
    c.persistence.level = persistence;
    c.because = because.to_string();
    c
}

/// Breadth of a filesystem effect: `unbounded` when recursing, `bounded` for a glob or
/// several operands, else `single`. Shared by rm/mkdir/touch/cp.
fn breadth_scale(operands: &[&str], recursive: bool) -> Scale {
    if recursive {
        Scale::Unbounded
    } else if operands.len() > 1 || operands.iter().any(|p| p.contains(['*', '?', '['])) {
        Scale::Bounded
    } else {
        Scale::Single
    }
}

/// Whether every flag (up to `--`) is recognized. `valued` flags (matched as whole tokens,
/// short or long) consume their following token as a value — skipped so a dash-leading
/// `mkdir DIR…` — creates directories. Non-destructive (fails on an existing target; `-p`
/// is idempotent), so reversibility is `trivial` — a fresh empty dir is `rmdir`-removable.
/// `-m`/`--mode`/`--context` take a value; `locus` per operand is `classify_locus`.
fn resolve_mkdir(tokens: &[Token]) -> Profile {
    const MKDIR: Flags = Flags {
        short: b"pvZ",
        valued_short: b"m",
        long: &["--parents", "--verbose", "--help", "--version"],
        valued_long: &["--mode", "--context"],
    };
    let Some(dirs) = MKDIR.positionals(tokens) else {
        return Profile::of(vec![Capability::worst("mkdir: unrecognized flag — worst-cased (§0)")]);
    };
    if dirs.is_empty() {
        return Profile::of(vec![Capability::worst("mkdir: no operand — worst-cased (§0)")]);
    }
    let scale = breadth_scale(&dirs, false);
    Profile::of(
        dirs.iter()
            .map(|p| {
                writes(
                    Operation::Create,
                    classify_locus(p),
                    scale,
                    Reversibility::Trivial,
                    PersistenceLevel::Data,
                    "mkdir creates a directory",
                )
            })
            .collect(),
    )
}

/// `touch FILE…` — creates empty files or bumps timestamps. It never destroys content, so
/// reversibility is `trivial`. `operation = create` covers both the create and the
/// mtime-mutate case (both admit at `write-local`, so the create/mutate ambiguity a static
/// classifier can't resolve is verdict-irrelevant). `-r`/`-t`/`-d` take a value; the `-r`
/// reference is only an mtime read (metadata, not content) and carries no read capability.
fn resolve_touch(tokens: &[Token]) -> Profile {
    const TOUCH: Flags = Flags {
        short: b"Aacmh",
        valued_short: b"rtd",
        long: &["--no-create", "--no-dereference", "--help", "--version"],
        valued_long: &["--reference", "--date", "--time"],
    };
    let Some(files) = TOUCH.positionals(tokens) else {
        return Profile::of(vec![Capability::worst("touch: unrecognized flag — worst-cased (§0)")]);
    };
    if files.is_empty() {
        return Profile::of(vec![Capability::worst("touch: no operand — worst-cased (§0)")]);
    }
    let scale = breadth_scale(&files, false);
    Profile::of(
        files
            .iter()
            .map(|p| {
                writes(
                    Operation::Create,
                    classify_locus(p),
                    scale,
                    Reversibility::Trivial,
                    PersistenceLevel::Data,
                    "touch creates a file or updates its timestamp",
                )
            })
            .collect(),
    )
}

/// `cp SRC… DEST` — the first resolver that both READS and WRITES, at potentially
/// different loci. Each source is an `observe` at its own locus (content flows file→file,
/// NOT to the model — no `local-process` disclosure); the destination is a `create` at its
/// locus. Two things fall out of the locus model with no secret detection: `cp
/// ~/.ssh/id_rsa ./x` denies because the SOURCE read is at `user` (above read-local), and
/// `cp ./x ~/y` denies because the DEST write is at `user` (above write-local). The
/// destination may silently overwrite existing content, so its reversibility is `effortful`
/// (→ `developer`) — UNLESS `-n`/`--no-clobber` guarantees no overwrite, which drops it to
/// `trivial` (→ `write-local`). That flag is the safe-form remediation, the same shape as
/// `--ignore-scripts`.
fn resolve_cp(tokens: &[Token]) -> Profile {
    const CP: Flags = Flags {
        short: b"HLNPRXacdfhilnprsuvx",
        valued_short: b"tS",
        long: &[
            "--archive", "--force", "--help", "--interactive", "--no-clobber", "--no-dereference",
            "--no-target-directory", "--one-file-system", "--parents", "--recursive",
            "--remove-destination", "--symbolic-link", "--update", "--verbose", "--version",
        ],
        valued_long: &["--backup", "--preserve", "--reflink", "--sparse", "--suffix", "--target-directory"],
    };
    let Some(operands) = CP.positionals(tokens) else {
        return Profile::of(vec![Capability::worst("cp: unrecognized flag — worst-cased (§0)")]);
    };
    // With -t/--target-directory the dest is the flag's value and every operand is a
    // source; otherwise the last operand is the dest and the rest are sources.
    let (sources, dest): (&[&str], &str) = match CP.value(tokens, b't', "--target-directory") {
        Some(d) => (operands.as_slice(), d),
        None => match operands.split_last() {
            Some((last, rest)) if !rest.is_empty() => (rest, *last),
            _ => return Profile::of(vec![Capability::worst("cp: needs a source and a destination — worst-cased (§0)")]),
        },
    };
    let recursive = has_flag(tokens, Some("-r"), Some("--recursive"))
        || has_flag(tokens, Some("-R"), None)
        || has_flag(tokens, Some("-a"), Some("--archive"));
    let no_clobber = has_flag(tokens, Some("-n"), Some("--no-clobber"));
    let scale = breadth_scale(sources, recursive);
    let dest_rev = if no_clobber { Reversibility::Trivial } else { Reversibility::Effortful };

    let mut caps: Vec<Capability> = sources.iter().map(|s| copies_source(classify_locus(s), scale)).collect();
    caps.push(writes(
        Operation::Create,
        classify_locus(dest),
        scale,
        dest_rev,
        PersistenceLevel::Data,
        "cp writes the destination; may overwrite existing content unless --no-clobber",
    ));
    Profile::of(caps)
}

/// A `cp` source read: `observe` at the source locus, but with NO `local-process`
/// disclosure — the bytes are copied to a file, not printed to the model.
fn copies_source(locus: LocalLocus, scale: Scale) -> Capability {
    let mut c = Capability::new(Operation::Observe);
    c.locus.local = locus;
    c.scale = scale;
    c.because = "cp reads the source file".to_string();
    c
}

/// A getopt-style flag spec for a fixed-flag-set command. `short` flags are single chars
/// that cluster (`-rf`); `valued_*` flags take a value, either glued (`-tDIR`, `--dir=X`)
/// or as the next token (`-t DIR`, `--dir X`). A valued short ends its cluster and takes
/// the glued remainder or the next token. (`grep` keeps its own parser — its shorts carry
/// richer semantics: a value-short can supply a *pattern* to read vs a count to skip.)
struct Flags {
    short: &'static [u8],
    valued_short: &'static [u8],
    long: &'static [&'static str],
    valued_long: &'static [&'static str],
}

enum FlagKind {
    Unknown,
    Boolean,
    ValuedGlued, // the value is inside this token (`-tX`, `--dir=X`)
    ValuedNext,  // the value is the following token (`-t X`, `--dir X`)
}

impl Flags {
    /// The positional operands up to `--`, or `None` if any flag is unrecognized (the
    /// caller then worst-cases, §0). A valued flag's value is consumed, never returned as a
    /// positional; a bare `-` is a positional (stdin).
    fn positionals<'a>(&self, tokens: &'a [Token]) -> Option<Vec<&'a str>> {
        let mut out = Vec::new();
        let mut flags_done = false;
        let mut i = 1;
        while i < tokens.len() {
            let t = tokens[i].as_str();
            if !flags_done && t == "--" {
                flags_done = true;
            } else if flags_done || t == "-" || !t.starts_with('-') {
                out.push(t);
            } else {
                match self.classify(t) {
                    FlagKind::Unknown => return None,
                    FlagKind::ValuedNext => i += 1, // also skip the value token
                    FlagKind::Boolean | FlagKind::ValuedGlued => {}
                }
            }
            i += 1;
        }
        Some(out)
    }

    /// Classify one flag token against the spec.
    fn classify(&self, t: &str) -> FlagKind {
        if let Some(rest) = t.strip_prefix("--") {
            let name_len = rest.split('=').next().unwrap_or(rest).len();
            let full = &t[..2 + name_len];
            let has_eq = t.len() > 2 + name_len;
            if self.valued_long.contains(&full) {
                return if has_eq { FlagKind::ValuedGlued } else { FlagKind::ValuedNext };
            }
            if self.long.contains(&full) {
                // A boolean long never consumes the NEXT token; a glued `=value` is its
                // optional-argument form (`rm --interactive=always`) — accept and consume
                // just this token.
                return if has_eq { FlagKind::ValuedGlued } else { FlagKind::Boolean };
            }
            return FlagKind::Unknown;
        }
        let bytes = t.as_bytes();
        let mut k = 1;
        while k < bytes.len() {
            let b = bytes[k];
            if self.valued_short.contains(&b) {
                return if k + 1 < bytes.len() { FlagKind::ValuedGlued } else { FlagKind::ValuedNext };
            }
            if self.short.contains(&b) {
                k += 1;
            } else {
                return FlagKind::Unknown;
            }
        }
        FlagKind::Boolean
    }

    /// The value of a specific valued flag (glued or next-token), if present.
    fn value<'a>(&self, tokens: &'a [Token], short: u8, long: &str) -> Option<&'a str> {
        let mut i = 1;
        while i < tokens.len() {
            let t = tokens[i].as_str();
            if t == "--" {
                break;
            }
            if t.starts_with("--") {
                if let Some(v) = t.strip_prefix(long).and_then(|r| r.strip_prefix('=')) {
                    return Some(v);
                }
                if t == long {
                    return tokens.get(i + 1).map(Token::as_str);
                }
            } else if t.starts_with('-') && t != "-" {
                let bytes = t.as_bytes();
                let mut k = 1;
                while k < bytes.len() {
                    let b = bytes[k];
                    if b == short && self.valued_short.contains(&b) {
                        let glued = &t[k + 1..];
                        return if glued.is_empty() { tokens.get(i + 1).map(Token::as_str) } else { Some(glued) };
                    }
                    if self.valued_short.contains(&b) {
                        break; // a different valued short consumes the rest of the cluster
                    }
                    k += 1;
                }
            }
            i += 1;
        }
        None
    }
}

/// The filesystem rung a path reaches (v1.4 §2.2). A value that cannot be pinned —
/// a `$VAR` expansion or a `..` parent-escape — takes the worst-case fs rung
/// (`machine`), matching the allowlist floor `is_safe_write_target` already enforces
/// by denying such targets.
///
/// The same classifier serves reads and writes; the *level* draws the line
/// (`read-local` admits `<= user`, `write-local` admits `<= worktree`), which is the
/// refinement the facet model buys over the old single boolean.
pub fn classify_locus(path: &str) -> LocalLocus {
    // Unpinnable FIRST (§0 fail-closed): a `$VAR` expansion or a `..` escape could name
    // anything, so no positive (lower) classification is sound — not even a `/tmp/`
    // prefix, since `/tmp/$X` can expand through `..` to anywhere. Worst-case to
    // `machine` (the top fs rung; raw devices need an explicit /dev/ match).
    if path.contains('$') || is_parent_escape(path) {
        return LocalLocus::Machine;
    }
    // Standard streams — no real filesystem is touched.
    if matches!(path, "/dev/null" | "/dev/stdout" | "/dev/stderr" | "/dev/tty")
        || path.starts_with("/dev/fd/")
    {
        return LocalLocus::Process;
    }
    // Raw block/char devices — beneath the filesystem (dd of=/dev/rdisk0, /dev/mem).
    if is_raw_device(path) {
        return LocalLocus::Device;
    }
    // Temp — process-scoped scratch.
    if path.starts_with("/tmp/")
        || path.starts_with("/private/tmp/")
        || path.starts_with("/var/tmp/")
    {
        return LocalLocus::Temp;
    }
    // Files another tool auto-executes or trusts (.git/ hooks & config, .envrc).
    if has_trusted_segment(path) {
        return LocalLocus::WorktreeTrusted;
    }
    // The user's own home (`~` or `~/…`). Another user's home (`~name…`) is a different
    // principal → machine, per the `machine` rung's "other users" definition.
    if path == "~" || path.starts_with("~/") {
        return LocalLocus::User;
    }
    if path.starts_with('~') {
        return LocalLocus::Machine;
    }
    // Any other absolute path — /etc, /usr, services, another user's home.
    if path.starts_with('/') {
        return LocalLocus::Machine;
    }
    // A plain relative path inside the working tree.
    LocalLocus::Worktree
}

/// A raw block/char device node — block storage or raw memory/ports, beneath the
/// filesystem (not a standard stream; those are handled first). Curated and
/// conservative; other `/dev/*` nodes fall through to the general `machine` rule.
fn is_raw_device(path: &str) -> bool {
    const DEVICE_PREFIXES: &[&str] = &[
        "/dev/disk", "/dev/rdisk", "/dev/sd", "/dev/nvme", "/dev/hd", "/dev/vd",
        "/dev/mmcblk", "/dev/loop", // block storage
        "/dev/mem", "/dev/kmem", "/dev/port", "/dev/mtd", // raw memory / ports / flash
    ];
    DEVICE_PREFIXES.iter().any(|p| path.starts_with(p))
}

fn is_parent_escape(path: &str) -> bool {
    path == ".." || path.starts_with("../") || path.contains("/../") || path.ends_with("/..")
}

/// Whether any path segment is a directory a tool auto-executes or trusts. Matches
/// today's `is_safe_write_target` (`.git`, `.envrc`); CI-config trees (`.github/`,
/// `.gitlab-ci.yml`) are a future refinement of this set.
fn has_trusted_segment(path: &str) -> bool {
    path.split('/').any(|seg| seg == ".git" || seg == ".envrc")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_streams_are_process_scoped() {
        assert_eq!(classify_locus("/dev/null"), LocalLocus::Process);
        assert_eq!(classify_locus("/dev/stdout"), LocalLocus::Process);
        assert_eq!(classify_locus("/dev/fd/3"), LocalLocus::Process);
    }

    #[test]
    fn raw_devices_are_device_rung() {
        assert_eq!(classify_locus("/dev/rdisk0"), LocalLocus::Device);
        assert_eq!(classify_locus("/dev/sda1"), LocalLocus::Device);
        assert_eq!(classify_locus("/dev/nvme0n1"), LocalLocus::Device);
        assert_eq!(classify_locus("/dev/mem"), LocalLocus::Device, "raw memory");
        assert_eq!(classify_locus("/dev/kmem"), LocalLocus::Device);
    }

    #[test]
    fn temp_paths_are_temp() {
        assert_eq!(classify_locus("/tmp/scratch"), LocalLocus::Temp);
        assert_eq!(classify_locus("/private/tmp/x"), LocalLocus::Temp);
        assert_eq!(classify_locus("/var/tmp/y"), LocalLocus::Temp);
    }

    #[test]
    fn plain_relative_paths_are_worktree() {
        assert_eq!(classify_locus("notes.md"), LocalLocus::Worktree);
        assert_eq!(classify_locus("src/engine/mod.rs"), LocalLocus::Worktree);
        assert_eq!(classify_locus("build/out"), LocalLocus::Worktree);
    }

    #[test]
    fn trusted_dotdirs_are_worktree_trusted() {
        assert_eq!(classify_locus(".git/hooks/pre-commit"), LocalLocus::WorktreeTrusted);
        assert_eq!(classify_locus(".git/config"), LocalLocus::WorktreeTrusted);
        assert_eq!(classify_locus(".envrc"), LocalLocus::WorktreeTrusted);
        assert_eq!(classify_locus("nested/.git/x"), LocalLocus::WorktreeTrusted);
    }

    #[test]
    fn home_paths_are_user() {
        assert_eq!(classify_locus("~/.ssh/id_rsa"), LocalLocus::User);
        assert_eq!(classify_locus("~/.config/foo"), LocalLocus::User);
        assert_eq!(classify_locus("~"), LocalLocus::User);
        assert_eq!(classify_locus("~bob/.ssh/id_rsa"), LocalLocus::Machine, "another user's home");
    }

    #[test]
    fn other_absolute_paths_are_machine() {
        assert_eq!(classify_locus("/etc/hosts"), LocalLocus::Machine);
        assert_eq!(classify_locus("/usr/local/bin/x"), LocalLocus::Machine);
        assert_eq!(classify_locus("/Users/someone/notes"), LocalLocus::Machine);
    }

    #[test]
    fn unresolvable_paths_worst_case_to_machine() {
        assert_eq!(classify_locus("$HOME/.ssh/id_rsa"), LocalLocus::Machine);
        assert_eq!(classify_locus("$OUT/file"), LocalLocus::Machine);
        assert_eq!(classify_locus("../secret"), LocalLocus::Machine);
        assert_eq!(classify_locus("a/../../etc/passwd"), LocalLocus::Machine);
        assert_eq!(classify_locus("dir/.."), LocalLocus::Machine);
    }

    #[test]
    fn an_unpinnable_marker_dominates_every_otherwise_safe_prefix() {
        // conservative: an unpinnable segment can't be trusted, even under a safe prefix
        assert_eq!(classify_locus("build/$ARTIFACT"), LocalLocus::Machine);
        assert_eq!(classify_locus("/tmp/$X"), LocalLocus::Machine, "$ beats the /tmp prefix");
        assert_eq!(classify_locus("/tmp/a/../../etc/passwd"), LocalLocus::Machine, ".. escapes /tmp");
        assert_eq!(classify_locus("/dev/null$"), LocalLocus::Machine, "$ beats /dev/null");
    }

    fn toks(parts: &[&str]) -> Vec<Token> {
        parts.iter().map(|p| Token::from_test(p)).collect()
    }

    fn level(name: &str) -> &'static crate::engine::level::Level {
        crate::engine::authoring::default_levels()
            .iter()
            .find(|l| l.name == name)
            .expect("level exists")
    }

    fn inert() -> &'static crate::engine::level::Level {
        level("inert")
    }

    fn read_local() -> &'static crate::engine::level::Level {
        level("read-local")
    }

    #[test]
    fn echo_resolves_to_a_benign_inert_profile() {
        let p = resolve(&toks(&["echo", "hi"])).expect("echo has a resolver");
        assert_eq!(p.capabilities.len(), 1);
        let c = &p.capabilities[0];
        assert_eq!(c.operation, Operation::Observe);
        assert_eq!(c.locus.local, LocalLocus::Process);
        assert_eq!(c.disclosure.audience, DisclosureAudience::LocalProcess);
        assert!(!c.because.is_empty(), "a structural certification cites its reason");
        // admitted at the *strictest* level — every facet (network/exec/secret/…) is zero
        assert!(inert().admits(&p), "echo is fully certified and inert-safe");
    }

    #[test]
    fn echo_flags_do_not_change_its_profile() {
        let bare = resolve(&toks(&["echo", "hi"])).expect("echo");
        let flagged = resolve(&toks(&["echo", "-n", "-e", "hi"])).expect("echo -n -e");
        assert_eq!(bare, flagged);
        assert!(inert().admits(&flagged));
    }

    #[test]
    fn an_unresearched_command_has_no_resolver() {
        assert!(resolve(&toks(UNRESOLVED_CMD)).is_none(), "unresearched → caller worst-cases");
        assert!(resolve(&[]).is_none(), "empty tokens");
    }

    #[test]
    fn cat_of_a_worktree_file_is_read_local() {
        let p = resolve(&toks(&["cat", "./notes.md"])).expect("cat");
        assert!(read_local().admits(&p), "cat ./notes.md");
        assert!(!inert().admits(&p), "reading a real file is above inert");
    }

    #[test]
    fn cat_beyond_the_worktree_is_denied_by_locus() {
        for path in ["~/.ssh/id_rsa", "/etc/hosts", "$SECRET", "../outside"] {
            let p = resolve(&toks(&["cat", path])).expect("cat");
            assert!(!read_local().admits(&p), "cat {path} is above read-local by locus");
        }
    }

    #[test]
    fn cat_stdin_is_process_scoped() {
        assert!(inert().admits(&resolve(&toks(&["cat"])).expect("cat")), "no operand → stdin");
        assert!(inert().admits(&resolve(&toks(&["cat", "-"])).expect("cat -")), "- → stdin");
    }

    #[test]
    fn cat_reads_every_file_operand_and_one_home_read_sinks_it() {
        let p = resolve(&toks(&["cat", "-n", "a.txt", "src/b.rs"])).expect("cat");
        assert_eq!(p.capabilities.len(), 2, "-n is a flag; two files");
        assert!(read_local().admits(&p), "both worktree");

        let mixed = resolve(&toks(&["cat", "a.txt", "~/.ssh/id_rsa"])).expect("cat");
        assert!(!read_local().admits(&mixed), "one home read sinks the whole profile");
    }

    #[test]
    fn cat_double_dash_treats_the_rest_as_files() {
        let p = resolve(&toks(&["cat", "--", "-n"])).expect("cat");
        assert_eq!(p.capabilities.len(), 1, "-n after -- is a filename");
        assert!(read_local().admits(&p));
    }

    #[test]
    fn grep_reads_its_files_not_the_pattern() {
        let p = resolve(&toks(&["grep", "foo", "file.txt"])).expect("grep");
        assert_eq!(p.capabilities.len(), 1, "the pattern is not a file");
        assert!(read_local().admits(&p));
    }

    #[test]
    fn grep_beyond_the_worktree_is_denied() {
        for args in [
            vec!["grep", "foo", "~/.ssh/config"],
            vec!["grep", "-r", "foo", "~"],
            vec!["grep", "foo", "$DIR"],
        ] {
            let p = resolve(&toks(&args)).expect("grep");
            assert!(!read_local().admits(&p), "{args:?}");
        }
    }

    #[test]
    fn grep_recursive_is_unbounded_and_defaults_to_cwd() {
        let p = resolve(&toks(&["grep", "-r", "foo", "src/"])).expect("grep");
        assert!(p.capabilities.iter().all(|c| c.scale == Scale::Unbounded), "-r → unbounded");
        assert!(read_local().admits(&p), "recursive worktree search");

        let cwd = resolve(&toks(&["grep", "-r", "foo"])).expect("grep");
        assert!(cwd.capabilities.iter().all(|c| c.locus.local == LocalLocus::Worktree), "cwd, not stdin");
        assert!(read_local().admits(&cwd));
    }

    #[test]
    fn grep_e_and_f_supply_the_pattern_so_positionals_are_files() {
        // -e: pattern is the flag's value; file.txt is the only file
        let e = resolve(&toks(&["grep", "-e", "foo", "file.txt"])).expect("grep -e");
        assert_eq!(e.capabilities.len(), 1);
        assert!(read_local().admits(&e));

        // -f: the pattern FILE is itself a read
        let f = resolve(&toks(&["grep", "-f", "patterns.txt", "file.txt"])).expect("grep -f");
        assert_eq!(f.capabilities.len(), 2, "patterns.txt + file.txt");
        assert!(read_local().admits(&f));

        let home = resolve(&toks(&["grep", "-f", "~/.secret-patterns", "file.txt"])).expect("grep -f");
        assert!(!read_local().admits(&home), "a home pattern file is denied by locus");

        // glued short value: -fpatterns.txt and -ifpatterns.txt both name a pattern file
        let glued = resolve(&toks(&["grep", "-fpatterns.txt", "file.txt"])).expect("grep -f glued");
        assert_eq!(glued.capabilities.len(), 2, "glued -f value is still a read");
        let glued_home = resolve(&toks(&["grep", "-if~/.secrets", "x"])).expect("grep -if glued");
        assert!(!read_local().admits(&glued_home), "glued home pattern file denied by locus");
    }

    #[test]
    fn grep_long_flags() {
        // --file / --file= name a pattern file grep also reads (2 caps)
        assert_eq!(resolve(&toks(&["grep", "--file", "p.txt", "f.txt"])).expect("grep").capabilities.len(), 2);
        assert_eq!(resolve(&toks(&["grep", "--file=p.txt", "f.txt"])).expect("grep").capabilities.len(), 2);

        // --regexp supplies the pattern; the positional is the file
        let r = resolve(&toks(&["grep", "--regexp", "foo", "f.txt"])).expect("grep");
        assert_eq!(r.capabilities.len(), 1);
        assert!(read_local().admits(&r));

        // a space-separated long value (`--max-count 5`) is imprecise — `5` is read as a
        // phantom positional — but FAIL-SAFE: still worktree-bounded, admitted at
        // read-local, never looser. (Precise handling needs the TOML flag schema.)
        let m = resolve(&toks(&["grep", "--max-count", "5", "foo", "f.txt"])).expect("grep");
        assert!(read_local().admits(&m), "--max-count 5 is fail-safe (imprecise)");

        // an unknown long flag worst-cases
        let bad = resolve(&toks(&["grep", "--perl-regexp", "foo", "f"])).expect("grep");
        assert!(!read_local().admits(&bad));
    }

    #[test]
    fn grep_stdin_and_standalone_flags() {
        assert!(inert().admits(&resolve(&toks(&["grep", "foo"])).expect("grep")), "no file → stdin");
        let p = resolve(&toks(&["grep", "-i", "-n", "foo", "file.txt"])).expect("grep");
        assert_eq!(p.capabilities.len(), 1, "-i -n standalone; foo pattern; file.txt file");
        assert!(read_local().admits(&p));
    }

    /// The complete resolved capability for a single-capability invocation, with
    /// `because` cleared so the assertion is over the **facets** (not the prose).
    fn one_cap(cmd: &[&str]) -> Capability {
        let p = resolve(&toks(cmd)).expect("resolves");
        assert_eq!(p.capabilities.len(), 1, "{cmd:?} is a single-capability invocation");
        let mut c = p.capabilities[0].clone();
        c.because = String::new();
        c
    }

    /// Golden profiles: assert **every** facet of the resolved capability for
    /// representative invocations. This is the "all facets covered" check (§0) — struct
    /// equality means a facet the resolver forgot (left at a wrong default) or set wrong
    /// fails the test, per command. When commands carry TOML profiles, the expected
    /// profile is derived from the TOML instead of hand-built here.
    #[test]
    fn golden_profiles_cover_every_facet() {
        // echo — the reference `structural` profile: observe, process-scoped, output to
        // the model, and every other axis provably zero.
        let mut echo = Capability::new(Operation::Observe);
        echo.disclosure.audience = DisclosureAudience::LocalProcess;
        assert_eq!(one_cap(&["echo", "hi"]), echo, "echo");

        // cat of a worktree file — observe · worktree · content-to-model.
        let mut cat = Capability::new(Operation::Observe);
        cat.locus.local = LocalLocus::Worktree;
        cat.disclosure.audience = DisclosureAudience::LocalProcess;
        assert_eq!(one_cap(&["cat", "./notes.md"]), cat, "cat ./notes.md");

        // cat of a home file — same, but locus rises to user (the only facet that moves).
        let mut cat_home = cat.clone();
        cat_home.locus.local = LocalLocus::User;
        assert_eq!(one_cap(&["cat", "~/.ssh/id_rsa"]), cat_home, "cat ~/.ssh/id_rsa");

        // grep of a worktree file — like cat, bounded to the single searched file.
        assert_eq!(one_cap(&["grep", "foo", "file.txt"]), cat, "grep foo file.txt");

        // grep -r — the recursive search raises scale to unbounded and nothing else.
        let mut grep_r = cat.clone();
        grep_r.scale = Scale::Unbounded;
        assert_eq!(one_cap(&["grep", "-r", "foo", "src/"]), grep_r, "grep -r foo src/");

        // rm — destroy · worktree · effortful; no net/exec/secret.
        let mut rm = Capability::new(Operation::Destroy);
        rm.locus.local = LocalLocus::Worktree;
        rm.reversibility = Reversibility::Effortful;
        assert_eq!(one_cap(&["rm", "./x"]), rm, "rm ./x");

        // mkdir — create · worktree · trivial · leaves data. A fresh dir is rmdir-removable.
        let mut mkdir = Capability::new(Operation::Create);
        mkdir.locus.local = LocalLocus::Worktree;
        mkdir.reversibility = Reversibility::Trivial;
        mkdir.persistence.level = PersistenceLevel::Data;
        assert_eq!(one_cap(&["mkdir", "./build"]), mkdir, "mkdir ./build");

        // touch — the same create · worktree · trivial · data shape as mkdir.
        assert_eq!(one_cap(&["touch", "./new.txt"]), mkdir, "touch ./new.txt");

        // cp -n ./a ./b — a guaranteed-non-clobbering copy is TWO capabilities:
        // a source read (observe, worktree, NO model disclosure) and a trivial dest create.
        let cp = resolve(&toks(&["cp", "-n", "./a", "./b"])).expect("cp");
        assert_eq!(cp.capabilities.len(), 2, "cp = source read + dest write");
        let mut src = Capability::new(Operation::Observe);
        src.locus.local = LocalLocus::Worktree; // disclosure.audience stays `none`: file→file
        assert_eq!(clear_because(&cp.capabilities[0]), src, "cp source read");
        let mut dst = Capability::new(Operation::Create);
        dst.locus.local = LocalLocus::Worktree;
        dst.reversibility = Reversibility::Trivial; // -n → cannot overwrite
        dst.persistence.level = PersistenceLevel::Data;
        assert_eq!(clear_because(&cp.capabilities[1]), dst, "cp -n dest write");
    }

    fn clear_because(c: &Capability) -> Capability {
        let mut c = c.clone();
        c.because = String::new();
        c
    }

    #[test]
    fn mkdir_creates_in_the_worktree_but_not_beyond_it() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};
        // a fresh dir is a trivial-reversibility create → write-local (SafeWrite)
        for cmd in [vec!["mkdir", "./build"], vec!["mkdir", "-p", "a/b/c"], vec!["mkdir", "-m", "755", "./x"]] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("mkdir")), Verdict::Allowed(SafetyLevel::SafeWrite), "{cmd:?}");
        }
        // outside the worktree → denied by locus
        for cmd in [vec!["mkdir", "/etc/evil"], vec!["mkdir", "~/newdir"], vec!["mkdir", "$HOME/x"]] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("mkdir")), Verdict::Denied, "{cmd:?}");
        }
        // a glued valued short (-m755) and its value must not be read as operands
        let g = resolve(&toks(&["mkdir", "-m755", "./x"])).expect("mkdir");
        assert_eq!(g.capabilities.len(), 1, "-m755 glued: only ./x is an operand");
        assert_eq!(g.capabilities[0].locus.local, LocalLocus::Worktree);
        // fail-closed on an unknown flag / no operand
        assert_eq!(project(&resolve(&toks(&["mkdir", "-Q", "x"])).expect("mkdir")), Verdict::Denied, "unknown flag");
        assert_eq!(project(&resolve(&toks(&["mkdir"])).expect("mkdir")), Verdict::Denied, "no operand");
    }

    #[test]
    fn cp_splits_source_and_dest_loci_and_overwrite_gates_the_level() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};

        // plain cp may clobber the dest → effortful create → developer (SafeWrite, the
        // default level). The -n form cannot clobber → trivial → write-local (also
        // SafeWrite today, but a genuinely lower level).
        assert_eq!(project(&resolve(&toks(&["cp", "./a", "./b"])).expect("cp")), Verdict::Allowed(SafetyLevel::SafeWrite), "cp ./a ./b");
        assert_eq!(project(&resolve(&toks(&["cp", "-n", "./a", "./b"])).expect("cp")), Verdict::Allowed(SafetyLevel::SafeWrite), "cp -n ./a ./b");

        // reading a home/system SOURCE is denied by the source locus — no secret detector,
        // just the read locus (cp can't smuggle ~/.ssh/id_rsa into the worktree).
        assert_eq!(project(&resolve(&toks(&["cp", "~/.ssh/id_rsa", "./x"])).expect("cp")), Verdict::Denied, "home source");
        assert_eq!(project(&resolve(&toks(&["cp", "/etc/shadow", "./x"])).expect("cp")), Verdict::Denied, "system source");
        // writing a home/system DEST is denied by the dest locus.
        assert_eq!(project(&resolve(&toks(&["cp", "./x", "~/backdoor"])).expect("cp")), Verdict::Denied, "home dest");
        assert_eq!(project(&resolve(&toks(&["cp", "./x", "/etc/cron.d/x"])).expect("cp")), Verdict::Denied, "system dest");

        // -t DIR makes every positional a source; the dir is the dest. All three spellings
        // (separate, --long=, and glued short) must parse the same way.
        for form in [
            vec!["cp", "-t", "./dest", "./a", "./b"],
            vec!["cp", "--target-directory=./dest", "./a", "./b"],
            vec!["cp", "-t./dest", "./a", "./b"], // glued short — previously worst-cased
        ] {
            let t = resolve(&toks(&form)).expect("cp -t");
            assert_eq!(t.capabilities.len(), 3, "{form:?}: 2 sources + 1 dest");
            assert_eq!(project(&t), Verdict::Allowed(SafetyLevel::SafeWrite), "{form:?}");
        }
        // a glued -t pointing outside the worktree is still denied by the dest locus.
        assert_eq!(project(&resolve(&toks(&["cp", "-t/etc", "./a"])).expect("cp")), Verdict::Denied, "cp -t/etc");

        // recursion raises scale to unbounded; a lone operand / unknown flag worst-cases.
        assert_eq!(resolve(&toks(&["cp", "-r", "./a", "./b"])).expect("cp").capabilities[0].scale, Scale::Unbounded);
        assert_eq!(project(&resolve(&toks(&["cp", "./only"])).expect("cp")), Verdict::Denied, "no dest");
        assert_eq!(project(&resolve(&toks(&["cp", "-Q", "./a", "./b"])).expect("cp")), Verdict::Denied, "unknown flag");
    }

    #[test]
    fn touch_creates_in_the_worktree_and_skips_valued_flag_values() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};
        for cmd in [
            vec!["touch", "./new.txt"],
            vec!["touch", "-c", "existing"],
            vec!["touch", "-r", "ref.txt", "./out"], // -r consumes ref.txt as its value, not an operand
            vec!["touch", "-d", "-1 day", "./out"],  // dash-leading value must not read as a flag
        ] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("touch")), Verdict::Allowed(SafetyLevel::SafeWrite), "{cmd:?}");
        }
        // the -r reference value is consumed, so `./out` (worktree) is the only operand —
        // a home reference file does NOT drag the whole command to the home locus.
        let p = resolve(&toks(&["touch", "-r", "~/.bashrc", "./out"])).expect("touch");
        assert_eq!(p.capabilities.len(), 1, "only ./out is an operand");
        assert_eq!(p.capabilities[0].locus.local, LocalLocus::Worktree);
        // beyond the worktree, and fail-closed cases
        assert_eq!(project(&resolve(&toks(&["touch", "/etc/x"])).expect("touch")), Verdict::Denied, "system path");
        assert_eq!(project(&resolve(&toks(&["touch", "-Z", "x"])).expect("touch")), Verdict::Denied, "unknown flag");
        assert_eq!(project(&resolve(&toks(&["touch"])).expect("touch")), Verdict::Denied, "no operand");
    }

    #[test]
    fn worst_case_is_denied_even_by_a_permissive_yolo_shaped_level() {
        use crate::engine::level::{Clause, Level, OrdBound};
        // a yolo-shaped level: allow anything local up to `machine`, minus a destroy corner
        let yolo = Level::new("yolo-ish")
            .allowing(Clause {
                local_locus: Some(OrdBound::at_most(LocalLocus::Machine)),
                ..Default::default()
            })
            .denying(Clause {
                operation: Some(vec![Operation::Destroy]),
                reversibility: Some(OrdBound::at_least(Reversibility::Irreversible)),
                ..Default::default()
            });
        let wc = Profile::of(vec![Capability::worst("test")]);
        assert!(!yolo.admits(&wc), "worst_case (locus=kernel) exceeds even a machine-capped allow");
    }

    #[test]
    fn rm_within_the_worktree_projects_to_developer_but_beyond_it_denies() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};
        // `developer` admits destroy WITHIN the worktree (golden-set decision 2), even
        // recursive/effortful; it maps to the legacy SafeWrite ceiling.
        for cmd in [
            vec!["rm", "./stale.log"],
            vec!["rm", "-rf", "./node_modules"],
            vec!["rm", "a", "b", "c"],
            vec!["rm", "--interactive=always", "./x"], // optional-arg long: must not worst-case
        ] {
            let p = resolve(&toks(&cmd)).expect("rm resolves");
            assert!(p.capabilities.iter().all(|c| c.operation == Operation::Destroy), "{cmd:?} destroys");
            assert_eq!(project(&p), Verdict::Allowed(SafetyLevel::SafeWrite), "{cmd:?} → developer");
        }
        // Deletion that reaches beyond the worktree (home/system) is above `developer`,
        // denied by locus — no clause admits a machine/user-scoped destroy.
        for cmd in [vec!["rm", "-rf", "/"], vec!["rm", "-rf", "~/notes"], vec!["rm", "/etc/hosts"]] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("rm")), Verdict::Denied, "{cmd:?} beyond worktree");
        }
    }

    #[test]
    fn rm_flag_and_operand_fail_closed() {
        use crate::engine::bridge::project;
        use crate::verdict::Verdict;
        for cmd in [
            vec!["rm", "--no-preserve-root", "-rf", "/"], // enables rm -rf / → must worst-case
            vec!["rm", "-Z", "x"],                        // unknown flag
            vec!["rm"],                                   // no operand (usage error)
            vec!["./rm", "x"],                            // basename spoof
        ] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("resolves")), Verdict::Denied, "{cmd:?}");
        }
    }

    #[test]
    fn rm_scale_and_force_semantics() {
        let cap = |cmd: &[&str]| resolve(&toks(cmd)).expect("rm").capabilities[0].clone();
        assert_eq!(cap(&["rm", "./x"]).scale, Scale::Single);
        assert_eq!(cap(&["rm", "a", "b"]).scale, Scale::Bounded, "multiple operands");
        assert_eq!(cap(&["rm", "*.log"]).scale, Scale::Bounded, "a glob");
        assert_eq!(cap(&["rm", "-r", "./dir"]).scale, Scale::Unbounded, "recursive");
        // -f only suppresses prompts — it does NOT raise reversibility for rm
        assert_eq!(cap(&["rm", "./x"]).reversibility, Reversibility::Effortful);
        assert_eq!(cap(&["rm", "-f", "./x"]).reversibility, Reversibility::Effortful, "-f is not a raiser");
    }

    #[test]
    fn a_resolvable_name_from_a_non_standard_path_worst_cases() {
        // ./cat, /tmp/cat, ~/bin/grep may be impostors → worst-case, not certified safe
        for cmd in [vec!["./cat", "x"], vec!["/tmp/cat", "x"], vec!["~/bin/grep", "foo", "f"]] {
            let p = resolve(&toks(&cmd)).expect("resolvable name");
            assert!(!read_local().admits(&p), "{cmd:?} from a non-standard path must worst-case");
        }
        // bare names and standard bin paths resolve normally
        assert!(read_local().admits(&resolve(&toks(&["cat", "./notes.md"])).expect("cat")));
        assert!(read_local().admits(&resolve(&toks(&["/usr/bin/cat", "./notes.md"])).expect("cat")));
        // a non-resolvable command from any path → None (the engine doesn't claim it)
        assert!(resolve(&toks(&["/tmp/mytool", "x"])).is_none());
    }

    #[test]
    fn unrecognized_flags_worst_case_fail_closed() {
        for cmd in [
            vec!["cat", "-Z", "./x"],
            vec!["cat", "--wat", "./x"],
            vec!["grep", "-P", "foo", "f"], // PCRE → (?{code}) executes; must not slip through
            vec!["grep", "--perl-regexp", "foo", "f"],
            vec!["grep", "-iP", "foo", "f"], // unknown char inside a cluster
            vec!["grep", "-R", "foo", "dir"], // -R follows symlinks → escapes locus (M2)
        ] {
            let p = resolve(&toks(&cmd)).expect("resolver");
            assert!(!inert().admits(&p) && !read_local().admits(&p), "{cmd:?} must worst-case");
        }
        // recognized-benign flags still resolve normally
        assert!(read_local().admits(&resolve(&toks(&["cat", "-nA", "./x"])).expect("cat")));
        assert!(read_local().admits(&resolve(&toks(&["grep", "-rin", "foo", "src/"])).expect("grep")));
    }

    use proptest::prelude::*;

    proptest! {
        /// Fail-closed (§0): a `$` anywhere forces the worst rung, whatever the rest
        /// looks like — the classifier can never be talked below `machine` by a
        /// safe-looking prefix wrapped around an unpinnable expansion.
        #[test]
        fn a_dollar_anywhere_forces_machine(s in ".{0,30}") {
            prop_assert_eq!(classify_locus(&format!("{s}$")), LocalLocus::Machine);
        }

        /// Fail-closed: a `..` parent-escape forces the worst rung.
        #[test]
        fn a_parent_escape_forces_machine(s in "[a-zA-Z0-9/_]{0,20}") {
            prop_assert_eq!(classify_locus(&format!("{s}/../x")), LocalLocus::Machine);
            prop_assert_eq!(classify_locus(&format!("../{s}")), LocalLocus::Machine);
        }
    }
}
