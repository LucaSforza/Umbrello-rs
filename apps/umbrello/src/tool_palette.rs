//! Tool palette UI and element creation logic.
//!
//! Defines the `ToolMode` enum representing the active tool (Select, or one of
//! the five creation tools: Class, Interface, Enum, Datatype, Package).
//! Provides methods on `UmbrelloApp` for creating elements and placing them on
//! the canvas via undoable commands.

use crate::app::UmbrelloApp;
use uml_core::{
    commands, Actor, AssociationType, Class, Datatype, Enum, Interface, ModelElement, Package,
    Point, Size, UmlId, UseCase,
};

/// The active tool in the tool palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolMode {
    /// Default: select and move existing nodes.
    Select,
    /// Create a new Class element on click.
    CreateClass,
    /// Create a new Interface element on click.
    CreateInterface,
    /// Create a new Enum element on click.
    CreateEnum,
    /// Create a new Datatype element on click.
    CreateDatatype,
    /// Create a new Package element on click.
    CreatePackage,
    // ── Node-creation tools (M20) ──
    /// Create a new Actor element on click.
    CreateActor,
    /// Create a new UseCase element on click.
    CreateUseCase,
    // ── Edge-creation tools (M19) ──
    /// Create a Generalization (hollow triangle arrowhead).
    CreateGeneralization,
    /// Create a Realization (dashed line + hollow triangle).
    CreateRealization,
    /// Create a plain Association (solid line).
    CreateAssociation,
    /// Create an Aggregation (hollow diamond at source).
    CreateAggregation,
    /// Create a Composition (filled diamond at source).
    CreateComposition,
    /// Create a Dependency (dashed line + open arrow).
    CreateDependency,
}

impl ToolMode {
    /// Human-readable label for the tool palette button.
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Select => "🖱 Select",
            Self::CreateClass => "📦 Class",
            Self::CreateInterface => "🔌 Interface",
            Self::CreateEnum => "🔢 Enum",
            Self::CreateDatatype => "📋 Datatype",
            Self::CreatePackage => "📁 Package",
            Self::CreateActor => "🧑 Actor",
            Self::CreateUseCase => "⬭ UseCase",
            Self::CreateGeneralization => "△ Generalization",
            Self::CreateRealization => "△ Realization",
            Self::CreateAssociation => "— Association",
            Self::CreateAggregation => "◇ Aggregation",
            Self::CreateComposition => "◆ Composition",
            Self::CreateDependency => "⇢ Dependency",
        }
    }

    /// Short tooltip description.
    #[allow(dead_code)]
    fn tooltip(&self) -> &'static str {
        match self {
            Self::Select => "Select and move elements (S, Esc)",
            Self::CreateClass => "Create a Class (C)",
            Self::CreateInterface => "Create an Interface (I)",
            Self::CreateEnum => "Create an Enum (E)",
            Self::CreateDatatype => "Create a Datatype (D)",
            Self::CreatePackage => "Create a Package (P)",
            Self::CreateActor => "Create an Actor (T)",
            Self::CreateUseCase => "Create a UseCase (U)",
            Self::CreateGeneralization => {
                "Create a Generalization (G) — click-drag from subclass to superclass"
            },
            Self::CreateRealization => {
                "Create a Realization (R) — click-drag from class to interface"
            },
            Self::CreateAssociation => "Create an Association (A) — click-drag between classes",
            Self::CreateAggregation => "Create an Aggregation — click-drag from whole to part",
            Self::CreateComposition => "Create a Composition — click-drag from whole to part",
            Self::CreateDependency => {
                "Create a Dependency (N) — click-drag from dependent to supplier"
            },
        }
    }

    /// Whether this tool creates a new node element (ghost preview, crosshair cursor).
    /// Edge tools return `false` — they use click-drag on existing nodes instead.
    pub(crate) fn is_creation_tool(&self) -> bool {
        matches!(
            self,
            Self::CreateClass
                | Self::CreateInterface
                | Self::CreateEnum
                | Self::CreateDatatype
                | Self::CreatePackage
                | Self::CreateActor
                | Self::CreateUseCase
        )
    }

    /// Whether this tool creates edges (click-drag between nodes).
    pub(crate) fn is_edge_tool(&self) -> bool {
        matches!(
            self,
            Self::CreateGeneralization
                | Self::CreateRealization
                | Self::CreateAssociation
                | Self::CreateAggregation
                | Self::CreateComposition
                | Self::CreateDependency
        )
    }

    /// Map the edge tool variant to the corresponding `AssociationType`.
    /// Returns `None` for non-edge tools (Select, node-creation tools).
    #[must_use]
    pub(crate) fn association_type(&self) -> Option<AssociationType> {
        match self {
            Self::CreateGeneralization => Some(AssociationType::Generalization),
            Self::CreateRealization => Some(AssociationType::Realization),
            Self::CreateAssociation => Some(AssociationType::Association),
            Self::CreateAggregation => Some(AssociationType::Aggregation),
            Self::CreateComposition => Some(AssociationType::Composition),
            Self::CreateDependency => Some(AssociationType::Dependency),
            _ => None,
        }
    }
}

impl UmbrelloApp {
    /// Create a `ModelElement` of the appropriate type with a default name.
    /// Edge tools should never reach this method — they use `place_edge()` instead.
    pub(crate) fn create_element_for_tool(&self, tool: ToolMode) -> ModelElement {
        match tool {
            ToolMode::CreateClass => {
                let name = self.generate_unique_name("Class");
                ModelElement::Class(Class::new(&name))
            },
            ToolMode::CreateInterface => {
                let name = self.generate_unique_name("Interface");
                let mut iface = Interface::new(&name);
                iface.base.is_abstract = true;
                ModelElement::Interface(iface)
            },
            ToolMode::CreateEnum => {
                let name = self.generate_unique_name("Enum");
                ModelElement::Enum(Enum::new(&name))
            },
            ToolMode::CreateDatatype => {
                let name = self.generate_unique_name("Datatype");
                ModelElement::Datatype(Datatype::new(&name))
            },
            ToolMode::CreatePackage => {
                let name = self.generate_unique_name("Package");
                ModelElement::Package(Package::new(&name))
            },
            ToolMode::CreateActor => {
                let name = self.generate_unique_name("Actor");
                ModelElement::Actor(Actor::new(&name))
            },
            ToolMode::CreateUseCase => {
                let name = self.generate_unique_name("UseCase");
                ModelElement::UseCase(UseCase::new(&name))
            },
            ToolMode::Select
            | ToolMode::CreateGeneralization
            | ToolMode::CreateRealization
            | ToolMode::CreateAssociation
            | ToolMode::CreateAggregation
            | ToolMode::CreateComposition
            | ToolMode::CreateDependency => {
                unreachable!("Non-creation tools should never call create_element_for_tool")
            },
        }
    }

    /// Place a newly created element on the active diagram at the given position.
    /// Executes `CreateElement` + `AddNodeToDiagram` commands.
    /// Returns `Ok(())` if both commands succeed.
    pub(crate) fn place_element(&mut self, tool: ToolMode, pos: Point) -> Result<(), String> {
        let diag_idx = self
            .active_diagram
            .ok_or_else(|| "No active diagram".to_string())?;
        let diagram_id = self.model.diagrams()[diag_idx].id;

        let elem = self.create_element_for_tool(tool);
        let elem_id = elem.id();

        self.execute_command(Box::new(commands::CreateElement::new(elem)));
        self.execute_command(Box::new(commands::AddNodeToDiagram::new(
            diagram_id,
            elem_id,
            pos,
            Size::new(160.0, 60.0),
        )));

        Ok(())
    }

    /// Place a new relationship edge between two nodes on the active diagram.
    /// Executes a single `CreateEdge` command (atomic, one undo step).
    /// Returns `Ok(())` if the command succeeds, or an error string otherwise.
    pub(crate) fn place_edge(
        &mut self,
        source_node_id: UmlId,
        target_node_id: UmlId,
    ) -> Result<(), String> {
        let kind = self
            .current_tool
            .association_type()
            .ok_or_else(|| "Current tool is not an edge tool".to_string())?;
        let diag_idx = self
            .active_diagram
            .ok_or_else(|| "No active diagram".to_string())?;
        let diagram_id = self.model.diagrams()[diag_idx].id;

        self.execute_command(Box::new(commands::CreateEdge::new(
            diagram_id,
            source_node_id,
            target_node_id,
            kind,
        )));

        Ok(())
    }

    /// Render the tool palette panel.
    pub(crate) fn render_tool_palette(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tools");
        ui.add_space(4.0);

        // ── Selection + node creation tools ──
        for tool in &[
            ToolMode::Select,
            ToolMode::CreateClass,
            ToolMode::CreateInterface,
            ToolMode::CreateEnum,
            ToolMode::CreateDatatype,
            ToolMode::CreatePackage,
            ToolMode::CreateActor,
            ToolMode::CreateUseCase,
        ] {
            let selected = self.current_tool == *tool;
            let button = egui::SelectableLabel::new(selected, tool.label());
            if ui.add(button).clicked() {
                self.current_tool = *tool;
                self.preview_position = None;
                self.drag_source_node_id = None;
                self.status_message = format!("Tool: {}", tool.label());
            }
        }

        ui.separator();
        ui.label(egui::RichText::new("Edges").weak());

        // ── Edge creation tools ──
        for tool in &[
            ToolMode::CreateGeneralization,
            ToolMode::CreateRealization,
            ToolMode::CreateAssociation,
            ToolMode::CreateAggregation,
            ToolMode::CreateComposition,
            ToolMode::CreateDependency,
        ] {
            let selected = self.current_tool == *tool;
            let button = egui::SelectableLabel::new(selected, tool.label());
            if ui.add(button).clicked() {
                self.current_tool = *tool;
                self.preview_position = None;
                self.drag_source_node_id = None;
                self.status_message = format!("Tool: {}", tool.label());
            }
        }

        ui.separator();
    }
}
