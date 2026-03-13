//! Centralized configuration constants for Fire Notes
//!
//! All magic numbers and tunable parameters should be defined here.
//! Sub-modules group constants by concern; this file re-exports them
//! all so that existing `crate::config::layout::FOO` paths continue to work.

pub mod cursor;
pub mod flame;
pub mod layout;
pub mod rendering;
pub mod scroll;
pub mod timing;
