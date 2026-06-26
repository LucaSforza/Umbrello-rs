//! Tool palette UI and element creation logic.
//!
//! Defines the `ToolMode` enum representing the active tool (Select, or one of
//! the five creation tools: Class, Interface, Enum, Datatype, Package).
//! Provides methods on `UmbrelloApp` for creating elements and placing them on
//! the canvas via undoable commands.

use crate::app::UmbrelloApp;
use uml_core::{commands, Class, Datatype, Enum, Interface, ModelElement, Package, Point, Size};

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
        }
    }

    /// Whether this tool creates a new element (i.e., changes cursor to crosshair).
    pub(crate) fn is_creation_tool(&self) -> bool {
        !matches!(self, Self::Select)
    }
}

impl UmbrelloApp {
    /// Create a `ModelElement` of the appropriate type with a default name.
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
            ToolMode::Select => {
                unreachable!("Select tool does not create elements")
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

    /// Render the tool palette panel.
    pub(crate) fn render_tool_palette(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tools");
        ui.add_space(4.0);
        for tool in &[
            ToolMode::Select,
            ToolMode::CreateClass,
            ToolMode::CreateInterface,
            ToolMode::CreateEnum,
            ToolMode::CreateDatatype,
            ToolMode::CreatePackage,
        ] {
            let selected = self.current_tool == *tool;
            let button = egui::SelectableLabel::new(selected, tool.label());
            if ui.add(button).clicked() {
                self.current_tool = *tool;
                self.preview_position = None;
                self.status_message = format!("Tool: {}", tool.label());
            }
        }
        ui.separator();
    }
}
