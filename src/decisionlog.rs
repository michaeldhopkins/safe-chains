//! The opt-in decision log: one JSON object per classification, appended to
//! `~/.local/state/safe-chains/log.jsonl`.
//!
//! Off unless the hook is invoked with `--log` (records what did NOT auto-approve) or
//! `--log-everything` (records approvals too). See `docs/design/decision-log.md`.
//!
//! Two properties govern everything here:
//!
//! **Logging can never change a verdict.** A `PreToolUse` hook that crashes fails OPEN — the harness
//! runs the command — so a logging fault must not propagate. Every path in this module swallows its
//! errors: a missing `$HOME`, an unwritable directory, a full disk and a read-only filesystem all
//! result in nothing being written and the classification proceeding untouched. There is no `?` that
//! escapes to the caller and no `unwrap`.
//!
//! **The mode is checked before the entry is built.** `--log`'s whole advantage is that it does
//! nothing on the overwhelmingly common path (an approval), and that is only true if the
//! allow/deny test comes first. `record` returns before touching the engine, the filesystem or the
//! clock when the outcome is not one this mode keeps.

use std::fmt;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Rotate once the current file reaches this size, keeping [`GENERATIONS`] older files.
///
/// The shape follows the platform conventions rather than a number picked from one machine's usage:
/// macOS ships `/etc/newsyslog.conf` entries at 1000 KB with a count of 5, and logrotate's own
/// manual example is `weekly` + `rotate 5`, with `size` given in units like `100k`/`100M`. Both
/// keep SEVERAL generations; a single old file is the part that was unconventional.
///
/// Size-based rather than time-based because safe-chains is a short-lived hook process, not a
/// daemon with a cron entry — there is nothing to run a weekly job, so the check happens on open.
///
/// 16 MB sits in logrotate's usual band for an application log and holds ~22k entries at the
/// measured 754-byte mean, so the five generations span ~110k decisions. Under `--log` (refusals
/// only) that is years; under `--log-everything` it is weeks of heavy use.
const ROTATE_AT_BYTES: u64 = 16 * 1024 * 1024;

/// How many rotated files to keep (`log.jsonl.1` … `log.jsonl.5`), matching newsyslog's count and
/// logrotate's `rotate 5`.
const GENERATIONS: usize = 5;

/// Schema version of an entry. Bump on any incompatible change to the field set.
const SCHEMA: u32 = 1;

/// What the log keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// No logging at all — no file is created.
    Off,
    /// Everything that did not auto-approve: denials, abstains, parse failures.
    NonApprovals,
    /// The above, plus approvals.
    Everything,
}

impl Mode {
    /// `--log` / `--log-everything`, resolved. `--log-everything` wins when both are given: it is
    /// the strictly wider request, so honouring it cannot lose an entry the user asked for.
    pub fn from_flags(log: bool, log_everything: bool) -> Self {
        match (log, log_everything) {
            (_, true) => Mode::Everything,
            (true, false) => Mode::NonApprovals,
            (false, false) => Mode::Off,
        }
    }

    /// Whether an entry with this outcome is kept. The gate that must run BEFORE an entry is built.
    pub fn keeps(self, outcome: Outcome) -> bool {
        match self {
            Mode::Off => false,
            Mode::Everything => true,
            Mode::NonApprovals => outcome != Outcome::Allowed,
        }
    }
}

/// What safe-chains decided. Distinct from a bare bool because the three non-approvals need telling
/// apart in triage: a denial is a classification, an abstain is a harness/tool mismatch, and a parse
/// failure is a availability signal — a rise in them is how a parser regression shows up in the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Allowed,
    Denied,
    Abstained,
    Unparseable,
}

impl Outcome {
    fn as_str(self) -> &'static str {
        match self {
            Outcome::Allowed => "allowed",
            Outcome::Denied => "denied",
            Outcome::Abstained => "abstained",
            Outcome::Unparseable => "unparseable",
        }
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Everything about the invocation that is not the verdict. Borrowed rather than owned so building
/// one costs nothing on the path where the mode discards it.
pub struct Context<'a> {
    pub command: &'a str,
    pub cwd: Option<&'a str>,
    pub root: Option<&'a str>,
    pub session_id: Option<&'a str>,
    /// The harness whose hook envelope was parsed (`claude`, `codex`, …), or `cli`.
    pub harness: &'a str,
    /// The auto-approve ceiling in force, as its level name.
    pub level: &'a str,
}

/// Append one entry, if this mode keeps that outcome. Never fails, never panics, never blocks the
/// caller's decision.
///
/// `explanation` supplies the per-segment breakdown. It is optional because the caller only has one
/// on the paths that computed it: an approval is decided without ever building an explanation, and
/// making one just to log it would put the cost back on the hot path that `--log` exists to keep
/// free.
pub fn record(
    mode: Mode,
    outcome: Outcome,
    ctx: &Context<'_>,
    explanation: Option<&crate::cst::Explanation>,
) {
    if !mode.keeps(outcome) {
        return;
    }
    let Some(path) = log_path() else { return };
    let entry = build_entry(outcome, ctx, explanation);
    let Ok(line) = serde_json::to_string(&entry) else { return };
    append_line(&path, &line);
}

/// `~/.local/state/safe-chains/log.jsonl`. FIXED — see the design doc: the only wrong locations are
/// ones a user would have to opt into (a log in the worktree is readable AND writable by the agent),
/// so not offering the choice is strictly safer than validating it.
///
/// `$HOME` unset → `None`, and nothing is logged. Deliberately not `XDG_STATE_HOME`: the agent's
/// environment reaches the hook, which is the same reason `registry::custom` refuses to honour
/// `XDG_CONFIG_HOME` for the trust root.
fn log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    if home.is_empty() {
        return None;
    }
    Some(PathBuf::from(home).join(".local/state/safe-chains/log.jsonl"))
}

fn build_entry(
    outcome: Outcome,
    ctx: &Context<'_>,
    explanation: Option<&crate::cst::Explanation>,
) -> Value {
    let now_ms = unix_millis();
    let mut digest = Sha256::new();
    digest.update(ctx.command.as_bytes());
    let hash: String = digest.finalize().iter().take(4).map(|b| format!("{b:02x}")).collect();

    // The refusal reason rides on the SEGMENT, not the entry. A whole-command `facets` field could
    // only ever be filled for a single-command entry — the engine resolves one command at a time —
    // so on a chain, which is the common case, it was null exactly when it was most wanted: the
    // entry named the failing segment and left "but why" to a manual `--explain`. Resolved per
    // denied segment instead, which costs nothing on the allowed ones and nothing at all under the
    // default mode's approval path.
    let segments: Vec<Value> = explanation
        .map(|e| {
            e.segments
                .iter()
                .map(|s| {
                    let allowed = s.verdict.is_allowed();
                    json!({
                        "text": s.text,
                        "verdict": if allowed { "allowed" } else { "denied" },
                        "culprit": s.culprit,
                        "facets": if allowed { Value::Null } else { facets_of(&s.text) },
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // An approval owes no triage and no facets: there is no refusal to explain and no registry gap,
    // and computing either would charge the hot path for something nothing reads.
    let (triage, unknown) = if outcome == Outcome::Allowed {
        ("allowed", Vec::new())
    } else {
        triage_of(ctx.command)
    };

    json!({
        "schema": SCHEMA,
        "id": format!("{now_ms}-{hash}"),
        "at": rfc3339_utc(now_ms),
        "version": env!("CARGO_PKG_VERSION"),
        "harness": ctx.harness,
        "outcome": outcome.as_str(),
        "level": ctx.level,
        "command": ctx.command,
        "cwd": ctx.cwd,
        "root": ctx.root,
        "session_id": ctx.session_id,
        "triage": triage,
        "unknown_commands": unknown,
        "segments": segments,
        "stateful": explanation.is_some_and(|e| e.stateful),
    })
}

/// The split that decides what a refusal MEANS: a registry gap we can close with a definition, or a
/// classification decision to defend or revisit. Reuses `suggest::analyze`, which already computes
/// exactly this and is otherwise only consumed by `--suggest`.
fn triage_of(command: &str) -> (&'static str, Vec<String>) {
    use crate::suggest::Outcome as S;
    match crate::suggest::analyze(command) {
        // Reachable when the command classifies allowed on its own but the caller recorded a
        // non-approval — an abstain, or a ceiling below the command's level. Not a registry gap.
        S::AlreadyAllowed => ("recognized-but-denied", Vec::new()),
        S::Unparseable => ("unparseable", Vec::new()),
        S::RecognizedButDenied { .. } => ("recognized-but-denied", Vec::new()),
        S::Generated { entries, .. } => {
            ("unknown-command", entries.iter().map(|e| e.name.clone()).collect())
        }
    }
}

/// The resolved facet profile and the clause that refused it — the structured form of what
/// `--explain` prints. `null` when no resolver claims the command (the legacy classifier decided, so
/// there are no facets), or when the command is not a single segment.
fn facets_of(command: &str) -> Value {
    if crate::cst::explain(command).segments.len() != 1 {
        return Value::Null;
    }
    let Ok(words) = shell_words::split(command) else { return Value::Null };
    if words.is_empty() {
        return Value::Null;
    }
    let tokens: Vec<crate::parse::Token> =
        words.into_iter().map(crate::parse::Token::from_raw).collect();
    let Some(ex) = crate::engine::bridge::explain_profile(&tokens) else {
        return Value::Null;
    };
    let capabilities: Vec<Value> = ex
        .capabilities
        .iter()
        .map(|(because, facets)| {
            let profile: serde_json::Map<String, Value> = facets
                .iter()
                .map(|(name, term)| ((*name).to_string(), Value::String((*term).to_string())))
                .collect();
            json!({ "because": because, "profile": profile })
        })
        .collect();
    json!({
        "capabilities": capabilities,
        "refused_by": ex.blocked_by.as_ref().map(|(level, mismatch)| json!({
            "level": level,
            "clause": mismatch.to_string(),
        })),
    })
}

/// Append one complete line, creating the file `0600` and rotating first if it has grown past the
/// cap. Every failure is silent by design — see the module header.
///
/// The write is a single `write_all` of the whole line to an `O_APPEND` handle. For a REGULAR file
/// both Linux and macOS hold the inode lock across the write, so concurrent appenders cannot
/// interleave and no advisory lock is needed. (The `PIPE_BUF` atomicity limit people reach for here
/// governs pipes and FIFOs, not regular files.) The line is therefore built fully in memory first —
/// streaming it out in pieces is what would tear it.
fn append_line(path: &std::path::Path, line: &str) {
    let Some(dir) = path.parent() else { return };
    if fs::create_dir_all(dir).is_err() {
        return;
    }
    rotate_if_large(path);

    let mut opts = fs::OpenOptions::new();
    opts.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // The file holds commands verbatim, credentials included. Owner-only from creation — a
        // later chmod would leave a window where it was not.
        opts.mode(0o600);
    }
    let Ok(mut file) = opts.open(path) else { return };
    let mut buf = String::with_capacity(line.len() + 1);
    buf.push_str(line);
    buf.push('\n');
    let _ = file.write_all(buf.as_bytes());
}

/// Shift the generations down and start a fresh file, the way newsyslog and logrotate do.
///
/// Two processes can both decide to rotate at once; the second's renames win and cost at most one
/// generation of history. Acceptable for a diagnostic, and cheaper than the lock that would prevent
/// it — the writes themselves are already safe without one (see `append_line`).
fn rotate_if_large(path: &std::path::Path) {
    rotate_at(path, ROTATE_AT_BYTES, GENERATIONS);
}

/// The cap and the count are parameters so the behaviour is testable without writing the real
/// 16 MB. Rotation is the one path here that DESTROYS data — the oldest generation is unlinked —
/// so it earns a test more than anything else in the module, and a 16 MB fixture is the kind of
/// cost that gets a test skipped.
fn rotate_at(path: &std::path::Path, cap: u64, generations: usize) {
    let Ok(meta) = fs::metadata(path) else { return };
    if meta.len() < cap {
        return;
    }
    let nth = |n: usize| path.with_extension(format!("jsonl.{n}"));
    // Oldest first: `.5` is removed, then `.4` becomes `.5`, and so on, so no rename ever clobbers
    // a generation that has not been moved out of the way yet.
    let _ = fs::remove_file(nth(generations));
    for n in (1..generations).rev() {
        let _ = fs::rename(nth(n), nth(n + 1));
    }
    let _ = fs::rename(path, nth(1));
}

fn unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// `1786790461233` → `2026-08-13T23:41:01.233Z`.
///
/// Hand-rolled rather than pulling in a date crate: the civil-from-days algorithm is fifteen lines
/// and fully testable, and a dependency added for one format string is a supply-chain and
/// license-audit cost the project would carry forever.
fn rfc3339_utc(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let millis = ms % 1000;
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let (h, mi, s) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{millis:03}Z")
}

/// Days since the Unix epoch → (year, month, day). Howard Hinnant's `civil_from_days`, which is
/// exact for the whole representable range and needs no lookup tables.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests;
