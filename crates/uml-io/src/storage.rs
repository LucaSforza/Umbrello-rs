//! Storage backend abstraction (was `uml-persistence`).

use std::path::Path;

/// Trait for storage backends.
#[allow(async_fn_in_trait)]
pub trait StorageBackend {
    /// Load a model from the given path.
    async fn load(&self, _path: &Path) -> Result<(), PersistenceError> {
        Ok(())
    }
    /// Save a model to the given path.
    async fn save(&self, _path: &Path) -> Result<(), PersistenceError> {
        Ok(())
    }
}

/// Persistence error type.
#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    /// I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Unsupported format.
    #[error("Unsupported format")]
    UnsupportedFormat,
}
