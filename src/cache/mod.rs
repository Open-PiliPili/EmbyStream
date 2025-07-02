pub mod crypto;
pub mod file;

pub use crypto::{
    Cache as CryptoCache,
    CacheManager as CryptoCacheManager
};

pub use file::{
    Cache as FileCache,
    Entry as FileEntry,
    Metadata as FileMetadata,
    Error as FileCacheError
};