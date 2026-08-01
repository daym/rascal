use rascal::semantic::{ConstantValue, LookupRequest, SymbolKind, TypeRef, bind_sources};

fn lookup_type(compilation: &rascal::semantic::SemanticCompilation, spelling: &str) -> TypeRef {
    let environment = compilation.files[0].environment;
    let name = compilation
        .binder
        .scopes
        .names()
        .lookup(spelling)
        .expect("interned type name");
    let result = compilation
        .binder
        .scopes
        .lookup_symbol(environment, name, LookupRequest::REQUIRED_TYPE)
        .expect("visible type");
    let SymbolKind::Type(ty) = compilation
        .binder
        .scopes
        .symbol(result.primary[0].symbol)
        .kind
    else {
        panic!("expected type symbol")
    };
    ty
}

#[test]
fn constants_enums_subranges_sets_arrays_and_case_labels_share_ordinal_semantics() {
    let source = "
        program Main;
        const
          Base = 2;
          Next = Base + 3;
        type
          TEnum = (Zero, Two = Base, Five = Next);
          TSub = Two..Five;
          TSet = set of TEnum;
          TArray = array[TSub] of LongInt;
        const
          Chosen: TEnum = Five;
          Values: TSet = [Zero, Two, Five];
        var Items: TArray;
        begin
          case Chosen of
            Zero: Items[Two] := 0;
            Two..Five: Items[Five] := Next;
          end;
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );

    let enum_type = lookup_type(&compilation, "tenum");
    assert_eq!(
        compilation.binder.types.ordinal_domain(enum_type),
        Some(rascal::semantic::OrdinalDomain { lower: 0, upper: 5 })
    );
    let subrange = lookup_type(&compilation, "tsub");
    assert_eq!(
        compilation.binder.types.ordinal_domain(subrange),
        Some(rascal::semantic::OrdinalDomain { lower: 2, upper: 5 })
    );
    let array = lookup_type(&compilation, "tarray");
    assert_eq!(
        compilation.binder.types.storage_layout(array).unwrap().size,
        16
    );
    let set = lookup_type(&compilation, "tset");
    assert_eq!(
        compilation.binder.types.storage_layout(set).unwrap().size,
        1
    );

    let environment = compilation.files[0].environment;
    let values_name = compilation.binder.scopes.names().lookup("values").unwrap();
    let values = compilation
        .binder
        .scopes
        .lookup_symbol(environment, values_name, LookupRequest::REQUIRED_VALUE)
        .unwrap()
        .primary[0]
        .symbol;
    assert!(matches!(
        &compilation.binder.constants.get(values).unwrap().value,
        ConstantValue::Set(elements)
            if elements.iter().copied().collect::<Vec<_>>() == vec![0, 2, 5]
    ));
}

#[test]
fn case_labels_must_be_constant_nonoverlapping_values_in_the_selector_domain() {
    let source = "
        program Main;
        var Selector, RuntimeValue, Sink: LongInt;
        begin
          case Selector of
            1..3: Sink := 1;
            3: Sink := 2;
            RuntimeValue: Sink := 3;
          end;
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("overlapping case label")),
        "{:#?}",
        compilation.diagnostics
    );
    assert!(
        compilation
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("case label is not constant")),
        "{:#?}",
        compilation.diagnostics
    );
}

#[test]
fn typed_constant_conversion_uses_the_declaration_range_mode_snapshot() {
    let source = "
        program Main;
        const
          {$R+}
          TooLarge: Byte = 256;
        begin
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("OutsideOrdinalDomain")),
        "{:#?}",
        compilation.diagnostics
    );
}
