//! The profile resolver — turning a parsed command into its behavior profile
//! (annex `behavioral-taxonomy-engine`). Runs via `engine::bridge` behind
//! `SAFE_CHAINS_ENGINE` (default `legacy`, so it is not authoritative by default).
//!
//! This file holds the dispatch (`resolve`) and the per-command `resolve_*` functions;
//! the shared toolkit they build on lives in submodules: [`flags`] (the getopt-style
//! flag walker), [`locus`] (`classify_locus` — the [`LocalLocus`] ladder that refines the
//! old `is_safe_write_target` boolean, v1.4 §2.2), and [`capability`] (the builders that
//! stamp out each `Capability` with the facet pairing its operation warrants).

use super::facet::*;
use crate::parse::{Token, has_flag};

mod capability;
mod flags;
mod locus;

use capability::{
    breadth_scale, creates, destroys, observes, overwrites, reads_content, reads_to_model,
    relocates, transfer_profile, worst,
};
use flags::Flags;
use locus::classify_locus;

/// Resolve a command's leaf tokens to its behavior profile, or `None` if the command
/// has no resolver yet (the caller then worst-cases / falls back to the legacy
/// classifier — §0 fail-closed). Redirects, substitutions, and chain semantics are the
/// surrounding CST's job, not this leaf's (annex `…-engine` §1).
pub fn resolve(tokens: &[Token]) -> Option<Profile> {
    let arg0 = tokens.first()?;
    let (_, resolver, _) = RESOLVERS.iter().find(|(name, _, _)| *name == arg0.command_name())?;
    // A resolvable basename reached via a NON-STANDARD path (`./cat`, `/tmp/cat`,
    // `~/bin/grep`) is not necessarily the real tool — a planted binary named `cat` would
    // be certified as safe coreutils. Don't certify it; worst-case (§0). Bare names and
    // standard bin paths are trusted. (Legacy classifies purely by basename and inherits
    // the spoof; the engine is stricter here, which keeps it never-looser.)
    if !trusted_command_path(arg0.as_str()) {
        return Some(worst("resolvable name invoked from a non-standard path — possible spoof (§0)"));
    }
    Some(resolver(tokens))
}

/// A per-command resolver: leaf tokens → behavior profile.
type Resolver = fn(&[Token]) -> Profile;

/// A resolver's operand contract — which POSITIONAL slots (after flag parsing) are touched
/// paths. Declared beside each resolver so the conservation sweep (HP-18 rung 3,
/// `every_touched_path_operand_is_gated`) derives its probes from one source of truth,
/// rather than a parallel test table that could drift from the resolvers.
#[derive(Clone, Copy)]
enum Operands {
    /// No path operands — only literal args (`echo`).
    None,
    /// Every positional is a touched path — cat/head/tail/wc/rm/mkdir/touch.
    Paths,
    /// Positional 0 is a pattern; the rest are touched paths (`grep`).
    PatternThenPaths,
    /// Sources… then a destination, every positional a touched path — cp/mv/ln.
    Transfer,
}

/// The dispatch table: every resolvable command, its resolver, and its operand contract. A
/// data table (rather than a `match`) so the conservation sweep enumerates the full set and
/// derives a probe per touched slot — adding a resolver means declaring its [`Operands`]
/// here, and the sweep covers it automatically.
const RESOLVERS: &[(&str, Resolver, Operands)] = &[
    ("echo", resolve_echo, Operands::None),
    ("cat", resolve_cat, Operands::Paths),
    ("head", resolve_head, Operands::Paths),
    ("tail", resolve_tail, Operands::Paths),
    ("wc", resolve_wc, Operands::Paths),
    ("grep", resolve_grep, Operands::PatternThenPaths),
    ("rm", resolve_rm, Operands::Paths),
    ("mkdir", resolve_mkdir, Operands::Paths),
    ("touch", resolve_touch, Operands::Paths),
    ("cp", resolve_cp, Operands::Transfer),
    ("mv", resolve_mv, Operands::Transfer),
    ("ln", resolve_ln, Operands::Transfer),
];

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
        numeric_shorthand: false,
    };
    let Some(files) = CAT.positionals(tokens) else {
        return worst("cat: unrecognized flag — worst-cased (§0)");
    };
    Profile::of(reads_to_model(&files, Scale::Single))
}

/// `head FILE…` / `tail FILE…` — read a bounded prefix/suffix of each file to the model.
/// Same `observe · content-to-model` shape as `cat`; the only wrinkles are the value flags
/// (`-n`/`-c`) and the obsolete `-NUM` count form (`head -20`), which `numeric_shorthand`
/// recognizes. `tail -f`/`-F` follows appends — a streaming read of the *same* file's
/// content, so it carries no extra capability (the locus already bounds what it can see).
fn resolve_head(tokens: &[Token]) -> Profile {
    const HEAD: Flags = Flags {
        short: b"Vhqvz",
        valued_short: b"cn",
        long: &["--help", "--quiet", "--silent", "--verbose", "--version", "--zero-terminated"],
        valued_long: &["--bytes", "--lines"],
        numeric_shorthand: true,
    };
    let Some(files) = HEAD.positionals(tokens) else {
        return worst("head: unrecognized flag — worst-cased (§0)");
    };
    Profile::of(reads_to_model(&files, Scale::Single))
}

fn resolve_tail(tokens: &[Token]) -> Profile {
    const TAIL: Flags = Flags {
        short: b"FVfhqrvz",
        valued_short: b"bcn",
        long: &[
            "--follow", "--help", "--quiet", "--retry", "--silent", "--verbose", "--version",
            "--zero-terminated",
        ],
        valued_long: &["--bytes", "--lines", "--max-unchanged-stats", "--pid", "--sleep-interval"],
        numeric_shorthand: true,
    };
    let Some(files) = TAIL.positionals(tokens) else {
        return worst("tail: unrecognized flag — worst-cased (§0)");
    };
    Profile::of(reads_to_model(&files, Scale::Single))
}

/// `wc FILE…` — reads each file's content (to count lines/words/bytes) and prints the
/// counts. It reads the same content `cat` does, so it is gated identically by `locus`
/// (only the *output* differs — counts, not content — which no facet distinguishes).
/// `--files0-from=F` reads an arbitrary, unpinnable set of paths listed in `F` (or stdin
/// with `-`), which cannot be classified → worst-case (§0).
fn resolve_wc(tokens: &[Token]) -> Profile {
    const WC: Flags = Flags {
        short: b"LVclmw",
        valued_short: &[],
        long: &[
            "--bytes", "--chars", "--help", "--lines", "--max-line-length", "--version",
            "--words", "--zero-terminated",
        ],
        valued_long: &["--files0-from"],
        numeric_shorthand: false,
    };
    if WC.value(tokens, 0, "--files0-from").is_some() {
        return worst("wc --files0-from reads an unpinnable set of files — worst-cased (§0)");
    }
    let Some(files) = WC.positionals(tokens) else {
        return worst("wc: unrecognized flag — worst-cased (§0)");
    };
    Profile::of(reads_to_model(&files, Scale::Single))
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
        return worst("grep: unrecognized flag — worst-cased (§0)");
    }
    if files.is_empty() {
        // No positional operand → grep has no pattern (a `-e`/`-f` pattern still needs a
        // search target). This is a usage error; the legacy classifier denies it, so the
        // engine must not be looser — worst-case (§0).
        return worst("grep: no pattern operand — worst-cased (§0)");
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
        numeric_shorthand: false,
    };
    let Some(operands) = RM.positionals(tokens) else {
        return worst("rm: unrecognized/dangerous flag — worst-cased (§0)");
    };
    if operands.is_empty() {
        return worst("rm: no operand — worst-cased (§0)");
    }
    let recursive = has_flag(tokens, Some("-r"), Some("--recursive")) || has_flag(tokens, Some("-R"), None);
    let scale = breadth_scale(&operands, recursive);
    Profile::of(operands.iter().map(|p| destroys(classify_locus(p), scale)).collect())
}

/// `mkdir DIR…` — creates directories. Non-destructive (fails on an existing target; `-p`
/// is idempotent), so reversibility is `trivial` — a fresh empty dir is `rmdir`-removable.
/// `-m`/`--mode`/`--context` take a value; `locus` per operand is `classify_locus`.
fn resolve_mkdir(tokens: &[Token]) -> Profile {
    const MKDIR: Flags = Flags {
        short: b"pvZ",
        valued_short: b"m",
        long: &["--parents", "--verbose", "--help", "--version"],
        valued_long: &["--mode", "--context"],
        numeric_shorthand: false,
    };
    let Some(dirs) = MKDIR.positionals(tokens) else {
        return worst("mkdir: unrecognized flag — worst-cased (§0)");
    };
    if dirs.is_empty() {
        return worst("mkdir: no operand — worst-cased (§0)");
    }
    let scale = breadth_scale(&dirs, false);
    Profile::of(dirs.iter().map(|p| creates(classify_locus(p), scale)).collect())
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
        numeric_shorthand: false,
    };
    let Some(files) = TOUCH.positionals(tokens) else {
        return worst("touch: unrecognized flag — worst-cased (§0)");
    };
    if files.is_empty() {
        return worst("touch: no operand — worst-cased (§0)");
    }
    let scale = breadth_scale(&files, false);
    Profile::of(files.iter().map(|p| creates(classify_locus(p), scale)).collect())
}

/// `cp SRC… DEST` — the first resolver that both READS and WRITES, at potentially
/// different loci. Each source is an `observe` at its own locus (content flows file→file,
/// NOT to the model — no `local-process` disclosure); the destination is a `create` at its
/// locus. Two things fall out of the locus model with no secret detection: `cp
/// ~/.ssh/id_rsa ./x` denies because the SOURCE read is at `user` (above read-local), and
/// `cp ./x ~/y` denies because the DEST write is at `user` (above write-local). Overwriting
/// a worktree file is `recoverable` (the repo-recoverable assumption, HP-8) — the same
/// write-local treatment the golden-set gives `echo > config.json`; `-n`/`--no-clobber`
/// cannot overwrite at all, so it is `trivial`. Both land at write-local: unlike a delete,
/// a copy is create/overwrite, not destroy.
fn resolve_cp(tokens: &[Token]) -> Profile {
    const CP: Flags = Flags {
        short: b"HLNPRXacdfhilnprsuvx",
        valued_short: b"tS",
        // `--backup`/`--preserve`/`--reflink`/`--sparse` take OPTIONAL args (`--backup[=X]`)
        // — only ever glued, never the next token — so they are boolean longs (a glued
        // `=value` is tolerated by `classify`), not `valued_long`.
        long: &[
            "--archive", "--backup", "--force", "--help", "--interactive", "--no-clobber",
            "--no-dereference", "--no-target-directory", "--one-file-system", "--parents",
            "--preserve", "--recursive", "--reflink", "--remove-destination", "--sparse",
            "--symbolic-link", "--update", "--verbose", "--version",
        ],
        valued_long: &["--suffix", "--target-directory"],
        numeric_shorthand: false,
    };
    let (sources, dest) = match sources_and_dest(&CP, tokens, "cp") {
        Ok(sd) => sd,
        Err(profile) => return profile,
    };
    let recursive = has_flag(tokens, Some("-r"), Some("--recursive"))
        || has_flag(tokens, Some("-R"), None)
        || has_flag(tokens, Some("-a"), Some("--archive"));
    let no_clobber = has_flag(tokens, Some("-n"), Some("--no-clobber"));
    let scale = breadth_scale(&sources, recursive);
    transfer_profile(
        &sources,
        dest,
        scale,
        |loc, sc| observes(loc, sc, "cp reads the source file"),
        |loc, sc| overwrites(loc, sc, no_clobber),
    )
}

/// Split a `SRC… DEST` invocation (`cp`/`mv`) into its sources and destination.
/// `-t`/`--target-directory` names the dest explicitly (every operand is then a source);
/// otherwise the last operand is the dest and the rest are sources. Fails closed (§0),
/// returning the ready-to-return worst-case `Profile`, on an unrecognized flag or a missing
/// source+dest pair.
fn sources_and_dest<'a>(
    flags: &Flags,
    tokens: &'a [Token],
    cmd: &str,
) -> Result<(Vec<&'a str>, &'a str), Profile> {
    let Some(operands) = flags.positionals(tokens) else {
        return Err(worst(&format!("{cmd}: unrecognized flag — worst-cased (§0)")));
    };
    if let Some(dest) = flags.value(tokens, b't', "--target-directory") {
        return Ok((operands, dest));
    }
    match operands.split_last() {
        Some((last, rest)) if !rest.is_empty() => Ok((rest.to_vec(), *last)),
        _ => Err(worst(&format!("{cmd}: needs a source and a destination — worst-cased (§0)"))),
    }
}

/// `mv SRC… DEST` — `cp` + `rm` fused, but the source side is the interesting difference:
/// `mv` *relocates* a file, it does not annihilate it. So the source is a **`mutate`** at
/// its locus (the entry leaves that directory) with **`trivial`** reversibility — `mv` it
/// back — NOT `rm`'s `effortful` `destroy`. That single facet keeps `mv ./a ./b` at
/// write-local while `rm ./a` waits for developer. Both operands are gated by their locus:
/// `mv ~/x ./y` denies on the source write (`user`), `mv ./x ~/y` on the dest write, and
/// `mv .git/config ./x` denies on the source (worktree-*trusted*, above write-local) even
/// though `cp .git/config ./x` is allowed — moving a trusted file mutates `.git`, copying
/// only reads it. The dest is a `create`/overwrite exactly like `cp` (recoverable, or
/// `trivial` under `-n`). `mv` has no recursion flag — a directory move is one rename.
fn resolve_mv(tokens: &[Token]) -> Profile {
    const MV: Flags = Flags {
        short: b"Tfhinuv",
        valued_short: b"tS",
        // `--backup` takes an optional arg (glued only) → boolean long, not valued_long.
        long: &[
            "--backup", "--force", "--help", "--interactive", "--no-clobber",
            "--no-target-directory", "--strip-trailing-slashes", "--update", "--verbose",
            "--version",
        ],
        valued_long: &["--suffix", "--target-directory"],
        numeric_shorthand: false,
    };
    let (sources, dest) = match sources_and_dest(&MV, tokens, "mv") {
        Ok(sd) => sd,
        Err(profile) => return profile,
    };
    let no_clobber = has_flag(tokens, Some("-n"), Some("--no-clobber"));
    let scale = breadth_scale(&sources, false); // mv has no recursion flag; a dir move is one rename
    transfer_profile(&sources, dest, scale, relocates, |loc, sc| overwrites(loc, sc, no_clobber))
}

/// `ln TARGET… LINK` — creates a link (hard, or symbolic with `-s`). It is **cp
/// by reference**: the link makes the target's content reachable at the LINK's locus, so a
/// link to a home/system target is the same bridge a `cp` of it would be. To stop `ln`
/// being a `cp`-bypass, the target is gated on its own locus exactly like `cp`'s source
/// (`observes`); `ln ~/.ssh/id_rsa ./x` denies just as `cp` does. The link itself is a
/// `create`/overwrite at the LINK's locus — `trivial` (rm the link), or `recoverable` when
/// `-f` clobbers an existing entry. Same `SRC… DEST` operand shape as `cp`/`mv`.
///
/// The target string is NOT followed (§0.2 scope): the bridge is caught only when we see
/// the `ln`; a pre-existing symlink read is the documented HP-5 residual.
fn resolve_ln(tokens: &[Token]) -> Profile {
    const LN: Flags = Flags {
        short: b"FLPTdfhinrsvw",
        valued_short: b"St",
        // `--backup` takes an optional arg (glued only) → boolean long, not valued_long.
        long: &[
            "--backup", "--directory", "--force", "--help", "--interactive", "--logical",
            "--no-dereference", "--no-target-directory", "--physical", "--relative",
            "--symbolic", "--verbose", "--version",
        ],
        valued_long: &["--suffix", "--target-directory"],
        numeric_shorthand: false,
    };
    let (targets, link) = match sources_and_dest(&LN, tokens, "ln") {
        Ok(sd) => sd,
        Err(profile) => return profile,
    };
    let force = has_flag(tokens, Some("-f"), Some("--force"));
    let scale = breadth_scale(&targets, false);
    transfer_profile(
        &targets,
        link,
        scale,
        |loc, sc| observes(loc, sc, "ln bridges the link to its target's locus (cp-by-reference)"),
        |loc, sc| overwrites(loc, sc, !force),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn head_tail_wc_read_like_cat_and_honor_numeric_shorthand() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};
        // worktree reads → read-local (SafeRead); home reads → denied by locus, same as cat.
        for cmd in [
            vec!["head", "README.md"],
            vec!["head", "-n", "5", "src/main.rs"],
            vec!["head", "-20", "src/main.rs"],   // obsolete -NUM form must parse
            vec!["tail", "-f", "./log.txt"],       // follow is still a bounded read
            vec!["tail", "-n", "100", "./log.txt"],
            vec!["wc", "-l", "./notes.md"],
        ] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("read")), Verdict::Allowed(SafetyLevel::SafeRead), "{cmd:?}");
        }
        // reading stdin (`-`) is process-scoped → inert, like `cat -`.
        assert_eq!(project(&resolve(&toks(&["wc", "-c", "-"])).expect("wc")), Verdict::Allowed(SafetyLevel::Inert), "wc stdin");
        for cmd in [vec!["head", "~/.ssh/id_rsa"], vec!["tail", "/etc/shadow"], vec!["wc", "-l", "$SECRET"]] {
            assert_eq!(project(&resolve(&toks(&cmd)).expect("read")), Verdict::Denied, "{cmd:?} beyond worktree");
        }
        // -NUM consumes no operand: `head -20 file` reads exactly `file`, not a phantom "20".
        let p = resolve(&toks(&["head", "-20", "src/main.rs"])).expect("head");
        assert_eq!(p.capabilities.len(), 1, "-20 is the count, not a file");
        // wc --files0-from reads an unpinnable set → worst-case → denied.
        assert_eq!(project(&resolve(&toks(&["wc", "--files0-from=list"])).expect("wc")), Verdict::Denied, "--files0-from");
        assert_eq!(project(&resolve(&toks(&["wc", "--files0-from", "-"])).expect("wc")), Verdict::Denied, "--files0-from -");
        // unknown flags fail closed.
        assert_eq!(project(&resolve(&toks(&["head", "-Z", "x"])).expect("head")), Verdict::Denied, "unknown flag");
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

        // mv ./a ./b — a relocation: source MUTATE (trivial, transient — the entry leaves)
        // + dest CREATE (recoverable overwrite). Contrast cp's source, which is an observe.
        let mv = resolve(&toks(&["mv", "./a", "./b"])).expect("mv");
        let mut mv_src = Capability::new(Operation::Mutate);
        mv_src.locus.local = LocalLocus::Worktree;
        mv_src.reversibility = Reversibility::Trivial;
        assert_eq!(clear_because(&mv.capabilities[0]), mv_src, "mv source relocation");
        let mut mv_dst = Capability::new(Operation::Create);
        mv_dst.locus.local = LocalLocus::Worktree;
        mv_dst.reversibility = Reversibility::Recoverable;
        mv_dst.persistence.level = PersistenceLevel::Data;
        assert_eq!(clear_because(&mv.capabilities[1]), mv_dst, "mv dest write");

        // ln ./a ./b — target bridged (observe, no model disclosure) + link create (trivial,
        // no -f). Same facet shapes as cp -n, the point being ln reuses `observes`.
        let ln = resolve(&toks(&["ln", "./a", "./b"])).expect("ln");
        let mut ln_tgt = Capability::new(Operation::Observe);
        ln_tgt.locus.local = LocalLocus::Worktree;
        assert_eq!(clear_because(&ln.capabilities[0]), ln_tgt, "ln target bridge");
        let mut ln_link = Capability::new(Operation::Create);
        ln_link.locus.local = LocalLocus::Worktree;
        ln_link.reversibility = Reversibility::Trivial;
        ln_link.persistence.level = PersistenceLevel::Data;
        assert_eq!(clear_because(&ln.capabilities[1]), ln_link, "ln link create");
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

        // a copy is a create/overwrite, not a destroy → write-local (SafeWrite), matching
        // echo > config.json. Overwriting is recoverable; -n can't clobber (trivial). Both
        // write-local — the destroy-vs-create boundary keeps cp below rm.
        let plain = resolve(&toks(&["cp", "./a", "./b"])).expect("cp");
        assert_eq!(plain.capabilities.last().unwrap().reversibility, Reversibility::Recoverable, "dest overwrite");
        assert_eq!(project(&plain), Verdict::Allowed(SafetyLevel::SafeWrite), "cp ./a ./b");
        let nc = resolve(&toks(&["cp", "-n", "./a", "./b"])).expect("cp");
        assert_eq!(nc.capabilities.last().unwrap().reversibility, Reversibility::Trivial, "-n cannot clobber");
        assert_eq!(project(&nc), Verdict::Allowed(SafetyLevel::SafeWrite), "cp -n ./a ./b");

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

        // optional-argument longs (--backup[=X], --preserve[=X]) must NOT swallow the
        // source operand: bare and glued forms both leave ./a a source and ./b the dest.
        for form in [
            vec!["cp", "--backup", "./a", "./b"],
            vec!["cp", "--preserve", "./a", "./b"],
            vec!["cp", "--preserve=mode", "./a", "./b"],
        ] {
            let c = resolve(&toks(&form)).expect("cp");
            assert_eq!(c.capabilities.len(), 2, "{form:?}: source read + dest write");
            assert_eq!(project(&c), Verdict::Allowed(SafetyLevel::SafeWrite), "{form:?}");
        }

        // recursion raises scale to unbounded; a lone operand / unknown flag worst-cases.
        assert_eq!(resolve(&toks(&["cp", "-r", "./a", "./b"])).expect("cp").capabilities[0].scale, Scale::Unbounded);
        assert_eq!(project(&resolve(&toks(&["cp", "./only"])).expect("cp")), Verdict::Denied, "no dest");
        assert_eq!(project(&resolve(&toks(&["cp", "-Q", "./a", "./b"])).expect("cp")), Verdict::Denied, "unknown flag");
    }

    #[test]
    fn mv_relocates_within_the_worktree_and_gates_both_loci() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};

        // a move within the worktree is a mutate (source) + create (dest), both trivial/
        // recoverable → write-local, NOT developer. Unlike rm, a move relocates, not destroys.
        let m = resolve(&toks(&["mv", "./a", "./b"])).expect("mv");
        assert_eq!(m.capabilities[0].operation, Operation::Mutate, "source is a relocation, not a destroy");
        assert_eq!(m.capabilities[0].reversibility, Reversibility::Trivial, "mv back");
        assert_eq!(project(&m), Verdict::Allowed(SafetyLevel::SafeWrite), "mv ./a ./b");

        // both loci are gated as writes: source-out and dest-out both deny.
        assert_eq!(project(&resolve(&toks(&["mv", "~/.ssh/id_rsa", "./x"])).expect("mv")), Verdict::Denied, "source in home");
        assert_eq!(project(&resolve(&toks(&["mv", "./x", "~/exfil"])).expect("mv")), Verdict::Denied, "dest in home");
        // moving a worktree-TRUSTED file mutates .git → denied, even though cp of it is
        // allowed (cp only READS .git/config; the dest write puts cp at SafeWrite).
        assert_eq!(project(&resolve(&toks(&["mv", ".git/config", "./x"])).expect("mv")), Verdict::Denied, "mv .git/config");
        assert_eq!(project(&resolve(&toks(&["cp", ".git/config", "./x"])).expect("cp")), Verdict::Allowed(SafetyLevel::SafeWrite), "cp .git/config reads");

        // -t DIR and glued forms; fail-closed on unknown flag / lone operand.
        let t = resolve(&toks(&["mv", "-t", "./dest", "./a", "./b"])).expect("mv -t");
        assert_eq!(t.capabilities.len(), 3, "2 sources + 1 dest");
        assert_eq!(project(&resolve(&toks(&["mv", "./only"])).expect("mv")), Verdict::Denied, "no dest");
        assert_eq!(project(&resolve(&toks(&["mv", "-Q", "./a", "./b"])).expect("mv")), Verdict::Denied, "unknown flag");
    }

    #[test]
    fn ln_is_cp_by_reference_and_gates_the_target_locus() {
        use crate::engine::bridge::project;
        use crate::verdict::{SafetyLevel, Verdict};

        // a worktree link (hard or symbolic) is target-read + link-create → write-local.
        for cmd in [vec!["ln", "./a", "./b"], vec!["ln", "-s", "./target", "./link"]] {
            let p = resolve(&toks(&cmd)).expect("ln");
            assert_eq!(p.capabilities[0].operation, Operation::Observe, "target is a bridged read");
            assert_eq!(project(&p), Verdict::Allowed(SafetyLevel::SafeWrite), "{cmd:?}");
        }
        // the cp-bypass is closed: linking a home/system TARGET denies on the target locus,
        // exactly as `cp` of it would (a link would otherwise alias the secret into the tree).
        assert_eq!(project(&resolve(&toks(&["ln", "~/.ssh/id_rsa", "./x"])).expect("ln")), Verdict::Denied, "hard link to home");
        assert_eq!(project(&resolve(&toks(&["ln", "-s", "/etc/passwd", "./x"])).expect("ln")), Verdict::Denied, "symlink to system");
        // writing the LINK outside the worktree denies on the link locus.
        assert_eq!(project(&resolve(&toks(&["ln", "-s", "./a", "~/evil"])).expect("ln")), Verdict::Denied, "link into home");
        // -t DIR, lone operand, unknown flag.
        assert_eq!(resolve(&toks(&["ln", "-t", "./dir", "./a", "./b"])).expect("ln -t").capabilities.len(), 3);
        assert_eq!(project(&resolve(&toks(&["ln", "./only"])).expect("ln")), Verdict::Denied, "no link name");
        assert_eq!(project(&resolve(&toks(&["ln", "-Q", "./a", "./b"])).expect("ln")), Verdict::Denied, "unknown flag");
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

    /// The content-transfer commands: every one moves/bridges content between a source and
    /// a destination operand, so BOTH roles must be locus-gated. Extend this list as
    /// `install`/`dd`/`rsync`/`tar` land — a resolver that forgets to gate a role then fails
    /// the property below (the `ln` cp-bypass class, §HP re: capability laundering).
    const TRANSFER_CMDS: &[&str] = &["cp", "mv", "ln"];

    /// A sensitive path that must never be laundered through a transfer command, in any
    /// role. Covers each locus rung above the worktree AND the two unpinnable markers.
    const HOT_PATHS: &[&str] = &["/etc/shadow", "~/.ssh/id_rsa", "$SECRET", "../out", "~/.aws"];

    proptest! {
        /// No capability laundering: a hot path in EITHER operand role of a transfer command
        /// denies — you can neither pull a secret in (`cp ~/.ssh/id_rsa ./x`) nor push one
        /// out (`cp ./x /etc/cron.d/y`). This is the STRICT property that catches an ignored
        /// operand; plain locus-monotonicity does not, because ignoring a role leaves the
        /// verdict unchanged, and unchanged is "not looser".
        #[test]
        fn transfer_commands_gate_both_operand_roles(
            cmd in prop::sample::select(TRANSFER_CMDS),
            hot in prop::sample::select(HOT_PATHS),
        ) {
            use crate::engine::bridge::project;
            use crate::verdict::Verdict;
            let hot_source = resolve(&toks(&[cmd, hot, "./safe"])).expect("resolves");
            prop_assert_eq!(project(&hot_source), Verdict::Denied, "{} hot SOURCE ({})", cmd, hot);
            let hot_dest = resolve(&toks(&[cmd, "./safe", hot])).expect("resolves");
            prop_assert_eq!(project(&hot_dest), Verdict::Denied, "{} hot DEST ({})", cmd, hot);
        }
    }

    /// The touched-path probe invocations for a resolver, derived from its declared
    /// [`Operands`] contract: `hot` fills each touched slot in turn, other slots get a benign
    /// path. Non-path slots (grep's pattern) are never filled with `hot`.
    fn probes<'a>(cmd: &'a str, kind: Operands, hot: &'a str) -> Vec<Vec<&'a str>> {
        match kind {
            Operands::None => vec![],
            Operands::Paths => vec![vec![cmd, hot]],
            Operands::PatternThenPaths => vec![vec![cmd, "PATTERN", hot]],
            Operands::Transfer => vec![vec![cmd, hot, "./safe"], vec![cmd, "./safe", hot]],
        }
    }

    /// The conservation law (HP-18): every touched path operand must contribute a capability
    /// at its own locus, so a hot path in ANY touched slot forces denial — no operand is
    /// silently dropped. Generalizes the transfer differential to the whole corpus and would
    /// catch a future single-file reader that forgets its `observe`. The probes are derived
    /// from each resolver's [`Operands`] contract in `RESOLVERS` — one source of truth, so
    /// adding a resolver (which must declare its `Operands`) is covered automatically.
    #[test]
    fn every_touched_path_operand_is_gated() {
        use crate::engine::bridge::project;
        use crate::verdict::Verdict;

        let mut exercised = 0usize;
        for (name, _, kind) in RESOLVERS {
            for hot in HOT_PATHS {
                for cmd in probes(name, *kind, hot) {
                    let profile = resolve(&toks(&cmd)).expect("resolves");
                    assert_eq!(project(&profile), Verdict::Denied, "{cmd:?}: touched hot path not gated");
                    exercised += 1;
                }
            }
        }
        // Non-vacuity: every path-bearing resolver contributed at least one probe per hot path.
        let path_bearing = RESOLVERS.iter().filter(|(_, _, k)| !matches!(k, Operands::None)).count();
        assert!(exercised >= path_bearing * HOT_PATHS.len(), "sweep ran only {exercised}");
    }
}
