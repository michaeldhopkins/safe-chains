//! The snapshot's text form and the comparison that reports what moved.
//!
//! One line per invocation, `invocation<TAB>verdict`, sorted bytewise, so a change to one command
//! is a few adjacent lines in a diff. The verdict is `denied`, `inert`, `safe-read` or
//! `safe-write`. Tabs, newlines, carriage returns and backslashes in an invocation are escaped so
//! every line holds exactly one tab.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use safe_chains::{SafetyLevel, Verdict};

pub type Snapshot = BTreeMap<String, String>;

pub fn verdict_label(v: Verdict) -> &'static str {
    match v {
        Verdict::Denied => "denied",
        Verdict::Allowed(SafetyLevel::Inert) => "inert",
        Verdict::Allowed(SafetyLevel::SafeRead) => "safe-read",
        Verdict::Allowed(SafetyLevel::SafeWrite) => "safe-write",
    }
}

fn rank(label: &str) -> Option<u8> {
    match label {
        "inert" => Some(0),
        "safe-read" => Some(1),
        "safe-write" => Some(2),
        _ => None,
    }
}

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

pub fn unescape(s: &str) -> Result<String, String> {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            other => {
                return Err(format!("bad escape \\{} in {s:?}", other.map(String::from).unwrap_or_default()));
            }
        }
    }
    Ok(out)
}

/// The snapshot is split by an invocation's first character, one file each, because jj refuses
/// to snapshot a new file over 1 MiB and the whole table is several. A byte-sorted table's split
/// keeps each file sorted, and a change to one command touches one file.
pub fn bucket_of(invocation: &str) -> String {
    match invocation.chars().next() {
        Some(c) if c.is_ascii_alphanumeric() => c.to_ascii_lowercase().to_string(),
        _ => "other".to_string(),
    }
}

pub fn render_buckets(snapshot: &Snapshot) -> BTreeMap<String, String> {
    let mut parts: BTreeMap<String, Snapshot> = BTreeMap::new();
    for (inv, verdict) in snapshot {
        parts.entry(bucket_of(inv)).or_default().insert(inv.clone(), verdict.clone());
    }
    parts.into_iter().map(|(bucket, part)| (bucket, render(&part))).collect()
}

pub fn render(snapshot: &Snapshot) -> String {
    let mut out = String::new();
    for (invocation, verdict) in snapshot {
        let _ = writeln!(out, "{}\t{verdict}", escape(invocation));
    }
    out
}

pub fn parse(text: &str) -> Result<Snapshot, String> {
    let mut snapshot = Snapshot::new();
    for (n, line) in text.lines().enumerate() {
        let (invocation, verdict) = line.split_once('\t').ok_or_else(|| format!("line {}: no tab: {line:?}", n + 1))?;
        if verdict != "denied" && rank(verdict).is_none() {
            return Err(format!("line {}: unknown verdict {verdict:?}", n + 1));
        }
        let invocation = unescape(invocation).map_err(|e| format!("line {}: {e}", n + 1))?;
        if snapshot.insert(invocation, verdict.to_string()).is_some() {
            return Err(format!("line {}: duplicate invocation", n + 1));
        }
    }
    Ok(snapshot)
}

/// How each invocation's verdict moved between the committed snapshot and the current one.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct Drift {
    /// Was allowed, now denied: an approval was lost.
    pub newly_denied: Vec<(String, String, String)>,
    /// Allowed at a stricter level than before (`safe-read` became `safe-write`): it drops out of
    /// the lower thresholds, which is a loss for a user on `reader`.
    pub raised: Vec<(String, String, String)>,
    /// Was denied, now allowed: needs a reviewer to agree.
    pub newly_allowed: Vec<(String, String, String)>,
    /// Allowed at a looser level than before: needs review like any widening.
    pub lowered: Vec<(String, String, String)>,
    /// Generated now but absent from the snapshot.
    pub added: Vec<(String, String)>,
    /// In the snapshot but no longer generated. The current verdict is still compared above, so a
    /// lost approval shows even when the node that produced it is gone.
    pub dropped: Vec<(String, String)>,
}

impl Drift {
    pub fn is_empty(&self) -> bool {
        *self == Drift::default()
    }

    /// `old` is the committed snapshot, `new` the verdicts of today's corpus, and `recheck` today's
    /// verdict for every committed invocation the corpus no longer generates.
    pub fn between(old: &Snapshot, new: &Snapshot, recheck: &Snapshot) -> Drift {
        let mut drift = Drift::default();
        for (inv, was) in old {
            let Some(now) = new.get(inv).or_else(|| recheck.get(inv)) else {
                continue;
            };
            if !new.contains_key(inv) {
                drift.dropped.push((inv.clone(), was.clone()));
            }
            let row = (inv.clone(), was.clone(), now.clone());
            match (rank(was), rank(now)) {
                (Some(_), None) => drift.newly_denied.push(row),
                (None, Some(_)) => drift.newly_allowed.push(row),
                (Some(a), Some(b)) if b > a => drift.raised.push(row),
                (Some(a), Some(b)) if b < a => drift.lowered.push(row),
                _ => {}
            }
        }
        for (inv, now) in new {
            if !old.contains_key(inv) {
                drift.added.push((inv.clone(), now.clone()));
            }
        }
        drift
    }

    pub fn summary(&self, limit: usize) -> String {
        let mut out = String::new();
        let moved = [
            ("REGRESSION newly denied", &self.newly_denied),
            ("REGRESSION allowed only at a higher level", &self.raised),
            ("REVIEW newly allowed", &self.newly_allowed),
            ("REVIEW allowed at a lower level", &self.lowered),
        ];
        for (title, rows) in moved {
            let _ = writeln!(out, "{title}: {}", rows.len());
            for (inv, was, now) in rows.iter().take(limit) {
                let _ = writeln!(out, "    {}    ({was} -> {now})", escape(inv));
            }
        }
        for (title, rows) in [("not in the snapshot", &self.added), ("no longer generated", &self.dropped)] {
            let _ = writeln!(out, "{title}: {}", rows.len());
            for (inv, verdict) in rows.iter().take(limit) {
                let _ = writeln!(out, "    {}    ({verdict})", escape(inv));
            }
        }
        out
    }
}
