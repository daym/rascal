use rascal::{
    PascalSectionKind,
    declaration_ast::{AggregateSyntaxKind, DeclarationSyntax, TypeSyntaxKind},
    declaration_parser, pascal_parser,
};

#[test]
fn parses_committed_type_sections_and_nested_aggregate_members() {
    let source = "
        program Main;
        type
          PNode = ^TNode;
          TNode = record
            Value: LongInt;
            type TNested = record Flag: Boolean; end;
          end;
        var Root: TNode;
        begin
        end.
    ";
    let parsed = pascal_parser::parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let declarations = declaration_parser::parse_file_declarations(parsed.file.as_ref().unwrap());
    assert!(
        declarations.diagnostics.is_empty(),
        "{:#?}",
        declarations.diagnostics
    );
    assert_eq!(declarations.sections.len(), 1);
    assert_eq!(
        declarations.sections[0].kind,
        PascalSectionKind::Declarations
    );
    let DeclarationSyntax::TypeSection {
        declarations: types,
        ..
    } = &declarations.sections[0].declarations[0]
    else {
        panic!("expected type section")
    };
    assert_eq!(types.len(), 2);
    assert!(matches!(types[0].ty.kind, TypeSyntaxKind::Pointer(_)));
    let TypeSyntaxKind::Aggregate { kind, members, .. } = &types[1].ty.kind else {
        panic!("expected record")
    };
    assert_eq!(*kind, AggregateSyntaxKind::Record);
    assert_eq!(members.len(), 2);
    assert_eq!(declarations.unsupported_count, 0);
}

#[test]
fn parses_nested_routine_declarations_before_their_body() {
    let source = "
        program Main;
        procedure Outer;
        var Before: LongInt;
        procedure Inner;
        type TLocal = LongInt;
        begin
        end;
        var After: LongInt;
        begin
        end;
        begin
        end.
    ";
    let parsed = pascal_parser::parse(source);
    let declarations = declaration_parser::parse_file_declarations(parsed.file.as_ref().unwrap());
    assert!(
        declarations.diagnostics.is_empty(),
        "{:#?}",
        declarations.diagnostics
    );
    let DeclarationSyntax::Routine(outer) = &declarations.sections[0].declarations[0] else {
        panic!("expected outer routine")
    };
    assert!(outer.has_body);
    assert_eq!(outer.body_declarations.len(), 3);
    let DeclarationSyntax::Routine(inner) = &outer.body_declarations[1] else {
        panic!("expected nested routine")
    };
    assert!(inner.has_body);
    assert_eq!(inner.body_declarations.len(), 1);
}
