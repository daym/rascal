use rascal::{
    declaration_ast::{DeclarationSyntax, RoutineSyntaxKind, TypeSyntaxKind},
    declaration_parser, lex, pascal_parser,
};

#[test]
fn rtl_files_lex_and_produce_structural_trees() {
    for (name, source) in [
        ("system.pp", include_str!("../rtl/system.pp")),
        ("sysutils.pp", include_str!("../rtl/sysutils.pp")),
        ("baseunix.pp", include_str!("../rtl/baseunix.pp")),
        ("unix.pp", include_str!("../rtl/unix.pp")),
    ] {
        let lexed = lex(source);
        assert!(
            lexed.diagnostics.is_empty(),
            "{name}: {:#?}",
            lexed.diagnostics
        );
        let parsed = pascal_parser::parse(source);
        assert!(parsed.file.is_some(), "{name}: missing CST");
        assert!(
            parsed.diagnostics.is_empty(),
            "{name}: {:#?}",
            parsed.diagnostics
        );
    }
}

#[test]
fn rtl_char_to_shortstring_declaration_is_an_implicit_conversion_contract() {
    let parsed = pascal_parser::parse(include_str!("../rtl/system.pp"));
    let file = parsed.file.unwrap();
    let declarations = declaration_parser::parse_file_declarations(&file);
    assert!(
        declarations.diagnostics.is_empty(),
        "{:#?}",
        declarations.diagnostics
    );
    let operator_count = declarations
        .sections
        .iter()
        .flat_map(|section| &section.declarations)
        .filter(|declaration| {
            matches!(
                declaration,
                DeclarationSyntax::Routine(routine)
                    if routine.kind == RoutineSyntaxKind::Operator
            )
        })
        .count();
    assert!(
        operator_count >= 200,
        "only {operator_count} of the RTL operator declarations reached the declaration parser"
    );
    let conversion = declarations
        .sections
        .iter()
        .flat_map(|section| &section.declarations)
        .find_map(|declaration| {
            let DeclarationSyntax::Routine(routine) = declaration else {
                return None;
            };
            (routine.kind == RoutineSyntaxKind::Operator
                && routine.name.spelling == ":="
                && matches!(
                    routine.result.as_ref().map(|result| &result.kind),
                    Some(TypeSyntaxKind::Named(path))
                        if path.last().is_some_and(|name| name.spelling == "shortstring")
                ))
            .then_some(routine)
        })
        .unwrap_or_else(|| {
            panic!(
                "RTL Char-to-ShortString implicit conversion; parsed operators: {:#?}",
                declarations
                    .sections
                    .iter()
                    .flat_map(|section| &section.declarations)
                    .filter_map(|declaration| {
                        let DeclarationSyntax::Routine(routine) = declaration else {
                            return None;
                        };
                        (routine.kind == RoutineSyntaxKind::Operator)
                            .then_some((&routine.name.spelling, &routine.result))
                    })
                    .collect::<Vec<_>>()
            )
        });

    assert_eq!(conversion.parameters.len(), 1);
    assert!(matches!(
        conversion.parameters[0]
            .ty
            .as_ref()
            .map(|parameter| &parameter.kind),
        Some(TypeSyntaxKind::Named(path))
            if path.last().is_some_and(|name| name.spelling == "char")
    ));
    assert!(conversion.is_forward, "`external` must be bodyless");
}
