use std::collections::HashSet;
use std::str::FromStr;
use uml_core::id::UmlId;

#[test]
fn new_produces_unique_ids() {
    let count = 10_000;
    let ids: HashSet<_> = (0..count).map(|_| UmlId::new()).collect();
    assert_eq!(ids.len(), count, "UmlId::new() produced duplicates");
}

#[test]
fn default_is_valid() {
    let id = UmlId::default();
    let s = id.to_string();
    assert!(!s.is_empty(), "default UmlId should produce non-empty Display");
}

#[test]
fn display_roundtrip() {
    let id = UmlId::new();
    let s = id.to_string();
    let parsed: UmlId = s.parse().expect("parsing Display output");
    assert_eq!(id, parsed);
}

#[test]
fn serde_roundtrip() {
    let id = UmlId::new();
    let json = serde_json::to_string(&id).expect("serialize");
    let back: UmlId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(id, back);
}

#[test]
fn clone_equality() {
    let id = UmlId::new();
    assert_eq!(id, id.clone());
}

#[test]
fn hash_consistency() {
    use std::hash::{Hash, Hasher};
    let id = UmlId::new();
    let mut h1 = std::collections::hash_map::DefaultHasher::new();
    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut h1);
    id.clone().hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
}

#[test]
fn order_consistent_with_eq() {
    let a = UmlId::new();
    let b = UmlId::new();
    if a == b {
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
    }
}

#[test]
fn parse_invalid_rejected() {
    assert!(UmlId::from_str("not-a-uuid").is_err());
}
