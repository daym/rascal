use rascal::semantic::{
    ApplicationReceiver, ApplicationSelection, ArgumentConversion, BoundApplicationTarget,
    BoundExpressionKind, BoundStatementKind, CandidateRejection, bind_sources,
};

fn expression_target(kind: &BoundStatementKind) -> &BoundApplicationTarget {
    let BoundStatementKind::Expression(expression) = kind else {
        panic!("expected expression statement")
    };
    let BoundExpressionKind::Application { target, .. } = &expression.kind else {
        panic!("expected application")
    };
    target
}

#[test]
fn default_only_overload_difference_is_ambiguous_and_retains_every_attempt() {
    let source = "
        program Main;
        procedure Pick(A: LongInt); overload; forward;
        procedure Pick(A: LongInt; B: LongInt = 0); overload; forward;
        begin
          Pick(1);
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("ambiguous overload for `pick`")),
        "{:#?}",
        compilation.diagnostics
    );
    let body = compilation
        .bodies
        .iter()
        .find(|body| body.owner.is_none())
        .unwrap();
    let BoundApplicationTarget::Routine { resolution } =
        expression_target(&body.statements[0].kind)
    else {
        panic!("expected routine resolution")
    };
    assert!(matches!(
        resolution.selection,
        ApplicationSelection::Ambiguous { .. }
    ));
    assert_eq!(resolution.attempts.len(), 2);
    assert!(
        resolution
            .attempts
            .iter()
            .all(|attempt| attempt.is_viable())
    );
    assert_eq!(resolution.attempts[0].defaults.len(), 0);
    assert_eq!(resolution.attempts[1].defaults.len(), 1);
    assert_eq!(
        resolution.attempts[1].defaults[0].value,
        rascal::semantic::ConstantValue::Integer(0)
    );
}

#[test]
fn pareto_ranking_rejects_cross_argument_tradeoffs_but_selects_a_dominator() {
    let ambiguous = "
        program Main;
        procedure Pick(A: LongInt; B: Int64); overload; forward;
        procedure Pick(A: Int64; B: LongInt); overload; forward;
        var X, Y: LongInt;
        begin
          Pick(X, Y);
        end.
    ";
    let compilation = bind_sources(&[("main.pp", ambiguous)]);
    let body = compilation
        .bodies
        .iter()
        .find(|body| body.owner.is_none())
        .unwrap();
    let BoundApplicationTarget::Routine { resolution } =
        expression_target(&body.statements[0].kind)
    else {
        panic!("expected routine resolution")
    };
    assert!(matches!(
        resolution.selection,
        ApplicationSelection::Ambiguous { .. }
    ));

    let dominated = "
        program Main;
        procedure Pick(A: LongInt; B: Int64); overload; forward;
        procedure Pick(A: Int64; B: Int64); overload; forward;
        var X, Y: LongInt;
        begin
          Pick(X, Y);
        end.
    ";
    let compilation = bind_sources(&[("main.pp", dominated)]);
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
    let BoundApplicationTarget::Routine { resolution } =
        expression_target(&body.statements[0].kind)
    else {
        panic!("expected routine resolution")
    };
    assert!(matches!(
        resolution.selection,
        ApplicationSelection::Selected { .. }
    ));
}

#[test]
fn methods_procedural_values_properties_operators_and_conversions_share_resolution_records() {
    let source = "
        program Main;
        type
          TProc = procedure(A: LongInt);
          TThing = class
            procedure Touch(A: LongInt);
            property Callback: TProc read GetCallback;
          end;
        var Thing: TThing;
        var P: TProc;
        var X: LongInt;
        begin
          Thing.Touch(X);
          P(X);
          Thing.Callback(X);
          X := LongInt(X + 1);
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
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

    let BoundApplicationTarget::Routine { resolution } =
        expression_target(&body.statements[0].kind)
    else {
        panic!("expected explicit method call")
    };
    assert_eq!(
        resolution.selected_attempt().unwrap().candidate.receiver(),
        ApplicationReceiver::Explicit
    );

    for statement in &body.statements[1..=2] {
        let BoundApplicationTarget::CallableValue { resolution } =
            expression_target(&statement.kind)
        else {
            panic!("expected procedural-value call")
        };
        assert!(matches!(
            resolution.selection,
            ApplicationSelection::Selected { .. }
        ));
        assert!(matches!(
            resolution.selected_attempt().unwrap().candidate.receiver(),
            ApplicationReceiver::CallableValue { .. }
        ));
    }

    let BoundStatementKind::Assignment(assignment) = &body.statements[3].kind else {
        panic!("expected assignment")
    };
    assert!(assignment.conversion.as_ref().unwrap().selected().is_some());
    let BoundExpressionKind::Application {
        target:
            BoundApplicationTarget::Conversion {
                resolution: conversion,
                ..
            },
        operands: converted,
        ..
    } = &assignment.source.kind
    else {
        panic!("expected direct conversion resolution")
    };
    assert!(matches!(
        conversion.attempts[0].arguments[0].conversion,
        Some(ArgumentConversion::Explicit(_))
    ));
    let BoundExpressionKind::Application {
        target: BoundApplicationTarget::Operator {
            resolution: add, ..
        },
        ..
    } = &converted[0].kind
    else {
        panic!("expected nested operator resolution")
    };
    assert!(add.selected_attempt().is_some());
}

#[test]
fn procedural_value_arity_failure_retains_the_rejection_reason() {
    let source = "
        program Main;
        type TProc = procedure(A: LongInt);
        var P: TProc;
        begin
          P();
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    let body = compilation
        .bodies
        .iter()
        .find(|body| body.owner.is_none())
        .unwrap();
    let BoundApplicationTarget::CallableValue { resolution } =
        expression_target(&body.statements[0].kind)
    else {
        panic!("expected procedural-value resolution")
    };
    assert!(matches!(
        resolution.selection,
        ApplicationSelection::NoViable
    ));
    assert!(matches!(
        resolution.attempts[0].rejections.as_slice(),
        [CandidateRejection::Arity {
            provided: 0,
            minimum: 1,
            maximum: 1,
        }]
    ));
}

#[test]
fn method_resolution_retains_a_with_receiver() {
    let source = "
        program Main;
        type
          TThing = class
            procedure Touch(A: LongInt);
          end;
        var Thing: TThing;
        var X: LongInt;
        begin
          with Thing do Touch(X);
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
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
    let BoundStatementKind::With { body, .. } = &body.statements[0].kind else {
        panic!("expected with statement")
    };
    let BoundApplicationTarget::Routine { resolution } = expression_target(&body.kind) else {
        panic!("expected method resolution")
    };
    assert!(matches!(
        resolution.selected_attempt().unwrap().candidate.receiver(),
        ApplicationReceiver::Lookup(_)
    ));
}

#[test]
fn a_property_is_not_storage_for_a_typed_var_formal() {
    let source = "
        program Main;
        type
          TThing = class
            property Value: LongInt read GetValue write SetValue;
          end;
        procedure Touch(var Destination: LongInt); forward;
        var Thing: TThing;
        begin
          Touch(Thing.Value);
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("no viable overload for `touch`")),
        "{:#?}",
        compilation.diagnostics
    );
    let body = compilation
        .bodies
        .iter()
        .find(|body| body.owner.is_none())
        .unwrap();
    let BoundApplicationTarget::Routine { resolution } =
        expression_target(&body.statements[0].kind)
    else {
        panic!("expected routine resolution")
    };
    assert!(matches!(
        resolution.attempts[0].rejections.as_slice(),
        [CandidateRejection::ArgumentNotAddressable { .. }]
    ));
}
