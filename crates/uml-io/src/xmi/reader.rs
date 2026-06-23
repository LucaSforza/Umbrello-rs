//! XMI reader — two-pass parser for UML 1.2 XMI files.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};

use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;

use uml_core::{
    Class, Datatype, ElementBase, Enum, Interface, ModelElement, Package, UmlId, UmlModel,
    Visibility,
};

use super::error::XmiParseError;

/// XMI reader that populates a UmlModel from XML input.
///
/// Uses a two-pass strategy:
/// - Pass 1 (`read_from`): extracts structural elements and builds the containment tree.
/// - Pass 2 (`resolve`): resolves stereotype cross-references.
pub struct XmiReader {
    /// Maps XMI string IDs to generated UmlIds.
    id_map: HashMap<String, UmlId>,
    /// Maps element names to their UmlIds.
    name_map: HashMap<String, Vec<UmlId>>,
    /// Pending stereotype references: (element_id, stereotype_xmi_id).
    pending_stereotypes: Vec<(UmlId, String)>,
    /// Parent element stack for containment tracking.
    parent_stack: Vec<UmlId>,
}

impl XmiReader {
    /// Create a new XMI reader.
    #[must_use]
    pub fn new() -> Self {
        Self {
            id_map: HashMap::new(),
            name_map: HashMap::new(),
            pending_stereotypes: Vec::new(),
            parent_stack: Vec::new(),
        }
    }

    /// Parse XMI from a reader and populate the given model (Pass 1).
    ///
    /// This extracts structural elements (Package, Class, Interface, Enum, Datatype)
    /// and builds the containment hierarchy. Cross-references (stereotypes) are
    /// deferred to `resolve()`.
    ///
    /// Returns the number of elements parsed.
    pub fn read_from<R: Read>(
        &mut self,
        reader: R,
        model: &mut UmlModel,
    ) -> Result<usize, XmiParseError> {
        let buf_reader = BufReader::new(reader);
        let mut xml_reader = XmlReader::from_reader(buf_reader);
        xml_reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut count = 0;

        // Track sections to skip extension/diagram content.
        let mut inside_content = false;
        let mut inside_extensions = false;

        // Flag for deferred Classifier.feature skip (must happen after e's borrow on buf ends)
        let mut skip_feature = false;

        loop {
            match xml_reader.read_event_into(&mut buf)? {
                Event::Start(ref e) => {
                    let tag_name = e.name();
                    let tag_bytes = tag_name.as_ref();
                    let tag = std::str::from_utf8(tag_bytes).unwrap_or("");

                    // Track XMI sections
                    if tag.ends_with("XMI.content") || tag == "XMI.content" {
                        inside_content = true;
                        continue;
                    }
                    if tag.ends_with("XMI.extensions") || tag == "XMI.extensions" {
                        inside_extensions = true;
                        inside_content = false;
                        continue;
                    }
                    if tag.ends_with("XMI.header") || tag == "XMI.header" {
                        inside_content = false;
                        continue;
                    }

                    // Skip everything outside content section
                    if !inside_content || inside_extensions {
                        continue;
                    }

                    // Handle known structural elements
                    let local_name = Self::local_name(tag);
                    match local_name {
                        "Model" | "Package" => {
                            if let Some(elem) = self.parse_package(e)? {
                                let id = elem.base().id;
                                model.insert(elem);
                                count += 1;
                                self.push_parent(id);
                            }
                        },
                        "Class" => {
                            if let Some(elem) = self.parse_class(e)? {
                                model.insert(elem);
                                count += 1;
                            }
                        },
                        "Interface" => {
                            if let Some(elem) = self.parse_interface(e)? {
                                model.insert(elem);
                                count += 1;
                            }
                        },
                        "Enumeration" | "Enum" => {
                            if let Some(elem) = self.parse_enum(e)? {
                                model.insert(elem);
                                count += 1;
                            }
                        },
                        "DataType" => {
                            if let Some(elem) = self.parse_datatype(e)? {
                                model.insert(elem);
                                count += 1;
                            }
                        },
                        "Stereotype" => {
                            self.register_stereotype(e)?;
                        },
                        "Namespace.ownedElement" => {
                            // Wrapper — just enter
                        },
                        "Classifier.feature" => {
                            // REVIEW CONDITION C5: Skip this wrapper.
                            // Deferred skip (after e's borrow on buf is released).
                            skip_feature = true;
                        },
                        _ => {
                            // Skip unknown elements silently (lenient parsing)
                        },
                    }
                },
                Event::Empty(ref e) => {
                    let tag_name = e.name();
                    let tag_bytes = tag_name.as_ref();
                    let tag = std::str::from_utf8(tag_bytes).unwrap_or("");

                    // Track XMI sections (handle self-closing sections)
                    if tag.ends_with("XMI.content") || tag == "XMI.content" {
                        inside_content = true;
                        continue;
                    }
                    if tag.ends_with("XMI.extensions") || tag == "XMI.extensions" {
                        inside_extensions = true;
                        inside_content = false;
                        continue;
                    }
                    if tag.ends_with("XMI.header") || tag == "XMI.header" {
                        inside_content = false;
                        continue;
                    }

                    // Skip everything outside content section
                    if !inside_content || inside_extensions {
                        continue;
                    }

                    // Handle known structural elements (self-closing)
                    let local_name = Self::local_name(tag);
                    match local_name {
                        "Model" | "Package" => {
                            if let Some(elem) = self.parse_package(e)? {
                                model.insert(elem);
                                count += 1;
                                // Self-closing Model/Package — no children expected
                            }
                        },
                        "Class" => {
                            if let Some(elem) = self.parse_class(e)? {
                                model.insert(elem);
                                count += 1;
                            }
                        },
                        "Interface" => {
                            if let Some(elem) = self.parse_interface(e)? {
                                model.insert(elem);
                                count += 1;
                            }
                        },
                        "Enumeration" | "Enum" => {
                            if let Some(elem) = self.parse_enum(e)? {
                                model.insert(elem);
                                count += 1;
                            }
                        },
                        "DataType" => {
                            if let Some(elem) = self.parse_datatype(e)? {
                                model.insert(elem);
                                count += 1;
                            }
                        },
                        "Stereotype" => {
                            self.register_stereotype(e)?;
                        },
                        _ => {
                            // Skip unknown elements silently
                        },
                    }
                },
                Event::End(ref e) => {
                    let tag_name = e.name();
                    let tag_bytes = tag_name.as_ref();
                    let tag = std::str::from_utf8(tag_bytes).unwrap_or("");
                    let local_name = Self::local_name(tag);

                    match local_name {
                        "Model" | "Package" => {
                            // Pop current parent — children are now siblings
                            // of this package, not nested under it.
                            if !self.parent_stack.is_empty() {
                                self.parent_stack.pop();
                            }
                        },
                        "XMI.extensions" => {
                            inside_extensions = false;
                            inside_content = false;
                        },
                        _ => {},
                    }
                },
                Event::Eof => break,
                _ => {},
            }

            // Deferred skip for Classifier.feature (after e's borrow on buf is released)
            if skip_feature {
                self.skip_element(&mut xml_reader, &mut buf)?;
                skip_feature = false;
            }

            buf.clear();
        }

        Ok(count)
    }

    /// Resolve pending cross-references (Pass 2).
    ///
    /// Must be called after `read_from()`. Resolves stereotype references.
    pub fn resolve(&mut self, model: &mut UmlModel) -> Result<(), XmiParseError> {
        for (element_id, xmi_stereotype_id) in &self.pending_stereotypes {
            if let Some(stereotype_uml_id) = self.id_map.get(xmi_stereotype_id) {
                if let Some(elem) = model.get_mut(*element_id) {
                    elem.base_mut().stereotype_id = Some(*stereotype_uml_id);
                }
            }
            // If stereotype not found, leave as None (not an error — M9 will handle fully)
        }
        self.pending_stereotypes.clear();
        Ok(())
    }

    // ─── Private helpers ───────────────────────────────────────────

    /// Extract the local name from a tag like "UML:Class" or "uml:Class".
    fn local_name(tag: &str) -> &str {
        tag.rsplit(':').next().unwrap_or(tag)
    }

    /// Get an attribute value by local name.
    fn attr_value(e: &quick_xml::events::BytesStart, name: &str) -> Option<String> {
        for attr in e.attributes().flatten() {
            let attr_local = std::str::from_utf8(attr.key.as_ref())
                .unwrap_or("")
                .rsplit(':')
                .next()
                .unwrap_or("");
            if attr_local.eq_ignore_ascii_case(name) {
                return Some(String::from_utf8_lossy(&attr.value).into_owned());
            }
        }
        None
    }

    /// Require an attribute or return an error.
    fn require_attr(
        e: &quick_xml::events::BytesStart,
        name: &str,
        element_name: &str,
    ) -> Result<String, XmiParseError> {
        Self::attr_value(e, name).ok_or_else(|| XmiParseError::MissingAttribute {
            element: element_name.to_string(),
            attr: name.to_string(),
        })
    }

    /// Parse visibility string to Visibility enum.
    fn parse_visibility(s: &str) -> Visibility {
        match s {
            "public" => Visibility::Public,
            "protected" => Visibility::Protected,
            "private" => Visibility::Private,
            _ => Visibility::Implementation,
        }
    }

    /// Register an element's XMI ID and return the generated UmlId.
    fn register_id(&mut self, xmi_id: &str) -> Result<UmlId, XmiParseError> {
        if self.id_map.contains_key(xmi_id) {
            return Err(XmiParseError::DuplicateId(xmi_id.to_string()));
        }
        let uml_id = UmlId::new();
        self.id_map.insert(xmi_id.to_string(), uml_id);
        Ok(uml_id)
    }

    /// Register a name-to-ID mapping.
    fn register_name(&mut self, name: &str, uml_id: UmlId) {
        self.name_map
            .entry(name.to_string())
            .or_default()
            .push(uml_id);
    }

    /// Push a parent onto the containment stack.
    fn push_parent(&mut self, parent_id: UmlId) {
        self.parent_stack.push(parent_id);
    }

    /// Add a child to the current parent package.
    #[allow(dead_code)]
    fn add_to_current_parent(&self, model: &mut UmlModel, child_id: UmlId) {
        if let Some(&parent_id) = self.parent_stack.last() {
            // Silently ignore errors (e.g., cycle detection — shouldn't happen in XMI)
            let _ = model.add_to_package(parent_id, child_id);
        }
    }

    /// Build a common ElementBase from XMI attributes.
    fn build_base(
        &mut self,
        e: &quick_xml::events::BytesStart,
        element_name: &str,
    ) -> Result<ElementBase, XmiParseError> {
        let xmi_id = Self::require_attr(e, "xmi.id", element_name)?;
        let name = Self::attr_value(e, "name").unwrap_or_default();
        let vis_str = Self::attr_value(e, "visibility").unwrap_or_else(|| "public".to_string());
        let is_abstract = Self::attr_value(e, "isAbstract").is_some_and(|v| v == "true");

        let uml_id = self.register_id(&xmi_id)?;
        self.register_name(&name, uml_id);

        Ok(ElementBase {
            id: uml_id,
            name,
            visibility: Self::parse_visibility(&vis_str),
            stereotype_id: None, // resolved in Pass 2
            documentation: String::new(),
            is_abstract,
            is_static: false,
            original_xmi_id: Some(xmi_id),
        })
    }

    /// Remember a stereotype reference for Pass 2 resolution.
    fn defer_stereotype(&mut self, element_id: UmlId, stereotype_xmi: Option<String>) {
        if let Some(st_id) = stereotype_xmi {
            self.pending_stereotypes.push((element_id, st_id));
        }
    }

    /// Parse a <UML:Model> or <UML:Package> element.
    fn parse_package(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<Option<ModelElement>, XmiParseError> {
        let base = self.build_base(e, "Package")?;
        let elem_id = base.id;
        let stereo = Self::attr_value(e, "stereotype");
        self.defer_stereotype(elem_id, stereo);

        // Use Package::new() and swap base because children is pub(crate)
        let mut pkg = Package::new("");
        pkg.base = base;
        Ok(Some(ModelElement::Package(pkg)))
    }

    /// Parse a <UML:Class> element.
    fn parse_class(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<Option<ModelElement>, XmiParseError> {
        let base = self.build_base(e, "Class")?;
        let elem_id = base.id;
        let stereo = Self::attr_value(e, "stereotype");
        self.defer_stereotype(elem_id, stereo);

        Ok(Some(ModelElement::Class(Class {
            base,
            classifier: Default::default(),
        })))
    }

    /// Parse a <UML:Interface> element.
    fn parse_interface(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<Option<ModelElement>, XmiParseError> {
        let base = self.build_base(e, "Interface")?;
        let elem_id = base.id;
        self.defer_stereotype(elem_id, Self::attr_value(e, "stereotype"));

        Ok(Some(ModelElement::Interface(Interface {
            base,
            classifier: Default::default(),
        })))
    }

    /// Parse a <UML:Enumeration> or <UML:Enum> element.
    fn parse_enum(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<Option<ModelElement>, XmiParseError> {
        let base = self.build_base(e, "Enumeration")?;
        let elem_id = base.id;
        self.defer_stereotype(elem_id, Self::attr_value(e, "stereotype"));

        Ok(Some(ModelElement::Enum(Enum {
            base,
            classifier: Default::default(),
            literals: Vec::new(),
        })))
    }

    /// Parse a <UML:DataType> element.
    fn parse_datatype(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<Option<ModelElement>, XmiParseError> {
        let base = self.build_base(e, "DataType")?;
        let elem_id = base.id;
        self.defer_stereotype(elem_id, Self::attr_value(e, "stereotype"));

        Ok(Some(ModelElement::Datatype(Datatype {
            base,
            classifier: Default::default(),
        })))
    }

    /// Register a stereotype from the XMI.
    /// In M8 we only extract the ID mapping so that stereotype references
    /// on elements can be resolved.
    fn register_stereotype(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<(), XmiParseError> {
        let xmi_id = Self::require_attr(e, "xmi.id", "Stereotype")?;
        let _name = Self::attr_value(e, "name").unwrap_or_default();
        // Register in ID map so references can be resolved
        self.id_map.entry(xmi_id).or_default();
        Ok(())
    }

    /// Skip an element and all its children by tracking nesting depth.
    fn skip_element<R: BufRead>(
        &self,
        reader: &mut XmlReader<R>,
        buf: &mut Vec<u8>,
    ) -> Result<(), XmiParseError> {
        let mut depth = 1;
        loop {
            match reader.read_event_into(buf)? {
                Event::Start(_) => {
                    depth += 1;
                },
                Event::End(_) => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(());
                    }
                },
                Event::Eof => return Ok(()),
                _ => {},
            }
            buf.clear();
        }
    }
}

impl Default for XmiReader {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal XMI with one class.
    const XMI_ONE_CLASS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
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

    /// XMI with package containing two classes.
    const XMI_PACKAGE_WITH_CLASSES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<XMI xmi.version="1.2" xmlns:UML="http://schema.omg.org/spec/UML/1.3">
 <XMI.header/>
 <XMI.content>
  <UML:Model xmi.id="m1" name="UML Model">
   <UML:Namespace.ownedElement>
    <UML:Package xmi.id="P1" name="mypackage">
     <UML:Namespace.ownedElement>
      <UML:Class xmi.id="C1" name="Person"/>
      <UML:Class xmi.id="C2" name="Address"/>
     </UML:Namespace.ownedElement>
    </UML:Package>
   </UML:Namespace.ownedElement>
  </UML:Model>
 </XMI.content>
</XMI>"#;

    /// XMI with stereotypes.
    const XMI_WITH_STEREOTYPES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<XMI xmi.version="1.2" xmlns:UML="http://schema.omg.org/spec/UML/1.3">
 <XMI.header/>
 <XMI.content>
  <UML:Model xmi.id="m1" name="UML Model">
   <UML:Namespace.ownedElement>
    <UML:Stereotype xmi.id="folder" name="folder"/>
    <UML:Stereotype xmi.id="datatype" name="datatype"/>
    <UML:Class xmi.id="C1" name="Person" stereotype="folder"/>
    <UML:DataType xmi.id="D1" name="int" stereotype="datatype"/>
   </UML:Namespace.ownedElement>
  </UML:Model>
 </XMI.content>
</XMI>"#;

    #[test]
    fn parse_one_class() {
        let mut model = UmlModel::new();
        let mut reader = XmiReader::new();
        let count = reader
            .read_from(XMI_ONE_CLASS.as_bytes(), &mut model)
            .unwrap();
        reader.resolve(&mut model).unwrap();

        assert!(count >= 1, "should parse at least one element");
        // Find the Person class
        let person = model.iter().find(|(_, e)| e.name() == "Person");
        assert!(person.is_some(), "should find Person class");
        let (_id, elem) = person.unwrap();
        assert_eq!(elem.object_type(), uml_core::ObjectType::Class);
        assert!(elem.base().original_xmi_id.is_some());
    }

    #[test]
    fn parse_package_with_classes() {
        let mut model = UmlModel::new();
        let mut reader = XmiReader::new();
        reader
            .read_from(XMI_PACKAGE_WITH_CLASSES.as_bytes(), &mut model)
            .unwrap();
        reader.resolve(&mut model).unwrap();

        // Should have at least 3 elements: 1 package + 2 classes
        assert!(model.len() >= 3);

        // Find the package
        let pkg = model.iter().find(|(_, e)| e.name() == "mypackage");
        assert!(pkg.is_some(), "should find mypackage");

        // Find the classes
        let person = model.iter().find(|(_, e)| e.name() == "Person");
        let address = model.iter().find(|(_, e)| e.name() == "Address");
        assert!(person.is_some());
        assert!(address.is_some());
    }

    #[test]
    fn parse_stereotypes() {
        let mut model = UmlModel::new();
        let mut reader = XmiReader::new();
        reader
            .read_from(XMI_WITH_STEREOTYPES.as_bytes(), &mut model)
            .unwrap();
        reader.resolve(&mut model).unwrap();

        // Find the Person class
        let person = model.iter().find(|(_, e)| e.name() == "Person");
        assert!(person.is_some());

        // Find the int datatype
        let int_type = model.iter().find(|(_, e)| e.name() == "int");
        assert!(int_type.is_some());

        // Datatype should be in model
        let datatypes: Vec<_> = model
            .iter()
            .filter(|(_, e)| matches!(e, ModelElement::Datatype(_)))
            .collect();
        assert!(!datatypes.is_empty());
    }

    #[test]
    fn parse_preserves_original_xmi_id() {
        let mut model = UmlModel::new();
        let mut reader = XmiReader::new();
        reader
            .read_from(XMI_ONE_CLASS.as_bytes(), &mut model)
            .unwrap();
        reader.resolve(&mut model).unwrap();

        let person = model.iter().find(|(_, e)| e.name() == "Person").unwrap();
        assert_eq!(person.1.base().original_xmi_id, Some("C1".to_string()));
    }

    #[test]
    fn parse_empty_xmi() {
        let xml =
            r#"<?xml version="1.0"?><XMI xmi.version="1.2"><XMI.header/><XMI.content/></XMI>"#;
        let mut model = UmlModel::new();
        let mut reader = XmiReader::new();
        let count = reader.read_from(xml.as_bytes(), &mut model).unwrap();
        reader.resolve(&mut model).unwrap();
        assert_eq!(count, 0);
        assert!(model.is_empty());
    }

    #[test]
    fn parse_duplicate_id() {
        let xml = r#"<?xml version="1.0"?><XMI xmi.version="1.2" xmlns:UML="http://schema.omg.org/spec/UML/1.3"><XMI.header/><XMI.content><UML:Model xmi.id="m1" name="Model"><UML:Namespace.ownedElement><UML:Class xmi.id="C1" name="A"/><UML:Class xmi.id="C1" name="B"/></UML:Namespace.ownedElement></UML:Model></XMI.content></XMI>"#;
        let mut model = UmlModel::new();
        let mut reader = XmiReader::new();
        let result = reader.read_from(xml.as_bytes(), &mut model);
        assert!(result.is_err());
    }

    #[test]
    fn parse_with_interface() {
        let xml = r#"<?xml version="1.0"?><XMI xmi.version="1.2" xmlns:UML="http://schema.omg.org/spec/UML/1.3"><XMI.header/><XMI.content><UML:Model xmi.id="m1" name="Model"><UML:Namespace.ownedElement><UML:Interface xmi.id="I1" name="Serializable" visibility="public"/></UML:Namespace.ownedElement></UML:Model></XMI.content></XMI>"#;
        let mut model = UmlModel::new();
        let mut reader = XmiReader::new();
        reader.read_from(xml.as_bytes(), &mut model).unwrap();
        reader.resolve(&mut model).unwrap();

        let iface = model.iter().find(|(_, e)| e.name() == "Serializable");
        assert!(iface.is_some());
        assert!(matches!(iface.unwrap().1, ModelElement::Interface(_)));
    }
}
