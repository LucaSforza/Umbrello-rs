//! Integration test: parse all real XMI files from the Umbrello test corpus.
//!
//! This test exercises the full XMI parser pipeline (read_from → resolve)
//! against every `.xmi` file in the `test/` directory of the Umbrello
//! repository. It validates that:
//!
//! - Every file parses without errors.
//! - Cross-references resolve without errors.
//! - The resulting model has reasonable element counts.
//! - No non-stereotype dangling references remain after resolution.
//!
//! # Known limitations
//!
//! - Stereotype references may produce dangling-reference validation errors
//!   because stereotypes are tracked in the ID map but not yet inserted as
//!   full `ModelElement` entries. These are filtered out of the failure check.

use std::fs;
use std::path::Path;

use uml_core::UmlModel;
use uml_io::xmi::XmiReader;

/// Find the Umbrello test directory relative to this crate.
fn find_test_dir() -> Option<String> {
    // Try via CARGO_MANIFEST_DIR first (most reliable)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = Path::new(&manifest_dir);
        // From crates/uml-io/, go up to repo root
        if let Some(repo_root) = manifest_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            let test_dir = repo_root.join("test");
            if test_dir.exists() && test_dir.is_dir() {
                return Some(test_dir.to_string_lossy().to_string());
            }
        }
    }

    // Fallback: try relative paths from the current directory
    let candidates = [
        "../../tests/data/xmi",
        "../../../tests/data/xmi",
        "tests/data/xmi",
    ];
    for candidate in &candidates {
        let path = Path::new(candidate);
        if path.exists() && path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    if entry.path().extension().is_some_and(|e| e == "xmi") {
                        return Some(candidate.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Check if a ReferenceError is stereotype-related (expected limitation).
fn is_stereotype_error(err: &uml_core::ReferenceError) -> bool {
    err.field == uml_core::ReferenceField::Stereotype
}

#[test]
fn parse_all_cpp_test_files() {
    let test_dir_str =
        find_test_dir().expect("Could not find Umbrello test/ directory with XMI files");

    let test_dir = Path::new(&test_dir_str);
    eprintln!("Using test directory: {}", test_dir.display());

    let xmi_files: Vec<_> = fs::read_dir(test_dir)
        .expect("Failed to read test directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "xmi") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    assert!(!xmi_files.is_empty(), "No XMI files found in {}", test_dir.display());

    eprintln!("Found {} XMI files to parse\n", xmi_files.len());

    let mut parsed_count = 0;
    let mut failures: Vec<String> = Vec::new();
    let mut total_elements = 0;
    let mut total_relationships = 0;
    let mut total_attrs = 0usize;
    let mut total_ops = 0usize;
    let mut total_diagrams = 0usize;
    let mut total_diagram_nodes = 0usize;
    let mut total_diagram_edges = 0usize;
    let mut files_with_diagrams = 0usize;

    for path in &xmi_files {
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut model = UmlModel::new();
        let mut reader = XmiReader::new();

        let file = match fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                failures.push(format!("{} (open error: {})", file_name, e));
                continue;
            },
        };

        match reader.read_from(std::io::BufReader::new(file), &mut model) {
            Ok(count) => {
                match reader.resolve(&mut model) {
                    Ok(_) => {
                        let errors = model.validate_references();
                        // Filter out expected stereotype errors
                        let real_errors: Vec<_> =
                            errors.iter().filter(|e| !is_stereotype_error(e)).collect();

                        if real_errors.is_empty() {
                            // Count features for statistics
                            let (n_attrs, n_ops) = count_features(&model);
                            let n_rels = count_relationships(&model);
                            total_attrs += n_attrs;
                            total_ops += n_ops;
                            total_relationships += n_rels;

                            // Count diagrams
                            let n_diagrams = model.diagrams().len();
                            let n_diag_nodes: usize =
                                model.diagrams().iter().map(|d| d.node_count()).sum();
                            let n_diag_edges: usize =
                                model.diagrams().iter().map(|d| d.edge_count()).sum();
                            total_diagrams += n_diagrams;
                            total_diagram_nodes += n_diag_nodes;
                            total_diagram_edges += n_diag_edges;
                            if n_diagrams > 0 {
                                files_with_diagrams += 1;
                            }

                            eprintln!(
                                "OK  {} — {} structural elements, {} total, {} rels, {} attrs, {} ops, {} diag(s), {} nodes, {} edges",
                                file_name,
                                count,
                                model.len(),
                                n_rels,
                                n_attrs,
                                n_ops,
                                n_diagrams,
                                n_diag_nodes,
                                n_diag_edges,
                            );
                            parsed_count += 1;
                            total_elements += model.len();
                        } else {
                            let err_details: Vec<String> = real_errors
                                .iter()
                                .map(|e| format!("  {:?} id={:?}", e.field, e.target_id))
                                .collect();
                            failures.push(format!(
                                "{} ({} unresolved references):\n{}",
                                file_name,
                                real_errors.len(),
                                err_details.join("\n")
                            ));
                        }
                    },
                    Err(e) => {
                        failures.push(format!("{} (resolve error: {})", file_name, e));
                    },
                }
            },
            Err(e) => {
                failures.push(format!("{} (parse error: {})", file_name, e));
            },
        }
    }

    eprintln!(
        "\nResults: {}/{} files parsed successfully ({} total elements, {} relationships, {} attributes, {} operations, {} diagrams, {} nodes, {} edges across {} files with diagrams)",
        parsed_count,
        xmi_files.len(),
        total_elements,
        total_relationships,
        total_attrs,
        total_ops,
        total_diagrams,
        total_diagram_nodes,
        total_diagram_edges,
        files_with_diagrams,
    );

    assert!(
        parsed_count > 0,
        "should parse at least one XMI file — check test directory path"
    );

    if !failures.is_empty() {
        panic!("Failed to parse {} files:\n{}", failures.len(), failures.join("\n---\n"));
    }

    // Verify meaningful data was extracted
    assert!(
        total_elements >= parsed_count,
        "each parsed file should contribute at least one element"
    );
}

/// Count relationships in a model.
fn count_relationships(model: &UmlModel) -> usize {
    model
        .iter()
        .filter(|(_, e)| matches!(e, uml_core::ModelElement::Relationship(_)))
        .count()
}

/// Count features (attributes + operations) across all classifiers.
fn count_features(model: &UmlModel) -> (usize, usize) {
    let mut attrs = 0usize;
    let mut ops = 0usize;
    for (_, elem) in model.iter() {
        if let Some(data) = elem.classifier_data() {
            attrs += data.attributes.len();
            ops += data.operations.len();
        }
    }
    (attrs, ops)
}
