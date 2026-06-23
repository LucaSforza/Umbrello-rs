//! Integration tests for the undo/redo command system.
//!
//! Tests the full lifecycle: create → rename → delete → undo → redo.

use uml_core::{
    commands,
    elements::{Class, Package},
    History, ModelElement, UmlModel,
};

#[test]
fn full_undo_redo_cycle() {
    let mut model = UmlModel::new();
    let mut history = History::new(100);

    // Step 1: Create class "Original"
    let cls = ModelElement::Class(Class::new("Original"));
    let id = cls.id();
    history
        .execute(Box::new(commands::CreateElement::new(cls)), &mut model)
        .unwrap();
    assert_eq!(model.len(), 1);
    assert_eq!(model.get(id).unwrap().name(), "Original");

    // Step 2: Rename to "Renamed"
    history
        .execute(
            Box::new(commands::RenameElement::new(&model, id, "Renamed".into()).unwrap()),
            &mut model,
        )
        .unwrap();
    assert_eq!(model.get(id).unwrap().name(), "Renamed");

    // Step 3: Delete it
    history
        .execute(Box::new(commands::DeleteElement::new(&model, id).unwrap()), &mut model)
        .unwrap();
    assert_eq!(model.len(), 0);

    // Undo 1: restore class (back to "Renamed")
    history.undo(&mut model).unwrap();
    assert_eq!(model.len(), 1);
    assert_eq!(model.get(id).unwrap().name(), "Renamed");

    // Undo 2: revert rename (back to "Original")
    history.undo(&mut model).unwrap();
    assert_eq!(model.get(id).unwrap().name(), "Original");

    // Undo 3: remove the class
    history.undo(&mut model).unwrap();
    assert_eq!(model.len(), 0);

    // Redo 1: recreate
    history.redo(&mut model).unwrap();
    assert_eq!(model.len(), 1);
    assert_eq!(model.get(id).unwrap().name(), "Original");

    // Redo 2: rename again
    history.redo(&mut model).unwrap();
    assert_eq!(model.get(id).unwrap().name(), "Renamed");

    // Redo 3: delete again
    history.redo(&mut model).unwrap();
    assert_eq!(model.len(), 0);
}

#[test]
fn undo_restores_same_id() {
    let mut model = UmlModel::new();
    let mut history = History::new(100);

    let cls = ModelElement::Class(Class::new("Persistent"));
    let original_id = cls.id();

    history
        .execute(Box::new(commands::CreateElement::new(cls)), &mut model)
        .unwrap();
    history.undo(&mut model).unwrap();
    history.redo(&mut model).unwrap();

    let restored = model.get(original_id).unwrap();
    assert_eq!(restored.name(), "Persistent");
    assert_eq!(restored.id(), original_id);
}

#[test]
fn create_package_with_child_then_undo() {
    let mut model = UmlModel::new();
    let mut history = History::new(100);

    let pkg = ModelElement::Package(Package::new("Root"));
    let pkg_id = pkg.id();
    let cls = ModelElement::Class(Class::new("Child"));
    let cls_id = cls.id();

    // Create both
    history
        .execute(Box::new(commands::CreateElement::new(pkg)), &mut model)
        .unwrap();
    history
        .execute(Box::new(commands::CreateElement::new(cls)), &mut model)
        .unwrap();

    // Move child into package
    let move_cmd = commands::MoveElement::new(&model, cls_id, Some(pkg_id)).unwrap();
    history.execute(Box::new(move_cmd), &mut model).unwrap();
    assert_eq!(model.parents_of(cls_id), Some(&[pkg_id][..]));

    // Undo move
    history.undo(&mut model).unwrap();
    assert_eq!(model.parents_of(cls_id), Some(&[][..]));

    // Undo create class
    history.undo(&mut model).unwrap();
    assert_eq!(model.len(), 1); // only package remains
}

#[test]
fn history_disabled_during_load() {
    let mut model = UmlModel::new();
    let mut history = History::new(100);
    history.set_disabled(true);

    // Simulate loading elements from XMI
    for i in 0..10 {
        history
            .execute(
                Box::new(commands::CreateElement::new(ModelElement::Class(Class::new(format!(
                    "Class{i}"
                ))))),
                &mut model,
            )
            .unwrap();
    }

    assert_eq!(model.len(), 10);
    assert!(!history.can_undo()); // nothing tracked
}
