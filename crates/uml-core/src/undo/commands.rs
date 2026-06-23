//! Concrete command implementations for model mutations.

use crate::elements::ModelElement;
use crate::id::UmlId;
use crate::repository::UmlModel;

use super::{Command, CommandError};

/// Command to create a new element in the model.
///
/// On execute: inserts the element. On undo: removes it.
/// The element is stored inside the command between execute/undo for restoration.
#[derive(Debug)]
pub struct CreateElement {
    element: Option<ModelElement>,
    element_id: UmlId,
    description: String,
}

impl CreateElement {
    /// Create a command that will insert the given element.
    #[must_use]
    pub fn new(element: ModelElement) -> Self {
        let id = element.id();
        let desc = format!("Create {} '{}'", element.object_type().as_str(), element.name());
        Self {
            element: Some(element),
            element_id: id,
            description: desc,
        }
    }
}

impl Command for CreateElement {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = self.element.take().ok_or_else(|| {
            CommandError::InvalidOperation("CreateElement already executed".into())
        })?;
        model.insert(elem);
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        self.element = model.remove(self.element_id);
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Command to delete an element from the model.
///
/// On execute: removes the element, storing it internally.
/// On undo: re-inserts the element with its original UmlId.
#[derive(Debug)]
pub struct DeleteElement {
    element: Option<ModelElement>,
    element_id: UmlId,
    description: String,
}

impl DeleteElement {
    /// Create a command that will delete the element with the given ID.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::ElementNotFound` if the element does not exist.
    pub fn new(model: &UmlModel, id: UmlId) -> Result<Self, CommandError> {
        let elem = model.get(id).ok_or(CommandError::ElementNotFound(id))?;
        let desc = format!("Delete {} '{}'", elem.object_type().as_str(), elem.name());
        Ok(Self {
            element: None,
            element_id: id,
            description: desc,
        })
    }
}

impl Command for DeleteElement {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        self.element = model.remove(self.element_id);
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = self
            .element
            .take()
            .ok_or_else(|| CommandError::InvalidOperation("DeleteElement already undone".into()))?;
        model.insert(elem);
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Command to rename an element.
///
/// Stores both the old and new names. Re-applying is idempotent
/// (sets to new_name), undoing sets back to old_name.
#[derive(Debug)]
pub struct RenameElement {
    element_id: UmlId,
    old_name: String,
    new_name: String,
    description: String,
}

impl RenameElement {
    /// Create a command that will rename the element.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::ElementNotFound` if the element does not exist.
    pub fn new(model: &UmlModel, id: UmlId, new_name: String) -> Result<Self, CommandError> {
        let elem = model.get(id).ok_or(CommandError::ElementNotFound(id))?;
        let old_name = elem.name().to_string();
        let desc = format!("Rename '{old_name}' → '{new_name}'");
        Ok(Self {
            element_id: id,
            old_name,
            new_name,
            description: desc,
        })
    }
}

impl Command for RenameElement {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = model
            .get_mut(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        elem.set_name(self.new_name.clone());
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = model
            .get_mut(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        elem.set_name(self.old_name.clone());
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Command to move an element between packages.
///
/// Tracks the source and destination package. On execute, moves from
/// source to destination. On undo, moves back.
#[derive(Debug)]
pub struct MoveElement {
    element_id: UmlId,
    from_package: Option<UmlId>,
    to_package: Option<UmlId>,
    description: String,
}

impl MoveElement {
    /// Create a command that will move the element to a new package.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::ElementNotFound` if any element does not exist.
    pub fn new(
        model: &UmlModel,
        element_id: UmlId,
        to_package: Option<UmlId>,
    ) -> Result<Self, CommandError> {
        let elem = model
            .get(element_id)
            .ok_or(CommandError::ElementNotFound(element_id))?;
        let from_package = model
            .parents_of(element_id)
            .and_then(|p| p.first().copied());
        let to_name = to_package
            .and_then(|id| model.get(id))
            .map_or_else(|| "root".to_string(), |e| e.name().to_string());
        let desc = format!("Move '{}' to '{}'", elem.name(), to_name);
        Ok(Self {
            element_id,
            from_package,
            to_package,
            description: desc,
        })
    }
}

impl Command for MoveElement {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        // Remove from current parent
        if let Some(from) = self.from_package {
            let _ = model.remove_from_package(from, self.element_id);
        }
        // Add to new parent
        if let Some(to) = self.to_package {
            model
                .add_to_package(to, self.element_id)
                .map_err(CommandError::Model)?;
        }
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        // Remove from destination
        if let Some(to) = self.to_package {
            let _ = model.remove_from_package(to, self.element_id);
        }
        // Add back to source
        if let Some(from) = self.from_package {
            model
                .add_to_package(from, self.element_id)
                .map_err(CommandError::Model)?;
        }
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::needless_pass_by_value)]
mod tests {
    use super::*;
    use crate::elements::{Class, Package};

    #[test]
    fn create_element_description() {
        let cmd = CreateElement::new(ModelElement::Class(Class::new("Person")));
        assert!(cmd.description().contains("Person"));
        assert!(cmd.description().contains("Create"));
    }

    #[test]
    fn delete_element_from_model() {
        let mut model = UmlModel::new();
        let cls = ModelElement::Class(Class::new("Test"));
        let id = cls.id();
        model.insert(cls);

        let mut cmd = DeleteElement::new(&model, id).unwrap();
        cmd.execute(&mut model).unwrap();
        assert!(!model.contains(id));
        cmd.undo(&mut model).unwrap();
        assert!(model.contains(id));
    }

    #[test]
    fn rename_element_roundtrip() {
        let mut model = UmlModel::new();
        let cls = ModelElement::Class(Class::new("Original"));
        let id = cls.id();
        model.insert(cls);

        let mut cmd = RenameElement::new(&model, id, "NewName".into()).unwrap();
        cmd.execute(&mut model).unwrap();
        assert_eq!(model.get(id).unwrap().name(), "NewName");

        cmd.undo(&mut model).unwrap();
        assert_eq!(model.get(id).unwrap().name(), "Original");
    }

    #[test]
    fn move_element_between_packages() {
        let mut model = UmlModel::new();

        let pkg1 = ModelElement::Package(Package::new("Pkg1"));
        let pkg1_id = pkg1.id();
        model.insert(pkg1);

        let pkg2 = ModelElement::Package(Package::new("Pkg2"));
        let pkg2_id = pkg2.id();
        model.insert(pkg2);

        let cls = ModelElement::Class(Class::new("Thing"));
        let cls_id = cls.id();
        model.insert(cls);
        model.add_to_package(pkg1_id, cls_id).unwrap();

        let mut cmd = MoveElement::new(&model, cls_id, Some(pkg2_id)).unwrap();
        cmd.execute(&mut model).unwrap();
        assert_eq!(model.parents_of(cls_id), Some(&[pkg2_id][..]));

        cmd.undo(&mut model).unwrap();
        assert_eq!(model.parents_of(cls_id), Some(&[pkg1_id][..]));
    }
}
