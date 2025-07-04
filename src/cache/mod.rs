pub mod general;
pub mod file;

pub use general::{
    Cache as GeneralCache
};

pub use file::{
    FileCache,
    Entry as FileEntry,
    Metadata as FileMetadata,
    Error as FileCacheError
};