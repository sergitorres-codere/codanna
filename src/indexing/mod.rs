pub mod file_info;
pub mod fs_watcher;
pub mod progress;
pub mod resolution_context;
pub mod resolver;
pub mod simple;
pub mod trait_resolver;
pub mod transaction;
pub mod walker;

#[cfg(test)]
mod cutover_test;

pub use file_info::{FileInfo, calculate_hash, get_utc_timestamp};
pub use fs_watcher::{FileSystemWatcher, WatchError};
pub use progress::IndexStats;
pub use resolution_context::{ResolutionContext, ScopedSymbol};
pub use resolver::{Import, ImportResolver};
pub use simple::SimpleIndexer;
pub use trait_resolver::TraitResolver;
pub use transaction::{FileTransaction, IndexTransaction};
pub use walker::FileWalker;
