use rascal::{PascalFileKind, PascalSectionKind, pascal_parser};

#[test]
fn parses_unit_sections_and_preserves_initial_modes() {
    let source = "
        {$I-}{$R+}{$Q+}{$V-}
        unit Demo;
        interface
        type T = Integer;
        implementation
        initialization
          Start;
        finalization
          Stop;
        end.
    ";
    let output = pascal_parser::parse(source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let file = output.file.expect("unit CST");
    assert_eq!(file.kind, PascalFileKind::Unit);
    assert_eq!(file.name.as_deref(), Some("demo"));
    assert!(!file.modes.io_checks);
    assert!(file.modes.range_checks);
    assert!(file.modes.overflow_checks);
    assert!(!file.modes.var_string_checks);
    assert_eq!(
        file.sections
            .iter()
            .map(|section| section.kind)
            .collect::<Vec<_>>(),
        [
            PascalSectionKind::Interface,
            PascalSectionKind::Implementation,
            PascalSectionKind::Initialization,
            PascalSectionKind::Finalization,
        ]
    );
}

#[test]
fn malformed_header_recovers_a_file_tree() {
    let output = pascal_parser::parse("unit Broken interface implementation end.");
    assert!(output.file.is_some());
    assert_eq!(output.diagnostics.len(), 1);
    assert!(output.diagnostics[0].message.contains("missing semicolon"));
}
