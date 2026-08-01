use rascal::{
    Application, Callee, Expr, ExprKind, ModeSnapshot, Operator, Statement, chumsky_parser,
    nom_parser,
};

fn parse_both(source: &str) -> (rascal::ParseOutput, rascal::ParseOutput) {
    let nom = nom_parser::parse(source);
    let chumsky = chumsky_parser::parse(source);
    assert_eq!(
        nom.statements, chumsky.statements,
        "parser ASTs differ for `{source}`\nnom: {nom:#?}\nchumsky: {chumsky:#?}"
    );
    (nom, chumsky)
}

fn only_expression(output: &rascal::ParseOutput) -> &Expr {
    assert!(
        output.diagnostics.is_empty(),
        "unexpected diagnostics: {:#?}",
        output.diagnostics
    );
    match output.statements.as_slice() {
        [Statement::Expression(expression)] => expression,
        statements => panic!("expected one expression, got {statements:#?}"),
    }
}

fn application(expression: &Expr) -> &Application {
    match &expression.kind {
        ExprKind::Application(application) => application,
        kind => panic!("expected an application, got {kind:#?}"),
    }
}

fn identifier(expression: &Expr) -> &str {
    match &expression.kind {
        ExprKind::Identifier(name) => name,
        kind => panic!("expected an identifier, got {kind:#?}"),
    }
}

#[test]
fn call_operator_corpus_produces_identical_asts() {
    let corpus = [
        "FillChar(Buffer, 0, SizeOf(Buffer))",
        "T(x)",
        "Factory()()",
        "Obj.Handler()",
        "P^.Items[I]",
        "@Routine",
        "@ProcVar",
        "@@ProcVar",
        "High(SmallInt) - Low(SmallInt)",
        "A + B * -C",
        "[1, 3..5] + OtherSet",
        "X := Left() + Right()",
        "inherited Create(A)",
        "Maker().Handlers[I]^(A)",
        "A div B mod C shl D",
        "A = B",
        "A in [B, C]",
        "A is TObject",
        "A as TObject",
    ];

    for source in corpus {
        let (nom, chumsky) = parse_both(source);
        assert!(
            nom.diagnostics.is_empty(),
            "nom rejected `{source}`: {:#?}",
            nom.diagnostics
        );
        assert!(
            chumsky.diagnostics.is_empty(),
            "chumsky rejected `{source}`: {:#?}",
            chumsky.diagnostics
        );
    }
}

#[test]
fn ambiguous_t_of_x_stays_an_unresolved_call_shaped_application() {
    let (nom, _) = parse_both("T(x)");
    let call = application(only_expression(&nom));
    assert_eq!(call.operands.len(), 1);
    assert_eq!(identifier(&call.operands[0]), "x");
    match &call.callee {
        Callee::Expression(callee) => assert_eq!(identifier(callee), "t"),
        other => panic!("the parser classified T as a semantic operator: {other:#?}"),
    }
}

#[test]
fn every_plan_operator_uses_the_common_application_node() {
    let source = "
        +A; -A; not A; @A; @@A;
        A * B; A / B; A div B; A mod B; A and B; A shl B; A shr B;
        A + B; A - B; A or B; A xor B;
        A = B; A <> B; A < B; A > B; A <= B; A >= B;
        A in B; A is B; A as B; A := B
    ";
    let expected = [
        Operator::Positive,
        Operator::Negative,
        Operator::Not,
        Operator::Address,
        Operator::ProcedureSlotAddress,
        Operator::Multiply,
        Operator::RealDivide,
        Operator::IntegerDivide,
        Operator::Modulo,
        Operator::And,
        Operator::ShiftLeft,
        Operator::ShiftRight,
        Operator::Add,
        Operator::Subtract,
        Operator::Or,
        Operator::Xor,
        Operator::Equal,
        Operator::NotEqual,
        Operator::Less,
        Operator::Greater,
        Operator::LessEqual,
        Operator::GreaterEqual,
        Operator::In,
        Operator::Is,
        Operator::As,
        Operator::Assign,
    ];

    let (nom, chumsky) = parse_both(source);
    assert!(nom.diagnostics.is_empty(), "{:#?}", nom.diagnostics);
    assert!(chumsky.diagnostics.is_empty(), "{:#?}", chumsky.diagnostics);
    assert_eq!(nom.statements.len(), expected.len());
    for (statement, expected_operator) in nom.statements.iter().zip(expected) {
        let application = match statement {
            Statement::Expression(expression) => application(expression),
            Statement::Assignment(application) => application,
            Statement::Error(span) => panic!("operator failed to parse at {span:?}"),
        };
        assert_eq!(
            application.callee,
            Callee::Operator(expected_operator),
            "wrong application for `{}`",
            expected_operator.spelling()
        );
    }
}

#[test]
fn bare_and_explicit_zero_argument_forms_remain_distinct_syntax() {
    let (bare, _) = parse_both("Routine");
    assert_eq!(identifier(only_expression(&bare)), "routine");

    let (explicit, _) = parse_both("Routine()");
    let call = application(only_expression(&explicit));
    assert!(call.operands.is_empty());
    match &call.callee {
        Callee::Expression(callee) => assert_eq!(identifier(callee), "routine"),
        other => panic!("expected an explicit value call, got {other:#?}"),
    }
}

#[test]
fn precedence_and_source_order_are_explicit_in_the_shared_application_ast() {
    let (nom, _) = parse_both("Left() + Middle() * -Right()");
    let add = application(only_expression(&nom));
    assert_eq!(add.callee, Callee::Operator(Operator::Add));
    assert_eq!(add.operands.len(), 2);

    let left_call = application(&add.operands[0]);
    match &left_call.callee {
        Callee::Expression(callee) => assert_eq!(identifier(callee), "left"),
        other => panic!("expected value call, got {other:#?}"),
    }

    let multiply = application(&add.operands[1]);
    assert_eq!(multiply.callee, Callee::Operator(Operator::Multiply));
    match &application(&multiply.operands[0]).callee {
        Callee::Expression(callee) => assert_eq!(identifier(callee), "middle"),
        other => panic!("expected value call, got {other:#?}"),
    }
    let negative = application(&multiply.operands[1]);
    assert_eq!(negative.callee, Callee::Operator(Operator::Negative));
    match &application(&negative.operands[0]).callee {
        Callee::Expression(callee) => assert_eq!(identifier(callee), "right"),
        other => panic!("expected value call, got {other:#?}"),
    }
}

#[test]
fn directives_are_snapshotted_at_the_plan_defined_syntax_points() {
    let (nom, _) = parse_both("{$V-}F(S)");
    assert!(!application(only_expression(&nom)).modes.var_string_checks);

    let (nom, _) = parse_both("A[I {$R+}]");
    match &only_expression(&nom).kind {
        ExprKind::Index { range_checks, .. } => assert!(*range_checks),
        kind => panic!("expected index, got {kind:#?}"),
    }

    let (nom, _) = parse_both("A {$Q+}+ B");
    assert!(application(only_expression(&nom)).modes.overflow_checks);

    let (nom, _) = parse_both("A {$B+}and B");
    assert!(
        application(only_expression(&nom))
            .modes
            .complete_boolean_eval
    );

    let (nom, _) = parse_both("{$I-}ReadLn(F)");
    assert!(!application(only_expression(&nom)).modes.io_checks);

    assert_eq!(
        ModeSnapshot::default(),
        ModeSnapshot {
            var_string_checks: true,
            range_checks: false,
            overflow_checks: false,
            io_checks: true,
            complete_boolean_eval: false,
        }
    );
}

#[test]
fn semicolon_synchronization_preserves_following_statements() {
    let source = "Broken(1, ; Good(2); A + ; Final(3)";
    let (nom, chumsky) = parse_both(source);
    assert_eq!(nom.statements.len(), 4);
    assert_eq!(nom.diagnostics.len(), 2);
    assert_eq!(chumsky.diagnostics.len(), 2);
    assert!(matches!(nom.statements[0], Statement::Error(_)));
    assert!(matches!(nom.statements[2], Statement::Error(_)));

    let Statement::Expression(good) = &nom.statements[1] else {
        panic!("the statement after the first error was not recovered");
    };
    match &application(good).callee {
        Callee::Expression(callee) => assert_eq!(identifier(callee), "good"),
        other => panic!("expected Good call, got {other:#?}"),
    }

    let Statement::Expression(final_call) = &nom.statements[3] else {
        panic!("the final valid statement was not parsed");
    };
    match &application(final_call).callee {
        Callee::Expression(callee) => assert_eq!(identifier(callee), "final"),
        other => panic!("expected Final call, got {other:#?}"),
    }
}
