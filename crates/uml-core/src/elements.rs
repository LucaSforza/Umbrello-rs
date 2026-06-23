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
use crate::types::{ObjectType, ParameterDirection, Visibility};
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
        }
    }
}

// ─── Attribute ────────────────────────────────────────────────────────

/// A classifier attribute (field / member variable).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    /// Attribute name.
    pub name: String,
    /// Type of the attribute (reference to a UML type by ID).
    pub type_id: Option<UmlId>,
    /// Type name (fallback when the type is not a UML model element).
    pub type_name: Option<String>,
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
    /// Parameter type (reference to a UML type by ID).
    pub type_id: Option<UmlId>,
    /// Parameter type name (fallback).
    pub type_name: Option<String>,
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
    /// Return type (reference to a UML type by ID).
    pub return_type_id: Option<UmlId>,
    /// Return type name (fallback).
    pub return_type_name: Option<String>,
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
        }
    }

    /// Return a mutable reference to the element's base metadata.
    pub fn base_mut(&mut self) -> &mut ElementBase {
        match self {
            Self::Package(p) => &mut p.base,
            Self::Class(c) => &mut c.base,
            Self::Interface(i) => &mut i.base,
            Self::Enum(e) => &mut e.base,
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
        matches!(self, Self::Class(_) | Self::Interface(_) | Self::Enum(_))
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
            Self::Package(_) => None,
        }
    }

    /// Return a mutable reference to the classifier data, if this is a classifier.
    pub fn classifier_data_mut(&mut self) -> Option<&mut ClassifierData> {
        match self {
            Self::Class(c) => Some(&mut c.classifier),
            Self::Interface(i) => Some(&mut i.classifier),
            Self::Enum(e) => Some(&mut e.classifier),
            Self::Package(_) => None,
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
            type_id: None,
            type_name: Some("int".into()),
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
            return_type_id: None,
            return_type_name: Some("void".into()),
            parameters: vec![Parameter {
                name: "x".into(),
                type_id: None,
                type_name: Some("int".into()),
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
            type_id: None,
            type_name: Some("int".into()),
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
            type_id: None,
            type_name: Some("String".into()),
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
}
