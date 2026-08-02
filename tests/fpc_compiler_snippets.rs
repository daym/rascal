use rascal::semantic::{
    ApplicationCandidate, ApplicationSelection, BoundApplicationTarget, BoundExpression,
    BoundExpressionKind, BoundStatement, BoundStatementKind, BuiltinInstantiation,
    BuiltinOperation, ConstantValue, LookupRequest, MetadataQuery, OrdinalOperation, StepOperation,
    SymbolKind, bind_sources,
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

// Adapted from fpc-3.2.0/compiler/constexp.pas:128..187 and 300..338.
// The source repeatedly uses High(Int64)/High(QWord) both in calculations
// folded by the compiler and in ordinary executable comparisons.
const CONSTEXP_LIMIT_SLICE: &str = "
    program ConstExprLimits;
    const
      SignedMaximum: QWord = QWord(High(Int64));
      UnsignedSpace: QWord = High(QWord) - QWord(High(Int64));

    function FitsSigned(Value: QWord): Boolean;
    begin
      Result := Value <= QWord(High(Int64));
    end;

    var RuntimeValue: QWord;
    begin
      RuntimeValue := UnsignedSpace;
      FitsSigned(RuntimeValue);
    end.
";

// Adapted from fpc-3.2.0/compiler/symtable.pas:1212..1219 and
// 2299..2307. It retains the enum traversal and the
// Pred(Length(...))/Inc(...) runtime shapes.
const SYMTABLE_MANAGEMENT_SLICE: &str = "
    program SymtableManagementSlice;
    type
      TManagementOperator = (moInitialize, moFinalize, moAddRef, moCopy);
      TVariantStarts = array[0..3] of LongInt;
    var
      Mop: TManagementOperator;
      Counter: LongInt;
      VariantStarts: TVariantStarts;
    begin
      for Mop := Low(TManagementOperator) to High(TManagementOperator) do
        Inc(Counter);
      if Counter < Pred(Length(VariantStarts)) then
        Inc(Counter);
    end.
";

// Adapted from fpc-3.2.0/compiler/ptype.pas:1285..1294 and
// 1442..1447. The Byte set ceiling is a required constant, while the
// SizeInt range comparison is an executable expression.
const PTYPE_ORDINAL_BOUND_SLICE: &str = "
    program PTypeOrdinalBounds;
    const
      MaxSetOrdinal = High(Byte);
    type
      TSetIndex = Byte(0)..MaxSetOrdinal;
      TByteSet = set of TSetIndex;
    var
      LowValue, HighValue: SizeInt;
      OutsideTargetRange: Boolean;
      Values: TByteSet;
    begin
      OutsideTargetRange :=
        (LowValue < Low(SizeInt)) or (HighValue > High(SizeInt));
      Values := [Byte(0), MaxSetOrdinal];
    end.
";

fn selected_builtin(expression: &BoundExpression) -> Option<&BuiltinOperation> {
    let BoundExpressionKind::Application { target, .. } = &expression.kind else {
        return None;
    };
    let resolution = match target {
        BoundApplicationTarget::Routine { resolution }
        | BoundApplicationTarget::CallableValue { resolution }
        | BoundApplicationTarget::Builtin { resolution }
        | BoundApplicationTarget::Conversion { resolution, .. }
        | BoundApplicationTarget::Operator { resolution, .. } => resolution,
        BoundApplicationTarget::Invalid => return None,
    };
    let ApplicationCandidate::Builtin {
        instantiation: BuiltinInstantiation::Complete(instance),
        ..
    } = &resolution.selected_attempt()?.candidate
    else {
        return None;
    };
    Some(&instance.operation)
}

fn expression_statement(statement: &BoundStatement) -> &BoundExpression {
    let BoundStatementKind::Expression(expression) = &statement.kind else {
        panic!("expected expression statement")
    };
    expression
}

fn constant_value<'a>(
    compilation: &'a rascal::semantic::SemanticCompilation,
    spelling: &str,
) -> &'a ConstantValue {
    let environment = compilation.files[0].environment;
    let name = compilation.binder.scopes.names().lookup(spelling).unwrap();
    let symbol = compilation
        .binder
        .scopes
        .lookup_symbol(environment, name, LookupRequest::REQUIRED_VALUE)
        .unwrap()
        .primary[0]
        .symbol;
    &compilation.binder.constants.get(symbol).unwrap().value
}

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
    assert!(
        resolution.attempts.iter().any(|attempt| {
            matches!(
                attempt.candidate,
                ApplicationCandidate::Builtin {
                    instantiation: BuiltinInstantiation::Rejected(_),
                    ..
                }
            ) && !attempt.is_viable()
        }),
        "the rejected parameterized System candidate must remain beside the selected custom operator"
    );
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

#[test]
fn constexp_limit_slice_uses_the_same_high_contract_for_constants_and_runtime() {
    let compilation = bind_sources(&[(
        "fpc-3.2.0/compiler/constexp.pas:128..187,300..338",
        CONSTEXP_LIMIT_SLICE,
    )]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    assert_eq!(
        constant_value(&compilation, "signedmaximum"),
        &ConstantValue::Integer(i64::MAX.into())
    );
    assert_eq!(
        constant_value(&compilation, "unsignedspace"),
        &ConstantValue::Integer((u64::MAX - i64::MAX as u64).into())
    );

    let function_body = compilation
        .bodies
        .iter()
        .find(|body| body.owner.is_some())
        .expect("FitsSigned body");
    let BoundStatementKind::Assignment(assignment) = &function_body.statements[0].kind else {
        panic!("expected Result assignment")
    };
    let BoundExpressionKind::Application { operands, .. } = &assignment.source.kind else {
        panic!("expected comparison")
    };
    let BoundExpressionKind::Application {
        operands: cast_operands,
        ..
    } = &operands[1].kind
    else {
        panic!("expected QWord conversion")
    };
    assert!(matches!(
        selected_builtin(&cast_operands[0]),
        Some(BuiltinOperation::Metadata {
            query: MetadataQuery::High,
            ..
        })
    ));
}

#[test]
fn symtable_slice_instantiates_enum_metadata_length_pred_and_mutation_contracts() {
    let compilation = bind_sources(&[(
        "fpc-3.2.0/compiler/symtable.pas:1212..1219,2299..2307",
        SYMTABLE_MANAGEMENT_SLICE,
    )]);
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
    let BoundStatementKind::For {
        initial,
        final_value,
        body: loop_body,
        ..
    } = &body.statements[0].kind
    else {
        panic!("expected enum for loop")
    };
    assert!(matches!(
        selected_builtin(initial),
        Some(BuiltinOperation::Metadata {
            query: MetadataQuery::Low,
            ..
        })
    ));
    assert!(matches!(
        selected_builtin(final_value),
        Some(BuiltinOperation::Metadata {
            query: MetadataQuery::High,
            ..
        })
    ));
    assert!(matches!(
        selected_builtin(expression_statement(loop_body)),
        Some(BuiltinOperation::StepMutation {
            operation: StepOperation::Increment,
            ..
        })
    ));

    let BoundStatementKind::If {
        condition,
        then_branch,
        ..
    } = &body.statements[1].kind
    else {
        panic!("expected Pred(Length(...)) condition")
    };
    let BoundExpressionKind::Application { operands, .. } = &condition.kind else {
        panic!("expected comparison expression")
    };
    let pred = &operands[1];
    assert!(matches!(
        selected_builtin(pred),
        Some(BuiltinOperation::Ordinal {
            operation: OrdinalOperation::Pred,
            ..
        })
    ));
    let BoundExpressionKind::Application {
        operands: pred_operands,
        ..
    } = &pred.kind
    else {
        unreachable!()
    };
    assert!(matches!(
        selected_builtin(&pred_operands[0]),
        Some(BuiltinOperation::Metadata {
            query: MetadataQuery::Length,
            ..
        })
    ));
    assert!(matches!(
        selected_builtin(expression_statement(then_branch)),
        Some(BuiltinOperation::StepMutation {
            operation: StepOperation::Increment,
            ..
        })
    ));
}

#[test]
fn ptype_slice_binds_constant_byte_and_runtime_sizeint_bounds() {
    let compilation = bind_sources(&[(
        "fpc-3.2.0/compiler/ptype.pas:1285..1294,1442..1447",
        PTYPE_ORDINAL_BOUND_SLICE,
    )]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    assert_eq!(
        constant_value(&compilation, "maxsetordinal"),
        &ConstantValue::Integer(u8::MAX.into())
    );
    let set_name = compilation
        .binder
        .scopes
        .names()
        .lookup("tbyteset")
        .unwrap();
    let set_symbol = compilation
        .binder
        .scopes
        .lookup_symbol(
            compilation.files[0].environment,
            set_name,
            LookupRequest::REQUIRED_TYPE,
        )
        .unwrap()
        .primary[0]
        .symbol;
    let SymbolKind::Type(set_type) = compilation.binder.scopes.symbol(set_symbol).kind else {
        panic!("expected set type")
    };
    assert_eq!(
        compilation
            .binder
            .types
            .storage_layout(set_type)
            .unwrap()
            .size,
        32
    );
}
