use uml_core::types::*;

/// Helper: round-trip a value through JSON.
fn roundtrip<
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + PartialEq + std::fmt::Debug,
>(
    value: &T,
) {
    let json = serde_json::to_string(value).expect("serialize");
    let back: T = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(value, &back, "round-trip failed for JSON: {json}");
}

#[test]
fn object_type_all_variants_roundtrip() {
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
        ObjectType::Stereotype,
        ObjectType::InstanceAttribute,
    ];
    for v in &variants {
        assert_eq!(v.as_str(), v.to_string());
        roundtrip(v);
    }
    // No two variants serialize to the same string
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            let si = serde_json::to_string(&variants[i]).unwrap();
            let sj = serde_json::to_string(&variants[j]).unwrap();
            assert_ne!(si, sj, "duplicate serialization: {si}");
        }
    }
}

#[test]
fn association_type_all_variants_roundtrip() {
    for v in &[
        AssociationType::Association,
        AssociationType::Generalization,
        AssociationType::Realization,
        AssociationType::Aggregation,
        AssociationType::Composition,
        AssociationType::Dependency,
    ] {
        roundtrip(v);
    }
}

#[test]
fn diagram_type_all_variants_roundtrip() {
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
        roundtrip(v);
    }
}

#[test]
fn visibility_all_variants_roundtrip() {
    for v in &[
        Visibility::Public,
        Visibility::Protected,
        Visibility::Private,
        Visibility::Implementation,
    ] {
        roundtrip(v);
    }
}

#[test]
fn relationship_all_variants_roundtrip() {
    use uml_core::elements::Relationship;
    use uml_core::UmlId;

    for kind in &[
        AssociationType::Generalization,
        AssociationType::Realization,
        AssociationType::Association,
        AssociationType::Aggregation,
        AssociationType::Composition,
        AssociationType::Dependency,
    ] {
        let rel = Relationship::new(*kind, UmlId::new(), UmlId::new());
        roundtrip(&rel);
    }
}

#[test]
fn parameter_direction_all_variants_roundtrip() {
    for v in &[
        ParameterDirection::In,
        ParameterDirection::Out,
        ParameterDirection::InOut,
        ParameterDirection::Return,
    ] {
        roundtrip(v);
    }
}
