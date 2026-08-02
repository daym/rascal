use rascal::semantic::{
    ArgumentConversion, BoundApplicationTarget, BoundExpressionKind, BoundStatementKind,
    ConversionCandidate, ConversionRejection, ConversionSelection, ExplicitConversion,
    ExpressionCategory, ResolvedConversion, SemanticUse, ValueConversionOperation, bind_sources,
};

fn program_body(
    compilation: &rascal::semantic::SemanticCompilation,
) -> &rascal::semantic::BoundBody {
    compilation
        .bodies
        .iter()
        .find(|body| body.owner.is_none())
        .expect("program body")
}

#[test]
fn assignment_uses_custom_implicit_conversion_but_remains_a_store() {
    let source = "
        program CustomImplicit;
        type
          TBox = record Value: LongInt; end;
        operator :=(N: LongInt): TBox; forward;
        var Box: TBox;
        begin
          Box := 1;
        end.
    ";
    let compilation = bind_sources(&[("custom_implicit.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let BoundStatementKind::Assignment(assignment) = &program_body(&compilation).statements[0].kind
    else {
        panic!("expected a bound store")
    };
    assert_eq!(
        assignment.target.category,
        ExpressionCategory::Storage { mutable: true }
    );
    assert_eq!(
        assignment.target.semantic_use,
        SemanticUse::AssignmentTarget
    );
    let resolution = assignment.conversion.as_ref().unwrap();
    let ResolvedConversion::Implicit(conversion) = resolution.selected().unwrap() else {
        panic!("expected implicit conversion")
    };
    assert!(matches!(
        conversion.operation,
        ValueConversionOperation::CustomOperator { .. }
    ));
}

#[test]
fn explicit_cast_uses_the_canonical_explicit_conversion_declaration() {
    let source = "
        program CustomExplicit;
        type
          TBox = record Value: LongInt; end;
        operator Explicit(N: LongInt): TBox; forward;
        var Box: TBox;
        begin
          Box := TBox(1);
        end.
    ";
    let compilation = bind_sources(&[("custom_explicit.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let BoundStatementKind::Assignment(assignment) = &program_body(&compilation).statements[0].kind
    else {
        panic!("expected assignment")
    };
    let BoundExpressionKind::Application {
        target: BoundApplicationTarget::Conversion { resolution, .. },
        ..
    } = &assignment.source.kind
    else {
        panic!("expected explicit cast")
    };
    let ArgumentConversion::Explicit(conversion) = resolution.attempts[0].arguments[0]
        .conversion
        .as_ref()
        .unwrap()
    else {
        panic!("expected explicit argument conversion")
    };
    assert!(matches!(
        conversion.selected(),
        Some(ResolvedConversion::Explicit(
            ExplicitConversion::CustomOperator { .. }
        ))
    ));
}

#[test]
fn predefined_conversion_ranks_before_custom_implicit_conversion() {
    let source = "
        program StandardWins;
        operator :=(N: Integer): Int64; forward;
        var Wide: Int64;
        begin
          Wide := 1;
        end.
    ";
    let compilation = bind_sources(&[("standard_wins.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let BoundStatementKind::Assignment(assignment) = &program_body(&compilation).statements[0].kind
    else {
        panic!("expected assignment")
    };
    assert!(matches!(
        assignment
            .conversion
            .as_ref()
            .unwrap()
            .selected_attempt()
            .unwrap()
            .candidate,
        ConversionCandidate::Predefined
    ));
}

#[test]
fn custom_conversion_input_does_not_recursively_use_another_custom_conversion() {
    let source = "
        program NoConversionChains;
        type
          TFirst = record Value: LongInt; end;
          TSecond = record Value: LongInt; end;
        operator :=(N: LongInt): TFirst; forward;
        operator :=(Value: TFirst): TSecond; forward;
        var ResultValue: TSecond;
        begin
          ResultValue := 1;
        end.
    ";
    let compilation = bind_sources(&[("no_conversion_chains.pp", source)]);
    assert!(
        compilation.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("assignment source is not convertible")),
        "{:#?}",
        compilation.diagnostics
    );
    let BoundStatementKind::Assignment(assignment) = &program_body(&compilation).statements[0].kind
    else {
        panic!("expected assignment")
    };
    let resolution = assignment.conversion.as_ref().unwrap();
    assert!(matches!(
        resolution.selection,
        ConversionSelection::NoViable
    ));
    assert!(resolution.attempts.iter().any(|attempt| {
        matches!(
            attempt.rejections.as_slice(),
            [ConversionRejection::NoPredefinedInputConversion { .. }]
        )
    }));
}

#[test]
fn properties_conditions_parameters_and_inline_initializers_use_the_same_resolver() {
    let source = "
        program TargetUses;
        type
          TBox = record Value: LongInt; end;
          THolder = class
            property Item: TBox read GetItem write SetItem;
          end;
        operator :=(N: LongInt): TBox; forward;
        operator :=(Value: TBox): Boolean; forward;
        procedure Consume(Value: TBox); forward;
        var Holder: THolder;
        begin
          Holder.Item := 1;
          Consume(1);
          if Holder.Item then
            var Local: TBox := 2;
        end.
    ";
    let compilation = bind_sources(&[("target_uses.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let body = program_body(&compilation);
    let BoundStatementKind::Assignment(property_write) = &body.statements[0].kind else {
        panic!("expected property write")
    };
    assert!(matches!(
        property_write.target.category,
        ExpressionCategory::Property { writable: true, .. }
    ));
    assert!(
        property_write
            .conversion
            .as_ref()
            .unwrap()
            .selected()
            .is_some()
    );

    let BoundStatementKind::Expression(call) = &body.statements[1].kind else {
        panic!("expected call")
    };
    let BoundExpressionKind::Application {
        target: BoundApplicationTarget::Routine { resolution },
        ..
    } = &call.kind
    else {
        panic!("expected routine call")
    };
    assert!(matches!(
        resolution.attempts[0].arguments[0].conversion,
        Some(ArgumentConversion::Implicit(_))
    ));

    let BoundStatementKind::If {
        condition,
        then_branch,
        ..
    } = &body.statements[2].kind
    else {
        panic!("expected condition")
    };
    assert_eq!(condition.semantic_use, SemanticUse::Condition);
    assert!(condition.conversion.as_ref().unwrap().selected().is_some());
    let BoundStatementKind::InlineVariable { initializer, .. } = &then_branch.kind else {
        panic!("expected inline variable")
    };
    assert!(
        initializer
            .as_ref()
            .unwrap()
            .conversion
            .as_ref()
            .unwrap()
            .selected()
            .is_some()
    );
}

#[test]
fn range_mode_selects_checked_or_unchecked_implicit_identifier() {
    let source = "
        program ConversionModes;
        type
          TChecked = record Value: LongInt; end;
          TUnchecked = record Value: LongInt; end;
        {$R+}
        operator :=(N: LongInt): TChecked; forward;
        {$R-}
        operator :=(N: LongInt): TUnchecked; forward;
        var Checked: TChecked;
        var Unchecked: TUnchecked;
        begin
          {$R+}
          Checked := 1;
          {$R-}
          Unchecked := 2;
        end.
    ";
    let compilation = bind_sources(&[("conversion_modes.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let body = program_body(&compilation);
    for (statement, identifier) in [
        (&body.statements[0], "&op_CheckedImplicit"),
        (&body.statements[1], "&op_Implicit"),
    ] {
        let BoundStatementKind::Assignment(assignment) = &statement.kind else {
            panic!("expected assignment")
        };
        let symbol = match assignment.conversion.as_ref().unwrap().selected_attempt() {
            Some(attempt) => match attempt.candidate {
                ConversionCandidate::Custom { symbol, .. } => symbol,
                ConversionCandidate::Predefined => panic!("expected custom conversion"),
            },
            None => panic!("expected selected conversion"),
        };
        assert_eq!(
            compilation.binder.scopes.symbol(symbol).name,
            compilation
                .binder
                .scopes
                .names()
                .lookup(identifier)
                .unwrap()
        );
    }
}

#[test]
fn custom_conversion_is_not_executed_to_manufacture_a_constant_default() {
    let source = "
        program ConstantDefault;
        type TBox = record Value: LongInt; end;
        operator :=(N: LongInt): TBox; forward;
        procedure Consume(Value: TBox = 1); forward;
        begin
        end.
    ";
    let compilation = bind_sources(&[("constant_default.pp", source)]);
    assert!(
        compilation.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("constant expression: NotConstant")
        }),
        "{:#?}",
        compilation.diagnostics
    );
}

#[test]
fn property_read_and_write_capabilities_control_semantic_use() {
    let source = "
        program PropertyCapabilities;
        type
          THolder = class
            property ReadOnly: LongInt read GetValue;
            property WriteOnly: LongInt write SetValue;
          end;
        var Holder: THolder;
        var Value: LongInt;
        begin
          Holder.ReadOnly := 1;
          Value := Holder.WriteOnly;
        end.
    ";
    let compilation = bind_sources(&[("property_capabilities.pp", source)]);
    assert!(
        compilation
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("not writable")),
        "{:#?}",
        compilation.diagnostics
    );
    assert!(
        compilation
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("readable value")),
        "{:#?}",
        compilation.diagnostics
    );
}
