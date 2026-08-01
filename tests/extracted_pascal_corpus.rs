use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use rascal::lex;

fn collect_pascal_files(root: &Path, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_pascal_files(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "pp") {
            output.push(path);
        }
    }
}

#[test]
fn extracted_pascal_fixtures_are_current() {
    let status = Command::new(env!("CARGO_BIN_EXE_extract_pascal_tests"))
        .arg("--check")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("run extract_pascal_tests --check");
    assert!(status.success(), "the extracted Pascal corpus is stale");
}

#[test]
fn every_extracted_pascal_fixture_lexes_without_errors() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/pascal/generated");
    let mut files = Vec::new();
    collect_pascal_files(&root, &mut files);
    files.sort();
    assert!(!files.is_empty(), "the extracted Pascal corpus is empty");

    let mut failures = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path).unwrap();
        let output = lex(&source);
        if !output.diagnostics.is_empty() {
            failures.push((path, output.diagnostics));
        }
    }
    assert!(
        failures.is_empty(),
        "extracted fixtures failed to lex: {failures:#?}"
    );
}
