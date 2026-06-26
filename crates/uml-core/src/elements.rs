//! UML model element types.
//!
//! This module defines the Rust-native UML metamodel using composition
//! (not inheritance). All element types share `ElementBase` for common
//! metadata, and classifier-like types share `ClassifierData` for
//! attributes/operations.
//!
//! The `ModelElement` enum provides type-safe dispatch without
//! manual RTTI or virtual methods.

use crate::id::UmlId;
use crate::types::{AssociationType, ObjectType, ParameterDirection, Visibility};
use serde::{Deserialize, Serialize};

// ─── NamedElement trait ──────────────────────────────────────────────

/// Common interface for all UML model elements.
///
/// Every model element has an identity (id), a human-readable name, and
/// a visibility level. The trait provides default implementations that
/// delegate to `ElementBase`.
pub trait NamedElement {
    /// Return a shared reference to the element's base metadata.
    fn base(&self) -> &ElementBase;

    /// Return a mutable reference to the element's base metadata.
    fn base_mut(&mut self) -> &mut ElementBase;

    /// The unique identifier of this element.
    fn id(&self) -> UmlId {
        self.base().id
    }

    /// The human-readable name.
    fn name(&self) -> &str {
        &self.base().name
    }

    /// Set the human-readable name.
    fn set_name(&mut self, name: String) {
        self.base_mut().name = name;
    }

    /// The visibility level.
    fn visibility(&self) -> Visibility {
        self.base().visibility
    }

    /// Set the visibility level.
    fn set_visibility(&mut self, vis: Visibility) {
        self.base_mut().visibility = vis;
    }

    /// The UML object type discriminant.
    fn object_type(&self) -> ObjectType;
}

// ─── ElementBase ─────────────────────────────────────────────────────

/// Common metadata shared by all UML model elements.
///
/// Replaces the fields from the C++ `UMLObject` base class.
/// Every variant of `ModelElement` embeds an `ElementBase`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementBase {
    /// Unique identifier.
    pub id: UmlId,
    /// Human-readable name (e.g. class name, package name).
    pub name: String,
    /// Access visibility.
    pub visibility: Visibility,
    /// Optional stereotype reference (by ID).
    pub stereotype_id: Option<UmlId>,
    /// Documentation / comment text.
    #[serde(default)]
    pub documentation: String,
    /// `true` if the element is abstract (cannot be instantiated).
    #[serde(default)]
    pub is_abstract: bool,
    /// `true` if the element is static (class-level, not instance-level).
    #[serde(default)]
    pub is_static: bool,
    /// Original XMI id from the source file.
    /// Preserved for round-trip XMI compatibility.
    /// None for elements created natively in Umbrello-RS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_xmi_id: Option<String>,
}

impl ElementBase {
    /// Create a new base with the given name and a freshly generated ID.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: UmlId::new(),
            name: name.into(),
            visibility: Visibility::default(),
            stereotype_id: None,
            documentation: String::new(),
            is_abstract: false,
            is_static: false,
            original_xmi_id: None,
        }
    }
}

// ─── TypeReference ────────────────────────────────────────────────────

/// A reference to a type in the UML model.
///
/// Types can be either:
/// - A UML classifier (class, interface, enumeration, datatype) referenced by `UmlId`
/// - A primitive or external type referenced by name (e.g., "int", "String")
///
/// At most one of `model_id` or `type_name` should be `Some`. Both `None`
/// means the type is unspecified (e.g., a void return type).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeReference {
    /// Reference to a UML model element (classifier).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<UmlId>,
    /// Type name for primitives or external types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
}

impl TypeReference {
    /// Create an unspecified type reference (both fields None).
    #[must_use]
    pub fn unspecified() -> Self {
        Self {
            model_id: None,
            type_name: None,
        }
    }

    /// Create a type reference to a UML model element.
    #[must_use]
    pub fn model(id: UmlId) -> Self {
        Self {
            model_id: Some(id),
            type_name: None,
        }
    }

    /// Create a type reference to a primitive or external type by name.
    #[must_use]
    pub fn primitive(name: impl Into<String>) -> Self {
        Self {
            model_id: None,
            type_name: Some(name.into()),
        }
    }

    /// Returns `true` if the type is resolved (has either model_id or type_name).
    #[must_use]
    pub fn is_resolved(&self) -> bool {
        self.model_id.is_some() || self.type_name.is_some()
    }

    /// Returns `true` if the type references a UML model element.
    #[must_use]
    pub fn is_model_type(&self) -> bool {
        self.model_id.is_some()
    }

    /// Returns `true` if the type is a primitive or external type name.
    #[must_use]
    pub fn is_primitive(&self) -> bool {
        self.type_name.is_some()
    }

    /// Returns `true` if this reference is internally consistent.
    ///
    /// Both `model_id` and `type_name` being `Some` is ambiguous and invalid.
    /// Both being `None` is valid (unspecified type).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !(self.model_id.is_some() && self.type_name.is_some())
    }

    /// Display the type as a human-readable string.
    ///
    /// Returns the `type_name` if present, otherwise looks up the model
    /// element by `model_id`. Returns `"void"` if neither is set, and
    /// `"<unknown:id>"` if the model_id does not resolve.
    #[must_use]
    pub fn display_name(&self, model: Option<&crate::repository::UmlModel>) -> String {
        if let Some(ref name) = self.type_name {
            name.clone()
        } else if let Some(id) = self.model_id {
            model
                .and_then(|m| m.get(id))
                .map_or_else(|| format!("<unknown:{id}>"), |e| e.name().to_string())
        } else {
            "void".to_string()
        }
    }
}

impl Default for TypeReference {
    fn default() -> Self {
        Self::unspecified()
    }
}

// ─── Attribute ────────────────────────────────────────────────────────

/// A classifier attribute (field / member variable).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    /// Attribute name.
    pub name: String,
    /// The type of this attribute.
    #[serde(default)]
    pub type_ref: TypeReference,
    /// Visibility.
    pub visibility: Visibility,
    /// Initial value expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_value: Option<String>,
    /// Whether the attribute is static (class-level).
    #[serde(default)]
    pub is_static: bool,
}

// ─── Parameter ────────────────────────────────────────────────────────

/// An operation parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name.
    pub name: String,
    /// The type of this parameter.
    #[serde(default)]
    pub type_ref: TypeReference,
    /// Parameter direction (in, out, inout, return).
    pub direction: ParameterDirection,
    /// Default value expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
}

// ─── Operation ────────────────────────────────────────────────────────

/// A classifier operation (method).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    /// Operation name.
    pub name: String,
    /// Return type.
    #[serde(default)]
    pub return_type: TypeReference,
    /// Formal parameters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
    /// Visibility.
    pub visibility: Visibility,
    /// Whether the operation is static (class-level).
    #[serde(default)]
    pub is_static: bool,
    /// Whether the operation has no implementation.
    #[serde(default)]
    pub is_abstract: bool,
    /// Whether the operation is virtual / overridable.
    #[serde(default)]
    pub is_virtual: bool,
}

// ─── TemplateParameter ────────────────────────────────────────────────

/// A template / generic type parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateParameter {
    /// Parameter name (e.g. `T`, `K`, `V`).
    pub name: String,
    /// Type constraint (e.g. `class`, `Comparable`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraint: Option<String>,
}

// ─── EnumLiteral ──────────────────────────────────────────────────────

/// An enumeration literal value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumLiteral {
    /// Literal name.
    pub name: String,
    /// Optional explicit value (e.g. `= 42`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

// ─── ClassifierData (composition, not inheritance) ────────────────────

/// Data shared by classifier-like element types.
///
/// Instead of a `UMLClassifier` base class in the inheritance chain,
/// this struct is embedded in `Class`, `Interface`, and `Enum`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassifierData {
    /// Attributes (member variables / fields).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<Attribute>,
    /// Operations (methods).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operations: Vec<Operation>,
    /// Template / generic parameters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<TemplateParameter>,
}

impl ClassifierData {
    /// Create empty classifier data.
    #[must_use]
    pub fn new() -> Self {
        Self {
            attributes: Vec::new(),
            operations: Vec::new(),
            templates: Vec::new(),
        }
    }

    /// Add an attribute.
    pub fn add_attribute(&mut self, attr: Attribute) {
        self.attributes.push(attr);
    }

    /// Add an operation.
    pub fn add_operation(&mut self, op: Operation) {
        self.operations.push(op);
    }

    /// Add a template parameter.
    pub fn add_template(&mut self, tparam: TemplateParameter) {
        self.templates.push(tparam);
    }
}

impl Default for ClassifierData {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Concrete element types ──────────────────────────────────────────

/// A UML package — a namespace that contains other model elements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Package {
    /// Common element metadata.
    pub base: ElementBase,
    /// IDs of child elements contained in this package.
    /// Use `UmlModel::add_to_package()` / `remove_from_package()` to modify.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) children: Vec<UmlId>,
}

impl Package {
    /// Create a new package with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            base: ElementBase::new(name),
            children: Vec::new(),
        }
    }

    /// Add a child element by ID.
    /// Prefer using `UmlModel::add_to_package()` which also maintains the parent index.
    #[allow(dead_code)]
    pub(crate) fn add_child(&mut self, child_id: UmlId) {
        self.children.push(child_id);
    }

    /// Remove a child by ID. Returns `true` if the child was found and removed.
    /// Prefer using `UmlModel::remove_from_package()` which also maintains the parent index.
    #[allow(dead_code)]
    pub(crate) fn remove_child(&mut self, child_id: UmlId) -> bool {
        if let Some(pos) = self.children.iter().position(|&id| id == child_id) {
            self.children.remove(pos);
            true
        } else {
            false
        }
    }

    /// Iterate over child element IDs.
    pub fn child_ids(&self) -> impl Iterator<Item = UmlId> + '_ {
        self.children.iter().copied()
    }

    /// Number of direct children.
    #[must_use]
    pub fn child_count(&self) -> usize {
        self.children.len()
    }
}

/// A UML class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Class {
    /// Common element metadata.
    pub base: ElementBase,
    /// Classifier data: attributes, operations, templates.
    #[serde(default)]
    pub classifier: ClassifierData,
}

impl Class {
    /// Create a new class with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            base: ElementBase::new(name),
            classifier: ClassifierData::default(),
        }
    }

    /// Create a new class that is abstract.
    #[must_use]
    pub fn new_abstract(name: impl Into<String>) -> Self {
        Self {
            base: ElementBase {
                is_abstract: true,
                ..ElementBase::new(name)
            },
            classifier: ClassifierData::default(),
        }
    }
}

/// A UML interface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interface {
    /// Common element metadata.
    pub base: ElementBase,
    /// Classifier data: attributes, operations, templates.
    #[serde(default)]
    pub classifier: ClassifierData,
}

impl Interface {
    /// Create a new interface with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            base: ElementBase {
                is_abstract: true, // interfaces are always abstract
                ..ElementBase::new(name)
            },
            classifier: ClassifierData::default(),
        }
    }
}

/// A UML enumeration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Enum {
    /// Common element metadata.
    pub base: ElementBase,
    /// Classifier data: attributes, operations, templates.
    #[serde(default)]
    pub classifier: ClassifierData,
    /// Enumeration literals.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub literals: Vec<EnumLiteral>,
}

impl Enum {
    /// Create a new enumeration with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            base: ElementBase::new(name),
            classifier: ClassifierData::default(),
            literals: Vec::new(),
        }
    }

    /// Add an enum literal.
    pub fn add_literal(&mut self, name: impl Into<String>, value: Option<String>) {
        self.literals.push(EnumLiteral {
            name: name.into(),
            value,
        });
    }

    /// Number of literals.
    #[must_use]
    pub fn literal_count(&self) -> usize {
        self.literals.len()
    }
}

/// A UML datatype (primitive or structured type).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Datatype {
    /// Common element metadata.
    pub base: ElementBase,
    /// Classifier data: attributes, operations, templates.
    #[serde(default)]
    pub classifier: ClassifierData,
}

impl Datatype {
    /// Create a new datatype with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            base: ElementBase::new(name),
            classifier: ClassifierData::default(),
        }
    }
}

// ─── Relationship ──────────────────────────────────────────────────────

/// A UML relationship between two model elements.
///
/// Relationships are first-class model elements with their own `UmlId`,
/// name, stereotype, and documentation. They connect a source element
/// to a target element via `UmlId` references.
///
/// # UML Semantics
///
/// - **Generalization** — source is the subclass, target is the superclass.
/// - **Realization** — source is the implementing class, target is the interface.
/// - **Association** — bidirectional or unidirectional reference.
/// - **Aggregation** — whole-part with shared lifecycle (source is whole).
/// - **Composition** — whole-part with exclusive lifecycle (source is whole).
/// - **Dependency** — source depends on target (uses relationship).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    /// Common element metadata.
    pub base: ElementBase,
    /// The kind of relationship.
    pub kind: AssociationType,
    /// The source element (e.g., subclass in a generalization).
    pub source_id: UmlId,
    /// The target element (e.g., superclass in a generalization).
    pub target_id: UmlId,
    /// Multiplicity at the source end (e.g., "1", "0..*", "1..*").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_multiplicity: Option<String>,
    /// Multiplicity at the target end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_multiplicity: Option<String>,
    /// Role name at the source end (e.g., "employee").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_role_name: Option<String>,
    /// Role name at the target end (e.g., "employer").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_role_name: Option<String>,
    /// Whether navigation from source to target is supported.
    #[serde(default)]
    pub source_to_target_navigable: bool,
    /// Whether navigation from target to source is supported.
    #[serde(default)]
    pub target_to_source_navigable: bool,
}

impl Relationship {
    /// Create a new relationship.
    #[must_use]
    pub fn new(kind: AssociationType, source_id: UmlId, target_id: UmlId) -> Self {
        Self {
            base: ElementBase::new(""),
            kind,
            source_id,
            target_id,
            source_multiplicity: None,
            target_multiplicity: None,
            source_role_name: None,
            target_role_name: None,
            source_to_target_navigable: false,
            target_to_source_navigable: false,
        }
    }

    /// Create a generalization (subclass → superclass).
    #[must_use]
    pub fn new_generalization(subclass_id: UmlId, superclass_id: UmlId) -> Self {
        Self::new(AssociationType::Generalization, subclass_id, superclass_id)
    }

    /// Create an interface realization.
    #[must_use]
    pub fn new_realization(class_id: UmlId, interface_id: UmlId) -> Self {
        Self::new(AssociationType::Realization, class_id, interface_id)
    }

    /// Create a plain association.
    #[must_use]
    pub fn new_association(source_id: UmlId, target_id: UmlId) -> Self {
        Self::new(AssociationType::Association, source_id, target_id)
    }

    /// Create an aggregation (source is the whole, target is the part).
    #[must_use]
    pub fn new_aggregation(whole_id: UmlId, part_id: UmlId) -> Self {
        Self::new(AssociationType::Aggregation, whole_id, part_id)
    }

    /// Create a composition (source is the whole, target is the part).
    #[must_use]
    pub fn new_composition(whole_id: UmlId, part_id: UmlId) -> Self {
        Self::new(AssociationType::Composition, whole_id, part_id)
    }

    /// Create a dependency.
    #[must_use]
    pub fn new_dependency(source_id: UmlId, target_id: UmlId) -> Self {
        Self::new(AssociationType::Dependency, source_id, target_id)
    }

    /// The ObjectType discriminant corresponding to this relationship's kind.
    #[must_use]
    pub fn object_type(&self) -> ObjectType {
        match self.kind {
            AssociationType::Generalization => ObjectType::Generalization,
            AssociationType::Realization => ObjectType::Realization,
            AssociationType::Dependency => ObjectType::Dependency,
            _ => ObjectType::Association,
        }
    }
}

// ─── Actor ────────────────────────────────────────────────────────────

/// A UML Actor — represents a role played by a user or external system.
///
/// Actors are participants in Use Case diagrams. They are non-classifier,
/// non-container elements that carry only identity and metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Actor {
    /// Common element metadata.
    pub base: ElementBase,
}

impl Actor {
    /// Create a new Actor with the given name.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            base: ElementBase {
                id: UmlId::new(),
                name: name.to_string(),
                visibility: Visibility::Public,
                stereotype_id: None,
                documentation: String::new(),
                is_abstract: false,
                is_static: false,
                original_xmi_id: None,
            },
        }
    }
}

// ─── UseCase ───────────────────────────────────────────────────────────

/// A UML UseCase — represents a unit of functionality provided by the system.
///
/// UseCases appear as ovals in Use Case diagrams. They are non-classifier,
/// non-container elements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UseCase {
    /// Common element metadata.
    pub base: ElementBase,
}

impl UseCase {
    /// Create a new UseCase with the given name.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            base: ElementBase {
                id: UmlId::new(),
                name: name.to_string(),
                visibility: Visibility::Public,
                stereotype_id: None,
                documentation: String::new(),
                is_abstract: false,
                is_static: false,
                original_xmi_id: None,
            },
        }
    }
}

// ─── ModelElement enum (type-safe dispatch) ─────────────────────────

/// A UML model element.
///
/// All element types are variants of this enum, enabling storage in a flat
/// arena and pattern-match dispatch. This replaces the C++ inheritance tree
/// and the 28 `isUML*()`/`asUML*()` manual RTTI methods.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelElement {
    /// A UML package — a namespace container.
    Package(Package),
    /// A UML class.
    Class(Class),
    /// A UML interface.
    Interface(Interface),
    /// A UML enumeration.
    Enum(Enum),
    /// A UML datatype (primitive or structured type).
    Datatype(Datatype),
    /// A UML relationship (generalization, association, aggregation, composition, dependency, realization).
    Relationship(Relationship),
    /// An Actor in a Use Case diagram.
    Actor(Actor),
    /// A UseCase in a Use Case diagram.
    UseCase(UseCase),
}

impl ModelElement {
    /// Return the `ObjectType` discriminant for this element.
    #[must_use]
    pub fn object_type(&self) -> ObjectType {
        match self {
            Self::Package(_) => ObjectType::Package,
            Self::Class(_) => ObjectType::Class,
            Self::Interface(_) => ObjectType::Interface,
            Self::Enum(_) => ObjectType::Enumeration,
            Self::Datatype(_) => ObjectType::Datatype,
            Self::Relationship(rel) => rel.object_type(),
            Self::Actor(_) => ObjectType::Actor,
            Self::UseCase(_) => ObjectType::UseCase,
        }
    }

    /// Return a shared reference to the element's base metadata.
    #[must_use]
    pub fn base(&self) -> &ElementBase {
        match self {
            Self::Package(p) => &p.base,
            Self::Class(c) => &c.base,
            Self::Interface(i) => &i.base,
            Self::Enum(e) => &e.base,
            Self::Datatype(d) => &d.base,
            Self::Relationship(r) => &r.base,
            Self::Actor(a) => &a.base,
            Self::UseCase(u) => &u.base,
        }
    }

    /// Return a mutable reference to the element's base metadata.
    pub fn base_mut(&mut self) -> &mut ElementBase {
        match self {
            Self::Package(p) => &mut p.base,
            Self::Class(c) => &mut c.base,
            Self::Interface(i) => &mut i.base,
            Self::Enum(e) => &mut e.base,
            Self::Datatype(d) => &mut d.base,
            Self::Relationship(r) => &mut r.base,
            Self::Actor(a) => &mut a.base,
            Self::UseCase(u) => &mut u.base,
        }
    }

    /// The unique identifier of this element.
    #[must_use]
    pub fn id(&self) -> UmlId {
        self.base().id
    }

    /// The human-readable name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.base().name
    }

    /// Set the name.
    pub fn set_name(&mut self, name: String) {
        self.base_mut().name = name;
    }

    /// Check if this element is a classifier (has attributes/operations).
    #[must_use]
    pub fn is_classifier(&self) -> bool {
        matches!(self, Self::Class(_) | Self::Interface(_) | Self::Enum(_) | Self::Datatype(_))
    }

    /// Check if this element is a package.
    #[must_use]
    pub fn is_package(&self) -> bool {
        matches!(self, Self::Package(_))
    }

    /// Return a reference to the classifier data, if this is a classifier.
    #[must_use]
    pub fn classifier_data(&self) -> Option<&ClassifierData> {
        match self {
            Self::Class(c) => Some(&c.classifier),
            Self::Interface(i) => Some(&i.classifier),
            Self::Enum(e) => Some(&e.classifier),
            Self::Datatype(d) => Some(&d.classifier),
            Self::Package(_) | Self::Relationship(_) | Self::Actor(_) | Self::UseCase(_) => None,
        }
    }

    /// Return a mutable reference to the classifier data, if this is a classifier.
    pub fn classifier_data_mut(&mut self) -> Option<&mut ClassifierData> {
        match self {
            Self::Class(c) => Some(&mut c.classifier),
            Self::Interface(i) => Some(&mut i.classifier),
            Self::Enum(e) => Some(&mut e.classifier),
            Self::Datatype(d) => Some(&mut d.classifier),
            Self::Package(_) | Self::Relationship(_) | Self::Actor(_) | Self::UseCase(_) => None,
        }
    }
}

// ─── NamedElement impl for ModelElement ───────────────────────────────

impl NamedElement for ModelElement {
    fn base(&self) -> &ElementBase {
        self.base()
    }

    fn base_mut(&mut self) -> &mut ElementBase {
        self.base_mut()
    }

    fn object_type(&self) -> ObjectType {
        self.object_type()
    }
}

// ─── Unit tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_creation() {
        let pkg = Package::new("MyPackage");
        assert_eq!(pkg.base.name, "MyPackage");
        assert_eq!(pkg.child_count(), 0);
        assert_eq!(pkg.base.visibility, Visibility::default());
    }

    #[test]
    fn package_add_remove_children() {
        let mut pkg = Package::new("Root");
        let child1 = UmlId::new();
        let child2 = UmlId::new();

        pkg.add_child(child1);
        pkg.add_child(child2);
        assert_eq!(pkg.child_count(), 2);

        let removed = pkg.remove_child(child1);
        assert!(removed);
        assert_eq!(pkg.child_count(), 1);

        // Removing non-existent returns false
        assert!(!pkg.remove_child(UmlId::new()));
    }

    #[test]
    fn class_creation() {
        let cls = Class::new("MyClass");
        assert_eq!(cls.base.name, "MyClass");
        assert!(!cls.base.is_abstract);
        assert_eq!(cls.classifier.attributes.len(), 0);
        assert_eq!(cls.classifier.operations.len(), 0);
    }

    #[test]
    fn class_abstract_creation() {
        let cls = Class::new_abstract("AbstractBase");
        assert!(cls.base.is_abstract);
    }

    #[test]
    fn interface_creation() {
        let iface = Interface::new("MyInterface");
        assert_eq!(iface.base.name, "MyInterface");
        assert!(iface.base.is_abstract, "interfaces are always abstract");
    }

    #[test]
    fn enum_creation_and_literals() {
        let mut enm = Enum::new("Color");
        assert_eq!(enm.literal_count(), 0);

        enm.add_literal("Red", Some("0".into()));
        enm.add_literal("Green", Some("1".into()));
        enm.add_literal("Blue", None);

        assert_eq!(enm.literal_count(), 3);
        assert_eq!(enm.literals[0].name, "Red");
        assert_eq!(enm.literals[0].value, Some("0".to_string()));
        assert_eq!(enm.literals[2].value, None);
    }

    #[test]
    fn classifier_data_add_attribute() {
        let mut data = ClassifierData::new();
        data.add_attribute(Attribute {
            name: "count".into(),
            type_ref: TypeReference::primitive("int"),
            visibility: Visibility::Private,
            initial_value: Some("0".into()),
            is_static: false,
        });
        assert_eq!(data.attributes.len(), 1);
        assert_eq!(data.attributes[0].name, "count");
    }

    #[test]
    fn classifier_data_add_operation() {
        let mut data = ClassifierData::new();
        data.add_operation(Operation {
            name: "doSomething".into(),
            return_type: TypeReference::primitive("void"),
            parameters: vec![Parameter {
                name: "x".into(),
                type_ref: TypeReference::primitive("int"),
                direction: ParameterDirection::In,
                default_value: None,
            }],
            visibility: Visibility::Public,
            is_static: false,
            is_abstract: false,
            is_virtual: true,
        });
        assert_eq!(data.operations.len(), 1);
        assert_eq!(data.operations[0].name, "doSomething");
        assert_eq!(data.operations[0].parameters.len(), 1);
    }

    #[test]
    fn model_element_object_type() {
        assert_eq!(ModelElement::Package(Package::new("P")).object_type(), ObjectType::Package);
        assert_eq!(ModelElement::Class(Class::new("C")).object_type(), ObjectType::Class);
        assert_eq!(
            ModelElement::Interface(Interface::new("I")).object_type(),
            ObjectType::Interface
        );
        assert_eq!(ModelElement::Enum(Enum::new("E")).object_type(), ObjectType::Enumeration);
    }

    #[test]
    fn model_element_is_classifier() {
        assert!(ModelElement::Class(Class::new("C")).is_classifier());
        assert!(ModelElement::Interface(Interface::new("I")).is_classifier());
        assert!(ModelElement::Enum(Enum::new("E")).is_classifier());
        assert!(!ModelElement::Package(Package::new("P")).is_classifier());
    }

    #[test]
    fn model_element_is_package() {
        assert!(ModelElement::Package(Package::new("P")).is_package());
        assert!(!ModelElement::Class(Class::new("C")).is_package());
    }

    #[test]
    fn model_element_classifier_data_access() {
        let mut cls = Class::new("C");
        cls.classifier.add_attribute(Attribute {
            name: "x".into(),
            type_ref: TypeReference::primitive("int"),
            visibility: Visibility::Private,
            initial_value: None,
            is_static: false,
        });

        let elem = ModelElement::Class(cls);
        assert!(elem.classifier_data().is_some());
        assert_eq!(elem.classifier_data().unwrap().attributes.len(), 1);

        let pkg_elem = ModelElement::Package(Package::new("P"));
        assert!(pkg_elem.classifier_data().is_none());
    }

    #[test]
    fn model_element_name_and_id() {
        let id = UmlId::new();
        let name = "TestClass";
        let mut base = ElementBase::new(name);
        base.id = id;

        let elem = ModelElement::Class(Class {
            base,
            classifier: ClassifierData::default(),
        });

        assert_eq!(elem.id(), id);
        assert_eq!(elem.name(), name);
    }

    #[test]
    fn model_element_set_name() {
        let mut elem = ModelElement::Class(Class::new("OldName"));
        elem.set_name("NewName".into());
        assert_eq!(elem.name(), "NewName");
    }

    #[test]
    fn named_element_trait() {
        let elem = ModelElement::Class(Class::new("TraitTest"));
        // Test that the trait methods work through the trait object
        assert_eq!(NamedElement::name(&elem), "TraitTest");
        assert_eq!(NamedElement::object_type(&elem), ObjectType::Class);
        assert!(!NamedElement::name(&elem).is_empty());
    }

    #[test]
    fn element_base_defaults() {
        let base = ElementBase::new("Test");
        assert_eq!(base.name, "Test");
        assert_eq!(base.visibility, Visibility::Public);
        assert!(!base.is_abstract);
        assert!(!base.is_static);
        assert!(base.documentation.is_empty());
        assert!(base.stereotype_id.is_none());
    }

    #[test]
    fn serde_roundtrip_package() {
        let mut pkg = Package::new("Root");
        pkg.add_child(UmlId::new());

        let json = serde_json::to_string(&pkg).unwrap();
        let back: Package = serde_json::from_str(&json).unwrap();
        assert_eq!(pkg, back);
    }

    #[test]
    fn serde_roundtrip_class() {
        let mut cls = Class::new("MyClass");
        cls.classifier.add_attribute(Attribute {
            name: "field".into(),
            type_ref: TypeReference::primitive("String"),
            visibility: Visibility::Private,
            initial_value: None,
            is_static: false,
        });

        let json = serde_json::to_string(&cls).unwrap();
        let back: Class = serde_json::from_str(&json).unwrap();
        assert_eq!(cls, back);
    }

    #[test]
    fn serde_roundtrip_interface() {
        let iface = Interface::new("Serializable");

        let json = serde_json::to_string(&iface).unwrap();
        let back: Interface = serde_json::from_str(&json).unwrap();
        assert_eq!(iface, back);
    }

    #[test]
    fn serde_roundtrip_enum() {
        let mut enm = Enum::new("Color");
        enm.add_literal("Red", Some("0xFF0000".into()));
        enm.add_literal("Green", None);

        let json = serde_json::to_string(&enm).unwrap();
        let back: Enum = serde_json::from_str(&json).unwrap();
        assert_eq!(enm, back);
    }

    #[test]
    fn serde_roundtrip_model_element() {
        let elem = ModelElement::Class(Class::new("Foo"));

        let json = serde_json::to_string(&elem).unwrap();
        let back: ModelElement = serde_json::from_str(&json).unwrap();
        assert_eq!(elem, back);
    }

    // ── Datatype tests ───────────────────────────────────────────────

    #[test]
    fn datatype_creation() {
        let dt = Datatype::new("int");
        assert_eq!(dt.base.name, "int");
        assert!(!dt.base.is_abstract);
    }

    #[test]
    fn model_element_datatype_object_type() {
        let elem = ModelElement::Datatype(Datatype::new("String"));
        assert_eq!(elem.object_type(), ObjectType::Datatype);
        assert!(elem.is_classifier());
        assert!(elem.classifier_data().is_some());
    }

    #[test]
    fn serde_roundtrip_datatype() {
        let dt = Datatype::new("float");
        let json = serde_json::to_string(&dt).unwrap();
        let back: Datatype = serde_json::from_str(&json).unwrap();
        assert_eq!(dt, back);
    }

    #[test]
    fn element_base_original_xmi_id() {
        let base = ElementBase::new("Test");
        assert!(base.original_xmi_id.is_none());

        let mut base = ElementBase::new("FromXMI");
        base.original_xmi_id = Some("O0JJV24XoKdQ".into());
        assert_eq!(base.original_xmi_id, Some("O0JJV24XoKdQ".to_string()));
    }

    // ── Relationship tests ───────────────────────────────────────────

    #[test]
    fn relationship_creation() {
        let source = UmlId::new();
        let target = UmlId::new();
        let rel = Relationship::new(AssociationType::Generalization, source, target);
        assert_eq!(rel.kind, AssociationType::Generalization);
        assert_eq!(rel.source_id, source);
        assert_eq!(rel.target_id, target);
        assert!(rel.source_multiplicity.is_none());
    }

    #[test]
    fn relationship_constructor_methods() {
        let a = UmlId::new();
        let b = UmlId::new();

        let gen = Relationship::new_generalization(a, b);
        assert_eq!(gen.kind, AssociationType::Generalization);

        let real = Relationship::new_realization(a, b);
        assert_eq!(real.kind, AssociationType::Realization);

        let assoc = Relationship::new_association(a, b);
        assert_eq!(assoc.kind, AssociationType::Association);

        let agg = Relationship::new_aggregation(a, b);
        assert_eq!(agg.kind, AssociationType::Aggregation);

        let comp = Relationship::new_composition(a, b);
        assert_eq!(comp.kind, AssociationType::Composition);

        let dep = Relationship::new_dependency(a, b);
        assert_eq!(dep.kind, AssociationType::Dependency);
    }

    #[test]
    fn relationship_object_type() {
        assert_eq!(
            Relationship::new_generalization(UmlId::new(), UmlId::new()).object_type(),
            ObjectType::Generalization
        );
        assert_eq!(
            Relationship::new_realization(UmlId::new(), UmlId::new()).object_type(),
            ObjectType::Realization
        );
        assert_eq!(
            Relationship::new_dependency(UmlId::new(), UmlId::new()).object_type(),
            ObjectType::Dependency
        );
        assert_eq!(
            Relationship::new_association(UmlId::new(), UmlId::new()).object_type(),
            ObjectType::Association
        );
    }

    #[test]
    fn model_element_relationship_object_type() {
        let rel = Relationship::new_generalization(UmlId::new(), UmlId::new());
        let elem = ModelElement::Relationship(rel);
        assert_eq!(elem.object_type(), ObjectType::Generalization);
        assert!(elem.classifier_data().is_none());
        assert!(!elem.is_classifier());
        assert!(!elem.is_package());
    }

    #[test]
    fn serde_roundtrip_relationship() {
        let mut rel = Relationship::new_generalization(UmlId::new(), UmlId::new());
        rel.source_multiplicity = Some("1".into());
        rel.target_multiplicity = Some("0..*".into());
        rel.source_role_name = Some("parent".into());
        rel.target_role_name = Some("child".into());

        let json = serde_json::to_string(&rel).unwrap();
        let back: Relationship = serde_json::from_str(&json).unwrap();
        assert_eq!(rel, back);
    }

    // ── TypeReference tests ──────────────────────────────────────────

    #[test]
    fn type_reference_unspecified() {
        let tr = TypeReference::unspecified();
        assert!(!tr.is_resolved());
        assert!(!tr.is_model_type());
        assert!(!tr.is_primitive());
        assert!(tr.is_valid());
        assert_eq!(tr.display_name(None), "void");
    }

    #[test]
    fn type_reference_model() {
        let id = UmlId::new();
        let tr = TypeReference::model(id);
        assert!(tr.is_resolved());
        assert!(tr.is_model_type());
        assert!(!tr.is_primitive());
        assert!(tr.is_valid());
        assert_eq!(tr.model_id, Some(id));
    }

    #[test]
    fn type_reference_primitive() {
        let tr = TypeReference::primitive("int");
        assert!(tr.is_resolved());
        assert!(!tr.is_model_type());
        assert!(tr.is_primitive());
        assert!(tr.is_valid());
        assert_eq!(tr.display_name(None), "int");
    }

    #[test]
    fn type_reference_both_set_is_invalid() {
        let tr = TypeReference {
            model_id: Some(UmlId::new()),
            type_name: Some("int".into()),
        };
        assert!(!tr.is_valid());
    }

    #[test]
    fn type_reference_default() {
        let tr = TypeReference::default();
        assert!(!tr.is_resolved());
        assert!(tr.is_valid());
    }

    #[test]
    fn type_reference_display_name_with_model() {
        use crate::repository::UmlModel;

        let mut model = UmlModel::new();
        let cls = Class::new("Person");
        let cls_id = cls.base.id;
        model.insert(ModelElement::Class(cls));

        let tr = TypeReference::model(cls_id);
        assert_eq!(tr.display_name(Some(&model)), "Person");
    }

    #[test]
    fn type_reference_display_name_dangling() {
        let dangling = UmlId::new();
        let tr = TypeReference::model(dangling);
        let display = tr.display_name(None);
        assert!(display.starts_with("<unknown:"));
    }

    #[test]
    fn type_reference_serde_roundtrip_all_states() {
        // Unspecified
        let tr = TypeReference::unspecified();
        let json = serde_json::to_string(&tr).unwrap();
        let back: TypeReference = serde_json::from_str(&json).unwrap();
        assert_eq!(tr, back);
        assert!(back.is_valid());

        // Model reference
        let tr = TypeReference::model(UmlId::new());
        let json = serde_json::to_string(&tr).unwrap();
        let back: TypeReference = serde_json::from_str(&json).unwrap();
        assert_eq!(tr, back);

        // Primitive
        let tr = TypeReference::primitive("String");
        let json = serde_json::to_string(&tr).unwrap();
        let back: TypeReference = serde_json::from_str(&json).unwrap();
        assert_eq!(tr, back);
    }

    // ── Actor tests ────────────────────────────────────────────────

    #[test]
    fn actor_creation() {
        let actor = Actor::new("User");
        assert_eq!(actor.base.name, "User");
        assert_eq!(actor.base.visibility, Visibility::Public);
        assert!(actor.base.stereotype_id.is_none());
        assert!(actor.base.documentation.is_empty());
        assert!(!actor.base.is_abstract);
        assert!(!actor.base.is_static);
    }

    #[test]
    fn actor_model_element_insert() {
        use crate::repository::UmlModel;

        let mut model = UmlModel::new();
        let actor = Actor::new("User");
        let id = actor.base.id;
        model.insert(ModelElement::Actor(actor));

        let retrieved = model.get(id).expect("Actor should be in model");
        assert_eq!(retrieved.object_type(), ObjectType::Actor);
        assert_eq!(retrieved.name(), "User");
    }

    #[test]
    fn actor_not_classifier() {
        let elem = ModelElement::Actor(Actor::new("User"));
        assert!(!elem.is_classifier());
    }

    #[test]
    fn actor_not_container() {
        let elem = ModelElement::Actor(Actor::new("User"));
        assert!(!elem.is_package());
    }

    #[test]
    fn serde_roundtrip_actor() {
        let actor = Actor::new("Administrator");
        let json = serde_json::to_string(&actor).unwrap();
        let back: Actor = serde_json::from_str(&json).unwrap();
        assert_eq!(actor, back);
    }

    // ── UseCase tests ──────────────────────────────────────────────

    #[test]
    fn usecase_creation() {
        let uc = UseCase::new("Login");
        assert_eq!(uc.base.name, "Login");
        assert_eq!(uc.base.visibility, Visibility::Public);
        assert!(uc.base.stereotype_id.is_none());
        assert!(uc.base.documentation.is_empty());
    }

    #[test]
    fn usecase_model_element_insert() {
        use crate::repository::UmlModel;

        let mut model = UmlModel::new();
        let uc = UseCase::new("Login");
        let id = uc.base.id;
        model.insert(ModelElement::UseCase(uc));

        let retrieved = model.get(id).expect("UseCase should be in model");
        assert_eq!(retrieved.object_type(), ObjectType::UseCase);
        assert_eq!(retrieved.name(), "Login");
    }

    #[test]
    fn usecase_not_classifier() {
        let elem = ModelElement::UseCase(UseCase::new("Search"));
        assert!(!elem.is_classifier());
    }

    #[test]
    fn serde_roundtrip_usecase() {
        let uc = UseCase::new("Checkout");
        let json = serde_json::to_string(&uc).unwrap();
        let back: UseCase = serde_json::from_str(&json).unwrap();
        assert_eq!(uc, back);
    }
}
