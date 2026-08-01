use rascal::semantic::{
    BoundApplicationTarget, BoundExpressionKind, BoundStatementKind, ParameterMode, bind_sources,
};

#[test]
fn binds_pointer_forward_and_nested_local_type_in_source_order() {
    let source = "
        program Main;
        procedure Run;
        type
          PNode = ^TNode;
          TNode = record Next: PNode; end;
        var Item: TNode;
        begin
        end;
        begin
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    assert_eq!(compilation.files.len(), 1);
    assert_eq!(compilation.files[0].unsupported_declarations, 0);
}

#[test]
fn rejects_pointer_forward_completed_in_a_later_type_section() {
    let source = "
        program Main;
        procedure Run;
        type
          PNode = ^TNode;
        type
          TNode = record Next: PNode; end;
        begin
        end;
        begin
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("end of type section")),
        "{:#?}",
        compilation.diagnostics
    );
}

#[test]
fn binds_imported_class_parent_and_alias_across_units() {
    let base = "
        unit Base;
        interface
        type
          TBase = class
            Value: LongInt;
          end;
        implementation
        end.
    ";
    let middle = "
        unit Middle;
        interface
        uses Base;
        type
          TDerived = class(TBase)
          end;
        implementation
        end.
    ";
    let aliases = "
        unit Aliases;
        interface
        uses Middle;
        type
          TAlias = TDerived;
        implementation
        end.
    ";
    let main = "
        program Main;
        uses Aliases;
        var Item: TAlias;
        begin
        end.
    ";
    let compilation = bind_sources(&[
        ("base.pp", base),
        ("middle.pp", middle),
        ("aliases.pp", aliases),
        ("main.pp", main),
    ]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    assert_eq!(compilation.files.len(), 4);
}

#[test]
fn explicit_class_forward_completes_the_same_declaration() {
    let source = "
        program Main;
        type
          TNode = class;
        type
          TNode = class
            Next: TNode;
          end;
        var Value: TNode;
        begin
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
}

#[test]
fn routine_formals_results_and_body_expressions_are_semantically_bound() {
    let source = "
        program Main;
        function AddOne(const A: LongInt; var B: LongInt): LongInt; cdecl;
        begin
          Result := A + B;
        end;
        var B: LongInt;
        begin
          B := 2;
          AddOne(1, B);
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let routine_body = compilation
        .bodies
        .iter()
        .find(|body| body.owner.is_some())
        .unwrap();
    let callable = compilation
        .binder
        .types
        .callable(routine_body.owner.unwrap())
        .unwrap();
    assert_eq!(callable.signature.parameters.len(), 2);
    assert_eq!(callable.signature.parameters[0].mode, ParameterMode::Const);
    assert_eq!(callable.signature.parameters[1].mode, ParameterMode::Var);
    assert_eq!(
        callable.signature.calling_convention,
        rascal::semantic::CallingConvention::Cdecl
    );
    assert!(callable.signature.result.is_some());
    assert_eq!(routine_body.statements.len(), 1);
}

#[test]
fn local_type_high_binds_as_a_cast_before_system_high() {
    let source = "
        program Main;
        procedure Run;
        type High = LongInt;
        var X: LongInt;
        begin
          X := High(1);
        end;
        begin
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
        .find(|body| body.owner.is_some())
        .unwrap();
    let BoundStatementKind::Assignment(assignment) = &body.statements[0].kind else {
        panic!("expected assignment")
    };
    let BoundExpressionKind::Application { operands, .. } = &assignment.kind else {
        panic!("expected bound assignment operator")
    };
    let BoundExpressionKind::Application { target, .. } = &operands[1].kind else {
        panic!("expected cast application")
    };
    assert!(matches!(target, BoundApplicationTarget::Conversion { .. }));
}

#[test]
fn nearer_parameter_blocks_outer_type_in_call_shaped_expression() {
    let source = "
        program Main;
        type X = LongInt;
        procedure Run(X: X);
        var Y: X;
        begin
          X(1);
        end;
        begin
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("nearest declaration `x` is not callable")
        }),
        "{:#?}",
        compilation.diagnostics
    );
    let body = compilation
        .bodies
        .iter()
        .find(|body| body.owner.is_some())
        .unwrap();
    let BoundStatementKind::Expression(expression) = &body.statements[0].kind else {
        panic!("expected expression statement")
    };
    let BoundExpressionKind::Application { target, .. } = &expression.kind else {
        panic!("expected application")
    };
    assert_eq!(*target, BoundApplicationTarget::Invalid);
}

#[test]
fn nested_body_binding_records_outer_variable_capture() {
    let source = "
        program Main;
        procedure Outer;
        var X: LongInt;
        procedure Inner;
        begin
          X := 1;
        end;
        begin
        end;
        begin
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let captured = compilation.bodies.iter().any(|body| {
        body.owner
            .and_then(|owner| compilation.binder.types.callable(owner))
            .is_some_and(|callable| !callable.captures.is_empty())
    });
    assert!(captured, "nested routine did not retain its outer capture");
}

#[test]
fn untyped_storage_and_trailing_defaults_are_part_of_call_viability() {
    let source = "
        program Main;
        procedure Touch(const Raw; var Dest: LongInt);
        begin
          Dest := 1;
        end;
        function WithDefault(A: LongInt = 7): LongInt;
        begin
          Result := A;
        end;
        var X: LongInt;
        begin
          Touch(X, X);
          WithDefault();
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let signatures = compilation
        .bodies
        .iter()
        .filter_map(|body| body.owner)
        .filter_map(|owner| compilation.binder.types.callable(owner))
        .map(|callable| &callable.signature)
        .collect::<Vec<_>>();
    assert!(signatures.iter().any(|signature| {
        signature.parameters.len() == 1
            && signature.parameters[0].default == Some(rascal::semantic::ConstantValue::Integer(7))
    }));
}

#[test]
fn typed_var_formal_rejects_a_literal_actual() {
    let source = "
        program Main;
        procedure Touch(var Dest: LongInt);
        begin
        end;
        begin
          Touch(1);
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
}
