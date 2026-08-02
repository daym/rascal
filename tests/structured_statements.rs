use rascal::{
    Statement, chumsky_parser,
    semantic::{BoundStatement, BoundStatementKind, bind_sources},
};

fn contains_bound_kind(
    statement: &BoundStatement,
    predicate: &impl Fn(&BoundStatementKind) -> bool,
) -> bool {
    if predicate(&statement.kind) {
        return true;
    }
    match &statement.kind {
        BoundStatementKind::Compound(statements)
        | BoundStatementKind::Repeat {
            body: statements, ..
        } => statements
            .iter()
            .any(|statement| contains_bound_kind(statement, predicate)),
        BoundStatementKind::If {
            then_branch,
            else_branch,
            ..
        } => {
            contains_bound_kind(then_branch, predicate)
                || else_branch
                    .as_deref()
                    .is_some_and(|branch| contains_bound_kind(branch, predicate))
        }
        BoundStatementKind::While { body, .. }
        | BoundStatementKind::For { body, .. }
        | BoundStatementKind::ForIn { body, .. }
        | BoundStatementKind::With { body, .. }
        | BoundStatementKind::Label {
            statement: body, ..
        } => contains_bound_kind(body, predicate),
        BoundStatementKind::Case {
            arms, otherwise, ..
        } => {
            arms.iter()
                .any(|arm| contains_bound_kind(&arm.statement, predicate))
                || otherwise
                    .iter()
                    .any(|statement| contains_bound_kind(statement, predicate))
        }
        BoundStatementKind::Try { body, continuation } => {
            body.iter()
                .any(|statement| contains_bound_kind(statement, predicate))
                || match continuation {
                    rascal::semantic::BoundTryContinuation::Finally(statements) => statements
                        .iter()
                        .any(|statement| contains_bound_kind(statement, predicate)),
                    rascal::semantic::BoundTryContinuation::Except {
                        handlers,
                        otherwise,
                    } => {
                        handlers
                            .iter()
                            .any(|handler| contains_bound_kind(&handler.body, predicate))
                            || otherwise
                                .iter()
                                .any(|statement| contains_bound_kind(statement, predicate))
                    }
                }
        }
        BoundStatementKind::Expression(_)
        | BoundStatementKind::Assignment(_)
        | BoundStatementKind::Raise { .. }
        | BoundStatementKind::Goto { .. }
        | BoundStatementKind::Break
        | BoundStatementKind::Continue
        | BoundStatementKind::Exit(_)
        | BoundStatementKind::InlineVariable { .. }
        | BoundStatementKind::Empty
        | BoundStatementKind::Error => false,
    }
}

#[test]
fn recursive_chumsky_parser_builds_structured_statement_nodes() {
    let source = "
        begin
          if True then X := 1 else X := 2;
          while True do begin X := X + 1; continue; break end;
          repeat X := X - 1 until X = 0;
          for X := 1 to 3 do X := X + 1;
          for X in A do X := X + 1;
          case X of
            0: X := 1;
            1..3: X := 2;
          else
            X := 4;
          end;
          with R do Value := X;
          try X := 1 finally X := 2 end;
          raise E at Address, Frame;
          goto Done;
          Done: ;
          exit(1);
        end
    ";
    let parsed = chumsky_parser::parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    assert_eq!(parsed.statements.len(), 1);
    let Statement::Compound { statements, .. } = &parsed.statements[0] else {
        panic!("expected compound statement")
    };
    assert_eq!(statements.len(), 12);
    assert!(matches!(statements[0], Statement::If { .. }));
    assert!(matches!(statements[4], Statement::ForIn { .. }));
    assert!(matches!(statements[5], Statement::Case { .. }));
    assert!(matches!(statements[7], Statement::Try { .. }));
    assert!(matches!(statements[8], Statement::Raise { .. }));
    assert!(matches!(statements[10], Statement::Label { .. }));
    assert!(matches!(statements[11], Statement::Exit { .. }));
}

#[test]
fn structured_statements_bind_recursively_with_scopes_and_types() {
    let source = "
        program Main;
        type
          TRec = record Value: LongInt; end;
          TError = class end;
          TArray = array of LongInt;
        label Done;
        function Early: LongInt;
        begin
          Exit(1);
        end;
        var I, X: LongInt;
        var R: TRec;
        var A: TArray;
        begin
          if True then X := 1 else X := 2;
          while X < 3 do begin
            X := X + 1;
            if X = 2 then Continue;
            if X = 3 then Break;
          end;
          repeat X := X - 1 until X = 0;
          for I := 1 to 3 do X := I;
          for I in A do X := I;
          case X of
            0: X := 1;
            1..3: X := 2;
          else
            X := 4;
          end;
          with R do Value := X;
          try X := 1 finally X := 2 end;
          try
            X := 1
          except
            on E: TError do X := 2;
          else
            X := 3;
          end;
          goto Done;
          Done: ;
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
    for predicate in [
        |kind: &BoundStatementKind| matches!(kind, BoundStatementKind::If { .. }),
        |kind: &BoundStatementKind| matches!(kind, BoundStatementKind::ForIn { .. }),
        |kind: &BoundStatementKind| matches!(kind, BoundStatementKind::Case { .. }),
        |kind: &BoundStatementKind| matches!(kind, BoundStatementKind::With { .. }),
        |kind: &BoundStatementKind| matches!(kind, BoundStatementKind::Try { .. }),
        |kind: &BoundStatementKind| matches!(kind, BoundStatementKind::Goto { .. }),
    ] {
        assert!(
            body.statements
                .iter()
                .any(|statement| contains_bound_kind(statement, &predicate))
        );
    }
    let with_receiver_is_retained = body.statements.iter().any(|statement| {
        let BoundStatementKind::With {
            body: with_body, ..
        } = &statement.kind
        else {
            return false;
        };
        let BoundStatementKind::Assignment(assignment) = &with_body.kind else {
            return false;
        };
        matches!(
            &assignment.target.kind,
            rascal::semantic::BoundExpressionKind::Symbol {
                receiver: Some(_),
                ..
            }
        )
    });
    assert!(
        with_receiver_is_retained,
        "with-bound field lost its receiver provenance"
    );
    assert!(compilation.bodies.iter().any(|body| {
        body.statements.iter().any(|statement| {
            contains_bound_kind(statement, &|kind| {
                matches!(kind, BoundStatementKind::Exit(_))
            })
        })
    }));
}

#[test]
fn inline_variables_follow_source_order_and_nested_block_scope() {
    let source = "
        program Main;
        var ResultValue: LongInt;
        begin
          var Outer := 1;
          ResultValue := Outer;
          begin
            var Inner: LongInt := Outer;
            ResultValue := Inner;
            var Inner: LongInt := Inner + 2;
            ResultValue := Inner;
          end;
          ResultValue := Outer;
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
        .expect("program body");
    let inner_block = body
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            BoundStatementKind::Compound(statements)
                if statements.len() == 4
                    && matches!(
                        statements[0].kind,
                        BoundStatementKind::InlineVariable { .. }
                    )
                    && matches!(
                        statements[2].kind,
                        BoundStatementKind::InlineVariable { .. }
                    ) =>
            {
                Some(statements)
            }
            _ => None,
        })
        .expect("nested block containing both Inner declarations");
    let BoundStatementKind::InlineVariable {
        symbols: first_symbols,
        ..
    } = &inner_block[0].kind
    else {
        unreachable!()
    };
    let BoundStatementKind::InlineVariable {
        symbols: second_symbols,
        initializer: Some(second_initializer),
        ..
    } = &inner_block[2].kind
    else {
        panic!("expected the second initialized Inner declaration")
    };
    let first_inner = first_symbols[0];
    let second_inner = second_symbols[0];
    assert_ne!(
        first_inner, second_inner,
        "the redeclaration must create a fresh symbol"
    );
    let rascal::semantic::BoundExpressionKind::Application { operands, .. } =
        &second_initializer.kind
    else {
        panic!("expected `Inner + 2` to bind as an operator application")
    };
    assert!(matches!(
        operands.first().map(|operand| &operand.kind),
        Some(rascal::semantic::BoundExpressionKind::Symbol {
            symbol,
            receiver: None,
        }) if *symbol == first_inner
    ));
    let BoundStatementKind::Assignment(assignment) = &inner_block[3].kind else {
        panic!("expected assignment after the redeclaration")
    };
    assert!(matches!(
        &assignment.source.kind,
        rascal::semantic::BoundExpressionKind::Symbol {
            symbol,
            receiver: None,
        } if *symbol == second_inner
    ));

    let leaking = "
        program Main;
        var ResultValue: LongInt;
        begin
          if True then var BranchValue := 1;
          ResultValue := BranchValue;
        end.
    ";
    let compilation = bind_sources(&[("main.pp", leaking)]);
    assert!(
        compilation.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("unknown identifier `branchvalue`")),
        "{:#?}",
        compilation.diagnostics
    );
}
