//! Type enumerations for UML concepts.
//!
//! These enums replace the scattered C++ enums from basictypes.h and UMLObject::ObjectType.

use serde::{Deserialize, Serialize};

/// Types of UML model elements.
///
/// This enum provides runtime type identification, replacing the C++
/// `UMLObject::ObjectType` enum and the 28 `isUML*()` methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    /// A UML class.
    Class,
    /// A UML interface.
    Interface,
    /// A UML enumeration.
    Enumeration,
    /// A UML datatype.
    Datatype,
    /// An entity-relationship entity.
    Entity,
    /// A UML package.
    Package,
    /// A UML folder (diagram container).
    Folder,
    /// A UML component.
    Component,
    /// A UML artifact.
    Artifact,
    /// A UML actor.
    Actor,
    /// A UML use case.
    UseCase,
    /// A UML deployment node.
    Node,
    /// A UML port.
    Port,
    /// A UML category (EER specialization).
    Category,
    /// A UML instance (object diagram).
    Instance,
    /// A classifier attribute.
    Attribute,
    /// A classifier operation/method.
    Operation,
    /// A template/generic parameter.
    Template,
    /// An enumeration literal.
    EnumLiteral,
    /// An entity attribute (database field).
    EntityAttribute,
    /// A unique constraint.
    UniqueConstraint,
    /// A foreign key constraint.
    ForeignKeyConstraint,
    /// A check constraint.
    CheckConstraint,
    /// A UML association between elements.
    Association,
    /// An association role/end.
    Role,
    /// A UML stereotype.
    Stereotype,
    /// An instance attribute value.
    InstanceAttribute,
}

/// Visibility levels for UML elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Visibility {
    /// Public visibility (`+`).
    Public,
    /// Protected visibility (`#`).
    Protected,
    /// Private visibility (`-`).
    Private,
    /// Implementation-level visibility (`~`).
    Implementation,
}

/// Diagram types supported by Umbrello.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiagramType {
    /// Undefined / unknown diagram type.
    Undefined,
    /// UML class diagram.
    Class,
    /// UML use case diagram.
    UseCase,
    /// UML sequence diagram.
    Sequence,
    /// UML collaboration diagram.
    Collaboration,
    /// UML state diagram.
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
