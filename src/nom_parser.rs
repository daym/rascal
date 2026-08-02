use nom::{
    Err as NomErr, IResult, Parser,
    combinator::{map, map_opt, opt, verify},
    error::{Error, ErrorKind},
    sequence::preceded,
};

use crate::{
    ast::{
        Application, ApplicationSyntax, Diagnostic, Expr, ExprKind, Literal, Operator, ParseOutput,
        SetElement, Span, Statement,
    },
    lexer::{Token, TokenKind, lex},
};

type Input<'a> = &'a [Token];
type NomResult<'a, T> = IResult<Input<'a>, T>;

fn failure<T>(input: Input<'_>, kind: ErrorKind) -> NomResult<'_, T> {
    Err(NomErr::Error(Error::new(input, kind)))
}

fn split_first(input: Input<'_>) -> NomResult<'_, Token> {
    match input.split_first() {
        Some((token, rest)) => Ok((rest, token.clone())),
        None => failure(input, ErrorKind::Eof),
    }
}

fn symbol<'a>(input: Input<'a>, expected: &TokenKind) -> NomResult<'a, Token> {
    verify(split_first, |token: &Token| token.kind == *expected).parse(input)
}

fn is_reserved_identifier(name: &str) -> bool {
    matches!(
        name,
        "and"
            | "as"
            | "div"
            | "false"
            | "in"
            | "inherited"
            | "is"
            | "mod"
            | "nil"
            | "not"
            | "or"
            | "shl"
            | "shr"
            | "true"
            | "xor"
    )
}

fn identifier(input: Input<'_>) -> NomResult<'_, (String, Token)> {
    map_opt(split_first, |token| match &token.kind {
        TokenKind::Identifier(name) if !is_reserved_identifier(name) => Some((name.clone(), token)),
        _ => None,
    })
    .parse(input)
}

fn parse_primary(input: Input<'_>) -> NomResult<'_, Expr> {
    let Some((token, rest)) = input.split_first() else {
        return failure(input, ErrorKind::Eof);
    };

    match &token.kind {
        TokenKind::Identifier(name) if name == "true" || name == "false" => Ok((
            rest,
            Expr::new(
                ExprKind::Literal(Literal::Boolean(name == "true")),
                token.span.clone(),
            ),
        )),
        TokenKind::Identifier(name) if name == "nil" => Ok((
            rest,
            Expr::new(ExprKind::Literal(Literal::Nil), token.span.clone()),
        )),
        TokenKind::Identifier(name) if name == "inherited" => {
            let (rest, inherited_name, end) = match identifier(rest) {
                Ok((remaining, (name, name_token))) => (remaining, Some(name), name_token.span.end),
                Err(_) => (rest, None, token.span.end),
            };
            Ok((
                rest,
                Expr::new(ExprKind::Inherited(inherited_name), token.span.start..end),
            ))
        }
        TokenKind::Identifier(name) if !is_reserved_identifier(name) => Ok((
            rest,
            Expr::new(ExprKind::Identifier(name.clone()), token.span.clone()),
        )),
        TokenKind::Integer(value) => Ok((
            rest,
            Expr::new(
                ExprKind::Literal(Literal::Integer(*value)),
                token.span.clone(),
            ),
        )),
        TokenKind::Real(value) => Ok((
            rest,
            Expr::new(
                ExprKind::Literal(Literal::Real(value.clone())),
                token.span.clone(),
            ),
        )),
        TokenKind::String(value) => Ok((
            rest,
            Expr::new(
                ExprKind::Literal(Literal::String(value.clone())),
                token.span.clone(),
            ),
        )),
        TokenKind::LeftParen => {
            let (rest, mut expression) = parse_expression(rest)?;
            let (rest, close) = symbol(rest, &TokenKind::RightParen)?;
            expression.span = token.span.start..close.span.end;
            Ok((rest, expression))
        }
        TokenKind::LeftBracket => parse_set(rest, token),
        _ => failure(input, ErrorKind::Alt),
    }
}

fn parse_set_element(input: Input<'_>) -> NomResult<'_, SetElement> {
    map(
        (
            parse_expression,
            opt(preceded(
                |input| symbol(input, &TokenKind::DotDot),
                parse_expression,
            )),
        ),
        |(low, high)| match high {
            Some(high) => SetElement::Range { low, high },
            None => SetElement::Value(low),
        },
    )
    .parse(input)
}

fn parse_set<'a>(mut input: Input<'a>, open: &Token) -> NomResult<'a, Expr> {
    let mut elements = Vec::new();
    if !matches!(
        input.first().map(|token| &token.kind),
        Some(TokenKind::RightBracket)
    ) {
        loop {
            let (rest, element) = parse_set_element(input)?;
            elements.push(element);
            input = rest;
            if matches!(
                input.first().map(|token| &token.kind),
                Some(TokenKind::Comma)
            ) {
                input = &input[1..];
            } else {
                break;
            }
        }
    }
    let (rest, close) = symbol(input, &TokenKind::RightBracket)?;
    Ok((
        rest,
        Expr::new(ExprKind::Set(elements), open.span.start..close.span.end),
    ))
}

fn parse_expression_list(mut input: Input<'_>) -> NomResult<'_, Vec<Expr>> {
    let mut expressions = Vec::new();
    if matches!(
        input.first().map(|token| &token.kind),
        Some(TokenKind::RightParen | TokenKind::RightBracket)
    ) {
        return Ok((input, expressions));
    }

    loop {
        let (rest, expression) = parse_expression(input)?;
        expressions.push(expression);
        input = rest;
        if matches!(
            input.first().map(|token| &token.kind),
            Some(TokenKind::Comma)
        ) {
            input = &input[1..];
        } else {
            return Ok((input, expressions));
        }
    }
}

fn parse_postfix(input: Input<'_>) -> NomResult<'_, Expr> {
    let (mut input, mut expression) = parse_primary(input)?;

    loop {
        let Some(token) = input.first() else {
            return Ok((input, expression));
        };
        match &token.kind {
            TokenKind::LeftParen => {
                let open = token.clone();
                let (rest, arguments) = parse_expression_list(&input[1..])?;
                let (rest, close) = symbol(rest, &TokenKind::RightParen)?;
                expression = Expr::application(Application::call(
                    expression,
                    arguments,
                    open.modes,
                    close.span.end,
                ));
                input = rest;
            }
            TokenKind::Dot => {
                let start = expression.span.start;
                let (rest, (member, member_token)) = identifier(&input[1..])?;
                expression = Expr::new(
                    ExprKind::Member {
                        base: Box::new(expression),
                        member,
                    },
                    start..member_token.span.end,
                );
                input = rest;
            }
            TokenKind::LeftBracket => {
                let start = expression.span.start;
                let (rest, indices) = parse_expression_list(&input[1..])?;
                if indices.is_empty() {
                    return failure(input, ErrorKind::SeparatedList);
                }
                let (rest, close) = symbol(rest, &TokenKind::RightBracket)?;
                expression = Expr::new(
                    ExprKind::Index {
                        base: Box::new(expression),
                        indices,
                        range_checks: close.modes.range_checks,
                        modes: close.modes,
                    },
                    start..close.span.end,
                );
                input = rest;
            }
            TokenKind::Caret => {
                let end = token.span.end;
                let start = expression.span.start;
                expression = Expr::new(ExprKind::Dereference(Box::new(expression)), start..end);
                input = &input[1..];
            }
            _ => return Ok((input, expression)),
        }
    }
}

fn prefix_operator(token: &Token) -> Option<Operator> {
    match &token.kind {
        TokenKind::Plus => Some(Operator::Positive),
        TokenKind::Minus => Some(Operator::Negative),
        TokenKind::At => Some(Operator::Address),
        TokenKind::AtAt => Some(Operator::ProcedureSlotAddress),
        TokenKind::Identifier(name) if name == "not" => Some(Operator::Not),
        _ => None,
    }
}

fn parse_prefix(input: Input<'_>) -> NomResult<'_, Expr> {
    if let Some(token) = input.first()
        && let Some(operator) = prefix_operator(token)
    {
        let (rest, operand) = parse_prefix(&input[1..])?;
        let span = token.span.start..operand.span.end;
        let application = Application::operator(
            operator,
            vec![operand],
            ApplicationSyntax::Prefix,
            token.modes,
            span,
        );
        return Ok((rest, Expr::application(application)));
    }
    parse_postfix(input)
}

fn multiplicative_operator(token: &Token) -> Option<Operator> {
    match &token.kind {
        TokenKind::Star => Some(Operator::Multiply),
        TokenKind::Slash => Some(Operator::RealDivide),
        TokenKind::Identifier(name) if name == "div" => Some(Operator::IntegerDivide),
        TokenKind::Identifier(name) if name == "mod" => Some(Operator::Modulo),
        TokenKind::Identifier(name) if name == "and" => Some(Operator::And),
        TokenKind::Identifier(name) if name == "shl" => Some(Operator::ShiftLeft),
        TokenKind::Identifier(name) if name == "shr" => Some(Operator::ShiftRight),
        _ => None,
    }
}

fn additive_operator(token: &Token) -> Option<Operator> {
    match &token.kind {
        TokenKind::Plus => Some(Operator::Add),
        TokenKind::Minus => Some(Operator::Subtract),
        TokenKind::Identifier(name) if name == "or" => Some(Operator::Or),
        TokenKind::Identifier(name) if name == "xor" => Some(Operator::Xor),
        _ => None,
    }
}

fn comparison_operator(token: &Token) -> Option<Operator> {
    match &token.kind {
        TokenKind::Equal => Some(Operator::Equal),
        TokenKind::NotEqual => Some(Operator::NotEqual),
        TokenKind::Less => Some(Operator::Less),
        TokenKind::Greater => Some(Operator::Greater),
        TokenKind::LessEqual => Some(Operator::LessEqual),
        TokenKind::GreaterEqual => Some(Operator::GreaterEqual),
        TokenKind::Identifier(name) if name == "in" => Some(Operator::In),
        TokenKind::Identifier(name) if name == "is" => Some(Operator::Is),
        TokenKind::Identifier(name) if name == "as" => Some(Operator::As),
        _ => None,
    }
}

fn combine_binary(left: Expr, token: &Token, operator: Operator, right: Expr) -> Expr {
    let span = left.span.start..right.span.end;
    Expr::application(Application::operator(
        operator,
        vec![left, right],
        ApplicationSyntax::Infix,
        token.modes,
        span,
    ))
}

fn parse_multiplicative(input: Input<'_>) -> NomResult<'_, Expr> {
    let (mut input, mut left) = parse_prefix(input)?;
    while let Some(token) = input.first() {
        let Some(operator) = multiplicative_operator(token) else {
            break;
        };
        let operator_token = token.clone();
        let (rest, right) = parse_prefix(&input[1..])?;
        left = combine_binary(left, &operator_token, operator, right);
        input = rest;
    }
    Ok((input, left))
}

fn parse_additive(input: Input<'_>) -> NomResult<'_, Expr> {
    let (mut input, mut left) = parse_multiplicative(input)?;
    while let Some(token) = input.first() {
        let Some(operator) = additive_operator(token) else {
            break;
        };
        let operator_token = token.clone();
        let (rest, right) = parse_multiplicative(&input[1..])?;
        left = combine_binary(left, &operator_token, operator, right);
        input = rest;
    }
    Ok((input, left))
}

pub fn parse_expression(input: Input<'_>) -> NomResult<'_, Expr> {
    let (input, left) = parse_additive(input)?;
    let Some(token) = input.first() else {
        return Ok((input, left));
    };
    let Some(operator) = comparison_operator(token) else {
        return Ok((input, left));
    };
    let operator_token = token.clone();
    let (input, right) = parse_additive(&input[1..])?;
    Ok((
        input,
        combine_binary(left, &operator_token, operator, right),
    ))
}

fn parse_statement(input: Input<'_>) -> NomResult<'_, Statement> {
    let (input, left) = parse_expression(input)?;
    if let Some(token) = input.first()
        && token.kind == TokenKind::Assign
    {
        let operator_token = token.clone();
        let (input, right) = parse_expression(&input[1..])?;
        let span = left.span.start..right.span.end;
        return Ok((
            input,
            Statement::Assignment(Application::operator(
                Operator::Assign,
                vec![left, right],
                ApplicationSyntax::Assignment,
                operator_token.modes,
                span,
            )),
        ));
    }
    Ok((input, Statement::Expression(left)))
}

fn source_span_for_error(segment: &[Token], failure: &[Token], fallback: Span) -> Span {
    failure
        .first()
        .or_else(|| segment.last())
        .map_or(fallback, |token| token.span.clone())
}

fn segment_span(segment: &[Token], end_of_source: usize) -> Span {
    segment
        .first()
        .zip(segment.last())
        .map_or(end_of_source..end_of_source, |(first, last)| {
            first.span.start..last.span.end
        })
}

pub fn parse_tokens(tokens: &[Token], end_of_source: usize) -> ParseOutput {
    let mut statements = Vec::new();
    let mut diagnostics = Vec::new();
    let mut start = 0;

    for end in (0..=tokens.len())
        .filter(|&index| index == tokens.len() || tokens[index].kind == TokenKind::Semicolon)
    {
        let segment = &tokens[start..end];
        if !segment.is_empty() {
            match parse_statement(segment) {
                Ok(([], statement)) => statements.push(statement),
                Ok((remaining, _)) => {
                    let span =
                        source_span_for_error(segment, remaining, end_of_source..end_of_source);
                    diagnostics.push(Diagnostic::new(
                        span.clone(),
                        format!("nom: unexpected {}", remaining[0].kind.describe()),
                    ));
                    statements.push(Statement::Error(segment_span(segment, end_of_source)));
                }
                Err(NomErr::Error(error) | NomErr::Failure(error)) => {
                    let span =
                        source_span_for_error(segment, error.input, end_of_source..end_of_source);
                    diagnostics.push(Diagnostic::new(
                        span.clone(),
                        "nom: could not parse expression statement",
                    ));
                    statements.push(Statement::Error(segment_span(segment, end_of_source)));
                }
                Err(NomErr::Incomplete(_)) => {
                    let span = end_of_source..end_of_source;
                    diagnostics.push(Diagnostic::new(
                        span.clone(),
                        "nom: incomplete expression statement",
                    ));
                    statements.push(Statement::Error(segment_span(segment, end_of_source)));
                }
            }
        }
        start = end.saturating_add(1);
    }

    ParseOutput {
        statements,
        diagnostics,
    }
}

pub fn parse(source: &str) -> ParseOutput {
    let lexed = lex(source);
    let mut output = parse_tokens(&lexed.tokens, lexed.logical_len);
    output.diagnostics.splice(0..0, lexed.diagnostics);
    output
}
