//! Concrete command implementations for model mutations.

use crate::elements::{ClassifierData, ModelElement, Relationship};
use crate::id::UmlId;
use crate::repository::UmlModel;
use crate::types::{AssociationType, Visibility};

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

// ─── Property editing commands ────────────────────────────────────

/// Command to change an element's visibility level.
#[derive(Debug)]
pub struct ChangeVisibility {
    element_id: UmlId,
    old_visibility: Visibility,
    new_visibility: Visibility,
    description: String,
}

impl ChangeVisibility {
    /// Create a command that will change the visibility of the element.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::ElementNotFound` if the element does not exist.
    pub fn new(model: &UmlModel, id: UmlId, visibility: Visibility) -> Result<Self, CommandError> {
        let elem = model.get(id).ok_or(CommandError::ElementNotFound(id))?;
        let old_visibility = elem.base().visibility;
        let desc = format!(
            "Change visibility of '{}': {} → {}",
            elem.name(),
            old_visibility.as_str(),
            visibility.as_str(),
        );
        Ok(Self {
            element_id: id,
            old_visibility,
            new_visibility: visibility,
            description: desc,
        })
    }
}

impl Command for ChangeVisibility {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = model
            .get_mut(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        elem.base_mut().visibility = self.new_visibility;
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = model
            .get_mut(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        elem.base_mut().visibility = self.old_visibility;
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Command to toggle abstract/static flags on an element.
///
/// Both flags are set atomically in a single command so that a pair of
/// rapid checkbox toggles merges cleanly.
#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct ChangeElementFlags {
    element_id: UmlId,
    is_abstract: bool,
    is_static: bool,
    old_abstract: bool,
    old_static: bool,
    description: String,
}

impl ChangeElementFlags {
    /// Create a command that will change the abstract and static flags.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::ElementNotFound` if the element does not exist.
    pub fn new(
        model: &UmlModel,
        id: UmlId,
        is_abstract: bool,
        is_static: bool,
    ) -> Result<Self, CommandError> {
        let elem = model.get(id).ok_or(CommandError::ElementNotFound(id))?;
        let old_abstract = elem.base().is_abstract;
        let old_static = elem.base().is_static;
        let desc = format!(
            "Set flags of '{}': abstract={}, static={}",
            elem.name(),
            is_abstract,
            is_static,
        );
        Ok(Self {
            element_id: id,
            is_abstract,
            is_static,
            old_abstract,
            old_static,
            description: desc,
        })
    }
}

impl Command for ChangeElementFlags {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = model
            .get_mut(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        let base = elem.base_mut();
        base.is_abstract = self.is_abstract;
        base.is_static = self.is_static;
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = model
            .get_mut(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        let base = elem.base_mut();
        base.is_abstract = self.old_abstract;
        base.is_static = self.old_static;
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Command to change an element's documentation text.
#[derive(Debug)]
pub struct ChangeDocumentation {
    element_id: UmlId,
    old_documentation: String,
    new_documentation: String,
    description: String,
}

impl ChangeDocumentation {
    /// Create a command that will change the documentation of the element.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::ElementNotFound` if the element does not exist.
    pub fn new(model: &UmlModel, id: UmlId, documentation: String) -> Result<Self, CommandError> {
        let elem = model.get(id).ok_or(CommandError::ElementNotFound(id))?;
        let old_documentation = elem.base().documentation.clone();
        let desc = format!("Change documentation of '{}'", elem.name(),);
        Ok(Self {
            element_id: id,
            old_documentation,
            new_documentation: documentation,
            description: desc,
        })
    }
}

impl Command for ChangeDocumentation {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = model
            .get_mut(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        elem.base_mut()
            .documentation
            .clone_from(&self.new_documentation);
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = model
            .get_mut(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        elem.base_mut()
            .documentation
            .clone_from(&self.old_documentation);
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

// ─── Diagram visual commands ─────────────────────────────────────

use crate::diagram::{
    Diagram, DiagramId, EdgeId, LineRouting, Point, Rect, Size, ViewEdge, ViewNode,
};

/// Command to create a diagram at a deterministic position in the model.
///
/// The complete diagram is retained as a snapshot. Undo and redo therefore
/// restore not only the diagram metadata, but also its nodes and edges.
#[derive(Debug)]
pub struct CreateDiagram {
    diagram: Diagram,
    position: usize,
    applied: bool,
    description: String,
}

impl CreateDiagram {
    /// Create a command that appends `diagram` to `model`.
    ///
    /// The append position is captured when the command is constructed, so
    /// redo restores the same insertion position even when other diagrams
    /// follow it.
    ///
    /// # Errors
    ///
    /// Returns an error if a diagram with the same ID already exists.
    pub fn new(model: &UmlModel, diagram: Diagram) -> Result<Self, CommandError> {
        Self::new_at(model, diagram, model.diagrams().len())
    }

    /// Create a command that inserts `diagram` at `position`.
    ///
    /// # Errors
    ///
    /// Returns an error if the position is outside the current diagram list
    /// or if a diagram with the same ID already exists.
    pub fn new_at(
        model: &UmlModel,
        diagram: Diagram,
        position: usize,
    ) -> Result<Self, CommandError> {
        if position > model.diagrams().len() {
            return Err(CommandError::InvalidOperation(
                "diagram insertion position is out of range".into(),
            ));
        }
        if model.get_diagram(diagram.id).is_some() {
            return Err(CommandError::InvalidOperation("diagram ID already exists".into()));
        }
        Ok(Self {
            description: format!("Create diagram '{}'", diagram.name),
            diagram,
            position,
            applied: false,
        })
    }
}

impl Command for CreateDiagram {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        if self.applied {
            return Err(CommandError::InvalidOperation("CreateDiagram already executed".into()));
        }
        if self.position > model.diagrams().len() {
            return Err(CommandError::InvalidOperation(
                "diagram insertion position is out of range".into(),
            ));
        }
        if model.get_diagram(self.diagram.id).is_some() {
            return Err(CommandError::InvalidOperation("diagram ID already exists".into()));
        }

        let mut diagrams = model.diagrams().to_vec();
        diagrams.insert(self.position, self.diagram.clone());
        replace_diagrams(model, diagrams);
        self.applied = true;
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        if !self.applied {
            return Err(CommandError::InvalidOperation(
                "CreateDiagram has not been executed".into(),
            ));
        }
        let Some(current) = model.diagrams().get(self.position) else {
            return Err(CommandError::InvalidOperation("created diagram is missing".into()));
        };
        if current.id != self.diagram.id {
            return Err(CommandError::InvalidOperation(
                "created diagram is not at its deterministic position".into(),
            ));
        }

        let mut diagrams = model.diagrams().to_vec();
        let removed = diagrams.remove(self.position);
        replace_diagrams(model, diagrams);
        self.diagram = removed;
        self.applied = false;
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Command to replace the editable semantics of an existing relationship.
///
/// Relationship identity and endpoints are immutable through this command.
/// The replacement's `original_xmi_id` is discarded in favor of the value
/// captured from the model, preserving XMI identity across edits.
#[derive(Debug)]
pub struct UpdateRelationship {
    relationship_id: UmlId,
    old: Relationship,
    new: Relationship,
    applied: bool,
    description: String,
}

impl UpdateRelationship {
    /// Create a command for replacing one relationship's editable fields.
    ///
    /// # Errors
    ///
    /// Returns an error when `relationship_id` is absent, does not identify a
    /// relationship, or when the replacement changes its ID or endpoints.
    pub fn new(
        model: &UmlModel,
        relationship_id: UmlId,
        mut replacement: Relationship,
    ) -> Result<Self, CommandError> {
        let current = model
            .get(relationship_id)
            .ok_or(CommandError::ElementNotFound(relationship_id))?;
        let ModelElement::Relationship(old) = current else {
            return Err(CommandError::InvalidOperation("element is not a relationship".into()));
        };
        if replacement.base.id != old.base.id
            || replacement.source_id != old.source_id
            || replacement.target_id != old.target_id
        {
            return Err(CommandError::InvalidOperation(
                "relationship ID or endpoints cannot be changed".into(),
            ));
        }
        replacement
            .base
            .original_xmi_id
            .clone_from(&old.base.original_xmi_id);
        Ok(Self {
            relationship_id,
            description: format!("Update relationship '{}'", old.base.name),
            old: old.clone(),
            new: replacement,
            applied: false,
        })
    }
}

impl Command for UpdateRelationship {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        if self.applied {
            return Err(CommandError::InvalidOperation(
                "UpdateRelationship already executed".into(),
            ));
        }
        replace_relationship(model, self.relationship_id, &self.old, &self.new)?;
        self.applied = true;
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        if !self.applied {
            return Err(CommandError::InvalidOperation(
                "UpdateRelationship has not been executed".into(),
            ));
        }
        replace_relationship(model, self.relationship_id, &self.new, &self.old)?;
        self.applied = false;
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

fn replace_diagrams(model: &mut UmlModel, diagrams: Vec<Diagram>) {
    let ids: Vec<_> = model.diagrams().iter().map(|diagram| diagram.id).collect();
    for id in ids {
        let _ = model.remove_diagram(id);
    }
    for diagram in diagrams {
        model.add_diagram(diagram);
    }
}

fn replace_relationship(
    model: &mut UmlModel,
    id: UmlId,
    expected: &Relationship,
    replacement: &Relationship,
) -> Result<(), CommandError> {
    let element = model.get_mut(id).ok_or(CommandError::ElementNotFound(id))?;
    let ModelElement::Relationship(current) = element else {
        return Err(CommandError::InvalidOperation("element is not a relationship".into()));
    };
    if current != expected {
        return Err(CommandError::InvalidOperation("relationship was modified externally".into()));
    }
    *current = replacement.clone();
    Ok(())
}

/// Command to replace the editable classifier features of an existing classifier.
///
/// Classifier identity, element base fields, and templates are preserved.
/// The command may change only `attributes` and `operations`.
/// Follows the optimistic snapshot pattern used by [`UpdateRelationship`]:
/// execute/undo verify the currently stored classifier data matches the
/// expected snapshot before replacing it.
///
/// # Errors
///
/// Construction fails when:
/// - `classifier_id` is absent
/// - The element is not a classifier (Class, Interface, Enum, or Datatype)
/// - The replacement's `templates` differ from the current templates
///
/// Execute/undo fail atomically when:
/// - The element is missing or no longer a classifier
/// - The classifier data was modified externally (stale snapshot)
/// - The command has already been executed / not yet executed
#[derive(Debug)]
pub struct UpdateClassifierFeatures {
    element_id: UmlId,
    old: ClassifierData,
    new: ClassifierData,
    applied: bool,
    description: String,
}

impl UpdateClassifierFeatures {
    /// Create a command for replacing one classifier's attributes and operations.
    ///
    /// # Errors
    ///
    /// Returns an error when `classifier_id` is absent, the element is not a
    /// classifier, or `replacement.templates` differs from the current templates.
    pub fn new(
        model: &UmlModel,
        classifier_id: UmlId,
        replacement: ClassifierData,
    ) -> Result<Self, CommandError> {
        let current = model
            .get(classifier_id)
            .ok_or(CommandError::ElementNotFound(classifier_id))?;
        let old = current
            .classifier_data()
            .ok_or_else(|| CommandError::InvalidOperation("element is not a classifier".into()))?
            .clone();
        if replacement.templates != old.templates {
            return Err(CommandError::InvalidOperation(
                "classifier templates cannot be changed through this command".into(),
            ));
        }
        Ok(Self {
            element_id: classifier_id,
            description: format!("Update classifier '{}'", current.name()),
            old,
            new: replacement,
            applied: false,
        })
    }
}

impl Command for UpdateClassifierFeatures {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        if self.applied {
            return Err(CommandError::InvalidOperation(
                "UpdateClassifierFeatures already executed".into(),
            ));
        }
        replace_classifier_features(model, self.element_id, &self.old, &self.new)?;
        self.applied = true;
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        if !self.applied {
            return Err(CommandError::InvalidOperation(
                "UpdateClassifierFeatures has not been executed".into(),
            ));
        }
        replace_classifier_features(model, self.element_id, &self.new, &self.old)?;
        self.applied = false;
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

fn replace_classifier_features(
    model: &mut UmlModel,
    id: UmlId,
    expected: &ClassifierData,
    replacement: &ClassifierData,
) -> Result<(), CommandError> {
    let element = model.get_mut(id).ok_or(CommandError::ElementNotFound(id))?;
    let current = element
        .classifier_data_mut()
        .ok_or_else(|| CommandError::InvalidOperation("element is not a classifier".into()))?;
    if current != expected {
        return Err(CommandError::InvalidOperation(
            "classifier features were modified externally".into(),
        ));
    }
    *current = replacement.clone();
    Ok(())
}

/// Atomically create a model element and its visual node in a diagram.
///
/// The constructor validates the diagram and IDs before the command is added
/// to history. The command then performs both insertions as one operation, so
/// a failed execution cannot leave a model element without its node.
#[derive(Debug)]
pub struct CreateElementWithNode {
    diagram_id: DiagramId,
    element_id: UmlId,
    element: Option<ModelElement>,
    position: Point,
    size: Size,
    description: String,
}

impl CreateElementWithNode {
    /// Create a command for inserting `element` and a node at `position`.
    ///
    /// # Errors
    ///
    /// Returns an error if the diagram is missing, the element ID already
    /// exists, or the diagram already contains a node for that ID.
    pub fn new(
        model: &UmlModel,
        diagram_id: DiagramId,
        element: ModelElement,
        position: Point,
        size: Size,
    ) -> Result<Self, CommandError> {
        let element_id = element.id();
        let diagram = model
            .get_diagram(diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        if model.contains(element_id) {
            return Err(CommandError::InvalidOperation("element ID already exists".into()));
        }
        if diagram.get_node(element_id).is_some() {
            return Err(CommandError::InvalidOperation(
                "diagram already contains a node for element".into(),
            ));
        }
        let description = format!("Create {} with node", element.object_type().as_str());
        Ok(Self {
            diagram_id,
            element_id,
            element: Some(element),
            position,
            size,
            description,
        })
    }
}

impl Command for CreateElementWithNode {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        if model.contains(self.element_id) {
            return Err(CommandError::InvalidOperation("element ID already exists".into()));
        }
        let element = self.element.take().ok_or_else(|| {
            CommandError::InvalidOperation("CreateElementWithNode already executed".into())
        })?;
        let diagram = model
            .get_diagram_mut(self.diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        if diagram.get_node(self.element_id).is_some() {
            self.element = Some(element);
            return Err(CommandError::InvalidOperation(
                "diagram already contains a node for element".into(),
            ));
        }
        model.insert(element);
        let Some(diagram) = model.get_diagram_mut(self.diagram_id) else {
            // Keep this rollback even though the precondition check above
            // makes the branch unreachable for the current model API.
            self.element = model.remove(self.element_id);
            return Err(CommandError::InvalidOperation("diagram not found".into()));
        };
        diagram.add_node(
            self.element_id,
            ViewNode::new(
                self.element_id,
                Rect::new(self.position.x, self.position.y, self.size.width, self.size.height),
            ),
        );
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let diagram = model
            .get_diagram_mut(self.diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        if diagram.remove_node(self.element_id).is_none() {
            return Err(CommandError::ElementNotFound(self.element_id));
        }
        self.element = model.remove(self.element_id);
        if self.element.is_none() {
            return Err(CommandError::ElementNotFound(self.element_id));
        }
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Command to add a node to a diagram.
#[derive(Debug)]
pub struct AddNodeToDiagram {
    diagram_id: DiagramId,
    element_id: UmlId,
    position: Point,
    size: Size,
    description: String,
}

impl AddNodeToDiagram {
    /// Create a command to add a node to a diagram.
    #[must_use]
    pub fn new(diagram_id: DiagramId, element_id: UmlId, position: Point, size: Size) -> Self {
        Self {
            diagram_id,
            element_id,
            position,
            size,
            description: "Add node to diagram".to_string(),
        }
    }
}

impl Command for AddNodeToDiagram {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let d = model
            .get_diagram_mut(self.diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        d.add_node(
            self.element_id,
            ViewNode::new(
                self.element_id,
                Rect::new(self.position.x, self.position.y, self.size.width, self.size.height),
            ),
        );
        Ok(())
    }
    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let d = model
            .get_diagram_mut(self.diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        d.remove_node(self.element_id);
        Ok(())
    }
    fn description(&self) -> &str {
        &self.description
    }
}

/// Command to remove a node from a diagram.
#[derive(Debug)]
pub struct RemoveNodeFromDiagram {
    diagram_id: DiagramId,
    element_id: UmlId,
    removed_node: Option<ViewNode>,
    description: String,
}

impl RemoveNodeFromDiagram {
    /// Create a command to remove a node from a diagram.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::ElementNotFound` if the node does not exist
    /// or `CommandError::InvalidOperation` if the diagram is not found.
    pub fn new(
        model: &UmlModel,
        diagram_id: DiagramId,
        element_id: UmlId,
    ) -> Result<Self, CommandError> {
        let d = model
            .get_diagram(diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        d.get_node(element_id)
            .ok_or(CommandError::ElementNotFound(element_id))?;
        Ok(Self {
            diagram_id,
            element_id,
            removed_node: None,
            description: "Remove node from diagram".to_string(),
        })
    }
}

impl Command for RemoveNodeFromDiagram {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let d = model
            .get_diagram_mut(self.diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        self.removed_node = d.remove_node(self.element_id);
        Ok(())
    }
    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let d = model
            .get_diagram_mut(self.diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        if let Some(node) = self.removed_node.take() {
            d.add_node(self.element_id, node);
        }
        Ok(())
    }
    fn description(&self) -> &str {
        &self.description
    }
}

/// Command to move a node on a diagram.
#[derive(Debug)]
pub struct MoveNode {
    diagram_id: DiagramId,
    element_id: UmlId,
    old_position: Option<Point>,
    new_position: Point,
    description: String,
}

impl MoveNode {
    /// Create a command to move a node on a diagram.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::ElementNotFound` if the node does not exist
    /// or `CommandError::InvalidOperation` if the diagram is not found.
    pub fn new(
        model: &UmlModel,
        diagram_id: DiagramId,
        element_id: UmlId,
        new_position: Point,
    ) -> Result<Self, CommandError> {
        let d = model
            .get_diagram(diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        d.get_node(element_id)
            .ok_or(CommandError::ElementNotFound(element_id))?;
        Ok(Self {
            diagram_id,
            element_id,
            old_position: None,
            new_position,
            description: format!("Move node to ({:.0}, {:.0})", new_position.x, new_position.y),
        })
    }
}

impl Command for MoveNode {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let d = model
            .get_diagram_mut(self.diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        let node = d
            .get_node_mut(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        self.old_position = Some(node.bounds.origin);
        node.bounds.origin = self.new_position;
        Ok(())
    }
    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let d = model
            .get_diagram_mut(self.diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        let node = d
            .get_node_mut(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        if let Some(old) = self.old_position {
            node.bounds.origin = old;
        }
        Ok(())
    }
    fn description(&self) -> &str {
        &self.description
    }
}

/// Command to resize a node on a diagram.
#[derive(Debug)]
pub struct ResizeNode {
    diagram_id: DiagramId,
    element_id: UmlId,
    old_size: Option<Size>,
    new_size: Size,
    description: String,
}

impl ResizeNode {
    /// Create a command to resize a node on a diagram.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::ElementNotFound` if the node does not exist
    /// or `CommandError::InvalidOperation` if the diagram is not found.
    pub fn new(
        model: &UmlModel,
        diagram_id: DiagramId,
        element_id: UmlId,
        new_size: Size,
    ) -> Result<Self, CommandError> {
        let d = model
            .get_diagram(diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        d.get_node(element_id)
            .ok_or(CommandError::ElementNotFound(element_id))?;
        Ok(Self {
            diagram_id,
            element_id,
            old_size: None,
            new_size,
            description: format!("Resize node to {}×{}", new_size.width, new_size.height),
        })
    }
}

impl Command for ResizeNode {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let d = model
            .get_diagram_mut(self.diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        let node = d
            .get_node_mut(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        self.old_size = Some(node.bounds.size);
        node.bounds.size = self.new_size;
        Ok(())
    }
    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let d = model
            .get_diagram_mut(self.diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        let node = d
            .get_node_mut(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        if let Some(old) = self.old_size {
            node.bounds.size = old;
        }
        Ok(())
    }
    fn description(&self) -> &str {
        &self.description
    }
}

// ─── Edge creation command ──────────────────────────────────────────

/// Command to create a relationship edge between two nodes on a diagram.
///
/// On execute: inserts the Relationship into UmlModel, adds a ViewEdge to the diagram.
/// On undo: removes the ViewEdge from the diagram, removes the Relationship from the model.
///
/// Follows the snapshot pattern:
/// - `relationship_element` is `Some` before first execute / after undo.
/// - `execute()` takes it and inserts into the model.
/// - `undo()` removes it from the model and stores it back.
#[derive(Debug)]
pub struct CreateEdge {
    /// The diagram to add the edge to.
    diagram_id: DiagramId,
    /// The UmlId of the created Relationship element.
    relationship_id: UmlId,
    /// The EdgeId of the created ViewEdge.
    edge_id: EdgeId,
    /// The source node's model element ID.
    source_node_id: UmlId,
    /// The target node's model element ID.
    target_node_id: UmlId,
    /// The Relationship element; consumed on execute, restored on undo.
    relationship_element: Option<ModelElement>,
    /// Human-readable description.
    description: String,
}

impl CreateEdge {
    /// Create a command that will create a new relationship edge between two nodes.
    ///
    /// The relationship is constructed using the appropriate `Relationship` constructor
    /// based on `kind`, and both a `UmlId` and `EdgeId` are generated automatically.
    #[must_use]
    pub fn new(
        diagram_id: DiagramId,
        source_node_id: UmlId,
        target_node_id: UmlId,
        kind: AssociationType,
    ) -> Self {
        let rel = match kind {
            AssociationType::Generalization => {
                Relationship::new_generalization(source_node_id, target_node_id)
            },
            AssociationType::Realization => {
                Relationship::new_realization(source_node_id, target_node_id)
            },
            AssociationType::Association => {
                Relationship::new_association(source_node_id, target_node_id)
            },
            AssociationType::Aggregation => {
                Relationship::new_aggregation(source_node_id, target_node_id)
            },
            AssociationType::Composition => {
                Relationship::new_composition(source_node_id, target_node_id)
            },
            AssociationType::Dependency => {
                Relationship::new_dependency(source_node_id, target_node_id)
            },
        };
        let rel_id = rel.base.id;
        let edge_id = EdgeId::new();
        let kind_name = match kind {
            AssociationType::Generalization => "Generalization",
            AssociationType::Realization => "Realization",
            AssociationType::Association => "Association",
            AssociationType::Aggregation => "Aggregation",
            AssociationType::Composition => "Composition",
            AssociationType::Dependency => "Dependency",
        };
        let desc = format!("Create {kind_name} edge");
        Self {
            diagram_id,
            relationship_id: rel_id,
            edge_id,
            source_node_id,
            target_node_id,
            relationship_element: Some(ModelElement::Relationship(rel)),
            description: desc,
        }
    }
}

impl Command for CreateEdge {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        // 1. Insert the relationship into the model
        let rel = self
            .relationship_element
            .take()
            .ok_or_else(|| CommandError::InvalidOperation("CreateEdge already executed".into()))?;
        model.insert(rel);

        // 2. Add the ViewEdge to the diagram
        let d = model
            .get_diagram_mut(self.diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        d.add_edge(
            self.edge_id,
            ViewEdge::new(
                self.relationship_id,
                self.source_node_id,
                self.target_node_id,
                LineRouting::Direct,
            ),
        );
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        // 1. Remove the ViewEdge from the diagram
        if let Some(d) = model.get_diagram_mut(self.diagram_id) {
            d.remove_edge(self.edge_id);
        }

        // 2. Remove the relationship from the model and store for re-execution
        self.relationship_element = model.remove(self.relationship_id);
        if self.relationship_element.is_none() {
            return Err(CommandError::ElementNotFound(self.relationship_id));
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
    use crate::diagram::{Diagram, DiagramKind, Rect, ViewNode};
    use crate::elements::{
        Attribute, Class, Datatype, Enum, Interface, Operation, Package, Parameter,
        TemplateParameter, TypeReference,
    };
    use crate::types::{AssociationType, ParameterDirection};

    fn same_diagram(left: &Diagram, right: &Diagram) -> bool {
        left.id == right.id
            && left.name == right.name
            && left.kind == right.kind
            && left.zoom_percent.to_bits() == right.zoom_percent.to_bits()
            && left.nodes == right.nodes
            && left.edges == right.edges
    }

    #[test]
    fn create_element_description() {
        let cmd = CreateElement::new(ModelElement::Class(Class::new("Person")));
        assert!(cmd.description().contains("Person"));
        assert!(cmd.description().contains("Create"));
    }

    #[test]
    fn create_element_with_node_is_atomic_across_undo_redo() {
        let mut model = UmlModel::new();
        let diagram = Diagram::new("Main", DiagramKind::Class);
        let diagram_id = diagram.id;
        model.add_diagram(diagram);
        let element = ModelElement::Class(Class::new("Placed"));
        let element_id = element.id();
        let command = CreateElementWithNode::new(
            &model,
            diagram_id,
            element,
            Point::new(10.0, 20.0),
            Size::new(160.0, 60.0),
        )
        .unwrap();
        let mut history = crate::undo::History::new(10);
        history.execute(Box::new(command), &mut model).unwrap();
        assert!(model.contains(element_id));
        assert!(model.diagrams()[0].get_node(element_id).is_some());
        history.undo(&mut model).unwrap();
        assert!(!model.contains(element_id));
        assert!(model.diagrams()[0].get_node(element_id).is_none());
        history.redo(&mut model).unwrap();
        assert!(model.contains(element_id));
        assert!(model.diagrams()[0].get_node(element_id).is_some());
    }

    #[test]
    fn create_element_with_node_rejects_invalid_diagram_without_mutation() {
        let model = UmlModel::new();
        let element = ModelElement::Class(Class::new("Unplaced"));
        let element_id = element.id();
        let result = CreateElementWithNode::new(
            &model,
            crate::diagram::DiagramId::new(),
            element,
            Point::new(0.0, 0.0),
            Size::new(160.0, 60.0),
        );
        assert!(result.is_err());
        assert!(!model.contains(element_id));
        assert!(model.diagrams().is_empty());
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

    // ── CMD-01 through CMD-09: Property editing commands ───────────

    #[test]
    fn change_visibility_execute() {
        let mut model = UmlModel::new();
        let cls = ModelElement::Class(Class::new("Test"));
        let id = cls.id();
        model.insert(cls);

        let mut cmd = ChangeVisibility::new(&model, id, Visibility::Private).unwrap();
        assert_eq!(model.get(id).unwrap().base().visibility, Visibility::Public);
        cmd.execute(&mut model).unwrap();
        assert_eq!(model.get(id).unwrap().base().visibility, Visibility::Private);
    }

    #[test]
    fn change_visibility_undo() {
        let mut model = UmlModel::new();
        let cls = ModelElement::Class(Class::new("Test"));
        let id = cls.id();
        model.insert(cls);

        let mut cmd = ChangeVisibility::new(&model, id, Visibility::Private).unwrap();
        cmd.execute(&mut model).unwrap();
        assert_eq!(model.get(id).unwrap().base().visibility, Visibility::Private);

        cmd.undo(&mut model).unwrap();
        assert_eq!(model.get(id).unwrap().base().visibility, Visibility::Public);
    }

    #[test]
    fn change_visibility_new_element_not_found() {
        let model = UmlModel::new();
        let id = crate::UmlId::new();
        let result = ChangeVisibility::new(&model, id, Visibility::Private);
        assert!(result.is_err());
        assert!(matches!(result, Err(CommandError::ElementNotFound(_))));
    }

    #[test]
    fn change_flags_execute() {
        let mut model = UmlModel::new();
        let cls = ModelElement::Class(Class::new("Test"));
        let id = cls.id();
        model.insert(cls);

        let mut cmd = ChangeElementFlags::new(&model, id, true, true).unwrap();
        let base = model.get(id).unwrap().base();
        assert!(!base.is_abstract);
        assert!(!base.is_static);

        cmd.execute(&mut model).unwrap();
        let base = model.get(id).unwrap().base();
        assert!(base.is_abstract);
        assert!(base.is_static);
    }

    #[test]
    fn change_flags_undo() {
        let mut model = UmlModel::new();
        let cls = ModelElement::Class(Class::new("Test"));
        let id = cls.id();
        model.insert(cls);

        let mut cmd = ChangeElementFlags::new(&model, id, true, true).unwrap();
        cmd.execute(&mut model).unwrap();

        cmd.undo(&mut model).unwrap();
        let base = model.get(id).unwrap().base();
        assert!(!base.is_abstract);
        assert!(!base.is_static);
    }

    #[test]
    fn change_flags_new_element_not_found() {
        let model = UmlModel::new();
        let id = crate::UmlId::new();
        let result = ChangeElementFlags::new(&model, id, true, true);
        assert!(result.is_err());
        assert!(matches!(result, Err(CommandError::ElementNotFound(_))));
    }

    #[test]
    fn change_documentation_execute() {
        let mut model = UmlModel::new();
        let cls = ModelElement::Class(Class::new("Test"));
        let id = cls.id();
        model.insert(cls);

        let mut cmd = ChangeDocumentation::new(&model, id, "A test class".into()).unwrap();
        assert_eq!(model.get(id).unwrap().base().documentation, "");

        cmd.execute(&mut model).unwrap();
        assert_eq!(model.get(id).unwrap().base().documentation, "A test class");
    }

    #[test]
    fn change_documentation_undo() {
        let mut model = UmlModel::new();
        let cls = ModelElement::Class(Class::new("Test"));
        let id = cls.id();
        model.insert(cls);

        let mut cmd = ChangeDocumentation::new(&model, id, "A test class".into()).unwrap();
        cmd.execute(&mut model).unwrap();
        assert_eq!(model.get(id).unwrap().base().documentation, "A test class");

        cmd.undo(&mut model).unwrap();
        assert_eq!(model.get(id).unwrap().base().documentation, "");
    }

    #[test]
    fn change_documentation_new_element_not_found() {
        let model = UmlModel::new();
        let id = crate::UmlId::new();
        let result = ChangeDocumentation::new(&model, id, "test".into());
        assert!(result.is_err());
        assert!(matches!(result, Err(CommandError::ElementNotFound(_))));
    }

    // ── CMD-10 through CMD-15: CreateEdge command tests ─────────────

    fn setup_model_with_two_nodes() -> (UmlModel, DiagramId, UmlId, UmlId) {
        let mut model = UmlModel::new();
        let diagram = Diagram::new("Test", DiagramKind::Class);
        let diagram_id = diagram.id;
        model.add_diagram(diagram);

        let cls1 = ModelElement::Class(Class::new("ClassA"));
        let src_id = cls1.id();
        model.insert(cls1);

        let cls2 = ModelElement::Class(Class::new("ClassB"));
        let tgt_id = cls2.id();
        model.insert(cls2);

        let d = model.get_diagram_mut(diagram_id).unwrap();
        d.add_node(src_id, ViewNode::new(src_id, Rect::new(0.0, 0.0, 100.0, 60.0)));
        d.add_node(tgt_id, ViewNode::new(tgt_id, Rect::new(200.0, 0.0, 100.0, 60.0)));

        (model, diagram_id, src_id, tgt_id)
    }

    #[test]
    fn create_edge_execute_generalization() {
        let (mut model, diagram_id, src_id, tgt_id) = setup_model_with_two_nodes();

        let mut cmd = CreateEdge::new(diagram_id, src_id, tgt_id, AssociationType::Generalization);
        cmd.execute(&mut model).unwrap();

        // Verify Relationship exists in model
        assert!(model.contains(cmd.relationship_id));
        let rel = model.get(cmd.relationship_id).unwrap();
        if let crate::elements::ModelElement::Relationship(r) = rel {
            assert_eq!(r.kind, AssociationType::Generalization);
            assert_eq!(r.source_id, src_id);
            assert_eq!(r.target_id, tgt_id);
        } else {
            panic!("Expected Relationship");
        }

        // Verify ViewEdge exists in diagram
        let d = model.get_diagram(diagram_id).unwrap();
        assert!(d.edges.contains_key(&cmd.edge_id));
        let edge = &d.edges[&cmd.edge_id];
        assert_eq!(edge.relationship_id, cmd.relationship_id);
        assert_eq!(edge.source_node_id, src_id);
        assert_eq!(edge.target_node_id, tgt_id);
        assert_eq!(edge.routing, crate::diagram::LineRouting::Direct);
    }

    #[test]
    fn create_edge_undo_generalization() {
        let (mut model, diagram_id, src_id, tgt_id) = setup_model_with_two_nodes();

        let mut cmd = CreateEdge::new(diagram_id, src_id, tgt_id, AssociationType::Generalization);
        let rel_id = cmd.relationship_id;
        let edge_id = cmd.edge_id;

        cmd.execute(&mut model).unwrap();
        assert!(model.contains(rel_id));
        assert!(model
            .get_diagram(diagram_id)
            .unwrap()
            .edges
            .contains_key(&edge_id));

        cmd.undo(&mut model).unwrap();
        assert!(!model.contains(rel_id));
        assert!(!model
            .get_diagram(diagram_id)
            .unwrap()
            .edges
            .contains_key(&edge_id));
    }

    #[test]
    fn create_edge_execute_all_kinds() {
        let kinds = [
            AssociationType::Generalization,
            AssociationType::Realization,
            AssociationType::Association,
            AssociationType::Aggregation,
            AssociationType::Composition,
            AssociationType::Dependency,
        ];

        for kind in &kinds {
            let (mut model, diagram_id, src_id, tgt_id) = setup_model_with_two_nodes();

            let mut cmd = CreateEdge::new(diagram_id, src_id, tgt_id, *kind);
            cmd.execute(&mut model).unwrap();

            let rel = model.get(cmd.relationship_id).unwrap();
            if let crate::elements::ModelElement::Relationship(r) = rel {
                assert_eq!(r.kind, *kind, "kind mismatch for {kind:?}");
            } else {
                panic!("Expected Relationship for {kind:?}");
            }

            let d = model.get_diagram(diagram_id).unwrap();
            assert!(d.edges.contains_key(&cmd.edge_id), "edge not found for {kind:?}");
        }
    }

    #[test]
    fn create_edge_diagram_not_found() {
        let mut model = UmlModel::new();
        let bad_id = crate::diagram::DiagramId::new();
        let src_id = crate::UmlId::new();
        let tgt_id = crate::UmlId::new();

        let mut cmd = CreateEdge::new(bad_id, src_id, tgt_id, AssociationType::Association);
        let result = cmd.execute(&mut model);
        assert!(result.is_err());
        assert!(matches!(result, Err(CommandError::InvalidOperation(_))));
    }

    #[test]
    fn create_edge_description() {
        let cmd = CreateEdge::new(
            crate::diagram::DiagramId::new(),
            crate::UmlId::new(),
            crate::UmlId::new(),
            AssociationType::Generalization,
        );
        assert!(cmd.description().contains("Generalization"));

        let cmd = CreateEdge::new(
            crate::diagram::DiagramId::new(),
            crate::UmlId::new(),
            crate::UmlId::new(),
            AssociationType::Dependency,
        );
        assert!(cmd.description().contains("Dependency"));
    }

    #[test]
    fn create_edge_undo_then_redo() {
        let (mut model, diagram_id, src_id, tgt_id) = setup_model_with_two_nodes();

        let mut cmd = CreateEdge::new(diagram_id, src_id, tgt_id, AssociationType::Association);
        let rel_id = cmd.relationship_id;
        let edge_id = cmd.edge_id;

        // Execute
        cmd.execute(&mut model).unwrap();
        assert!(model.contains(rel_id));
        assert!(model
            .get_diagram(diagram_id)
            .unwrap()
            .edges
            .contains_key(&edge_id));

        // Undo
        cmd.undo(&mut model).unwrap();
        assert!(!model.contains(rel_id));
        assert!(!model
            .get_diagram(diagram_id)
            .unwrap()
            .edges
            .contains_key(&edge_id));

        // Re-execute (redo)
        cmd.execute(&mut model).unwrap();
        assert!(model.contains(rel_id));
        assert!(model
            .get_diagram(diagram_id)
            .unwrap()
            .edges
            .contains_key(&edge_id));
    }

    #[test]
    fn create_diagram_preserves_snapshot_and_insertion_position() {
        let mut model = UmlModel::new();
        let first = Diagram::new("First", DiagramKind::Class);
        let last = Diagram::new("Last", DiagramKind::Deployment);
        model.add_diagram(first.clone());
        model.add_diagram(last.clone());

        let mut created = Diagram::new("Created", DiagramKind::Component);
        created.set_zoom_percent(250.0);
        let node_id = UmlId::new();
        created.add_node(node_id, ViewNode::new(node_id, Rect::new(1.0, 2.0, 3.0, 4.0)));
        let edge_id = EdgeId::new();
        created.add_edge(edge_id, ViewEdge::new(node_id, node_id, node_id, LineRouting::Direct));
        let created_id = created.id;
        let snapshot = created.clone();
        let mut command = CreateDiagram::new_at(&model, created, 1).unwrap();

        command.execute(&mut model).unwrap();
        assert_eq!(model.diagrams()[1].id, created_id);
        assert!(same_diagram(&model.diagrams()[1], &snapshot));
        command.undo(&mut model).unwrap();
        assert_eq!(model.diagrams().len(), 2);
        assert!(same_diagram(&model.diagrams()[0], &first));
        assert!(same_diagram(&model.diagrams()[1], &last));
        command.execute(&mut model).unwrap();
        assert_eq!(model.diagrams()[1].id, created_id);
        assert!(same_diagram(&model.diagrams()[1], &snapshot));
    }

    #[test]
    fn create_diagram_failure_is_atomic_and_repeated_transitions_are_rejected() {
        let mut model = UmlModel::new();
        let existing = Diagram::new("Existing", DiagramKind::Class);
        let existing_id = existing.id;
        model.add_diagram(existing.clone());
        let duplicate = CreateDiagram::new(&model, Diagram::new("New", DiagramKind::Class));
        assert!(duplicate.is_ok());

        let mut command =
            CreateDiagram::new_at(&model, Diagram::new("New", DiagramKind::Class), 1).unwrap();
        command.execute(&mut model).unwrap();
        assert!(command.execute(&mut model).is_err());
        command.undo(&mut model).unwrap();
        assert!(command.undo(&mut model).is_err());
        assert_eq!(model.diagrams().len(), 1);
        assert!(same_diagram(&model.diagrams()[0], &existing));
        assert_eq!(model.diagrams()[0].id, existing_id);
    }

    #[test]
    fn create_diagram_undo_accepts_non_history_zoom_and_redo_restores_it() {
        let mut model = UmlModel::new();
        let diagram = Diagram::new("Zoomed", DiagramKind::Class);
        let diagram_id = diagram.id;
        let mut history = crate::undo::History::new(10);
        history
            .execute(Box::new(CreateDiagram::new(&model, diagram).unwrap()), &mut model)
            .unwrap();

        model
            .get_diagram_mut(diagram_id)
            .unwrap()
            .set_zoom_percent(275.0);
        history.undo(&mut model).unwrap();
        assert!(model.get_diagram(diagram_id).is_none());
        history.redo(&mut model).unwrap();
        let restored = model.get_diagram(diagram_id).unwrap();
        assert_eq!(restored.id, diagram_id);
        assert!((restored.zoom_percent() - 275.0).abs() < f64::EPSILON);
    }

    fn relationship_fixture() -> (UmlModel, UmlId, UmlId, UmlId) {
        let mut model = UmlModel::new();
        let source = ModelElement::Class(Class::new("Source"));
        let source_id = source.id();
        let target = ModelElement::Class(Class::new("Target"));
        let target_id = target.id();
        model.insert(source);
        model.insert(target);
        let relationship = Relationship::new_association(source_id, target_id);
        let relationship_id = relationship.base.id;
        model.insert(ModelElement::Relationship(relationship));
        (model, relationship_id, source_id, target_id)
    }

    #[test]
    fn update_relationship_covers_editable_fields_and_roundtrip() {
        let (mut model, relationship_id, source_id, target_id) = relationship_fixture();
        let original_xmi_id = "xmi-rel".to_string();
        if let ModelElement::Relationship(relationship) = model.get_mut(relationship_id).unwrap() {
            relationship.base.original_xmi_id = Some(original_xmi_id.clone());
        }
        let mut replacement = Relationship::new(AssociationType::Composition, source_id, target_id);
        replacement.base.id = relationship_id;
        replacement.base.name = "owns".into();
        replacement.base.documentation = "owned relationship".into();
        replacement.source_role_name = Some("whole".into());
        replacement.target_role_name = Some("part".into());
        replacement.source_multiplicity = Some("1".into());
        replacement.target_multiplicity = Some("0..*".into());
        replacement.source_to_target_navigable = true;
        replacement.target_to_source_navigable = true;

        let mut command = UpdateRelationship::new(&model, relationship_id, replacement).unwrap();
        command.execute(&mut model).unwrap();
        let ModelElement::Relationship(updated) = model.get(relationship_id).unwrap() else {
            panic!("expected relationship");
        };
        assert_eq!(updated.kind, AssociationType::Composition);
        assert_eq!(updated.base.name, "owns");
        assert_eq!(updated.base.documentation, "owned relationship");
        assert_eq!(updated.source_role_name.as_deref(), Some("whole"));
        assert_eq!(updated.target_role_name.as_deref(), Some("part"));
        assert_eq!(updated.source_multiplicity.as_deref(), Some("1"));
        assert_eq!(updated.target_multiplicity.as_deref(), Some("0..*"));
        assert!(updated.source_to_target_navigable && updated.target_to_source_navigable);
        assert_eq!(updated.base.original_xmi_id.as_deref(), Some(original_xmi_id.as_str()));

        command.undo(&mut model).unwrap();
        let ModelElement::Relationship(restored) = model.get(relationship_id).unwrap() else {
            panic!("expected relationship");
        };
        assert_eq!(restored.kind, AssociationType::Association);
        assert_eq!(restored.source_id, source_id);
        assert_eq!(restored.target_id, target_id);
        assert_eq!(restored.base.original_xmi_id.as_deref(), Some(original_xmi_id.as_str()));
        command.execute(&mut model).unwrap();
        assert_eq!(model.get(relationship_id).unwrap().base().name, "owns");
    }

    #[test]
    fn update_relationship_rejects_invalid_snapshots_atomically() {
        let (mut model, relationship_id, source_id, target_id) = relationship_fixture();
        let before = model.get(relationship_id).unwrap().clone();
        let mut replacement = Relationship::new_association(source_id, target_id);
        replacement.base.id = UmlId::new();
        assert!(UpdateRelationship::new(&model, relationship_id, replacement).is_err());
        assert_eq!(model.get(relationship_id).unwrap(), &before);

        let non_relationship = ModelElement::Class(Class::new("Not a relationship"));
        let non_relationship_id = non_relationship.id();
        model.insert(non_relationship);
        let replacement = Relationship::new_association(source_id, target_id);
        assert!(UpdateRelationship::new(&model, non_relationship_id, replacement).is_err());

        let missing_id = UmlId::new();
        assert!(UpdateRelationship::new(
            &model,
            missing_id,
            Relationship::new_association(source_id, target_id)
        )
        .is_err());
        assert_eq!(model.get(relationship_id).unwrap(), &before);
    }

    #[test]
    fn update_relationship_rejects_repeated_transitions_without_mutation() {
        let (mut model, relationship_id, source_id, target_id) = relationship_fixture();
        let mut replacement = Relationship::new_dependency(source_id, target_id);
        replacement.base.id = relationship_id;
        replacement.base.name = "uses".into();
        let mut command = UpdateRelationship::new(&model, relationship_id, replacement).unwrap();
        command.execute(&mut model).unwrap();
        let applied = model.get(relationship_id).unwrap().clone();
        assert!(command.execute(&mut model).is_err());
        assert_eq!(model.get(relationship_id).unwrap(), &applied);
        command.undo(&mut model).unwrap();
        let undone = model.get(relationship_id).unwrap().clone();
        assert!(command.undo(&mut model).is_err());
        assert_eq!(model.get(relationship_id).unwrap(), &undone);
    }

    // ── UpdateClassifierFeatures: CMD-16 through CMD-27 ────────────

    fn classifier_fixture() -> (UmlModel, UmlId) {
        let mut model = UmlModel::new();
        let mut cls = Class::new("Person");
        cls.classifier.add_attribute(Attribute {
            name: "name".into(),
            type_ref: TypeReference::primitive("String"),
            visibility: Visibility::Private,
            initial_value: None,
            is_static: false,
        });
        cls.classifier.add_operation(Operation {
            name: "getName".into(),
            return_type: TypeReference::primitive("String"),
            parameters: vec![],
            visibility: Visibility::Public,
            is_static: false,
            is_abstract: false,
            is_virtual: true,
        });
        let id = cls.base.id;
        model.insert(ModelElement::Class(cls));
        (model, id)
    }

    fn rich_classifier_data() -> ClassifierData {
        ClassifierData {
            attributes: vec![
                Attribute {
                    name: "count".into(),
                    type_ref: TypeReference::primitive("int"),
                    visibility: Visibility::Private,
                    initial_value: Some("0".into()),
                    is_static: true,
                },
                Attribute {
                    name: "label".into(),
                    type_ref: TypeReference::unspecified(),
                    visibility: Visibility::Public,
                    initial_value: None,
                    is_static: false,
                },
            ],
            operations: vec![
                Operation {
                    name: "increment".into(),
                    return_type: TypeReference::primitive("void"),
                    parameters: vec![Parameter {
                        name: "delta".into(),
                        type_ref: TypeReference::primitive("int"),
                        direction: ParameterDirection::In,
                        default_value: Some("1".into()),
                    }],
                    visibility: Visibility::Public,
                    is_static: false,
                    is_abstract: false,
                    is_virtual: false,
                },
                Operation {
                    name: "reset".into(),
                    return_type: TypeReference::unspecified(),
                    parameters: vec![],
                    visibility: Visibility::Protected,
                    is_static: true,
                    is_abstract: false,
                    is_virtual: false,
                },
            ],
            templates: vec![],
        }
    }

    #[test]
    fn update_classifier_features_constructs_for_all_classifier_types() {
        let mut model = UmlModel::new();
        let cls_el = ModelElement::Class(Class::new("C"));
        let cls_id = cls_el.id();
        model.insert(cls_el);
        let iface_el = ModelElement::Interface(Interface::new("I"));
        let iface_id = iface_el.id();
        model.insert(iface_el);
        let enum_el = ModelElement::Enum(Enum::new("E"));
        let enum_id = enum_el.id();
        model.insert(enum_el);
        let dt_el = ModelElement::Datatype(Datatype::new("D"));
        let dt_id = dt_el.id();
        model.insert(dt_el);

        let data = ClassifierData::new();
        for &cid in &[cls_id, iface_id, enum_id, dt_id] {
            let cmd = UpdateClassifierFeatures::new(&model, cid, data.clone());
            assert!(cmd.is_ok(), "should accept {cid}");
        }
    }

    #[test]
    fn update_classifier_features_rejects_missing_id() {
        let model = UmlModel::new();
        let result = UpdateClassifierFeatures::new(&model, UmlId::new(), ClassifierData::new());
        assert!(result.is_err());
        assert!(matches!(result, Err(CommandError::ElementNotFound(_))));
    }

    #[test]
    fn update_classifier_features_rejects_non_classifier() {
        let mut model = UmlModel::new();
        let pkg = ModelElement::Package(Package::new("Pkg"));
        let pkg_id = pkg.id();
        model.insert(pkg);
        let result = UpdateClassifierFeatures::new(&model, pkg_id, ClassifierData::new());
        assert!(result.is_err());
        assert!(matches!(result, Err(CommandError::InvalidOperation(_))));
    }

    #[test]
    fn update_classifier_features_rejects_template_change() {
        let (model, id) = classifier_fixture();
        let mut modified = ClassifierData::new();
        modified.templates.push(TemplateParameter {
            name: "T".into(),
            constraint: None,
        });
        let result = UpdateClassifierFeatures::new(&model, id, modified);
        assert!(result.is_err());
        assert!(matches!(result, Err(CommandError::InvalidOperation(_))));
    }

    #[test]
    fn update_classifier_features_execute_replaces_features() {
        let (mut model, id) = classifier_fixture();
        let replacement = rich_classifier_data();
        let before = model.get(id).unwrap().classifier_data().unwrap().clone();

        let mut cmd = UpdateClassifierFeatures::new(&model, id, replacement.clone()).unwrap();
        cmd.execute(&mut model).unwrap();

        let current = model.get(id).unwrap().classifier_data().unwrap();
        assert_eq!(current, &replacement, "features should match replacement");
        assert_ne!(current, &before, "features should differ from original");
    }

    #[test]
    fn update_classifier_features_undo_restores_original() {
        let (mut model, id) = classifier_fixture();
        let before = model.get(id).unwrap().classifier_data().unwrap().clone();

        let mut cmd = UpdateClassifierFeatures::new(&model, id, rich_classifier_data()).unwrap();
        cmd.execute(&mut model).unwrap();
        cmd.undo(&mut model).unwrap();

        let current = model.get(id).unwrap().classifier_data().unwrap();
        assert_eq!(current, &before, "undo should restore original features");
    }

    #[test]
    fn update_classifier_features_full_roundtrip() {
        let (mut model, id) = classifier_fixture();
        let replacement = rich_classifier_data();
        let original = model.get(id).unwrap().classifier_data().unwrap().clone();

        let mut cmd = UpdateClassifierFeatures::new(&model, id, replacement.clone()).unwrap();

        // Execute
        cmd.execute(&mut model).unwrap();
        assert_eq!(model.get(id).unwrap().classifier_data().unwrap(), &replacement);

        // Undo
        cmd.undo(&mut model).unwrap();
        assert_eq!(model.get(id).unwrap().classifier_data().unwrap(), &original);

        // Re-execute (redo)
        cmd.execute(&mut model).unwrap();
        assert_eq!(model.get(id).unwrap().classifier_data().unwrap(), &replacement);
    }

    #[test]
    fn update_classifier_features_rejects_stale_snapshot_on_execute() {
        let (mut model, id) = classifier_fixture();

        let mut cmd = UpdateClassifierFeatures::new(&model, id, rich_classifier_data()).unwrap();

        // Externally modify features
        if let Some(data) = model.get_mut(id).unwrap().classifier_data_mut() {
            data.attributes.push(Attribute {
                name: "sneaky".into(),
                type_ref: TypeReference::primitive("int"),
                visibility: Visibility::Public,
                initial_value: None,
                is_static: false,
            });
        }

        // Execute should fail because snapshot is stale
        let result = cmd.execute(&mut model);
        assert!(result.is_err());
        assert!(matches!(result, Err(CommandError::InvalidOperation(_))));
    }

    #[test]
    fn update_classifier_features_rejects_stale_snapshot_on_undo() {
        let (mut model, id) = classifier_fixture();
        let replacement = rich_classifier_data();

        let mut cmd = UpdateClassifierFeatures::new(&model, id, replacement.clone()).unwrap();
        cmd.execute(&mut model).unwrap();

        // Externally modify features after execute
        if let Some(data) = model.get_mut(id).unwrap().classifier_data_mut() {
            data.operations.push(Operation {
                name: "hack".into(),
                return_type: TypeReference::unspecified(),
                parameters: vec![],
                visibility: Visibility::Private,
                is_static: false,
                is_abstract: false,
                is_virtual: false,
            });
        }

        // Undo should fail because snapshot is stale
        let result = cmd.undo(&mut model);
        assert!(result.is_err());
        assert!(matches!(result, Err(CommandError::InvalidOperation(_))));
    }

    #[test]
    fn update_classifier_features_rejects_repeated_execute() {
        let (mut model, id) = classifier_fixture();

        let mut cmd = UpdateClassifierFeatures::new(&model, id, rich_classifier_data()).unwrap();
        cmd.execute(&mut model).unwrap();
        let applied = model.get(id).unwrap().classifier_data().unwrap().clone();

        // Second execute must fail without mutation
        assert!(cmd.execute(&mut model).is_err());
        assert_eq!(model.get(id).unwrap().classifier_data().unwrap(), &applied);
    }

    #[test]
    fn update_classifier_features_rejects_undo_without_execute() {
        let (model, id) = classifier_fixture();
        let data = rich_classifier_data();

        // Take ownership of cmd without executing
        let mut cmd = UpdateClassifierFeatures::new(&model, id, data).unwrap();
        let mut model = model;
        assert!(cmd.undo(&mut model).is_err());
        assert!(matches!(cmd.undo(&mut model), Err(CommandError::InvalidOperation(_))));
    }

    #[test]
    fn update_classifier_features_preserves_all_field_values_across_roundtrip() {
        let (mut model, id) = classifier_fixture();
        let replacement = rich_classifier_data();

        let mut cmd = UpdateClassifierFeatures::new(&model, id, replacement.clone()).unwrap();

        // Execute
        cmd.execute(&mut model).unwrap();
        let after_execute = model.get(id).unwrap().classifier_data().unwrap().clone();

        // Verify all attribute fields
        assert_eq!(after_execute.attributes.len(), 2);
        assert_eq!(after_execute.attributes[0].name, "count");
        assert_eq!(after_execute.attributes[0].type_ref, TypeReference::primitive("int"));
        assert_eq!(after_execute.attributes[0].visibility, Visibility::Private);
        assert_eq!(after_execute.attributes[0].initial_value, Some("0".to_string()));
        assert!(after_execute.attributes[0].is_static);

        assert_eq!(after_execute.attributes[1].name, "label");
        assert_eq!(after_execute.attributes[1].type_ref, TypeReference::unspecified());
        assert_eq!(after_execute.attributes[1].visibility, Visibility::Public);
        assert_eq!(after_execute.attributes[1].initial_value, None);
        assert!(!after_execute.attributes[1].is_static);

        // Verify all operation and parameter fields
        assert_eq!(after_execute.operations.len(), 2);
        assert_eq!(after_execute.operations[0].name, "increment");
        assert_eq!(after_execute.operations[0].return_type, TypeReference::primitive("void"));
        assert_eq!(after_execute.operations[0].visibility, Visibility::Public);
        assert!(!after_execute.operations[0].is_static);
        assert!(!after_execute.operations[0].is_abstract);
        assert!(!after_execute.operations[0].is_virtual);
        assert_eq!(after_execute.operations[0].parameters.len(), 1);
        assert_eq!(after_execute.operations[0].parameters[0].name, "delta");
        assert_eq!(
            after_execute.operations[0].parameters[0].type_ref,
            TypeReference::primitive("int")
        );
        assert_eq!(after_execute.operations[0].parameters[0].direction, ParameterDirection::In);
        assert_eq!(after_execute.operations[0].parameters[0].default_value, Some("1".to_string()));

        assert_eq!(after_execute.operations[1].name, "reset");
        assert_eq!(after_execute.operations[1].return_type, TypeReference::unspecified());
        assert_eq!(after_execute.operations[1].visibility, Visibility::Protected);
        assert!(after_execute.operations[1].is_static);

        // Verify templates were preserved (empty in original fixture, empty in replacement)
        assert!(after_execute.templates.is_empty());

        // Undo and verify original values restored
        cmd.undo(&mut model).unwrap();
        let after_undo = model.get(id).unwrap().classifier_data().unwrap().clone();
        assert_eq!(after_undo.attributes.len(), 1);
        assert_eq!(after_undo.attributes[0].name, "name");
        assert_eq!(after_undo.operations.len(), 1);
        assert_eq!(after_undo.operations[0].name, "getName");

        // Re-execute and verify replacement restored
        cmd.execute(&mut model).unwrap();
        let after_redo = model.get(id).unwrap().classifier_data().unwrap().clone();
        assert_eq!(after_redo, replacement);
    }

    #[test]
    fn update_classifier_features_works_through_history() {
        let mut model = UmlModel::new();
        let cls = Class::new("Service");
        let id = cls.base.id;
        model.insert(ModelElement::Class(cls));

        let mut history = crate::undo::History::new(10);
        let replacement = ClassifierData {
            attributes: vec![Attribute {
                name: "port".into(),
                type_ref: TypeReference::primitive("u16"),
                visibility: Visibility::Private,
                initial_value: Some("8080".into()),
                is_static: false,
            }],
            operations: vec![],
            templates: vec![],
        };

        let cmd = UpdateClassifierFeatures::new(&model, id, replacement.clone()).unwrap();
        history.execute(Box::new(cmd), &mut model).unwrap();

        assert_eq!(model.get(id).unwrap().classifier_data().unwrap(), &replacement);

        history.undo(&mut model).unwrap();
        let after_undo = model.get(id).unwrap().classifier_data().unwrap();
        assert!(after_undo.attributes.is_empty());
        assert!(after_undo.operations.is_empty());

        history.redo(&mut model).unwrap();
        assert_eq!(model.get(id).unwrap().classifier_data().unwrap(), &replacement);
    }
}
