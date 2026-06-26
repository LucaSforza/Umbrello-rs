//! Concrete command implementations for model mutations.

use crate::elements::{ModelElement, Relationship};
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

use crate::diagram::{DiagramId, EdgeId, LineRouting, Point, Rect, Size, ViewEdge, ViewNode};

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
    use crate::elements::{Class, Package};
    use crate::types::AssociationType;

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
}
