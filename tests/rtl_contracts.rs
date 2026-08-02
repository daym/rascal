use rascal::{
    declaration_ast::{DeclarationSyntax, RoutineSyntaxKind, TypeSyntaxKind},
    declaration_parser, lex, pascal_parser,
    semantic::{
        BuiltinContract, LookupRequest, MetadataQuery, RegionOwner, SymbolKind, bind_sources,
    },
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
    let byte_backend = declarations
        .sections
        .iter()
        .flat_map(|section| &section.declarations)
        .find_map(|declaration| {
            let DeclarationSyntax::TypeSection { declarations, .. } = declaration else {
                return None;
            };
            declarations.iter().find_map(|declaration| {
                (declaration.name.spelling == "byte").then_some(&declaration.ty.kind)
            })
        })
        .expect("System.Byte declaration");
    assert!(matches!(
        byte_backend,
        TypeSyntaxKind::External { backend_name }
            if backend_name == "::u_system::t_byte"
    ));
    let error_code = declarations
        .sections
        .iter()
        .flat_map(|section| &section.declarations)
        .find_map(|declaration| {
            let DeclarationSyntax::Variables(value) = declaration else {
                return None;
            };
            value
                .names
                .iter()
                .any(|name| name.spelling == "errorcode")
                .then_some(value)
        })
        .expect("System.ErrorCode declaration");
    assert_eq!(
        error_code.external_name.as_deref(),
        Some("::u_system::p_errorcode")
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
    assert!(conversion.is_external);
    assert_eq!(
        conversion.external_name.as_deref(),
        Some("::u_system::o_implicit")
    );
}

#[test]
fn bundled_system_is_source_bound_and_intrinsic_metadata_uses_its_symbol() {
    let compilation = bind_sources(&[("source_bound_system.pp", "program P; begin end.")]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let environment = compilation.files[0].environment;
    let high = compilation.binder.scopes.names().lookup("high").unwrap();
    let high_symbol = compilation
        .binder
        .scopes
        .lookup_symbol(environment, high, LookupRequest::ORDINARY)
        .unwrap()
        .primary[0]
        .symbol;
    assert!(matches!(
        compilation.binder.scopes.symbol(high_symbol).kind,
        SymbolKind::Routine(_)
    ));
    assert!(matches!(
        compilation
            .binder
            .scopes
            .region_owner(compilation.binder.scopes.symbol(high_symbol).region),
        RegionOwner::Module(_)
    ));
    let family = compilation
        .builtin_families
        .family_for_symbol(high_symbol)
        .expect("source-declared System.High carries intrinsic metadata");
    assert_eq!(
        compilation.builtin_families.get(family).contract,
        BuiltinContract::Metadata(MetadataQuery::High)
    );
}
