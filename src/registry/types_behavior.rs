//! The lowered forms of `[command.behavior]` and `[command.output]`, out of `types.rs`.

use super::TransferSpec;

/// A command's declarative facet behavior, lowered from `[command.behavior]` (`TomlBehavior`)
/// with every facet string resolved to its enum at build time. The generic resolver reads this
/// plus the tokens and builds a `Profile`. Clone so it can be attached uniformly across the
/// `build_command` construction sites.
#[derive(Debug, Clone)]
pub(crate) struct BehaviorSpec {
    pub operation: crate::engine::facet::Operation,
    pub positionals: PositionalRole,
    pub scale: ScaleModel,
    /// Behavior's own flag grammar, pre-split for the shared `walk_positionals`.
    pub short: Vec<u8>,
    pub valued_short: Vec<u8>,
    pub long: Vec<String>,
    pub valued_long: Vec<String>,
    pub numeric_shorthand: bool,
    /// Flags whose presence widens the scale to unbounded (`rm -r`, `grep -r`).
    pub unbounded_flags: Vec<String>,
    /// Valued flags whose VALUE is a path to gate (`touch -r REF` reads REF), with its role.
    pub path_flags: Vec<PathFlag>,
    pub hook: Option<BehaviorHook>,
    /// Transfer semantics, present iff `positionals == Transfer`.
    pub transfer: Option<TransferSpec>,
}

/// A valued flag whose value is a path safe-chains gates by locus. One spelling per entry (a
/// flag with both short and long forms is two entries); the resolver scans for each.
#[derive(Debug, Clone)]
pub(crate) struct PathFlag {
    pub short: Option<u8>,
    pub long: Option<String>,
    pub role: PathRole,
}

/// The role a path-flag's value plays — read (gated by read locus) or write (by write locus).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathRole {
    Read,
    Write,
}

/// The closed set of operand-role shapes (§ design doc: the `Operands` enum, as data).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PositionalRole {
    None,
    Read,
    Write,
    PatternThenRead,
    Transfer,
}

/// How a command's `Scale` is computed from its operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScaleModel {
    /// Every operation is a single item regardless of operand count (cat/head).
    Single,
    /// Count, glob, or a recursion flag widen it (`breadth_scale`) — rm/mkdir.
    Breadth,
}

/// Runtime form of `[command.output]`: how to derive the locus of a command's stdout.
#[derive(Debug, Clone)]
pub(crate) struct OutputSpec {
    pub locus_from: OutputLocus,
    /// Flags that void the claim (see `TomlOutput::invalidated_by`).
    pub invalidated_by: Vec<String>,
    /// Value-taking flags (see `TomlOutput::valued`).
    pub valued: Vec<String>,
    /// Flags of which at least one must be present (see `TomlOutput::requires`).
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputLocus {
    /// Beneath the command's own path operands — the worst `read_locus` over them, or the cwd
    /// when it has none (`fd pattern` with no root searches `.`).
    Operands,
    /// The working directory itself (`pwd`).
    Cwd,
    /// A subset of what it was piped (`head -1`, `sort`, `uniq`) — so the locus is the PREVIOUS
    /// pipeline stage's. Only when the command has no path operand: `head f.txt` prints the
    /// contents of a file rather than filtering a stream, and contents are not paths.
    Stdin,
    /// The output words are ATOMS: they carry no path separator, so splicing one into a path
    /// cannot move which directory the path names. `seq` prints integers; `basename` prints a
    /// single component by definition.
    ///
    /// This is a different KIND of claim from the others, which all answer "which locus does this
    /// output name". An atom names no locus at all — the point is that it cannot CHANGE one. That
    /// is what makes `for i in $(seq 1 4); do … > "$SP/dx_$i.txt"; done` confinable: the prefix is
    /// literal, and an atom spliced into the leaf cannot escape it.
    ///
    /// Separator-freedom alone is not sufficient — an atom that IS a whole component could be
    /// `..`. Confinement additionally requires the interpolation to be flanked by literal text
    /// within its component, which is a property of the PATH, not of this declaration. See
    /// `docs/design/behavioral-taxonomy-*` and the plan recorded in TODO.md.
    Atom,
}

/// A named thin resolver hook for irreducible token logic a declaration can't express — a
/// command whose operand syntax is not getopt positional (grep's pattern disambiguation, dd's
/// `key=value`, tar's dashless mode bundles, sed's mini-language script). The hook parses the
/// tokens; the facets still come from the declaration + the builders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BehaviorHook {
    Grep,
    Dd,
    Tar,
    Sed,
    Perl,
}
