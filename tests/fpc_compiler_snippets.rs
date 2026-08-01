use rascal::semantic::{
    ApplicationSelection, BoundApplicationTarget, BoundExpressionKind, BoundStatementKind,
    LookupRequest, SymbolKind, bind_sources,
};

// Adapted from fpc-3.2.0/compiler/constexp.pas:29..84.
const CONSTEXP_INTERFACE_SLICE: &str = "
    program ConstExprSlice;
    type
      Tconstexprint = record
        overflow: boolean;
        case signed: boolean of
          false: (uvalue: qword);
          true: (svalue: int64);
      end;
      errorproc = procedure(i: longint);

    operator + (const a, b: Tconstexprint): Tconstexprint; forward;

    var
      handler: errorproc;
      a, b: Tconstexprint;
    begin
      a + b;
    end.
";

#[test]
fn constexp_slice_binds_variant_record_procvar_and_custom_operator() {
    let compilation = bind_sources(&[(
        "fpc-3.2.0/compiler/constexp.pas:29..84",
        CONSTEXP_INTERFACE_SLICE,
    )]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let environment = compilation.files[0].environment;
    let name = compilation
        .binder
        .scopes
        .names()
        .lookup("tconstexprint")
        .unwrap();
    let result = compilation
        .binder
        .scopes
        .lookup_symbol(environment, name, LookupRequest::REQUIRED_TYPE)
        .unwrap();
    let SymbolKind::Type(record) = compilation
        .binder
        .scopes
        .symbol(result.primary[0].symbol)
        .kind
    else {
        panic!("expected record type")
    };
    let variant = compilation.binder.types.variant_part(record).unwrap();
    assert!(variant.selector.is_some());
    assert_eq!(variant.alternatives.len(), 2);
    assert_eq!(variant.alternatives[0].labels, vec![0]);
    assert_eq!(variant.alternatives[1].labels, vec![1]);
    assert_eq!(
        variant.alternatives[0].fields[0].layout.byte_offset,
        variant.alternatives[1].fields[0].layout.byte_offset
    );

    let body = compilation
        .bodies
        .iter()
        .find(|body| body.owner.is_none())
        .unwrap();
    let BoundStatementKind::Expression(expression) = &body.statements[0].kind else {
        panic!("expected custom-operator expression")
    };
    let BoundExpressionKind::Application {
        target: BoundApplicationTarget::Operator { resolution, .. },
        ..
    } = &expression.kind
    else {
        panic!("expected operator resolution")
    };
    assert!(matches!(
        resolution.selection,
        ApplicationSelection::Selected { .. }
    ));
    let selected = resolution.selected_symbol().unwrap();
    let declared_name = compilation.binder.scopes.symbol(selected).name;
    let unchecked_addition = compilation
        .binder
        .scopes
        .names()
        .lookup("&op_Addition")
        .unwrap();
    assert_eq!(declared_name, unchecked_addition);
    assert!(compilation.binder.scopes.names().lookup("+").is_none());
}

#[test]
fn operator_declaration_and_invocation_share_the_catalog_identifier_in_ordinary_lookup() {
    let source = "
        program CanonicalOperatorIdentity;
        type
          TNumber = record
            Value: LongInt;
          end;

        {$Q+}
        operator Add(const Left, Right: TNumber): TNumber; forward;

        var Left, Right: TNumber;
        begin
          Left + Right;
        end.
    ";
    let compilation = bind_sources(&[("canonical_operator_identity.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );

    let body = compilation
        .bodies
        .iter()
        .find(|body| body.owner.is_none())
        .unwrap();
    let BoundStatementKind::Expression(expression) = &body.statements[0].kind else {
        panic!("expected operator expression")
    };
    let BoundExpressionKind::Application {
        target: BoundApplicationTarget::Operator { resolution, .. },
        ..
    } = &expression.kind
    else {
        panic!("expected operator resolution")
    };
    let selected = resolution.selected_symbol().unwrap();
    let declared_name = compilation.binder.scopes.symbol(selected).name;
    let catalog_name = compilation
        .binder
        .scopes
        .names()
        .lookup("&op_CheckedAddition")
        .unwrap();

    assert_eq!(declared_name, catalog_name);
    assert!(compilation.binder.scopes.names().lookup("add").is_none());
    assert!(compilation.binder.scopes.names().lookup("+").is_none());
}
