//! Core types for Mina.
//!
//! This is an internal crate that exists primarily to support Mina's proc macros, and should not be
//! used directly. All important types are re-exported by Mina.

pub mod animator;
pub mod easing;
pub mod interpolation;
pub mod time_scale;
pub mod timeline;
pub mod timeline_helpers;
