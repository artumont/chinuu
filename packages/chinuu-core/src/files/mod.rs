//! File operations on a vault: index it, read, write and delete notes, and watch
//! it for changes.
//!
//! One rule holds the module together. Everything inside a vault is addressed by
//! a **vault-relative id**: its path relative to the vault root, always joined
//! with `/`. The root itself is the empty id, [`ROOT_ID`]. An id is stable
//! across sessions and machines, unlike an inode number, and it is the same
//! whether the vault sits at `/home/me/vault` or `D:\Notes`. The indexer
//! produces these ids and the watcher reports them. The git layer needs paths of
//! the same shape, because repository paths are relative and separator-stable as
//! well.
//!
//! The module has two halves. [`index_directory`] and [`index_cwd`] walk a vault
//! into a tree, [`read_file`] and [`write_file`] act on a single path, which is
//! normally the vault root joined with an id, and [`delete_file`] and
//! [`delete_folder`] remove one. [`VaultWatcher`] is the long-lived half: it
//! reports changes as they happen, so a UI can patch its tree instead of
//! polling.
//!
//! Indexing and watching agree about which paths are hidden, because both match
//! through [`IgnoreRules`]: the built-in defaults plus the vault's own
//! [`IGNORE_FILE_NAME`]. A path missing from the tree cannot arrive as an event.
//!
//! Deleting is the only destructive operation here. [`delete_folder`] removes a
//! folder and everything inside it, and neither delete function follows a
//! symbolic link, so a link is unlinked where it stands rather than reaching its
//! target.
//!
//! Everything here blocks, and nothing depends on an async runtime. The watcher
//! runs its callback on a thread of its own, so a caller that has to stay
//! responsive moves work to a background thread without this module knowing
//! anything about it.
//!
//! Two limits are worth knowing before relying on this. Indexing returns a
//! snapshot, so there is no incremental update. And the ignore rules come from a
//! single file at the vault root, so a vault cannot vary them per directory.

// Submodules are private so every item has one public path, `files::Type`.
mod delete;
mod ignore;
mod index;
mod read;
mod types;
mod watcher;
mod write;

pub use delete::{delete_file, delete_folder};
pub use ignore::{IgnoreRules, DEFAULT_PATTERNS, IGNORE_FILE_NAME};
pub use index::{index_cwd, index_directory, index_directory_with};
pub use read::{read_bytes, read_file};
pub use types::{FsIndex, FsNode, ROOT_ID};
pub use watcher::{
    VaultWatcher, WatchEvent, WatchEventKind, WatchFailure, WatchOptions, WatchUpdate,
    DEFAULT_DEBOUNCE,
};
pub use write::{create_folder, move_path, write_bytes, write_file, TEMP_PREFIX};
