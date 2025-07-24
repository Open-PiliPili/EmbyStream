pub mod general;
pub mod metadata;

pub use general::Cache as GeneralCache;

pub use metadata::{
    Error as MetadataCacheError, Metadata as FileMetadata, MetadataCache,
};
