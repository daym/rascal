use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use rascal::{declaration_parser, lex, pascal_parser, semantic};

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

#[test]
fn every_extracted_pascal_fixture_produces_a_chumsky_cst() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/pascal/generated");
    let mut files = Vec::new();
    collect_pascal_files(&root, &mut files);
    files.sort();

    let mut missing_trees = Vec::new();
    let mut unexpected_diagnostics = Vec::new();
    let mut expected_recovery_diagnostics = 0usize;
    for path in files {
        let source = fs::read_to_string(&path).unwrap();
        let output = pascal_parser::parse(&source);
        if output.file.is_none() {
            missing_trees.push(path.clone());
        }
        if path.ends_with("test_parser/error_recovery_basic.pp") {
            expected_recovery_diagnostics += output.diagnostics.len();
        } else if !output.diagnostics.is_empty() {
            unexpected_diagnostics.push((path, output.diagnostics));
        }
    }

    assert!(missing_trees.is_empty(), "missing CSTs: {missing_trees:#?}");
    assert_eq!(
        expected_recovery_diagnostics, 1,
        "the malformed-header recovery fixture should produce one diagnostic"
    );
    assert!(
        unexpected_diagnostics.is_empty(),
        "unexpected structural diagnostics: {unexpected_diagnostics:#?}"
    );
}

#[test]
fn every_extracted_pascal_fixture_reaches_the_semantic_boundary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/pascal/generated");
    let mut files = Vec::new();
    collect_pascal_files(&root, &mut files);
    files.sort();

    let mut failures = Vec::new();
    let mut sources = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path).unwrap();
        let parsed = pascal_parser::parse(&source);
        let Some(file) = parsed.file else {
            failures.push((path, "missing file CST".to_owned()));
            continue;
        };
        let declarations = declaration_parser::parse_file_declarations(&file);
        if !declarations.diagnostics.is_empty() {
            failures.push((
                path.clone(),
                format!("declaration diagnostics: {:#?}", declarations.diagnostics),
            ));
            continue;
        }
        sources.push((path, source));
    }
    // Binding a batch shares one source-bound System prelude. The corpus test
    // checks that every fixture crosses the boundary, not unit-link validity.
    for chunk in sources.chunks(64) {
        let names = chunk
            .iter()
            .map(|(path, source)| (path.to_string_lossy().into_owned(), source.as_str()))
            .collect::<Vec<_>>();
        let views = names
            .iter()
            .map(|(name, source)| (name.as_str(), *source))
            .collect::<Vec<_>>();
        let compilation = semantic::bind_sources(&views);
        if compilation.files.len() != chunk.len() {
            for (path, _) in chunk {
                failures.push((
                    path.clone(),
                    "semantic binder did not return the batched file".to_owned(),
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "fixtures failed before semantic binding: {failures:#?}"
    );
}
