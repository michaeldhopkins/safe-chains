//! The transfer knobs of a `cp`/`mv`/`ln`-shaped command, as declared in TOML and as lowered.

use serde::Deserialize;

/// The differing knobs of a transfer command (`cp`/`mv`/`ln`): every source operand is read at
/// its own locus and the destination is a create/overwrite at its locus, but the source
/// *operation* and the clobber/recursion flags differ per command.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::registry) struct TomlTransfer {
    /// The source-operand operation: `observe` (cp/ln read the source into the dest/link) or
    /// `relocate` (mv removes the source from its old location).
    pub source: String,
    /// True when the destination operand is REBOUND rather than written through: `ln` points the
    /// destination name at something else. Defaults false (cp/mv write bytes at the destination).
    #[serde(default)]
    pub rebinds_destination: bool,
    /// Flags whose PRESENCE means the destination will not be overwritten (`cp`/`mv`: `-n`,
    /// `--no-clobber`). Mutually exclusive with `clobber_flags`.
    #[serde(default)]
    pub no_clobber_flags: Vec<String>,
    /// Flags whose PRESENCE means the destination WILL be overwritten, the default being
    /// no-clobber (`ln`: `-f`, `--force`). Mutually exclusive with `no_clobber_flags`.
    #[serde(default)]
    pub clobber_flags: Vec<String>,
    /// Flags whose presence widens the scale to unbounded (`cp`: `-r`/`-R`/`-a`).
    #[serde(default)]
    pub recursive_flags: Vec<String>,
    /// Flags whose presence makes the created entry a symbolic link (`ln`: `-s`, `--symbolic`), whose
    /// relative target resolves against the link's directory rather than the working directory.
    #[serde(default)]
    pub symbolic_flags: Vec<String>,
    /// Flags whose presence makes a symbolic link's target resolve as written, against the working
    /// directory (`ln`: `-r`, `--relative`).
    #[serde(default)]
    pub relative_flags: Vec<String>,
}

/// Lowered `[command.behavior.transfer]` — the per-command transfer knobs, terms resolved.
#[derive(Debug, Clone)]
pub(crate) struct TransferSpec {
    pub source: TransferSource,
    /// Whether the DESTINATION operand is a rebind rather than an ordinary write: `ln` makes the
    /// destination name refer to somewhere else, while `cp` and `mv` put bytes at or under it.
    /// Both are `create`/`transfer` to the engine, so the operation cannot tell them apart.
    pub rebinds_destination: bool,
    pub no_clobber_flags: Vec<String>,
    pub clobber_flags: Vec<String>,
    pub recursive_flags: Vec<String>,
    pub symbolic_flags: Vec<String>,
    pub relative_flags: Vec<String>,
}

/// The source-operand operation of a transfer command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferSource {
    /// cp/ln: read the source into the destination/link (no disclosure to the model).
    Observe,
    /// mv: remove the source from its old location (trivially reversible).
    Relocate,
}
