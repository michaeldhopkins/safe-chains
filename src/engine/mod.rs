//! The behavioral capability model engine (`docs/design/behavioral-taxonomy-v1.4.md`).
//!
//! `facet` is the capability vocabulary (v1.4 §2), `level` the predicate language
//! (§4.1), `authoring` compiles the level TOML, `resolve` turns a command into a
//! profile, and `bridge` runs the engine AUTHORITATIVELY: `leaf_verdict` uses
//! `engine_verdict(tokens).unwrap_or(legacy)`, so for every command the engine can
//! resolve, its verdict wins — the legacy classifier is only the fallback. See the
//! `behavioral-taxonomy-engine` annex.

pub mod archetype;
pub mod authoring;
pub mod bridge;
pub mod facet;
pub mod level;
pub mod resolve;

#[cfg(test)]
mod testgen;
