//! XMI writer — serializes UmlModel to XMI 1.2 XML.
//!
//! Produces XMI 1.2 output compatible with legacy Umbrello C++ format.
//! Uses `original_xmi_id` when available for XMI IDs, generates new IDs
//! (prefixed `rs`) for native elements.

use std::collections::HashMap;
use std::io::Write;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::writer::Writer as XmlWriter;

use uml_core::{AssociationType, ModelElement, Relationship, TypeReference, UmlId, UmlModel};

/// Errors during XMI writing.
#[derive(Debug, thiserror::Error)]
pub enum XmiWriteError {
    /// XML serialization error.
    #[error("XML write error: {0}")]
    Xml(#[from] quick_xml::Error),
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// XMI writer that serializes a `UmlModel` to XMI 1.2 XML.
///
/// # Example
///
/// ```rust
/// use uml_core::UmlModel;
/// use uml_io::xmi::XmiWriter;
///
/// let model = UmlModel::new();
/// let mut output = Vec::new();
/// let mut writer = XmiWriter::new(&mut output);
/// writer.write_document(&model).unwrap();
/// ```
pub struct XmiWriter<W: Write> {
    /// The underlying quick-xml writer.
    writer: XmlWriter<W>,
    /// UmlId → XMI ID string (built during pre-assign and used during writing).
    id_map: HashMap<UmlId, String>,
    /// Counter for generating new XMI IDs.
    next_id: u64,
}

impl<W: Write> XmiWriter<W> {
    /// Create a new XMI writer that writes to the given output.
    #[must_use]
    pub fn new(inner: W) -> Self {
        let writer = XmlWriter::new_with_indent(inner, b' ', 1);
        Self {
            writer,
            id_map: HashMap::new(),
            next_id: 0,
        }
    }

    /// Consume the writer and return the inner writer.
    #[must_use]
    pub fn into_inner(self) -> W {
        self.writer.into_inner()
    }

    /// Write the full XMI document for the given model.
    pub fn write_document(&mut self, model: &UmlModel) -> Result<(), XmiWriteError> {
        // Phase 0: pre-assign XMI IDs to all elements
        self.pre_assign_ids(model);

        // 1. XML declaration
        self.writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        // 2. XMI root element
        let mut root = BytesStart::new("XMI");
        root.push_attribute(("xmi.version", "1.2"));
        root.push_attribute(("xmlns:UML", "http://schema.omg.org/spec/UML/1.3"));
        self.writer.write_event(Event::Start(root))?;

        // 3. XMI header
        self.writer
            .write_event(Event::Empty(BytesStart::new("XMI.header")))?;

        // 4. XMI content
        self.writer
            .write_event(Event::Start(BytesStart::new("XMI.content")))?;

        // 5. UML:Model wrapper
        self.write_model_wrapper(model)?;

        // 6. Close XMI.content
        self.writer
            .write_event(Event::End(BytesEnd::new("XMI.content")))?;

        // 7. XMI.extensions (empty for now)
        self.writer
            .write_event(Event::Start(BytesStart::new("XMI.extensions")))?;
        self.writer
            .write_event(Event::End(BytesEnd::new("XMI.extensions")))?;

        // 8. Close XMI root
        self.writer.write_event(Event::End(BytesEnd::new("XMI")))?;

        Ok(())
    }

    // ─── ID management ─────────────────────────────────────────────────

    /// Pre-assign XMI IDs for every element in the model.
    fn pre_assign_ids(&mut self, model: &UmlModel) {
        for (id, elem) in model.iter() {
            let orig = elem.base().original_xmi_id.as_deref();
            self.get_or_create_xmi_id(id, orig);
        }
    }

    /// Get or create the XMI string ID for a `UmlId`.
    fn get_or_create_xmi_id(&mut self, uml_id: UmlId, original: Option<&str>) -> String {
        if let Some(cached) = self.id_map.get(&uml_id) {
            return cached.clone();
        }
        let xmi_id = if let Some(orig) = original {
            orig.to_string()
        } else {
            self.next_id += 1;
            format!("rs{:08x}", self.next_id)
        };
        self.id_map.insert(uml_id, xmi_id.clone());
        xmi_id
    }

    /// Look up the XMI ID for a `UmlId` (panics if not found — use after pre-assign).
    fn lookup_xmi_id(&self, uml_id: UmlId) -> String {
        self.id_map
            .get(&uml_id)
            .cloned()
            .unwrap_or_else(|| panic!("XMI ID not found for UmlId {uml_id}"))
    }

    /// Generate a fresh XMI ID for sub-elements that have no corresponding UmlId.
    fn gen_sub_id(&mut self) -> String {
        self.next_id += 1;
        format!("rs{:08x}", self.next_id)
    }

    /// Get a type reference for use as the `type` attribute value in XMI.
    /// For model references we return the XMI ID, for primitives the type name.
    fn attr_type_value(&self, type_ref: &TypeReference) -> Option<String> {
        if let Some(model_id) = type_ref.model_id {
            Some(self.lookup_xmi_id(model_id))
        } else {
            type_ref.type_name.clone()
        }
    }

    // ─── Model writing ─────────────────────────────────────────────────

    /// Write `<UML:Model>` wrapper and all elements inside.
    fn write_model_wrapper(&mut self, model: &UmlModel) -> Result<(), XmiWriteError> {
        // Find a suitable root package for the UML:Model wrapper
        let root = self.find_root_model_id(model);

        // Collect all elements that should be written as top-level children
        // (anything that is not the root Model wrapper)
        let top_level_ids: Vec<UmlId> = model
            .iter()
            .map(|(id, _)| id)
            .filter(|&id| Some(id) != root)
            .collect();

        // Separate structural elements from relationships
        let struct_ids: Vec<UmlId> = top_level_ids
            .iter()
            .copied()
            .filter(|id| !matches!(model.get(*id), Some(ModelElement::Relationship(_))))
            .collect();
        let rel_ids: Vec<UmlId> = top_level_ids
            .iter()
            .copied()
            .filter(|id| matches!(model.get(*id), Some(ModelElement::Relationship(_))))
            .collect();

        let model_xmi = root.map(|id| self.lookup_xmi_id(id)).unwrap_or_else(|| {
            self.next_id += 1;
            format!("rs{:08x}", self.next_id)
        });
        let model_name = root
            .and_then(|id| model.get(id))
            .map(|e| e.name().to_string())
            .unwrap_or_else(|| "UML Model".to_string());

        // Write <UML:Model ...>
        let mut model_tag = BytesStart::new("UML:Model");
        model_tag.push_attribute(("xmi.id", model_xmi.as_str()));
        model_tag.push_attribute(("name", model_name.as_str()));
        model_tag.push_attribute(("isSpecification", "false"));
        model_tag.push_attribute(("isAbstract", "false"));
        model_tag.push_attribute(("isLeaf", "false"));
        model_tag.push_attribute(("isRoot", "false"));
        model_tag.push_attribute(("visibility", "public"));
        self.writer.write_event(Event::Start(model_tag))?;

        if !struct_ids.is_empty() || !rel_ids.is_empty() {
            self.writer
                .write_event(Event::Start(BytesStart::new("UML:Namespace.ownedElement")))?;

            // Write structural elements
            for id in &struct_ids {
                if let Some(elem) = model.get(*id) {
                    self.write_element(elem, model)?;
                }
            }

            // Write relationships
            for id in &rel_ids {
                if let Some(ModelElement::Relationship(rel)) = model.get(*id) {
                    self.write_relationship(rel)?;
                }
            }

            self.writer
                .write_event(Event::End(BytesEnd::new("UML:Namespace.ownedElement")))?;
        }

        self.writer
            .write_event(Event::End(BytesEnd::new("UML:Model")))?;

        Ok(())
    }

    /// Find a root package ID to use as the UML:Model wrapper.
    /// Returns `None` if no suitable package exists (e.g., empty model).
    fn find_root_model_id(&self, model: &UmlModel) -> Option<UmlId> {
        // Prefer a Package named "UML Model" — this is the typical root.
        for (id, elem) in model.iter() {
            if elem.name() == "UML Model" && matches!(elem, ModelElement::Package(_)) {
                return Some(id);
            }
        }
        // Otherwise use the first Package element.
        for (id, elem) in model.iter() {
            if matches!(elem, ModelElement::Package(_)) {
                return Some(id);
            }
        }
        None
    }

    // ─── Element dispatch ──────────────────────────────────────────────

    /// Write a single model element.
    fn write_element(
        &mut self,
        elem: &ModelElement,
        model: &UmlModel,
    ) -> Result<(), XmiWriteError> {
        match elem {
            ModelElement::Package(pkg) => self.write_package(pkg, model),
            ModelElement::Class(cls) => self.write_class(elem, &cls.base, &cls.classifier, model),
            ModelElement::Interface(iface) => {
                self.write_class(elem, &iface.base, &iface.classifier, model)
            },
            ModelElement::Enum(enm) => {
                self.write_enum(elem, &enm.base, enm.literals.as_slice(), model)
            },
            ModelElement::Datatype(dt) => self.write_class(elem, &dt.base, &dt.classifier, model),
            ModelElement::Relationship(_) => {
                // Relationships are written separately in write_model_wrapper
                Ok(())
            },
        }
    }

    // ─── Package ───────────────────────────────────────────────────────

    /// Write a `<UML:Package>` element.
    fn write_package(
        &mut self,
        pkg: &uml_core::Package,
        model: &UmlModel,
    ) -> Result<(), XmiWriteError> {
        let xmi_id = self.lookup_xmi_id(pkg.base.id);
        let mut tag = BytesStart::new("UML:Package");
        tag.push_attribute(("xmi.id", xmi_id.as_str()));
        tag.push_attribute(("name", pkg.base.name.as_str()));
        tag.push_attribute(("visibility", pkg.base.visibility.as_str()));
        tag.push_attribute(("isSpecification", "false"));
        tag.push_attribute((
            "isAbstract",
            if pkg.base.is_abstract {
                "true"
            } else {
                "false"
            },
        ));
        tag.push_attribute(("isLeaf", "false"));
        tag.push_attribute(("isRoot", "false"));

        // Write stereotype reference if set
        if let Some(st_id) = pkg.base.stereotype_id {
            let st_xmi = self.lookup_xmi_id(st_id);
            tag.push_attribute(("stereotype", st_xmi.as_str()));
        }

        self.writer.write_event(Event::Start(tag))?;

        // Write children recursively (using the public API)
        let child_ids: Vec<UmlId> = pkg.child_ids().collect();
        if !child_ids.is_empty() {
            self.writer
                .write_event(Event::Start(BytesStart::new("UML:Namespace.ownedElement")))?;
            for child_id in child_ids {
                if let Some(child) = model.get(child_id) {
                    self.write_element(child, model)?;
                }
            }
            self.writer
                .write_event(Event::End(BytesEnd::new("UML:Namespace.ownedElement")))?;
        }

        self.writer
            .write_event(Event::End(BytesEnd::new("UML:Package")))?;
        Ok(())
    }

    // ─── Classifier (Class, Interface, Datatype) ───────────────────────

    /// Write a classifier element (Class, Interface, or Datatype).
    fn write_class(
        &mut self,
        elem: &ModelElement,
        base: &uml_core::ElementBase,
        classifier: &uml_core::ClassifierData,
        _model: &UmlModel,
    ) -> Result<(), XmiWriteError> {
        let (tag_name, has_features) = match elem {
            ModelElement::Class(_) => ("UML:Class", true),
            ModelElement::Interface(_) => ("UML:Interface", true),
            ModelElement::Datatype(_) => ("UML:DataType", true),
            _ => return Ok(()),
        };

        let xmi_id = self.lookup_xmi_id(base.id);
        let mut tag = BytesStart::new(tag_name);
        tag.push_attribute(("xmi.id", xmi_id.as_str()));
        tag.push_attribute(("name", base.name.as_str()));
        tag.push_attribute(("visibility", base.visibility.as_str()));
        tag.push_attribute(("isSpecification", "false"));
        tag.push_attribute(("isAbstract", if base.is_abstract { "true" } else { "false" }));
        tag.push_attribute(("isLeaf", "false"));
        tag.push_attribute(("isRoot", "false"));

        // Write stereotype reference if set
        if let Some(st_id) = base.stereotype_id {
            let st_xmi = self.lookup_xmi_id(st_id);
            tag.push_attribute(("stereotype", st_xmi.as_str()));
        }

        // Check if we need to write children (features, generalizations)
        let has_features = has_features
            && (!classifier.attributes.is_empty() || !classifier.operations.is_empty());

        if has_features {
            self.writer.write_event(Event::Start(tag))?;

            // Write Classifier.feature
            self.writer
                .write_event(Event::Start(BytesStart::new("UML:Classifier.feature")))?;
            for attr in &classifier.attributes {
                self.write_attribute(attr)?;
            }
            for op in &classifier.operations {
                self.write_operation(op, classifier.attributes.len())?;
            }
            self.writer
                .write_event(Event::End(BytesEnd::new("UML:Classifier.feature")))?;

            self.writer
                .write_event(Event::End(BytesEnd::new(tag_name)))?;
        } else {
            // Self-closing if no children
            self.writer.write_event(Event::Empty(tag))?;
        }

        Ok(())
    }

    // ─── Enumeration ──────────────────────────────────────────────────

    /// Write a `<UML:Enumeration>` element.
    fn write_enum(
        &mut self,
        elem: &ModelElement,
        base: &uml_core::ElementBase,
        literals: &[uml_core::EnumLiteral],
        _model: &UmlModel,
    ) -> Result<(), XmiWriteError> {
        let xmi_id = self.lookup_xmi_id(base.id);
        let mut tag = BytesStart::new("UML:Enumeration");
        tag.push_attribute(("xmi.id", xmi_id.as_str()));
        tag.push_attribute(("name", base.name.as_str()));
        tag.push_attribute(("visibility", base.visibility.as_str()));
        tag.push_attribute(("isSpecification", "false"));
        tag.push_attribute(("isAbstract", if base.is_abstract { "true" } else { "false" }));
        tag.push_attribute(("isLeaf", "false"));
        tag.push_attribute(("isRoot", "false"));

        if let Some(st_id) = base.stereotype_id {
            let st_xmi = self.lookup_xmi_id(st_id);
            tag.push_attribute(("stereotype", st_xmi.as_str()));
        }

        // Enum -> write classifier.feature with literals as attributes
        if let ModelElement::Enum(enm) = elem {
            let has_literals = !enm.literals.is_empty();
            if has_literals {
                self.writer.write_event(Event::Start(tag))?;
                self.writer
                    .write_event(Event::Start(BytesStart::new("UML:Classifier.feature")))?;
                for lit in literals {
                    let lit_id = self.gen_sub_id();
                    let mut lit_tag = BytesStart::new("UML:Attribute");
                    lit_tag.push_attribute(("xmi.id", lit_id.as_str()));
                    lit_tag.push_attribute(("name", lit.name.as_str()));
                    lit_tag.push_attribute(("visibility", "public"));
                    lit_tag.push_attribute(("isSpecification", "false"));
                    lit_tag.push_attribute(("isLeaf", "false"));
                    lit_tag.push_attribute(("isRoot", "false"));
                    lit_tag.push_attribute(("changeability", "changeable"));
                    lit_tag.push_attribute(("ownerScope", "instance"));
                    if let Some(ref val) = lit.value {
                        lit_tag.push_attribute(("initialValue", val.as_str()));
                    }
                    self.writer.write_event(Event::Empty(lit_tag))?;
                }
                self.writer
                    .write_event(Event::End(BytesEnd::new("UML:Classifier.feature")))?;
                self.writer
                    .write_event(Event::End(BytesEnd::new("UML:Enumeration")))?;
            } else {
                self.writer.write_event(Event::Empty(tag))?;
            }
        } else {
            self.writer.write_event(Event::Empty(tag))?;
        }

        Ok(())
    }

    // ─── Attribute ─────────────────────────────────────────────────────

    /// Write a `<UML:Attribute>` element (self-closing).
    fn write_attribute(&mut self, attr: &uml_core::Attribute) -> Result<(), XmiWriteError> {
        let attr_id = self.gen_sub_id();
        let mut tag = BytesStart::new("UML:Attribute");
        tag.push_attribute(("xmi.id", attr_id.as_str()));
        tag.push_attribute(("name", attr.name.as_str()));
        tag.push_attribute(("visibility", attr.visibility.as_str()));
        tag.push_attribute(("isSpecification", "false"));
        if let Some(ref type_val) = self.attr_type_value(&attr.type_ref) {
            tag.push_attribute(("type", type_val.as_str()));
        }
        if let Some(ref iv) = attr.initial_value {
            tag.push_attribute(("initialValue", iv.as_str()));
        }
        if attr.is_static {
            tag.push_attribute(("isStatic", "true"));
        }
        self.writer.write_event(Event::Empty(tag))?;
        Ok(())
    }

    // ─── Operation ─────────────────────────────────────────────────────

    /// Write a `<UML:Operation>` element.
    fn write_operation(
        &mut self,
        op: &uml_core::Operation,
        _attr_count: usize,
    ) -> Result<(), XmiWriteError> {
        let op_id = self.gen_sub_id();
        let mut tag = BytesStart::new("UML:Operation");
        tag.push_attribute(("xmi.id", op_id.as_str()));
        tag.push_attribute(("name", op.name.as_str()));
        tag.push_attribute(("visibility", op.visibility.as_str()));
        tag.push_attribute(("isSpecification", "false"));
        if op.is_abstract {
            tag.push_attribute(("isAbstract", "true"));
        }
        if op.is_static {
            tag.push_attribute(("isStatic", "true"));
        }

        let has_params = !op.parameters.is_empty() || op.return_type.is_resolved();

        if has_params {
            self.writer.write_event(Event::Start(tag))?;
            self.writer
                .write_event(Event::Start(BytesStart::new("UML:BehavioralFeature.parameter")))?;

            // Write return type as first parameter (kind="return")
            if op.return_type.is_resolved() {
                let param_id = self.gen_sub_id();
                let mut param_tag = BytesStart::new("UML:Parameter");
                param_tag.push_attribute(("xmi.id", param_id.as_str()));
                param_tag.push_attribute(("kind", "return"));
                if let Some(ref type_val) = self.attr_type_value(&op.return_type) {
                    param_tag.push_attribute(("type", type_val.as_str()));
                }
                self.writer.write_event(Event::Empty(param_tag))?;
            }

            // Write regular parameters
            for param in &op.parameters {
                let param_id = self.gen_sub_id();
                let mut param_tag = BytesStart::new("UML:Parameter");
                param_tag.push_attribute(("xmi.id", param_id.as_str()));
                param_tag.push_attribute(("kind", param.direction.as_str()));
                if !param.name.is_empty() {
                    param_tag.push_attribute(("name", param.name.as_str()));
                }
                if let Some(ref type_val) = self.attr_type_value(&param.type_ref) {
                    param_tag.push_attribute(("type", type_val.as_str()));
                }
                self.writer.write_event(Event::Empty(param_tag))?;
            }

            self.writer
                .write_event(Event::End(BytesEnd::new("UML:BehavioralFeature.parameter")))?;
            self.writer
                .write_event(Event::End(BytesEnd::new("UML:Operation")))?;
        } else {
            self.writer.write_event(Event::Empty(tag))?;
        }

        Ok(())
    }

    // ─── Relationships ─────────────────────────────────────────────────

    /// Write a single relationship element.
    fn write_relationship(&mut self, rel: &Relationship) -> Result<(), XmiWriteError> {
        let xmi_id = self.lookup_xmi_id(rel.base.id);
        let source_xmi = self.lookup_xmi_id(rel.source_id);
        let target_xmi = self.lookup_xmi_id(rel.target_id);

        match rel.kind {
            AssociationType::Generalization => {
                let mut tag = BytesStart::new("UML:Generalization");
                tag.push_attribute(("xmi.id", xmi_id.as_str()));
                tag.push_attribute(("child", source_xmi.as_str()));
                tag.push_attribute(("parent", target_xmi.as_str()));
                tag.push_attribute(("isSpecification", "false"));
                self.writer.write_event(Event::Empty(tag))?;
            },
            AssociationType::Association
            | AssociationType::Aggregation
            | AssociationType::Composition => {
                let aggregation = match rel.kind {
                    AssociationType::Aggregation => "aggregate",
                    AssociationType::Composition => "composite",
                    _ => "none",
                };

                let mut assoc_tag = BytesStart::new("UML:Association");
                assoc_tag.push_attribute(("xmi.id", xmi_id.as_str()));
                let name = rel.base.name.as_str();
                assoc_tag.push_attribute(("name", if name.is_empty() { "" } else { name }));
                assoc_tag.push_attribute(("visibility", "public"));
                assoc_tag.push_attribute(("isSpecification", "false"));
                self.writer.write_event(Event::Start(assoc_tag))?;

                self.writer
                    .write_event(Event::Start(BytesStart::new("UML:Association.connection")))?;

                // Source end
                let end1_id = self.gen_sub_id();
                let mut end1 = BytesStart::new("UML:AssociationEnd");
                end1.push_attribute(("xmi.id", end1_id.as_str()));
                end1.push_attribute(("type", source_xmi.as_str()));
                end1.push_attribute(("name", rel.source_role_name.as_deref().unwrap_or("")));
                end1.push_attribute(("aggregation", aggregation));
                end1.push_attribute((
                    "isNavigable",
                    if rel.source_to_target_navigable {
                        "true"
                    } else {
                        "false"
                    },
                ));
                end1.push_attribute(("visibility", "public"));
                end1.push_attribute(("isSpecification", "false"));
                end1.push_attribute(("changeability", "changeable"));
                if let Some(ref mult) = rel.source_multiplicity {
                    end1.push_attribute(("multiplicity", mult.as_str()));
                }
                self.writer.write_event(Event::Empty(end1))?;

                // Target end
                let end2_id = self.gen_sub_id();
                let mut end2 = BytesStart::new("UML:AssociationEnd");
                end2.push_attribute(("xmi.id", end2_id.as_str()));
                end2.push_attribute(("type", target_xmi.as_str()));
                end2.push_attribute(("name", rel.target_role_name.as_deref().unwrap_or("")));
                end2.push_attribute(("aggregation", "none"));
                end2.push_attribute((
                    "isNavigable",
                    if rel.target_to_source_navigable {
                        "true"
                    } else {
                        "false"
                    },
                ));
                end2.push_attribute(("visibility", "public"));
                end2.push_attribute(("isSpecification", "false"));
                end2.push_attribute(("changeability", "changeable"));
                if let Some(ref mult) = rel.target_multiplicity {
                    end2.push_attribute(("multiplicity", mult.as_str()));
                }
                self.writer.write_event(Event::Empty(end2))?;

                self.writer
                    .write_event(Event::End(BytesEnd::new("UML:Association.connection")))?;
                self.writer
                    .write_event(Event::End(BytesEnd::new("UML:Association")))?;
            },
            AssociationType::Dependency => {
                let mut tag = BytesStart::new("UML:Dependency");
                tag.push_attribute(("xmi.id", xmi_id.as_str()));
                tag.push_attribute(("supplier", target_xmi.as_str()));
                tag.push_attribute(("client", source_xmi.as_str()));
                let name = rel.base.name.as_str();
                tag.push_attribute(("name", if name.is_empty() { "" } else { name }));
                tag.push_attribute(("visibility", "public"));
                tag.push_attribute(("isSpecification", "false"));
                self.writer.write_event(Event::Empty(tag))?;
            },
            AssociationType::Realization => {
                let mut tag = BytesStart::new("UML:Abstraction");
                tag.push_attribute(("xmi.id", xmi_id.as_str()));
                tag.push_attribute(("supplier", target_xmi.as_str()));
                tag.push_attribute(("client", source_xmi.as_str()));
                let name = rel.base.name.as_str();
                tag.push_attribute(("name", if name.is_empty() { "" } else { name }));
                tag.push_attribute(("visibility", "public"));
                tag.push_attribute(("isSpecification", "false"));
                self.writer.write_event(Event::Empty(tag))?;
            },
        }

        Ok(())
    }
}

impl<W: Write> std::fmt::Debug for XmiWriter<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XmiWriter")
            .field("id_map_len", &self.id_map.len())
            .field("next_id", &self.next_id)
            .finish()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xmi::reader::XmiReader;
    use uml_core::{
        Attribute, Class, Datatype, Enum, Interface, ModelElement, Operation, Package, Parameter,
        ParameterDirection, TypeReference, Visibility,
    };

    /// Helper: create a simple model with one class.
    fn model_with_one_class() -> UmlModel {
        let mut model = UmlModel::new();
        let cls = Class::new("Person");
        model.insert(ModelElement::Class(cls));
        model
    }

    /// Helper: create a model with nested package and classifier.
    fn model_with_package_and_class() -> UmlModel {
        let mut model = UmlModel::new();
        let pkg = Package::new("UML Model");
        let pkg_id = pkg.base.id;
        model.insert(ModelElement::Package(pkg));

        let cls = Class::new("Person");
        let cls_id = cls.base.id;
        model.insert(ModelElement::Class(cls));

        model.add_to_package(pkg_id, cls_id).unwrap();
        model
    }

    /// Helper: create a model with attributes and operations.
    fn model_with_features() -> UmlModel {
        let mut model = UmlModel::new();

        let mut cls = Class::new("Person");
        cls.classifier.attributes.push(Attribute {
            name: "age".into(),
            type_ref: TypeReference::primitive("int"),
            visibility: Visibility::Private,
            initial_value: None,
            is_static: false,
        });
        cls.classifier.attributes.push(Attribute {
            name: "name".into(),
            type_ref: TypeReference::primitive("string"),
            visibility: Visibility::Public,
            initial_value: None,
            is_static: false,
        });
        cls.classifier.operations.push(Operation {
            name: "getName".into(),
            return_type: TypeReference::primitive("string"),
            parameters: Vec::new(),
            visibility: Visibility::Public,
            is_static: false,
            is_abstract: false,
            is_virtual: false,
        });
        cls.classifier.operations.push(Operation {
            name: "setAge".into(),
            return_type: TypeReference::unspecified(),
            parameters: vec![Parameter {
                name: "newAge".into(),
                type_ref: TypeReference::primitive("int"),
                direction: ParameterDirection::In,
                default_value: None,
            }],
            visibility: Visibility::Public,
            is_static: false,
            is_abstract: false,
            is_virtual: false,
        });

        model.insert(ModelElement::Class(cls));
        model
    }

    /// Helper: create a model with a generalization relationship.
    fn model_with_generalization() -> UmlModel {
        let mut model = UmlModel::new();

        let sub = Class::new("SubClass");
        let sub_id = sub.base.id;
        model.insert(ModelElement::Class(sub));

        let sup = Class::new("SuperClass");
        let sup_id = sup.base.id;
        model.insert(ModelElement::Class(sup));

        let mut rel = uml_core::Relationship::new_generalization(sub_id, sup_id);
        rel.base.name = String::new();
        model.insert(ModelElement::Relationship(rel));

        model
    }

    /// Helper: create a model with an association.
    fn model_with_association() -> UmlModel {
        let mut model = UmlModel::new();

        let c1 = Class::new("Company");
        let c1_id = c1.base.id;
        model.insert(ModelElement::Class(c1));

        let c2 = Class::new("Employee");
        let c2_id = c2.base.id;
        model.insert(ModelElement::Class(c2));

        let mut rel = uml_core::Relationship::new_association(c1_id, c2_id);
        rel.source_to_target_navigable = true;
        rel.target_to_source_navigable = true;
        model.insert(ModelElement::Relationship(rel));

        model
    }

    /// Helper: create a model with various element types.
    fn model_with_various_types() -> UmlModel {
        let mut model = UmlModel::new();

        model.insert(ModelElement::Class(Class::new("Person")));
        model.insert(ModelElement::Interface(Interface::new("Serializable")));
        model.insert(ModelElement::Enum(Enum::new("Color")));
        model.insert(ModelElement::Datatype(Datatype::new("int")));

        model
    }

    /// Helper: write a model to a String buffer and return the XML string.
    fn write_to_string(model: &UmlModel) -> String {
        let mut output = Vec::new();
        let mut writer = XmiWriter::new(&mut output);
        writer.write_document(model).unwrap();
        String::from_utf8(output).unwrap()
    }

    /// Helper: round-trip a model through write+read and compare structural equivalence.
    fn round_trip_and_compare(model: &UmlModel) {
        let xml = write_to_string(model);

        // Read back
        let mut model2 = UmlModel::new();
        let mut reader = XmiReader::new();
        let _count = reader.read_from(xml.as_bytes(), &mut model2).unwrap();
        reader.resolve(&mut model2).unwrap();

        // Structural comparison: compare non-package element counts.
        // The reader adds a <UML:Model> Package wrapper during parsing,
        // so model2 will have 1 extra Package element.
        let count_non_pkg = |m: &UmlModel| {
            m.iter()
                .filter(|(_, e)| !matches!(e, ModelElement::Package(_)))
                .count()
        };
        assert_eq!(
            count_non_pkg(model),
            count_non_pkg(&model2),
            "non-package element count mismatch: {} vs {}\nXML:\n{}",
            count_non_pkg(model),
            count_non_pkg(&model2),
            xml
        );

        // Compare classifier counts
        let count_classes = |m: &UmlModel| {
            m.iter()
                .filter(|(_, e)| matches!(e, ModelElement::Class(_)))
                .count()
        };
        let count_interfaces = |m: &UmlModel| {
            m.iter()
                .filter(|(_, e)| matches!(e, ModelElement::Interface(_)))
                .count()
        };
        let count_enums = |m: &UmlModel| {
            m.iter()
                .filter(|(_, e)| matches!(e, ModelElement::Enum(_)))
                .count()
        };
        let count_datatypes = |m: &UmlModel| {
            m.iter()
                .filter(|(_, e)| matches!(e, ModelElement::Datatype(_)))
                .count()
        };
        let count_rels = |m: &UmlModel| {
            m.iter()
                .filter(|(_, e)| matches!(e, ModelElement::Relationship(_)))
                .count()
        };

        assert_eq!(count_classes(model), count_classes(&model2), "class count mismatch");
        assert_eq!(count_interfaces(model), count_interfaces(&model2), "interface count mismatch");
        assert_eq!(count_enums(model), count_enums(&model2), "enum count mismatch");
        assert_eq!(count_datatypes(model), count_datatypes(&model2), "datatype count mismatch");
        assert_eq!(count_rels(model), count_rels(&model2), "relationship count mismatch");

        // Compare specific class names
        for (_, elem) in model.iter() {
            if let ModelElement::Class(c) = elem {
                let found = model2
                    .iter()
                    .any(|(_, e)| e.name() == c.base.name && matches!(e, ModelElement::Class(_)));
                assert!(found, "Class '{}' not found after round-trip", c.base.name);
            }
        }

        // Validate references on the re-parsed model
        let errors = model2.validate_references();
        assert!(errors.is_empty(), "dangling references after round-trip: {:?}", errors);
    }

    // ─── Unit tests ────────────────────────────────────────────────────

    #[test]
    fn write_empty_model() {
        let model = UmlModel::new();
        let xml = write_to_string(&model);
        assert!(xml.contains("XMI"), "should contain XMI root");
        assert!(xml.contains("xmi.version=\"1.2\""), "should have XMI version");
        assert!(xml.contains("UML:Model"), "should contain UML:Model");
    }

    #[test]
    fn write_single_class() {
        let model = model_with_one_class();
        let xml = write_to_string(&model);
        assert!(xml.contains("Person"), "should contain class name");
        assert!(xml.contains("UML:Class"), "should contain UML:Class tag");
    }

    #[test]
    fn write_model_with_features() {
        let model = model_with_features();
        let xml = write_to_string(&model);

        assert!(xml.contains("UML:Classifier.feature"));
        assert!(xml.contains("name=\"age\""));
        assert!(xml.contains("name=\"getName\""));
        assert!(xml.contains("UML:BehavioralFeature.parameter"));
        assert!(xml.contains("kind=\"return\""));
        assert!(xml.contains("kind=\"in\""));
    }

    #[test]
    fn write_model_with_generalization() {
        let model = model_with_generalization();
        let xml = write_to_string(&model);

        assert!(xml.contains("UML:Generalization"));
        assert!(xml.contains("SubClass"));
        assert!(xml.contains("SuperClass"));
    }

    #[test]
    fn write_model_with_association() {
        let model = model_with_association();
        let xml = write_to_string(&model);

        assert!(xml.contains("UML:Association"));
        assert!(xml.contains("UML:AssociationEnd"));
    }

    #[test]
    fn write_various_types() {
        let model = model_with_various_types();
        let xml = write_to_string(&model);

        assert!(xml.contains("UML:Class"));
        assert!(xml.contains("UML:Interface"));
        assert!(xml.contains("UML:Enumeration"));
        assert!(xml.contains("UML:DataType"));
    }

    // ─── Round-trip tests ──────────────────────────────────────────────

    #[test]
    fn round_trip_empty_model() {
        let model = UmlModel::new();
        round_trip_and_compare(&model);
    }

    #[test]
    fn round_trip_single_class() {
        let model = model_with_one_class();
        round_trip_and_compare(&model);
    }

    #[test]
    fn round_trip_with_features() {
        let model = model_with_features();
        round_trip_and_compare(&model);
    }

    #[test]
    fn round_trip_with_generalization() {
        let model = model_with_generalization();
        round_trip_and_compare(&model);
    }

    #[test]
    fn round_trip_with_association() {
        let model = model_with_association();
        round_trip_and_compare(&model);
    }

    #[test]
    fn round_trip_various_types() {
        let model = model_with_various_types();
        round_trip_and_compare(&model);
    }

    #[test]
    fn round_trip_package_and_class() {
        let model = model_with_package_and_class();
        round_trip_and_compare(&model);
    }

    #[test]
    fn round_trip_full_xmi_from_reader_tests() {
        // Test that XMI produced by the reader's test data can be written and re-read.

        // One class
        let xml1 = r#"<?xml version="1.0" encoding="UTF-8"?>
<XMI xmi.version="1.2" xmlns:UML="http://schema.omg.org/spec/UML/1.3">
 <XMI.header/>
 <XMI.content>
  <UML:Model xmi.id="m1" name="UML Model">
   <UML:Namespace.ownedElement>
    <UML:Class xmi.id="C1" name="Person" visibility="public"/>
   </UML:Namespace.ownedElement>
  </UML:Model>
 </XMI.content>
</XMI>"#;

        let mut model1 = UmlModel::new();
        let mut reader = XmiReader::new();
        reader.read_from(xml1.as_bytes(), &mut model1).unwrap();
        reader.resolve(&mut model1).unwrap();

        round_trip_and_compare(&model1);
    }

    #[test]
    fn round_trip_full_xmi_features() {
        let xml = r#"<?xml version="1.0"?><XMI xmi.version="1.2" xmlns:UML="http://schema.omg.org/spec/UML/1.3"><XMI.header/><XMI.content><UML:Model xmi.id="m1" name="UML Model"><UML:Namespace.ownedElement><UML:Class xmi.id="C1" name="Person"><UML:Classifier.feature><UML:Attribute visibility="private" xmi.id="A1" type="int" name="age"/><UML:Attribute visibility="public" xmi.id="A2" type="string" name="name"/><UML:Operation visibility="public" xmi.id="O1" name="getName"><UML:BehavioralFeature.parameter><UML:Parameter kind="return" xmi.id="P1" type="string"/></UML:BehavioralFeature.parameter></UML:Operation></UML:Classifier.feature></UML:Class></UML:Namespace.ownedElement></UML:Model></XMI.content></XMI>"#;

        let mut model = UmlModel::new();
        let mut reader = XmiReader::new();
        reader.read_from(xml.as_bytes(), &mut model).unwrap();
        reader.resolve(&mut model).unwrap();

        round_trip_and_compare(&model);
    }

    #[test]
    fn round_trip_full_xmi_association() {
        let xml = r#"<?xml version="1.0"?><XMI xmi.version="1.2" xmlns:UML="http://schema.omg.org/spec/UML/1.3"><XMI.header/><XMI.content><UML:Model xmi.id="m1" name="UML Model"><UML:Namespace.ownedElement><UML:Class xmi.id="C1" name="Company"/><UML:Class xmi.id="C2" name="Employee"/><UML:Association visibility="public" xmi.id="A1" name=""><UML:Association.connection><UML:AssociationEnd changeability="changeable" visibility="public" isNavigable="true" xmi.id="E1" type="C1" name="" aggregation="none"/><UML:AssociationEnd changeability="changeable" visibility="public" isNavigable="true" xmi.id="E2" type="C2" name="" aggregation="none"/></UML:Association.connection></UML:Association></UML:Namespace.ownedElement></UML:Model></XMI.content></XMI>"#;

        let mut model = UmlModel::new();
        let mut reader = XmiReader::new();
        reader.read_from(xml.as_bytes(), &mut model).unwrap();
        reader.resolve(&mut model).unwrap();

        round_trip_and_compare(&model);
    }
}
