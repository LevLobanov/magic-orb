//! Implementation of **thread-safe**, **overwrite**, **heap-stored**, **fix-sized**, **locking** `Ring buffer`
//!
//! Docs will be later, sorry

#![cfg_attr(not(feature = "not_nightly"), feature(sync_unsafe_cell))]
// #![warn(missing_docs)]

#[cfg(feature = "not_nightly")]
mod sync_unsafe_cell;

pub mod magic_orb;
pub use magic_orb::MagicOrb;
