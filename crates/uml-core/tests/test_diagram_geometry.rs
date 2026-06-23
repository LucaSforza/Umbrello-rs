//! Integration tests for diagram geometry — node placement, move, resize,
//! and undo/redo of visual operations.

use uml_core::{
    commands, Class, Diagram, DiagramKind, History, ModelElement, Point, Size, UmlModel,
};

#[test]
fn place_node_then_move_and_resize_with_undo() {
    let mut model = UmlModel::new();
    let mut history: History = History::new(100);

    // Create a class in the model
    let cls = ModelElement::Class(Class::new("Widget"));
    let cls_id = cls.id();
    history
        .execute(Box::new(commands::CreateElement::new(cls)), &mut model)
        .unwrap();

    // Create a diagram
    let diag = Diagram::new("Main", DiagramKind::Class);
    let diag_id = diag.id;
    model.add_diagram(diag);

    // Step 1: Place node at (10, 10) with size (100, 50)
    let add_cmd = commands::AddNodeToDiagram::new(
        diag_id,
        cls_id,
        Point::new(10.0, 10.0),
        Size::new(100.0, 50.0),
    );
    history.execute(Box::new(add_cmd), &mut model).unwrap();

    // Verify position
    {
        let d = model.get_diagram(diag_id).unwrap();
        let node = d.get_node(cls_id).unwrap();
        assert_eq!(node.bounds.origin, Point::new(10.0, 10.0));
        assert_eq!(node.bounds.size, Size::new(100.0, 50.0));
    }

    // Step 2: Move to (50, 100)
    let move_cmd =
        commands::MoveNode::new(&model, diag_id, cls_id, Point::new(50.0, 100.0)).unwrap();
    history.execute(Box::new(move_cmd), &mut model).unwrap();

    {
        let d = model.get_diagram(diag_id).unwrap();
        let node = d.get_node(cls_id).unwrap();
        assert_eq!(node.bounds.origin, Point::new(50.0, 100.0));
    }

    // Step 3: Resize to (150, 80)
    let resize_cmd =
        commands::ResizeNode::new(&model, diag_id, cls_id, Size::new(150.0, 80.0)).unwrap();
    history.execute(Box::new(resize_cmd), &mut model).unwrap();

    {
        let d = model.get_diagram(diag_id).unwrap();
        let node = d.get_node(cls_id).unwrap();
        assert_eq!(node.bounds.size, Size::new(150.0, 80.0));
    }

    // Undo resize: back to (100, 50)
    history.undo(&mut model).unwrap();
    {
        let d = model.get_diagram(diag_id).unwrap();
        assert_eq!(d.get_node(cls_id).unwrap().bounds.size, Size::new(100.0, 50.0));
    }

    // Undo move: back to (10, 10)
    history.undo(&mut model).unwrap();
    {
        let d = model.get_diagram(diag_id).unwrap();
        assert_eq!(d.get_node(cls_id).unwrap().bounds.origin, Point::new(10.0, 10.0));
    }

    // Redo move
    history.redo(&mut model).unwrap();
    {
        let d = model.get_diagram(diag_id).unwrap();
        assert_eq!(d.get_node(cls_id).unwrap().bounds.origin, Point::new(50.0, 100.0));
    }

    // Redo resize
    history.redo(&mut model).unwrap();
    {
        let d = model.get_diagram(diag_id).unwrap();
        assert_eq!(d.get_node(cls_id).unwrap().bounds.size, Size::new(150.0, 80.0));
    }
}

#[test]
fn add_and_remove_node_from_diagram() {
    let mut model = UmlModel::new();
    let mut history = History::new(100);

    let cls = ModelElement::Class(Class::new("Test"));
    let cls_id = cls.id();
    history
        .execute(Box::new(commands::CreateElement::new(cls)), &mut model)
        .unwrap();

    let diag = Diagram::new("D", DiagramKind::Class);
    let diag_id = diag.id;
    model.add_diagram(diag);

    // Add node
    history
        .execute(
            Box::new(commands::AddNodeToDiagram::new(
                diag_id,
                cls_id,
                Point::new(0.0, 0.0),
                Size::new(100.0, 50.0),
            )),
            &mut model,
        )
        .unwrap();
    assert_eq!(model.get_diagram(diag_id).unwrap().node_count(), 1);

    // Remove node
    let rm_cmd = commands::RemoveNodeFromDiagram::new(&model, diag_id, cls_id).unwrap();
    history.execute(Box::new(rm_cmd), &mut model).unwrap();
    assert_eq!(model.get_diagram(diag_id).unwrap().node_count(), 0);

    // Undo: node is back
    history.undo(&mut model).unwrap();
    assert_eq!(model.get_diagram(diag_id).unwrap().node_count(), 1);

    // Redo: node gone again
    history.redo(&mut model).unwrap();
    assert_eq!(model.get_diagram(diag_id).unwrap().node_count(), 0);
}
