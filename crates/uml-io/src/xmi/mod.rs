//! XMI persistence for Umbrello-RS.
//!
//! Reads legacy Umbrello C++ XMI 1.2 files and populates `UmlModel`.
//! Uses a two-pass strategy: Pass 1 extracts structural elements,
//! Pass 2 resolves cross-references (stereotype IDs).

pub mod error;
pub mod reader;
pub mod writer;

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use uml_core::UmlModel;

pub use error::XmiParseError;
pub use reader::XmiReader;
pub use writer::{XmiWriteError, XmiWriter};

/// Save a `UmlModel` to an XMI file at the given path.
///
/// This is a convenience wrapper around `XmiWriter::write_document`.
///
/// # Errors
///
/// Returns `XmiWriteError` if the file cannot be created or the XMI
/// serialization fails.
pub fn save_xmi_to_file(model: &UmlModel, path: &Path) -> Result<(), XmiWriteError> {
    let file = File::create(path)?;
    let mut writer = XmiWriter::new(BufWriter::new(file));
    writer.write_document(model)?;
    Ok(())
}

/// Load an XMI file from the given path into a fresh `UmlModel`.
///
/// Returns the populated model on success.
///
/// # Errors
///
/// Returns `XmiParseError` if the file cannot be opened or parsed.
pub fn load_xmi_from_file(path: &Path) -> Result<UmlModel, XmiParseError> {
    let file = File::open(path)?;
    let mut model = UmlModel::new();
    let mut reader = XmiReader::new();
    reader.read_from(BufReader::new(file), &mut model)?;
    reader.resolve(&mut model)?;
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uml_core::Class;
    use uml_core::ModelElement;

    #[test]
    fn save_xmi_to_file_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_roundtrip.xmi");

        // Create a model with one class
        let mut model = UmlModel::new();
        let cls = Class::new("TestClass");
        model.insert(ModelElement::Class(cls));

        // Save it
        save_xmi_to_file(&model, &path).expect("save should succeed");

        // Load it back
        let loaded = load_xmi_from_file(&path).expect("load should succeed");

        // Verify — loaded model contains the class we saved
        assert!(!loaded.is_empty());
        assert!(loaded.iter().any(|(_, e)| e.name() == "TestClass"));

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_xmi_to_file_error_on_bad_path() {
        let model = UmlModel::new();
        let bad_path = Path::new("/nonexistent_directory_xyzzy/test.xmi");
        let result = save_xmi_to_file(&model, bad_path);
        assert!(result.is_err());
    }
}
