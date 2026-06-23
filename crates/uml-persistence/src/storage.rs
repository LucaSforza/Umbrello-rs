//! Storage backend abstraction.

use std::path::Path;

/// Trait for storage backends.
///
/// Each implementation handles a specific file format (XMI, Rose, ArgoUML, etc.).
/// The `StorageBackend` trait allows the load/save pipeline to dispatch to the
/// correct format handler without knowing format details.
#[allow(async_fn_in_trait)]
pub trait StorageBackend {
    /// Load a model from the given path.
    async fn load(&self, _path: &Path) -> Result<(), crate::PersistenceError> {
        Ok(())
    }

    /// Save a model to the given path.
    async fn save(&self, _path: &Path) -> Result<(), crate::PersistenceError> {
        Ok(())
    }
}
