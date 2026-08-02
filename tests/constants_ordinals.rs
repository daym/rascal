use rascal::semantic::{
    BuiltinContract, ConstantValue, LookupRequest, MetadataQuery, SymbolKind, TypeRef, bind_sources,
};

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

fn lookup_constant<'a>(
    compilation: &'a rascal::semantic::SemanticCompilation,
    spelling: &str,
) -> &'a ConstantValue {
    let environment = compilation.files[0].environment;
    let name = compilation
        .binder
        .scopes
        .names()
        .lookup(spelling)
        .expect("interned constant name");
    let symbol = compilation
        .binder
        .scopes
        .lookup_symbol(environment, name, LookupRequest::REQUIRED_VALUE)
        .expect("visible constant")
        .primary[0]
        .symbol;
    &compilation
        .binder
        .constants
        .get(symbol)
        .expect("evaluated constant")
        .value
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

#[test]
fn parameterized_builtin_contracts_share_constant_and_runtime_binding() {
    let source = "
        program BuiltinContracts;
        type
          TIndex = 2..5;
          TItems = array[TIndex] of Byte;
        const
          SignedLow = Low(Int64);
          SignedHigh = High(Int64);
          UnsignedHigh = High(QWord);
          ItemLow = Low(TItems);
          ItemHigh = High(TItems);
          Int64Bytes = SizeOf(Int64);
          FiveIsOdd = Odd(5);
          NextValue = Succ(4);
          SquareValue = Sqr(6);
        var
          RuntimeValue: Int64;
          Items: TItems;
        begin
          RuntimeValue := High(Int64);
          RuntimeValue := Pred(RuntimeValue);
          Inc(RuntimeValue);
          if Odd(RuntimeValue) then
            RuntimeValue := Low(Int64);
          RuntimeValue := Length(Items);
        end.
    ";
    let compilation = bind_sources(&[("builtin_contracts.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    assert_eq!(
        lookup_constant(&compilation, "signedlow"),
        &ConstantValue::Integer(i64::MIN.into())
    );
    assert_eq!(
        lookup_constant(&compilation, "signedhigh"),
        &ConstantValue::Integer(i64::MAX.into())
    );
    assert_eq!(
        lookup_constant(&compilation, "unsignedhigh"),
        &ConstantValue::Integer(u64::MAX.into())
    );
    assert_eq!(
        lookup_constant(&compilation, "itemlow"),
        &ConstantValue::Integer(2)
    );
    assert_eq!(
        lookup_constant(&compilation, "itemhigh"),
        &ConstantValue::Integer(5)
    );
    assert_eq!(
        lookup_constant(&compilation, "int64bytes"),
        &ConstantValue::Integer(8)
    );
    assert_eq!(
        lookup_constant(&compilation, "fiveisodd"),
        &ConstantValue::Boolean(true)
    );
    assert_eq!(
        lookup_constant(&compilation, "nextvalue"),
        &ConstantValue::Integer(5)
    );
    assert_eq!(
        lookup_constant(&compilation, "squarevalue"),
        &ConstantValue::Integer(36)
    );
    let high_name = compilation.binder.scopes.names().lookup("high").unwrap();
    let high_symbol = compilation
        .binder
        .scopes
        .lookup_symbol(
            compilation.files[0].environment,
            high_name,
            LookupRequest::ORDINARY,
        )
        .unwrap()
        .primary[0]
        .symbol;
    let SymbolKind::Routine(_) = compilation.binder.scopes.symbol(high_symbol).kind else {
        panic!("System.High must retain the routine-category symbol declared by rtl/system.pp")
    };
    let high_family = compilation
        .builtin_families
        .family_for_symbol(high_symbol)
        .expect("System.High has parameterized semantic metadata");
    assert_eq!(
        compilation.builtin_families.get(high_family).contract,
        BuiltinContract::Metadata(MetadataQuery::High)
    );
}
