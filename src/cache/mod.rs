pub mod file;
pub mod general;

pub use general::Cache as GeneralCache;

pub use file::{
    Entry as FileEntry, Error as FileCacheError, FileCache,
    Metadata as FileMetadata,
};
