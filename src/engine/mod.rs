//! The behavioral capability model engine (`docs/design/behavioral-taxonomy-v1.4.md`).
//!
//! `facet` is the capability vocabulary (v1.4 §2), `level` the predicate language
//! (§4.1), `authoring` compiles the level TOML, `resolve` turns a command into a
//! profile, and `bridge` runs the engine alongside the legacy classifier behind
//! `SAFE_CHAINS_ENGINE` (default `legacy` — the engine does not run). See the
//! `behavioral-taxonomy-engine` annex.

pub mod authoring;
pub mod bridge;
pub mod facet;
pub mod level;
pub mod resolve;

#[cfg(test)]
mod testgen;
