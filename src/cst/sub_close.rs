//! Locating the `)` that closes a substitution body, without parsing it.

/// Byte offset of the `)` that closes a substitution body starting at `body[0]` — the first `)` at
/// paren-depth zero — or `None` if it is never closed. Quote (`'…'`, `"…"`), backtick, and backslash
/// spans are skipped so a `)` inside them does not count, mirroring how the grammar's own
/// `single_quoted`/`double_quoted`/`backtick`/`escaped` parsers treat those regions. This is what
/// keeps `cmd_sub`/`proc_sub` linear: the interior is parsed only once, over a bounded slice, instead
/// of the old `delimited(script, ')')` shape that recursed into the tail BEFORE knowing a close even
/// existed — the source of the `a$(a<(a` × N exponential.
pub(super) fn find_sub_close(body: &str) -> Option<usize> {
    let b = body.as_bytes();
    let mut i = 0;
    let mut depth: usize = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1, // escape: skip the next byte too (the trailing `+= 1` handles it)
            b'\'' => {
                i += 1;
                while i < b.len() && b[i] != b'\'' {
                    i += 1;
                }
                if i >= b.len() {
                    return None;
                }
            }
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                if i >= b.len() {
                    return None;
                }
            }
            b'`' => {
                i += 1;
                while i < b.len() && b[i] != b'`' {
                    // `bt_escape` treats `\<any>` inside backticks as a literal, so an escaped
                    // backtick does NOT close the span — skip the escaped byte too.
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                if i >= b.len() {
                    return None;
                }
            }
            b'(' => depth += 1,
            b')' => {
                if depth == 0 {
                    return Some(i);
                }
                depth -= 1;
            }
            _ => {}
        }
        i += 1;
    }
    None
}
