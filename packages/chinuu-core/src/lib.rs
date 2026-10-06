//! The Rust half of chinuu: a note vault on disk.
//!
//! Three modules, and one rule that ties them together. Everything inside a
//! vault is addressed by a **vault-relative id**: its path relative to the vault
//! root, always joined with `/`. The root itself is the empty id. An id is stable
//! across sessions and machines, and it is the same wherever the vault happens to
//! live, so it is safe to store, to send to a frontend, or to hand to git.
//!
//! | Module | What it does |
//! | ------ | ------------ |
//! | [`files`] | Index a vault, read and write notes, move and delete them, watch for changes, and decide which paths to ignore. |
//! | [`git`] | Open, inspect, stage, commit, fetch, pull and push, through libgit2. |
//! | [`error`] | [`CoreError`], one error type for the crate, with a stable machine-readable code per variant. |
//!
//! Everything here blocks, and nothing depends on an async runtime, so a caller
//! is free to pick its own. The two long-lived pieces, [`files::VaultWatcher`]
//! and [`git::GitRepo`], are not meant to be shared between threads: a watcher
//! owns a thread and a repository handle is not `Sync`, so open one per task.
//!
//! # Reading and writing
//!
//! Bytes are never transformed. [`files::read_file`] returns exactly what is on
//! disk and [`files::write_file`] writes exactly what it is given, with no
//! newline translation, no trailing newline added and no byte order mark
//! stripped. Writing goes through a temporary file and a rename, so a crash
//! cannot leave a half-written note behind. That pair of promises is the
//! contract the editor package depends on, and `tests/editing.rs` pins it.
//!
//! # Serialization
//!
//! A frontend talking to this crate over IPC wants the data types as JSON, which
//! is opt-in:
//!
//! ```toml
//! chinuu-core = { path = "../packages/chinuu-core", features = ["serde"] }
//! ```
//!
//! With it on, the public data types implement `serde::Serialize`, and
//! [`CoreError`] serializes as `{ "code", "message", "path" }` so a frontend can
//! branch on the code instead of parsing the message. [`git::Auth`] is left out
//! on purpose, because it holds credentials.

pub mod error;
pub mod files;
pub mod git;

pub use error::{CoreError, Result};
