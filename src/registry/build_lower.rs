//! Lowering the declarative `[command.output]` and `[command.behavior]` blocks into their typed
//! specs. A child of `build`, whose `fail!` it uses.

use super::super::types::*;

/// Lower a `[command.behavior]` block into a typed `BehaviorSpec`. Every facet string is
/// resolved to its enum here (via `FacetTerm::from_term`); an unknown term PANICS naming the
/// command, so a typo fails the build (the registry loads in a test) rather than silently
/// mis-classifying. `None` when the command declares no behavior.
pub(super) fn lower_output(name: &str, o: Option<&TomlOutput>) -> Result<Option<OutputSpec>, String> {
    let Some(o) = o else { return Ok(None) };
    let locus_from = match o.locus_from.as_str() {
        "operands" => OutputLocus::Operands,
        "cwd" => OutputLocus::Cwd,
        "stdin" => OutputLocus::Stdin,
        "atom" => OutputLocus::Atom,
        other => fail!("command '{name}': unknown output locus_from `{other}` (known: operands, cwd, stdin, atom)"),
    };
    Ok(Some(OutputSpec { locus_from, invalidated_by: o.invalidated_by.clone(), valued: o.valued.clone(), requires: o.requires.clone() }))
}

pub(super) fn lower_behavior(name: &str, b: Option<&TomlBehavior>) -> Result<Option<BehaviorSpec>, String> {
    use crate::engine::facet::{FacetTerm, Operation};
    let Some(b) = b else { return Ok(None) };
    let Some(operation) = Operation::from_term(&b.operation) else {
        fail!("command '{name}': unknown behavior operation `{}`", b.operation)
    };
    let positionals = match b.positionals.as_str() {
        "none" => PositionalRole::None,
        "read" => PositionalRole::Read,
        "write" => PositionalRole::Write,
        "pattern-then-read" => PositionalRole::PatternThenRead,
        "transfer" => PositionalRole::Transfer,
        other => fail!("command '{name}': unknown behavior positionals `{other}` (known: none, read, write, pattern-then-read, transfer)"),
    };
    let scale = match b.scale.as_deref() {
        None | Some("single") => ScaleModel::Single,
        Some("breadth") => ScaleModel::Breadth,
        Some(other) => fail!("command '{name}': unknown behavior scale `{other}` (known: single, breadth)"),
    };
    let hook = match b.hook.as_deref() {
        None => None,
        Some("grep") => Some(BehaviorHook::Grep),
        Some("dd") => Some(BehaviorHook::Dd),
        Some("tar") => Some(BehaviorHook::Tar),
        Some("sed") => Some(BehaviorHook::Sed),
        Some("perl") => Some(BehaviorHook::Perl),
        Some(other) => fail!("command '{name}': unknown behavior hook `{other}` (known: grep, dd, tar, sed, perl)"),
    };
    let (short, long) = split_flag_forms(&b.standalone);
    let (valued_short, valued_long) = split_flag_forms(&b.valued);
    let mut unbounded_flags = Vec::new();
    let mut path_flags = Vec::new();
    for (flag, delta) in &b.flags {
        if delta.scale.as_deref() == Some("unbounded") {
            unbounded_flags.push(flag.clone());
        } else if let Some(other) = delta.scale.as_deref() {
            fail!("command '{name}': behavior flag `{flag}` has unknown scale `{other}` (known: unbounded)");
        }
        if let Some(kind) = delta.kind.as_deref() {
            let role = match kind {
                "read" => PathRole::Read,
                "write" => PathRole::Write,
                other => {
                    fail!("command '{name}': behavior flag `{flag}` has unknown kind `{other}` (known: read, write)")
                }
            };
            if !b.valued.contains(flag) {
                fail!("command '{name}': behavior path-flag `{flag}` (kind = {kind}) must also be listed in `valued`");
            }
            let (short, long) = if let Some(rest) = flag.strip_prefix("--") {
                (None, Some(format!("--{rest}")))
            } else if let Some(rest) = flag.strip_prefix('-') {
                if rest.len() == 1 {
                    (Some(rest.as_bytes()[0]), None)
                } else {
                    fail!("command '{name}': behavior path-flag `{flag}` must be a single-char short or a `--long`");
                }
            } else {
                fail!("command '{name}': behavior path-flag `{flag}` must start with `-`");
            };
            path_flags.push(PathFlag { short, long, role });
        }
    }
    let transfer = lower_transfer(name, b.transfer.as_ref())?;
    // A transfer role needs its knobs; anything else must not carry them.
    match (positionals, &transfer) {
        (PositionalRole::Transfer, None) => {
            fail!("command '{name}': positionals = \"transfer\" requires a [command.behavior.transfer] block")
        }
        (role, Some(_)) if role != PositionalRole::Transfer => {
            fail!("command '{name}': [command.behavior.transfer] is only valid with positionals = \"transfer\"")
        }
        _ => {}
    }
    Ok(Some(BehaviorSpec {
        operation,
        positionals,
        scale,
        short,
        valued_short,
        long,
        valued_long,
        numeric_shorthand: b.numeric_shorthand.unwrap_or(false),
        unbounded_flags,
        path_flags,
        hook,
        transfer,
    }))
}

/// Lower a `[command.behavior.transfer]` block, resolving the `source` term and enforcing that
/// the two clobber-flag sets are mutually exclusive (a command declares whether the default is
/// clobber or no-clobber, never both).
fn lower_transfer(name: &str, t: Option<&TomlTransfer>) -> Result<Option<TransferSpec>, String> {
    let Some(t) = t else { return Ok(None) };
    let source = match t.source.as_str() {
        "observe" => TransferSource::Observe,
        "relocate" => TransferSource::Relocate,
        other => fail!("command '{name}': unknown transfer source `{other}` (known: observe, relocate)"),
    };
    if !t.no_clobber_flags.is_empty() && !t.clobber_flags.is_empty() {
        fail!("command '{name}': transfer declares both no_clobber_flags and clobber_flags (mutually exclusive)");
    }
    Ok(Some(TransferSpec {
        source,
        rebinds_destination: t.rebinds_destination,
        no_clobber_flags: t.no_clobber_flags.clone(),
        clobber_flags: t.clobber_flags.clone(),
        recursive_flags: t.recursive_flags.clone(),
    }))
}

/// Split a behavior flag list into (single-dash single-char shorts as bytes, `--long`
/// tokens). A single-dash multi-char token is kept whole in `long` — it then only matches as
/// a literal, which for a non-`--` token means it never classifies as known and fails closed.
fn split_flag_forms(tokens: &[String]) -> (Vec<u8>, Vec<String>) {
    let mut short = Vec::new();
    let mut long = Vec::new();
    for t in tokens {
        if t.starts_with("--") {
            long.push(t.clone());
        } else if let Some(rest) = t.strip_prefix('-') {
            if rest.len() == 1 {
                short.push(rest.as_bytes()[0]);
            } else {
                long.push(t.clone());
            }
        }
    }
    (short, long)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// `locus_from` lowers exactly for the four sources it names, and never panics on another.
        #[test]
        fn output_lowers_only_a_known_source(from in prop_oneof!["operands|cwd|stdin|atom", ".{0,10}"]) {
            let toml = TomlOutput { locus_from: from.clone(), invalidated_by: Vec::new(), valued: Vec::new(), requires: Vec::new() };
            let known = ["operands", "cwd", "stdin", "atom"].contains(&from.as_str());
            prop_assert_eq!(lower_output("x", Some(&toml)).is_ok(), known);
            prop_assert!(matches!(lower_output("x", None), Ok(None)));
        }
    }
}
