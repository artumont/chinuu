//! The Rust half of chinuu
//!
//! Three modules, and one rule that ties them together. Everything inside a
//! vault is addressed by a vault-relative id: its path relative to the vault
//! root, always joined with `/`. The root itself is the empty id. An id is stable
//! across sessions and machines, and it is the same wherever the vault happens to
//! live, so it is safe to store, to send to a frontend, or to hand to git.

pub mod error;
pub mod files;
pub mod git;

pub use error::{CoreError, Result};
