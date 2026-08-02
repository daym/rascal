use rascal::{
    declaration_ast::{DeclarationSyntax, RoutineSyntaxKind, TypeSyntaxKind},
    declaration_parser, lex, pascal_parser,
    semantic::{
        BuiltinContract, ConstantValue, FormalTypeKind, LookupRequest, MetadataQuery, RegionOwner,
        SetMutationOperation, SymbolKind, bind_sources,
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
    let family = compilation.builtin_families.get(family);
    assert_eq!(
        family.contract,
        BuiltinContract::Metadata(MetadataQuery::High)
    );
    assert_eq!(family.external_selector, "::u_system::p_high");
    assert_eq!(family.omitted_formals, vec![true]);
}

#[test]
fn external_selector_not_pascal_spelling_selects_the_builtin_handler() {
    let compilation = bind_sources(&[(
        "external_selector.pp",
        "
        program ExternalSelector;
        function Ceiling(const x): Integer;
          external name '::u_system::p_high';
        const LastByte = Ceiling(Byte);
        begin end.
        ",
    )]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let environment = compilation.files[0].environment;
    let ceiling = compilation.binder.scopes.names().lookup("ceiling").unwrap();
    let symbol = compilation
        .binder
        .scopes
        .lookup_symbol(environment, ceiling, LookupRequest::ORDINARY)
        .unwrap()
        .primary[0]
        .symbol;
    let family = compilation
        .builtin_families
        .family_for_symbol(symbol)
        .expect("external selector attaches the High handler");
    assert_eq!(
        compilation.builtin_families.get(family).contract,
        BuiltinContract::Metadata(MetadataQuery::High)
    );
    let last_byte = compilation
        .binder
        .scopes
        .names()
        .lookup("lastbyte")
        .unwrap();
    let constant = compilation
        .binder
        .scopes
        .lookup_symbol(environment, last_byte, LookupRequest::ORDINARY)
        .unwrap()
        .primary[0]
        .symbol;
    assert_eq!(
        compilation.binder.constants.get(constant).unwrap().value,
        ConstantValue::Integer(u8::MAX.into())
    );
}

#[test]
fn omitted_formals_are_explicit_and_do_not_compare_against_a_fake_type() {
    let compilation = bind_sources(&[(
        "untyped_external.pp",
        "
        program UntypedExternal;
        procedure Inspect(const x); external name 'backend_inspect';
        procedure Mutate(var x); external name 'backend_mutate';
        var Value: LongInt;
        begin
          Inspect(1);
          Mutate(Value);
        end.
        ",
    )]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let environment = compilation.files[0].environment;
    for spelling in ["inspect", "mutate"] {
        let name = compilation.binder.scopes.names().lookup(spelling).unwrap();
        let symbol = compilation
            .binder
            .scopes
            .lookup_symbol(environment, name, LookupRequest::ORDINARY)
            .unwrap()
            .primary[0]
            .symbol;
        let SymbolKind::Routine(callable) = compilation.binder.scopes.symbol(symbol).kind else {
            panic!("{spelling} must be a routine")
        };
        assert_eq!(
            compilation
                .binder
                .types
                .callable(callable)
                .unwrap()
                .signature
                .parameters[0]
                .type_kind,
            FormalTypeKind::Omitted
        );
        assert!(
            compilation
                .builtin_families
                .family_for_symbol(symbol)
                .is_none(),
            "an unknown backend selector must not manufacture a builtin"
        );
    }

    let rejected = bind_sources(&[(
        "untyped_var_literal.pp",
        "
        program UntypedVarLiteral;
        procedure Mutate(var x); external name 'backend_mutate';
        begin
          Mutate(1);
        end.
        ",
    )]);
    assert!(rejected.diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("no viable overload for `mutate`")
    }));
}

#[test]
fn selector_handler_supplies_relationships_omitted_from_pascal_formals() {
    let valid = bind_sources(&[(
        "set_mutation.pp",
        "
        program SetMutation;
        type
          TIndex = 0..7;
          TValues = set of TIndex;
        var
          Values: TValues;
          Element: TIndex;
        begin
          Include(Values, Element);
          Exclude(Values, Element);
        end.
        ",
    )]);
    assert!(valid.diagnostics.is_empty(), "{:#?}", valid.diagnostics);
    let include = valid.binder.scopes.names().lookup("include").unwrap();
    let symbol = valid
        .binder
        .scopes
        .lookup_symbol(valid.files[0].environment, include, LookupRequest::ORDINARY)
        .unwrap()
        .primary[0]
        .symbol;
    let family = valid
        .builtin_families
        .family_for_symbol(symbol)
        .expect("p_include selects the set relationship handler");
    assert_eq!(
        valid.builtin_families.get(family).contract,
        BuiltinContract::SetMutation(SetMutationOperation::Include)
    );

    let invalid = bind_sources(&[(
        "bad_set_mutation.pp",
        "
        program BadSetMutation;
        type
          TIndex = 0..7;
          TValues = set of TIndex;
        var
          Values: TValues;
          Item: Pointer;
        begin
          Include(Values, Item);
        end.
        ",
    )]);
    assert!(invalid.diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("no viable overload for `include`")
    }));
}
