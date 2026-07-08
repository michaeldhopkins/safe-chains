//! The behavioral capability model engine (`docs/design/behavioral-taxonomy-v1.4.md`).
//!
//! Dormant scaffolding: nothing in this module is wired into the live classifier
//! yet. `facet` defines the capability vocabulary (v1.4 §2); later commits add the
//! level predicate language (§4.1) and the profile-resolution engine (annex
//! `behavioral-taxonomy-engine`).

pub mod authoring;
pub mod facet;
pub mod level;
pub mod resolve;

#[cfg(test)]
mod testgen;
