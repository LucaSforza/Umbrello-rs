//! Type enumerations for UML concepts.
//!
//! These enums replace the scattered C++ enums from `basictypes.h`
//! and the `UMLObject::ObjectType` runtime type identification system.

use serde::{Deserialize, Serialize};
use std::fmt;

// ─── ObjectType ───────────────────────────────────────────────────────

/// Discriminant for UML model element types.
///
/// Replaces the 30-value C++ `UMLObject::ObjectType` enum and the 28
/// `isUML*()` / `asUML*()` manual RTTI methods.  Each variant corresponds to
/// a concrete UML concept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    // ── Structural classifiers ───────────────────────────────────────
    /// A UML class.
    Class,
    /// A UML interface.
    Interface,
    /// A UML enumeration.
    Enumeration,
    /// A UML datatype (primitive or structured).
    Datatype,
    /// An entity-relationship entity (database table).
    Entity,

    // ── Containers (namespace-like) ──────────────────────────────────
    /// A UML package.
    Package,
    /// A diagram container (folder).
    Folder,
    /// A UML component.
    Component,
    /// A UML artifact (file, library, table).
    Artifact,

    // ── Leaf diagram nodes ───────────────────────────────────────────
    /// A UML actor.
    Actor,
    /// A UML use case.
    UseCase,
    /// A UML deployment node.
    Node,
    /// A UML port.
    Port,
    /// An EER category (disjoint/overlapping/union specialisation).
    Category,
    /// A UML instance (object diagram).
    Instance,

    // ── Classifier children ──────────────────────────────────────────
    /// A classifier attribute (field).
    Attribute,
    /// A classifier operation (method).
    Operation,
    /// A template / generic parameter.
    Template,
    /// An enumeration literal value.
    EnumLiteral,
    /// An entity attribute (database column).
    EntityAttribute,

    // ── Constraints (entity-relationship) ────────────────────────────
    /// A unique constraint on entity attributes.
    UniqueConstraint,
    /// A foreign-key constraint referencing another entity.
    ForeignKeyConstraint,
    /// A check constraint (SQL `CHECK` clause).
    CheckConstraint,

    // ── Relationships ────────────────────────────────────────────────
    /// An association between two model elements.
    Association,
    /// An association role (one end of an association).
    Role,
    /// A UML generalization (inheritance).
    Generalization,
    /// A UML interface realization.
    Realization,
    /// A UML dependency.
    Dependency,

    // ── Infrastructure ───────────────────────────────────────────────
    /// A UML stereotype.
    Stereotype,
    /// An instance-attribute value binding.
    InstanceAttribute,
}

impl ObjectType {
    /// Human-readable display name, matching the C++ `toString()` output.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Class => "Class",
            Self::Interface => "Interface",
            Self::Enumeration => "Enum",
            Self::Datatype => "Datatype",
            Self::Entity => "Entity",
            Self::Package => "Package",
            Self::Folder => "Folder",
            Self::Component => "Component",
            Self::Artifact => "Artifact",
            Self::Actor => "Actor",
            Self::UseCase => "UseCase",
            Self::Node => "Node",
            Self::Port => "Port",
            Self::Category => "Category",
            Self::Instance => "Instance",
            Self::Attribute => "Attribute",
            Self::Operation => "Operation",
            Self::Template => "Template",
            Self::EnumLiteral => "EnumLiteral",
            Self::EntityAttribute => "EntityAttribute",
            Self::UniqueConstraint => "UniqueConstraint",
            Self::ForeignKeyConstraint => "ForeignKeyConstraint",
            Self::CheckConstraint => "CheckConstraint",
            Self::Association => "Association",
            Self::Role => "Role",
            Self::Generalization => "Generalization",
            Self::Realization => "Realization",
            Self::Dependency => "Dependency",
            Self::Stereotype => "Stereotype",
            Self::InstanceAttribute => "InstanceAttribute",
        }
    }

    /// Returns `true` if the type represents a classifier (class, interface,
    /// enum, datatype, or entity).
    #[must_use]
    pub fn is_classifier(self) -> bool {
        matches!(
            self,
            Self::Class | Self::Interface | Self::Enumeration | Self::Datatype | Self::Entity
        )
    }

    /// Returns `true` if the type represents a container (package, folder,
    /// component, or artifact).
    #[must_use]
    pub fn is_container(self) -> bool {
        matches!(self, Self::Package | Self::Folder | Self::Component | Self::Artifact)
    }
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─── AssociationType ──────────────────────────────────────────────────

/// Kinds of UML associations.
///
/// Replaces the 25+ value C++ `Uml::AssociationType::Enum`.  The values
/// start at 500 in the C++ codebase to leave room for file-format anchors;
/// that is an implementation detail we do not replicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AssociationType {
    /// A plain association — the default.
    Association,
    /// Directed association (navigable in one direction).
    DirectedAssociation,
    /// Generalization (inheritance).
    Generalization,
    /// Interface realisation.
    Realization,
    /// Aggregation (whole-part, shared).
    Aggregation,
    /// Composition (whole-part, exclusive lifecycle).
    Composition,
    /// UML dependency.
    Dependency,
    /// Anchor relationship (used in node diagrams).
    Anchor,
    /// Containment (parent-child in component diagrams).
    Containment,
    /// Exception relationship.
    Exception,
    /// Category-to-parent (EER specialisation).
    Category2Parent,
    /// Child-to-category (EER specialisation).
    Child2Category,
}

impl AssociationType {
    /// Human-readable name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Association => "Association",
            Self::DirectedAssociation => "Directed Association",
            Self::Generalization => "Generalization",
            Self::Realization => "Realization",
            Self::Aggregation => "Aggregation",
            Self::Composition => "Composition",
            Self::Dependency => "Dependency",
            Self::Anchor => "Anchor",
            Self::Containment => "Containment",
            Self::Exception => "Exception",
            Self::Category2Parent => "Category to Parent",
            Self::Child2Category => "Child to Category",
        }
    }

    /// Returns `true` if this type has a visual representation on a diagram.
    #[must_use]
    pub fn has_visual_representation(self) -> bool {
        !matches!(self, Self::Exception | Self::Anchor)
    }
}

impl fmt::Display for AssociationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─── DiagramType ──────────────────────────────────────────────────────

/// UML diagram categories.
///
/// Replaces the C++ `Uml::DiagramType::Enum` (values 0–10).
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum DiagramType {
    /// Not yet set / unknown.
    #[default]
    Undefined,
    /// UML class diagram.
    Class,
    /// UML use-case diagram.
    UseCase,
    /// UML sequence diagram.
    Sequence,
    /// UML collaboration / communication diagram.
    Collaboration,
    /// UML statechart / state-machine diagram.
    State,
    /// UML activity diagram.
    Activity,
    /// UML component diagram.
    Component,
    /// UML deployment diagram.
    Deployment,
    /// Entity-relationship diagram.
    EntityRelationship,
    /// UML object diagram.
    Object,
}

impl DiagramType {
    /// Human-readable name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Undefined => "Undefined",
            Self::Class => "Class",
            Self::UseCase => "UseCase",
            Self::Sequence => "Sequence",
            Self::Collaboration => "Collaboration",
            Self::State => "State",
            Self::Activity => "Activity",
            Self::Component => "Component",
            Self::Deployment => "Deployment",
            Self::EntityRelationship => "Entity Relationship",
            Self::Object => "Object",
        }
    }
}

impl fmt::Display for DiagramType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─── Visibility ───────────────────────────────────────────────────────

/// Access-visibility level for UML elements.
///
/// Replaces the C++ `Uml::Visibility::Enum` (4 values).
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Visibility {
    /// Public (`+`).
    #[default]
    Public,
    /// Protected (`#`).
    Protected,
    /// Private (`-`).
    Private,
    /// Package-level / implementation (`~`).
    Implementation,
}

impl Visibility {
    /// UML symbol: `+` `#` `-` `~`.
    #[must_use]
    pub fn symbol(self) -> char {
        match self {
            Self::Public => '+',
            Self::Protected => '#',
            Self::Private => '-',
            Self::Implementation => '~',
        }
    }

    /// Human-readable name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Protected => "protected",
            Self::Private => "private",
            Self::Implementation => "implementation",
        }
    }
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─── ParameterDirection ───────────────────────────────────────────────

/// Direction of an operation parameter.
///
/// Replaces the C++ `Uml::ParameterDirection::Enum`.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum ParameterDirection {
    /// Input parameter (call-by-value or const-ref).
    #[default]
    In,
    /// Output parameter.
    Out,
    /// Input-output parameter.
    InOut,
    /// Return value.
    Return,
}

impl ParameterDirection {
    /// Human-readable name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Out => "out",
            Self::InOut => "inout",
            Self::Return => "return",
        }
    }
}

impl fmt::Display for ParameterDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ObjectType ───────────────────────────────────────────────────

    #[test]
    fn test_object_type_as_str_all_variants() {
        let cases = [
            (ObjectType::Class, "Class"),
            (ObjectType::Interface, "Interface"),
            (ObjectType::Enumeration, "Enum"),
            (ObjectType::Datatype, "Datatype"),
            (ObjectType::Entity, "Entity"),
            (ObjectType::Package, "Package"),
            (ObjectType::Folder, "Folder"),
            (ObjectType::Component, "Component"),
            (ObjectType::Artifact, "Artifact"),
            (ObjectType::Actor, "Actor"),
            (ObjectType::UseCase, "UseCase"),
            (ObjectType::Node, "Node"),
            (ObjectType::Port, "Port"),
            (ObjectType::Category, "Category"),
            (ObjectType::Instance, "Instance"),
            (ObjectType::Attribute, "Attribute"),
            (ObjectType::Operation, "Operation"),
            (ObjectType::Template, "Template"),
            (ObjectType::EnumLiteral, "EnumLiteral"),
            (ObjectType::EntityAttribute, "EntityAttribute"),
            (ObjectType::UniqueConstraint, "UniqueConstraint"),
            (ObjectType::ForeignKeyConstraint, "ForeignKeyConstraint"),
            (ObjectType::CheckConstraint, "CheckConstraint"),
            (ObjectType::Association, "Association"),
            (ObjectType::Role, "Role"),
            (ObjectType::Generalization, "Generalization"),
            (ObjectType::Realization, "Realization"),
            (ObjectType::Dependency, "Dependency"),
            (ObjectType::Stereotype, "Stereotype"),
            (ObjectType::InstanceAttribute, "InstanceAttribute"),
        ];
        for (variant, expected) in &cases {
            assert_eq!(variant.as_str(), *expected);
            assert_eq!(variant.to_string(), *expected);
        }
    }

    #[test]
    fn test_object_type_is_classifier() {
        assert!(ObjectType::Class.is_classifier());
        assert!(ObjectType::Interface.is_classifier());
        assert!(ObjectType::Enumeration.is_classifier());
        assert!(ObjectType::Datatype.is_classifier());
        assert!(ObjectType::Entity.is_classifier());
        assert!(!ObjectType::Package.is_classifier());
        assert!(!ObjectType::Folder.is_classifier());
        assert!(!ObjectType::Actor.is_classifier());
    }

    #[test]
    fn test_object_type_is_container() {
        assert!(ObjectType::Package.is_container());
        assert!(ObjectType::Folder.is_container());
        assert!(ObjectType::Component.is_container());
        assert!(ObjectType::Artifact.is_container());
        assert!(!ObjectType::Class.is_container());
        assert!(!ObjectType::Interface.is_container());
        assert!(!ObjectType::Actor.is_container());
    }

    #[test]
    fn test_object_type_serde_roundtrip() {
        let variants = [
            ObjectType::Class,
            ObjectType::Interface,
            ObjectType::Enumeration,
            ObjectType::Datatype,
            ObjectType::Entity,
            ObjectType::Package,
            ObjectType::Folder,
            ObjectType::Component,
            ObjectType::Artifact,
            ObjectType::Actor,
            ObjectType::UseCase,
            ObjectType::Node,
            ObjectType::Port,
            ObjectType::Category,
            ObjectType::Instance,
            ObjectType::Attribute,
            ObjectType::Operation,
            ObjectType::Template,
            ObjectType::EnumLiteral,
            ObjectType::EntityAttribute,
            ObjectType::UniqueConstraint,
            ObjectType::ForeignKeyConstraint,
            ObjectType::CheckConstraint,
            ObjectType::Association,
            ObjectType::Role,
            ObjectType::Generalization,
            ObjectType::Realization,
            ObjectType::Dependency,
            ObjectType::Stereotype,
            ObjectType::InstanceAttribute,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).expect("serialize");
            let back: ObjectType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(v, &back, "round-trip failed for {v}");
        }
    }

    #[test]
    fn test_object_type_all_display_strings_unique() {
        let variants = [
            ObjectType::Class,
            ObjectType::Interface,
            ObjectType::Enumeration,
            ObjectType::Datatype,
            ObjectType::Entity,
            ObjectType::Package,
            ObjectType::Folder,
            ObjectType::Component,
            ObjectType::Artifact,
            ObjectType::Actor,
            ObjectType::UseCase,
            ObjectType::Node,
            ObjectType::Port,
            ObjectType::Category,
            ObjectType::Instance,
            ObjectType::Attribute,
            ObjectType::Operation,
            ObjectType::Template,
            ObjectType::EnumLiteral,
            ObjectType::EntityAttribute,
            ObjectType::UniqueConstraint,
            ObjectType::ForeignKeyConstraint,
            ObjectType::CheckConstraint,
            ObjectType::Association,
            ObjectType::Role,
            ObjectType::Generalization,
            ObjectType::Realization,
            ObjectType::Dependency,
            ObjectType::Stereotype,
            ObjectType::InstanceAttribute,
        ];
        let mut seen = std::collections::HashSet::new();
        for v in &variants {
            assert!(seen.insert(v.to_string()), "duplicate display string for {v}");
        }
    }

    // ── AssociationType ──────────────────────────────────────────────

    #[test]
    fn test_association_type_as_str_all_variants() {
        let cases = [
            (AssociationType::Association, "Association"),
            (AssociationType::DirectedAssociation, "Directed Association"),
            (AssociationType::Generalization, "Generalization"),
            (AssociationType::Realization, "Realization"),
            (AssociationType::Aggregation, "Aggregation"),
            (AssociationType::Composition, "Composition"),
            (AssociationType::Dependency, "Dependency"),
            (AssociationType::Anchor, "Anchor"),
            (AssociationType::Containment, "Containment"),
            (AssociationType::Exception, "Exception"),
            (AssociationType::Category2Parent, "Category to Parent"),
            (AssociationType::Child2Category, "Child to Category"),
        ];
        for (variant, expected) in &cases {
            assert_eq!(variant.as_str(), *expected);
            assert_eq!(variant.to_string(), *expected);
        }
    }

    #[test]
    fn test_association_type_has_visual_representation() {
        assert!(AssociationType::Association.has_visual_representation());
        assert!(AssociationType::Generalization.has_visual_representation());
        assert!(AssociationType::Composition.has_visual_representation());
        assert!(!AssociationType::Exception.has_visual_representation());
        assert!(!AssociationType::Anchor.has_visual_representation());
    }

    #[test]
    fn test_association_type_serde_roundtrip() {
        for v in &[
            AssociationType::Association,
            AssociationType::DirectedAssociation,
            AssociationType::Generalization,
            AssociationType::Realization,
            AssociationType::Aggregation,
            AssociationType::Composition,
            AssociationType::Dependency,
            AssociationType::Anchor,
            AssociationType::Containment,
            AssociationType::Exception,
            AssociationType::Category2Parent,
            AssociationType::Child2Category,
        ] {
            let json = serde_json::to_string(v).expect("serialize");
            let back: AssociationType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(v, &back, "round-trip failed for {v}");
        }
    }

    // ── DiagramType ──────────────────────────────────────────────────

    #[test]
    fn test_diagram_type_as_str_all_variants() {
        let cases = [
            (DiagramType::Undefined, "Undefined"),
            (DiagramType::Class, "Class"),
            (DiagramType::UseCase, "UseCase"),
            (DiagramType::Sequence, "Sequence"),
            (DiagramType::Collaboration, "Collaboration"),
            (DiagramType::State, "State"),
            (DiagramType::Activity, "Activity"),
            (DiagramType::Component, "Component"),
            (DiagramType::Deployment, "Deployment"),
            (DiagramType::EntityRelationship, "Entity Relationship"),
            (DiagramType::Object, "Object"),
        ];
        for (variant, expected) in &cases {
            assert_eq!(variant.as_str(), *expected);
            assert_eq!(variant.to_string(), *expected);
        }
    }

    #[test]
    fn test_diagram_type_default() {
        assert_eq!(DiagramType::default(), DiagramType::Undefined);
    }

    #[test]
    fn test_diagram_type_serde_roundtrip() {
        for v in &[
            DiagramType::Undefined,
            DiagramType::Class,
            DiagramType::UseCase,
            DiagramType::Sequence,
            DiagramType::Collaboration,
            DiagramType::State,
            DiagramType::Activity,
            DiagramType::Component,
            DiagramType::Deployment,
            DiagramType::EntityRelationship,
            DiagramType::Object,
        ] {
            let json = serde_json::to_string(v).expect("serialize");
            let back: DiagramType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(v, &back, "round-trip failed for {v}");
        }
    }

    // ── Visibility ───────────────────────────────────────────────────

    #[test]
    fn test_visibility_symbols() {
        assert_eq!(Visibility::Public.symbol(), '+');
        assert_eq!(Visibility::Protected.symbol(), '#');
        assert_eq!(Visibility::Private.symbol(), '-');
        assert_eq!(Visibility::Implementation.symbol(), '~');
    }

    #[test]
    fn test_visibility_as_str() {
        assert_eq!(Visibility::Public.as_str(), "public");
        assert_eq!(Visibility::Protected.as_str(), "protected");
        assert_eq!(Visibility::Private.as_str(), "private");
        assert_eq!(Visibility::Implementation.as_str(), "implementation");
    }

    #[test]
    fn test_visibility_display() {
        assert_eq!(Visibility::Public.to_string(), "public");
        assert_eq!(Visibility::Protected.to_string(), "protected");
        assert_eq!(Visibility::Private.to_string(), "private");
        assert_eq!(Visibility::Implementation.to_string(), "implementation");
    }

    #[test]
    fn test_visibility_default() {
        assert_eq!(Visibility::default(), Visibility::Public);
    }

    #[test]
    fn test_visibility_serde_roundtrip() {
        for v in &[
            Visibility::Public,
            Visibility::Protected,
            Visibility::Private,
            Visibility::Implementation,
        ] {
            let json = serde_json::to_string(v).expect("serialize");
            let back: Visibility = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(v, &back, "round-trip failed for {v}");
        }
    }

    // ── ParameterDirection ───────────────────────────────────────────

    #[test]
    fn test_parameter_direction_as_str() {
        assert_eq!(ParameterDirection::In.as_str(), "in");
        assert_eq!(ParameterDirection::Out.as_str(), "out");
        assert_eq!(ParameterDirection::InOut.as_str(), "inout");
        assert_eq!(ParameterDirection::Return.as_str(), "return");
    }

    #[test]
    fn test_parameter_direction_display() {
        assert_eq!(ParameterDirection::In.to_string(), "in");
        assert_eq!(ParameterDirection::Out.to_string(), "out");
        assert_eq!(ParameterDirection::InOut.to_string(), "inout");
        assert_eq!(ParameterDirection::Return.to_string(), "return");
    }

    #[test]
    fn test_parameter_direction_default() {
        assert_eq!(ParameterDirection::default(), ParameterDirection::In);
    }

    #[test]
    fn test_parameter_direction_serde_roundtrip() {
        for v in &[
            ParameterDirection::In,
            ParameterDirection::Out,
            ParameterDirection::InOut,
            ParameterDirection::Return,
        ] {
            let json = serde_json::to_string(v).expect("serialize");
            let back: ParameterDirection = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(v, &back, "round-trip failed for {v}");
        }
    }

    // ── Cross-enum ordering tests ────────────────────────────────────

    #[test]
    fn test_enums_are_send_and_sync() {
        // Compile-time check: all enums must be Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ObjectType>();
        assert_send_sync::<AssociationType>();
        assert_send_sync::<DiagramType>();
        assert_send_sync::<Visibility>();
        assert_send_sync::<ParameterDirection>();
    }

    #[test]
    fn test_enums_are_copy() {
        // Compile-time check: all enums must be Copy
        fn assert_copy<T: Copy>() {}
        assert_copy::<ObjectType>();
        assert_copy::<AssociationType>();
        assert_copy::<DiagramType>();
        assert_copy::<Visibility>();
        assert_copy::<ParameterDirection>();
    }

    #[test]
    fn test_object_type_serde_names_unique() {
        use std::collections::HashSet;

        let mut seen = HashSet::new();
        for v in &[
            ObjectType::Class,
            ObjectType::Interface,
            ObjectType::Enumeration,
            ObjectType::Datatype,
            ObjectType::Entity,
            ObjectType::Package,
            ObjectType::Folder,
            ObjectType::Component,
            ObjectType::Artifact,
            ObjectType::Actor,
            ObjectType::UseCase,
            ObjectType::Node,
            ObjectType::Port,
            ObjectType::Category,
            ObjectType::Instance,
            ObjectType::Attribute,
            ObjectType::Operation,
            ObjectType::Template,
            ObjectType::EnumLiteral,
            ObjectType::EntityAttribute,
            ObjectType::UniqueConstraint,
            ObjectType::ForeignKeyConstraint,
            ObjectType::CheckConstraint,
            ObjectType::Association,
            ObjectType::Role,
            ObjectType::Generalization,
            ObjectType::Realization,
            ObjectType::Dependency,
            ObjectType::Stereotype,
            ObjectType::InstanceAttribute,
        ] {
            let s = serde_json::to_string(v).unwrap();
            assert!(seen.insert(s), "duplicate serde name for {v}");
        }
    }

    #[test]
    fn test_association_type_serde_names_unique() {
        use std::collections::HashSet;

        let mut seen = HashSet::new();
        for v in &[
            AssociationType::Association,
            AssociationType::DirectedAssociation,
            AssociationType::Generalization,
            AssociationType::Realization,
            AssociationType::Aggregation,
            AssociationType::Composition,
            AssociationType::Dependency,
            AssociationType::Anchor,
            AssociationType::Containment,
            AssociationType::Exception,
            AssociationType::Category2Parent,
            AssociationType::Child2Category,
        ] {
            let s = serde_json::to_string(v).unwrap();
            assert!(seen.insert(s), "duplicate serde name for {v}");
        }
    }
}
