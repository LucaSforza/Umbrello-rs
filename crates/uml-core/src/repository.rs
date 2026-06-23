//! Model repository — central storage for all UML model elements.
//!
//! The `UmlModel` owns all `ModelElement` values. Packages reference elements
//! by `UmlId` — they do not own them. Uses `IndexMap` for deterministic
//! insertion-order iteration and O(1) lookup by ID.

use std::collections::HashMap;

use indexmap::IndexMap;

use crate::elements::{ClassifierData, ModelElement};
use crate::id::UmlId;

/// Central storage for all UML model elements.
///
/// Owns all elements by value. Packages reference elements via `UmlId` —
/// they do not own them. Uses `IndexMap` for deterministic insertion-order
/// iteration and O(1) lookup by ID.
///
/// # Ownership Model
///
/// The repository is the single source of truth for element ownership:
/// - Elements are stored by value in `elements: IndexMap<UmlId, ModelElement>`.
/// - Packages store only `Vec<UmlId>` references to their children.
/// - The `parent_index` maintains a reverse mapping for O(1) membership queries.
///
/// # Deterministic Iteration
///
/// Elements iterate in insertion order. During XMI loading, elements are
/// inserted in tree-walk order (root packages first, then children). Tests
/// can assert on iteration order without sorting.
#[derive(Debug, Clone)]
pub struct UmlModel {
    /// All elements, keyed by UmlId. Insertion order is preserved.
    elements: IndexMap<UmlId, ModelElement>,
    /// Reverse index: element_id → set of package_ids that contain it.
    /// Maintained automatically by add_to_package / remove_from_package.
    parent_index: HashMap<UmlId, Vec<UmlId>>,
}

/// Errors that can occur during model operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ModelError {
    /// Element with the given ID was not found.
    #[error("element not found: {0}")]
    ElementNotFound(UmlId),

    /// The target element is not a child of the specified package.
    #[error("element {child} is not a child of package {parent}")]
    NotAChild {
        /// The package that was expected to contain the child.
        parent: UmlId,
        /// The element that is not a child.
        child: UmlId,
    },

    /// Adding the child would create a containment cycle.
    #[error("adding {child} to {parent} would create a containment cycle")]
    WouldCreateCycle {
        /// The package being added to.
        parent: UmlId,
        /// The element that would create a cycle.
        child: UmlId,
    },

    /// Operation is not supported for this element type.
    #[error("operation not supported for element type")]
    UnsupportedOperation,
}

/// A dangling reference found during validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceError {
    /// The ID of the element that contains the dangling reference.
    pub source_id: UmlId,
    /// The field or context where the dangling reference was found.
    pub field: ReferenceField,
    /// The dangling ID that does not resolve to any element.
    pub target_id: UmlId,
}

/// The specific field where a dangling reference was found.
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceField {
    /// In a Package's children list.
    PackageChild,
    /// In an Attribute's type_id.
    AttributeType,
    /// In an Operation's return_type_id.
    OperationReturnType,
    /// In a Parameter's type_id.
    ParameterType,
    /// In an ElementBase's stereotype_id.
    Stereotype,
}

impl UmlModel {
    /// Create a new, empty model.
    #[must_use]
    pub fn new() -> Self {
        Self {
            elements: IndexMap::new(),
            parent_index: HashMap::new(),
        }
    }

    /// Insert an element. The element's embedded `UmlId` is used as the key.
    ///
    /// If an element with the same ID already exists, the old element is
    /// replaced and returned as `Some(old_element)`.
    pub fn insert(&mut self, element: ModelElement) -> Option<ModelElement> {
        let id = element.id();
        self.elements.insert(id, element)
    }

    /// Remove an element by ID.
    ///
    /// Performs cascading cleanup:
    /// 1. Removes the element from `parent_index`.
    /// 2. Removes the element's ID from every package's `children` list
    ///    (using `parent_index` to find all parent packages).
    /// 3. Removes the element from the elements map.
    ///
    /// Returns the element if it existed.
    pub fn remove(&mut self, id: UmlId) -> Option<ModelElement> {
        // 1. Get the parent packages (if any) before removing from parent_index
        let parent_ids: Vec<UmlId> = self.parent_index.remove(&id).unwrap_or_default();

        // 2. Remove this element's ID from every parent package's children list
        for parent_id in &parent_ids {
            if let Some(ModelElement::Package(ref mut pkg)) = self.elements.get_mut(parent_id) {
                pkg.children.retain(|&child_id| child_id != id);
            }
        }

        // 3. Remove from elements (shift_remove preserves insertion order)
        self.elements.shift_remove(&id)
    }

    /// Get a reference to an element by ID.
    #[must_use]
    pub fn get(&self, id: UmlId) -> Option<&ModelElement> {
        self.elements.get(&id)
    }

    /// Get a mutable reference to an element by ID.
    pub fn get_mut(&mut self, id: UmlId) -> Option<&mut ModelElement> {
        self.elements.get_mut(&id)
    }

    /// Iterate over all `(UmlId, &ModelElement)` pairs in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (UmlId, &ModelElement)> {
        self.elements.iter().map(|(&id, elem)| (id, elem))
    }

    /// Number of elements in the model.
    #[must_use]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Returns `true` if the model contains no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Returns `true` if an element with the given ID exists.
    #[must_use]
    pub fn contains(&self, id: UmlId) -> bool {
        self.elements.contains_key(&id)
    }

    /// Add a child element to a package.
    ///
    /// Updates both `Package::children` and the `parent_index`.
    ///
    /// # Errors
    ///
    /// - `ModelError::ElementNotFound` if either ID does not exist.
    /// - `ModelError::WouldCreateCycle` if adding the child would create a
    ///   containment cycle (the package is already contained by the child).
    pub fn add_to_package(&mut self, package_id: UmlId, child_id: UmlId) -> Result<(), ModelError> {
        // Validate both elements exist
        if !self.contains(package_id) {
            return Err(ModelError::ElementNotFound(package_id));
        }
        if !self.contains(child_id) {
            return Err(ModelError::ElementNotFound(child_id));
        }

        // Check for cycle: if child_id is an ancestor of package_id,
        // adding package_id as parent of child_id would create a cycle.
        if self.would_create_cycle(package_id, child_id) {
            return Err(ModelError::WouldCreateCycle {
                parent: package_id,
                child: child_id,
            });
        }

        // Add child to package's children list (only if container is a Package)
        if let Some(ModelElement::Package(ref mut pkg)) = self.elements.get_mut(&package_id) {
            if !pkg.children.contains(&child_id) {
                pkg.children.push(child_id);
            }
        }
        // Note: in UML, Classifiers can also nest (they are also namespaces).
        // For now, require the container to be a Package variant.

        // Update parent_index
        self.parent_index
            .entry(child_id)
            .or_default()
            .push(package_id);

        Ok(())
    }

    /// Remove a child element from a package.
    ///
    /// Updates both `Package::children` and the `parent_index`.
    ///
    /// # Errors
    ///
    /// - `ModelError::ElementNotFound` if either ID does not exist.
    /// - `ModelError::NotAChild` if `child_id` is not a child of the package.
    pub fn remove_from_package(
        &mut self,
        package_id: UmlId,
        child_id: UmlId,
    ) -> Result<(), ModelError> {
        if !self.contains(package_id) {
            return Err(ModelError::ElementNotFound(package_id));
        }
        if !self.contains(child_id) {
            return Err(ModelError::ElementNotFound(child_id));
        }

        // Remove from package's children list
        if let Some(ModelElement::Package(ref mut pkg)) = self.elements.get_mut(&package_id) {
            let before = pkg.children.len();
            pkg.children.retain(|&id| id != child_id);
            if pkg.children.len() == before {
                return Err(ModelError::NotAChild {
                    parent: package_id,
                    child: child_id,
                });
            }
        }

        // Remove from parent_index
        if let Some(parents) = self.parent_index.get_mut(&child_id) {
            parents.retain(|&pid| pid != package_id);
        }

        Ok(())
    }

    /// Get the package IDs that contain the given element.
    ///
    /// Returns `None` if the element does not exist in the model.
    /// Returns `Some(&[])` if the element exists but has no parents.
    #[must_use]
    pub fn parents_of(&self, element_id: UmlId) -> Option<&[UmlId]> {
        if !self.contains(element_id) {
            return None;
        }
        Some(
            self.parent_index
                .get(&element_id)
                .map_or(&[], |v| v.as_slice()),
        )
    }

    /// Remove all elements that do NOT match the predicate.
    ///
    /// Elements for which the predicate returns `true` are kept. All others
    /// are removed with full cascading cleanup (parent_index + package children).
    pub fn retain(&mut self, mut predicate: impl FnMut(UmlId, &ModelElement) -> bool) {
        let to_remove: Vec<UmlId> = self
            .elements
            .iter()
            .filter(|(&id, elem)| !predicate(id, elem))
            .map(|(&id, _)| id)
            .collect();
        for id in to_remove {
            self.remove(id);
        }
    }

    /// Remove all elements and return an iterator over them.
    ///
    /// Clears parent_index as well.
    pub fn drain(&mut self) -> impl Iterator<Item = (UmlId, ModelElement)> {
        self.parent_index.clear();
        // IndexMap doesn't have drain(), so we use std::mem::take
        let elements = std::mem::take(&mut self.elements);
        elements.into_iter()
    }

    /// Validate all inter-element references in the model.
    ///
    /// Checks that every `UmlId` reference points to an existing element:
    /// - `Package::children` — each child ID must exist
    /// - `Attribute::type_id` — if Some, must exist
    /// - `Operation::return_type_id` — if Some, must exist
    /// - `Parameter::type_id` — if Some, must exist
    /// - `ElementBase::stereotype_id` — if Some, must exist
    ///
    /// Returns a list of all dangling references found. An empty list means
    /// the model is fully consistent.
    #[must_use]
    pub fn validate_references(&self) -> Vec<ReferenceError> {
        let mut errors = Vec::new();

        for (&id, element) in &self.elements {
            match element {
                ModelElement::Package(pkg) => {
                    for &child_id in &pkg.children {
                        if !self.contains(child_id) {
                            errors.push(ReferenceError {
                                source_id: id,
                                field: ReferenceField::PackageChild,
                                target_id: child_id,
                            });
                        }
                    }
                },
                ModelElement::Class(cls) => {
                    self.validate_classifier_references(id, &cls.classifier, &mut errors);
                },
                ModelElement::Interface(iface) => {
                    self.validate_classifier_references(id, &iface.classifier, &mut errors);
                },
                ModelElement::Enum(enm) => {
                    self.validate_classifier_references(id, &enm.classifier, &mut errors);
                },
            }

            // Validate stereotype reference
            if let Some(stereotype_id) = element.base().stereotype_id {
                if !self.contains(stereotype_id) {
                    errors.push(ReferenceError {
                        source_id: id,
                        field: ReferenceField::Stereotype,
                        target_id: stereotype_id,
                    });
                }
            }
        }

        errors
    }

    /// Helper: validate type references within ClassifierData.
    fn validate_classifier_references(
        &self,
        source_id: UmlId,
        classifier: &ClassifierData,
        errors: &mut Vec<ReferenceError>,
    ) {
        for attr in &classifier.attributes {
            if let Some(type_id) = attr.type_id {
                if !self.contains(type_id) {
                    errors.push(ReferenceError {
                        source_id,
                        field: ReferenceField::AttributeType,
                        target_id: type_id,
                    });
                }
            }
        }
        for op in &classifier.operations {
            if let Some(ret_id) = op.return_type_id {
                if !self.contains(ret_id) {
                    errors.push(ReferenceError {
                        source_id,
                        field: ReferenceField::OperationReturnType,
                        target_id: ret_id,
                    });
                }
            }
            for param in &op.parameters {
                if let Some(param_type_id) = param.type_id {
                    if !self.contains(param_type_id) {
                        errors.push(ReferenceError {
                            source_id,
                            field: ReferenceField::ParameterType,
                            target_id: param_type_id,
                        });
                    }
                }
            }
        }
    }

    /// Check whether adding `child_id` to `package_id` would create a cycle.
    ///
    /// A cycle occurs if `child_id` is already an ancestor of `package_id`
    /// (i.e., walking up from `package_id` through the parent chain reaches
    /// `child_id`). In that case, adding `package_id` as a parent of
    /// `child_id` would create a containment cycle.
    fn would_create_cycle(&self, package_id: UmlId, child_id: UmlId) -> bool {
        // Walk up from package_id through its ancestors using parent_index.
        // If we reach child_id, then child_id is already an ancestor of
        // package_id, meaning adding package_id as parent of child_id
        // would create a cycle: package_id → ... → child_id → package_id
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![package_id];

        while let Some(current) = stack.pop() {
            if current == child_id {
                return true;
            }
            if !visited.insert(current) {
                continue;
            }
            if let Some(parents) = self.parent_index.get(&current) {
                for &parent_id in parents {
                    stack.push(parent_id);
                }
            }
        }

        false
    }
}

impl Default for UmlModel {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Unit tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::{Attribute, Class, ClassifierData, ElementBase, Package};
    use crate::types::Visibility;

    fn make_package(name: &str) -> ModelElement {
        ModelElement::Package(Package::new(name))
    }

    fn make_class(name: &str) -> ModelElement {
        ModelElement::Class(Class::new(name))
    }

    #[test]
    fn new_model_is_empty() {
        let model = UmlModel::new();
        assert!(model.is_empty());
        assert_eq!(model.len(), 0);
    }

    #[test]
    fn insert_and_get() {
        let mut model = UmlModel::new();
        let elem = make_class("TestClass");
        let id = elem.id();

        let old = model.insert(elem.clone());
        assert!(old.is_none());

        let retrieved = model.get(id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), &elem);
    }

    #[test]
    fn insert_duplicate_replaces() {
        let mut model = UmlModel::new();

        let class1 = Class::new("C1");
        let id = class1.base.id;
        let old = model.insert(ModelElement::Class(class1));
        assert!(old.is_none());

        let class2 = Class {
            base: ElementBase {
                id,
                name: "C2".into(),
                ..ElementBase::new("")
            },
            classifier: ClassifierData::default(),
        };
        let replaced = model.insert(ModelElement::Class(class2));
        assert!(replaced.is_some());
        assert_eq!(replaced.unwrap().name(), "C1");

        assert_eq!(model.get(id).unwrap().name(), "C2");
        assert_eq!(model.len(), 1);
    }

    #[test]
    fn remove_existing() {
        let mut model = UmlModel::new();
        let elem = make_class("C");
        let id = elem.id();
        model.insert(elem);

        let removed = model.remove(id);
        assert!(removed.is_some());
        assert!(!model.contains(id));
        assert!(model.is_empty());
    }

    #[test]
    fn remove_nonexistent() {
        let mut model = UmlModel::new();
        let removed = model.remove(UmlId::new());
        assert!(removed.is_none());
    }

    #[test]
    fn contains_and_len() {
        let mut model = UmlModel::new();
        assert!(!model.contains(UmlId::new()));

        let elem = make_class("C");
        let id = elem.id();
        model.insert(elem);
        assert!(model.contains(id));
        assert_eq!(model.len(), 1);

        model.remove(id);
        assert!(!model.contains(id));
        assert_eq!(model.len(), 0);
    }

    #[test]
    fn iteration_is_insertion_order() {
        let mut model = UmlModel::new();
        model.insert(make_class("A"));
        model.insert(make_class("B"));
        model.insert(make_class("C"));

        let names: Vec<String> = model.iter().map(|(_, e)| e.name().to_string()).collect();
        assert_eq!(names, vec!["A", "B", "C"]);
    }

    #[test]
    fn add_to_package_success() {
        let mut model = UmlModel::new();
        let pkg = make_package("Root");
        let pkg_id = pkg.id();
        model.insert(pkg);
        let cls = make_class("Person");
        let cls_id = cls.id();
        model.insert(cls);

        model.add_to_package(pkg_id, cls_id).unwrap();

        // Check package has child
        if let ModelElement::Package(ref pkg) = model.get(pkg_id).unwrap() {
            assert!(pkg.children.contains(&cls_id));
        } else {
            panic!("expected package");
        }

        // Check parent_index
        assert_eq!(model.parents_of(cls_id), Some(&[pkg_id][..]));
    }

    #[test]
    fn add_to_package_element_not_found() {
        let mut model = UmlModel::new();
        let bad_id = UmlId::new();
        let result = model.add_to_package(bad_id, UmlId::new());
        assert!(matches!(result, Err(ModelError::ElementNotFound(_))));
    }

    #[test]
    fn add_to_package_child_not_found() {
        let mut model = UmlModel::new();
        let pkg = make_package("Root");
        let pkg_id = pkg.id();
        model.insert(pkg);
        let bad_id = UmlId::new();
        let result = model.add_to_package(pkg_id, bad_id);
        assert!(matches!(result, Err(ModelError::ElementNotFound(_))));
    }

    #[test]
    fn add_to_package_cycle_detection_direct() {
        let mut model = UmlModel::new();
        let pkg = make_package("A");
        let pkg_id = pkg.id();
        model.insert(pkg);

        // Adding A as child of A should be caught
        let result = model.add_to_package(pkg_id, pkg_id);
        assert!(matches!(result, Err(ModelError::WouldCreateCycle { .. })));
    }

    #[test]
    fn add_to_package_cycle_detection_indirect() {
        let mut model = UmlModel::new();
        let a = make_package("A");
        let pkg_a = a.id();
        model.insert(a);
        let b = make_package("B");
        let pkg_b = b.id();
        model.insert(b);
        let c = make_package("C");
        let pkg_c = c.id();
        model.insert(c);

        // A → B → C
        model.add_to_package(pkg_a, pkg_b).unwrap();
        model.add_to_package(pkg_b, pkg_c).unwrap();

        // C → A would create A → B → C → A cycle
        let result = model.add_to_package(pkg_c, pkg_a);
        assert!(matches!(result, Err(ModelError::WouldCreateCycle { .. })));
    }

    #[test]
    fn remove_from_package_success() {
        let mut model = UmlModel::new();
        let pkg = make_package("Root");
        let pkg_id = pkg.id();
        model.insert(pkg);
        let cls = make_class("Person");
        let cls_id = cls.id();
        model.insert(cls);

        model.add_to_package(pkg_id, cls_id).unwrap();
        model.remove_from_package(pkg_id, cls_id).unwrap();

        // Package no longer has child
        if let ModelElement::Package(ref pkg) = model.get(pkg_id).unwrap() {
            assert!(!pkg.children.contains(&cls_id));
        }

        // parent_index cleaned
        assert_eq!(model.parents_of(cls_id), Some(&[][..]));
    }

    #[test]
    fn remove_from_package_not_a_child() {
        let mut model = UmlModel::new();
        let pkg = make_package("Root");
        let pkg_id = pkg.id();
        model.insert(pkg);
        let cls = make_class("Person");
        let cls_id = cls.id();
        model.insert(cls);

        let result = model.remove_from_package(pkg_id, cls_id);
        assert!(matches!(result, Err(ModelError::NotAChild { .. })));
    }

    #[test]
    fn remove_cascading_cleanup() {
        let mut model = UmlModel::new();
        let pkg = make_package("Root");
        let pkg_id = pkg.id();
        model.insert(pkg);
        let cls = make_class("Person");
        let cls_id = cls.id();
        model.insert(cls);

        model.add_to_package(pkg_id, cls_id).unwrap();

        // Remove the child — should clean parent_index and package.children
        model.remove(cls_id);

        // Check cleanup
        assert!(!model.contains(cls_id));
        assert_eq!(model.parents_of(cls_id), None);

        if let ModelElement::Package(ref pkg) = model.get(pkg_id).unwrap() {
            assert!(!pkg.children.contains(&cls_id));
        }
    }

    #[test]
    fn parents_of_nonexistent() {
        let model = UmlModel::new();
        assert_eq!(model.parents_of(UmlId::new()), None);
    }

    #[test]
    fn parents_of_no_parents() {
        let mut model = UmlModel::new();
        let elem = make_class("Orphan");
        let id = elem.id();
        model.insert(elem);
        assert_eq!(model.parents_of(id), Some(&[][..]));
    }

    #[test]
    fn retain_keeps_matching() {
        let mut model = UmlModel::new();
        model.insert(make_class("Keep"));
        model.insert(make_class("Remove"));

        model.retain(|_id, elem| elem.name() == "Keep");

        assert_eq!(model.len(), 1);
        let names: Vec<String> = model.iter().map(|(_, e)| e.name().to_string()).collect();
        assert_eq!(names, vec!["Keep"]);
    }

    #[test]
    fn drain_clears() {
        let mut model = UmlModel::new();
        model.insert(make_class("A"));
        model.insert(make_class("B"));

        let drained: Vec<_> = model.drain().collect();
        assert_eq!(drained.len(), 2);
        assert!(model.is_empty());
    }

    #[test]
    fn validate_references_empty() {
        let model = UmlModel::new();
        assert!(model.validate_references().is_empty());
    }

    #[test]
    fn validate_references_clean() {
        let mut model = UmlModel::new();
        let pkg = make_package("Root");
        let pkg_id = pkg.id();
        model.insert(pkg);
        let cls = make_class("Person");
        let cls_id = cls.id();
        model.insert(cls);
        model.add_to_package(pkg_id, cls_id).unwrap();

        let errors = model.validate_references();
        assert!(errors.is_empty());
    }

    #[test]
    fn validate_references_dangling_child() {
        let mut model = UmlModel::new();
        let pkg = make_package("Root");
        let pkg_id = pkg.id();
        model.insert(pkg);
        let dangling = UmlId::new();

        // Manually add dangling ID to package children (bypassing add_to_package)
        if let ModelElement::Package(ref mut pkg) = model.get_mut(pkg_id).unwrap() {
            pkg.children.push(dangling);
        }

        let errors = model.validate_references();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].source_id, pkg_id);
        assert_eq!(errors[0].target_id, dangling);
        assert_eq!(errors[0].field, ReferenceField::PackageChild);
    }

    #[test]
    fn validate_references_dangling_type() {
        let mut model = UmlModel::new();
        let mut cls = Class::new("Person");
        let dangling = UmlId::new();
        cls.classifier.add_attribute(Attribute {
            name: "address".into(),
            type_id: Some(dangling),
            type_name: None,
            visibility: Visibility::Private,
            initial_value: None,
            is_static: false,
        });
        model.insert(ModelElement::Class(cls));

        let errors = model.validate_references();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, ReferenceField::AttributeType);
    }

    #[test]
    fn validate_references_dangling_stereotype() {
        let mut model = UmlModel::new();
        let mut base = ElementBase::new("Entity");
        base.stereotype_id = Some(UmlId::new());
        let elem = ModelElement::Class(Class {
            base,
            classifier: ClassifierData::default(),
        });
        model.insert(elem);

        let errors = model.validate_references();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, ReferenceField::Stereotype);
    }

    #[test]
    fn multiple_elements_in_package() {
        let mut model = UmlModel::new();
        let root_pkg = make_package("Root");
        let root = root_pkg.id();
        model.insert(root_pkg);
        let sub_pkg = make_package("Sub");
        let sub = sub_pkg.id();
        model.insert(sub_pkg);
        let cls_elem = make_class("Person");
        let cls = cls_elem.id();
        model.insert(cls_elem);

        model.add_to_package(root, sub).unwrap();
        model.add_to_package(root, cls).unwrap();

        assert_eq!(model.parents_of(sub), Some(&[root][..]));
        assert_eq!(model.parents_of(cls), Some(&[root][..]));
    }

    #[test]
    fn child_in_multiple_packages() {
        let mut model = UmlModel::new();
        let pkg1_elem = make_package("P1");
        let pkg1 = pkg1_elem.id();
        model.insert(pkg1_elem);
        let pkg2_elem = make_package("P2");
        let pkg2 = pkg2_elem.id();
        model.insert(pkg2_elem);
        let shared_elem = make_class("Shared");
        let shared = shared_elem.id();
        model.insert(shared_elem);

        model.add_to_package(pkg1, shared).unwrap();
        model.add_to_package(pkg2, shared).unwrap();

        let parents = model.parents_of(shared).unwrap();
        assert_eq!(parents.len(), 2);
        assert!(parents.contains(&pkg1));
        assert!(parents.contains(&pkg2));
    }

    #[test]
    fn get_mut_allows_mutation() {
        let mut model = UmlModel::new();
        let elem = make_class("Original");
        let id = elem.id();
        model.insert(elem);

        let elem = model.get_mut(id).unwrap();
        elem.set_name("Modified".into());
        // drop the mutable reference before getting again
        // drop the mutable borrow
        let _ = elem;

        assert_eq!(model.get(id).unwrap().name(), "Modified");
    }

    #[test]
    fn model_is_clonable() {
        let mut model = UmlModel::new();
        model.insert(make_class("A"));
        model.insert(make_class("B"));

        let clone = model.clone();
        assert_eq!(clone.len(), 2);
        assert!(clone.contains(model.iter().next().unwrap().0));
    }
}
