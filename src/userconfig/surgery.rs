//! Edits made to the file's text, so every byte outside the edited table stays as it was.
//!
//! Letting `toml_edit` lay the whole document out again was not enough: fuzzing found it moving
//! dotted keys, turning `\r\n` into `\n`, and dropping blank lines it was not asked about. Here a
//! new table is appended, a removed one is cut out by its byte range, and a changed value is
//! replaced where it stands. `toml_edit` is only asked where things are.

use std::ops::Range;

use toml_edit::{ImDocument, Item, Table};

/// The newline the file already uses.
pub(super) fn newline(source: &str) -> &'static str {
    if source.contains("\r\n") { "\r\n" } else { "\n" }
}

/// `source` with `block` (written with `\n`) added at the end, after a blank line.
pub(super) fn append(source: &str, block: &str) -> String {
    let nl = newline(source);
    let mut out = source.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push_str(nl);
    }
    if !out.is_empty() {
        out.push_str(nl);
    }
    out.push_str(&block.replace('\n', nl));
    out
}

/// Each element of the array of tables `key`, in file order, with the bytes it occupies: its
/// header and the comment lines directly above it, through the end of the last line it or any of
/// its sub-tables uses. `None` when a span is missing, which means the edit cannot be made by
/// cutting text.
pub(super) fn elements(source: &str, key: &str) -> Option<Vec<Range<usize>>> {
    let doc = ImDocument::parse(source).ok()?;
    let Some(item) = doc.get(key) else { return Some(Vec::new()) };
    let tables = item.as_array_of_tables()?;
    tables
        .iter()
        .map(|table| {
            let span = extent(table)?;
            let range = attached_comments_start(source, line_start(source, span.start))..line_end(source, span.end);
            Some(with_its_blank_line(source, range))
        })
        .collect()
}

/// Take one blank line along with a block, so cutting it leaves the spacing as it was before the
/// block arrived: the one above it when it ends the file, else the one below when there is one
/// above as well.
fn with_its_blank_line(source: &str, range: Range<usize>) -> Range<usize> {
    let blank_above = range.start > 0 && {
        let above = line_start(source, range.start - 1);
        source[above..range.start].trim().is_empty()
    };
    if range.end == source.len() && blank_above {
        return line_start(source, range.start - 1)..range.end;
    }
    let below = line_end(source, range.end);
    if blank_above && range.end < source.len() && source[range.end..below].trim().is_empty() {
        return range.start..below;
    }
    range
}

/// The span of a table together with every table nested in it.
fn extent(table: &Table) -> Option<Range<usize>> {
    let mut span = table.span()?;
    for (_, item) in table.iter() {
        let nested: Vec<&Table> = match item {
            Item::Table(t) if !t.is_implicit() => vec![t],
            Item::ArrayOfTables(a) => a.iter().collect(),
            _ => Vec::new(),
        };
        for t in nested {
            let inner = extent(t)?;
            span = span.start.min(inner.start)..span.end.max(inner.end);
        }
    }
    Some(span)
}

fn line_start(source: &str, at: usize) -> usize {
    source[..at].rfind('\n').map_or(0, |i| i + 1)
}

fn line_end(source: &str, at: usize) -> usize {
    source[at..].find('\n').map_or(source.len(), |i| at + i + 1)
}

/// Move `start` up over the comment lines that sit directly on the header and describe it. Only a
/// block with a blank line (or the start of the file) above it counts, and only if none of its
/// lines is commented-out TOML: review found `# [[grant]]` blocks, kept on purpose, cut away with
/// the table below them.
fn attached_comments_start(source: &str, header: usize) -> usize {
    let mut start = header;
    while start > 0 {
        let previous = line_start(source, start - 1);
        let line = source[previous..start].trim();
        if !line.starts_with('#') {
            break;
        }
        if looks_like_toml(line.trim_start_matches('#').trim()) {
            return header;
        }
        start = previous;
    }
    let blank_or_top = start == 0 || source[line_start(source, start - 1)..start].trim().is_empty();
    if blank_or_top { start } else { header }
}

fn looks_like_toml(text: &str) -> bool {
    text.starts_with('[')
        || text.split_once('=').is_some_and(|(key, _)| {
            let key = key.trim();
            !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '"'))
        })
}

/// `source` without the bytes in `ranges`. Ranges may touch or overlap: two adjacent blocks can
/// each claim the blank line between them.
pub(super) fn cut(source: &str, ranges: &[Range<usize>]) -> String {
    let mut sorted = ranges.to_vec();
    sorted.sort_by_key(|r| r.start);
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in sorted {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    let mut out = String::with_capacity(source.len());
    let mut at = 0;
    for range in merged {
        out.push_str(&source[at..range.start]);
        at = range.end;
    }
    out.push_str(&source[at..]);
    out
}

/// Set boolean `keys` on the `index`th element of the array of tables `key`: a value already
/// there is replaced where it stands, a missing one is added on a line after the table's last.
pub(super) fn set_flags(source: &str, key: &str, index: usize, flags: &[(&str, bool)]) -> Option<String> {
    let doc = ImDocument::parse(source).ok()?;
    let table = doc.get(key)?.as_array_of_tables()?.get(index)?;
    let nl = newline(source);
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    let mut appended = String::new();
    for (name, wanted) in flags {
        match table.get(name) {
            Some(item) if item.as_bool() == Some(*wanted) => {}
            Some(item) => edits.push((item.span()?, wanted.to_string())),
            None => appended.push_str(&format!("{name} = {wanted}{nl}")),
        }
    }
    if !appended.is_empty() {
        let at = line_end(source, table.span()?.end);
        if at == source.len() && !source.ends_with('\n') {
            appended.insert_str(0, nl);
        }
        edits.push((at..at, appended));
    }
    let mut out = source.to_string();
    edits.sort_by_key(|(r, _)| std::cmp::Reverse(r.start));
    for (range, text) in edits {
        out.replace_range(range, &text);
    }
    Some(out)
}
