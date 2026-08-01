use chumsky::{error::Rich, extra, prelude::*};

use crate::{
    ast::{
        Application, ApplicationSyntax, Diagnostic, Expr, ExprKind, Literal, ModeSnapshot,
        Operator, ParseOutput, SetElement, Span, Statement,
    },
    lexer::{Token, TokenKind, lex},
};

type Extra<'a> = extra::Err<Rich<'a, Token>>;

#[derive(Clone, Debug)]
enum Suffix {
    Call {
        arguments: Vec<Expr>,
        modes: ModeSnapshot,
        end: usize,
    },
    Member {
        name: String,
        end: usize,
    },
    Index {
        indices: Vec<Expr>,
        range_checks: bool,
        end: usize,
    },
    Dereference {
        end: usize,
    },
}

fn symbol<'a>(expected: TokenKind) -> impl Parser<'a, &'a [Token], Token, Extra<'a>> + Clone {
    any().filter(move |token: &Token| token.kind == expected)
}

fn keyword<'a>(expected: &'static str) -> impl Parser<'a, &'a [Token], Token, Extra<'a>> + Clone {
    any().filter(
        move |token: &Token| matches!(&token.kind, TokenKind::Identifier(name) if name == expected),
    )
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

fn identifier<'a>() -> impl Parser<'a, &'a [Token], (String, Token), Extra<'a>> + Clone {
    any()
        .filter(|token: &Token| {
            matches!(&token.kind, TokenKind::Identifier(name) if !is_reserved_identifier(name))
        })
        .map(|token| {
            let TokenKind::Identifier(name) = &token.kind else {
                unreachable!("identifier parser filtered this token")
            };
            (name.clone(), token)
        })
        .labelled("identifier")
}

fn apply_suffix(base: Expr, suffix: Suffix) -> Expr {
    let start = base.span.start;
    match suffix {
        Suffix::Call {
            arguments,
            modes,
            end,
        } => Expr::application(Application::call(base, arguments, modes, end)),
        Suffix::Member { name, end } => Expr::new(
            ExprKind::Member {
                base: Box::new(base),
                member: name,
            },
            start..end,
        ),
        Suffix::Index {
            indices,
            range_checks,
            end,
        } => Expr::new(
            ExprKind::Index {
                base: Box::new(base),
                indices,
                range_checks,
            },
            start..end,
        ),
        Suffix::Dereference { end } => Expr::new(ExprKind::Dereference(Box::new(base)), start..end),
    }
}

fn combine_prefix((operator, token): (Operator, Token), operand: Expr) -> Expr {
    let span = token.span.start..operand.span.end;
    Expr::application(Application::operator(
        operator,
        vec![operand],
        ApplicationSyntax::Prefix,
        token.modes,
        span,
    ))
}

fn combine_binary(left: Expr, ((operator, token), right): ((Operator, Token), Expr)) -> Expr {
    let span = left.span.start..right.span.end;
    Expr::application(Application::operator(
        operator,
        vec![left, right],
        ApplicationSyntax::Infix,
        token.modes,
        span,
    ))
}

fn prefix_operator<'a>() -> impl Parser<'a, &'a [Token], (Operator, Token), Extra<'a>> + Clone {
    choice((
        symbol(TokenKind::Plus).map(|token| (Operator::Positive, token)),
        symbol(TokenKind::Minus).map(|token| (Operator::Negative, token)),
        keyword("not").map(|token| (Operator::Not, token)),
        symbol(TokenKind::AtAt).map(|token| (Operator::ProcedureSlotAddress, token)),
        symbol(TokenKind::At).map(|token| (Operator::Address, token)),
    ))
}

fn multiplicative_operator<'a>()
-> impl Parser<'a, &'a [Token], (Operator, Token), Extra<'a>> + Clone {
    choice((
        symbol(TokenKind::Star).map(|token| (Operator::Multiply, token)),
        symbol(TokenKind::Slash).map(|token| (Operator::RealDivide, token)),
        keyword("div").map(|token| (Operator::IntegerDivide, token)),
        keyword("mod").map(|token| (Operator::Modulo, token)),
        keyword("and").map(|token| (Operator::And, token)),
        keyword("shl").map(|token| (Operator::ShiftLeft, token)),
        keyword("shr").map(|token| (Operator::ShiftRight, token)),
    ))
}

fn additive_operator<'a>() -> impl Parser<'a, &'a [Token], (Operator, Token), Extra<'a>> + Clone {
    choice((
        symbol(TokenKind::Plus).map(|token| (Operator::Add, token)),
        symbol(TokenKind::Minus).map(|token| (Operator::Subtract, token)),
        keyword("or").map(|token| (Operator::Or, token)),
        keyword("xor").map(|token| (Operator::Xor, token)),
    ))
}

fn comparison_operator<'a>() -> impl Parser<'a, &'a [Token], (Operator, Token), Extra<'a>> + Clone {
    choice((
        symbol(TokenKind::Equal).map(|token| (Operator::Equal, token)),
        symbol(TokenKind::NotEqual).map(|token| (Operator::NotEqual, token)),
        symbol(TokenKind::LessEqual).map(|token| (Operator::LessEqual, token)),
        symbol(TokenKind::GreaterEqual).map(|token| (Operator::GreaterEqual, token)),
        symbol(TokenKind::Less).map(|token| (Operator::Less, token)),
        symbol(TokenKind::Greater).map(|token| (Operator::Greater, token)),
        keyword("in").map(|token| (Operator::In, token)),
        keyword("is").map(|token| (Operator::Is, token)),
        keyword("as").map(|token| (Operator::As, token)),
    ))
}

pub fn expression_parser<'a>() -> impl Parser<'a, &'a [Token], Expr, Extra<'a>> + Clone {
    recursive(
        |expression: Recursive<dyn Parser<'a, &'a [Token], Expr, Extra<'a>>>| {
            let boolean = choice((keyword("true").to(true), keyword("false").to(false))).map_with(
                |value, extra| {
                    Expr::new(
                        ExprKind::Literal(Literal::Boolean(value)),
                        extra.span().into_range(),
                    )
                },
            );

            let nil =
                keyword("nil").map(|token| Expr::new(ExprKind::Literal(Literal::Nil), token.span));

            let inherited =
                keyword("inherited")
                    .then(identifier().or_not())
                    .map(|(inherited_token, name)| {
                        let end = name
                            .as_ref()
                            .map_or(inherited_token.span.end, |(_, token)| token.span.end);
                        Expr::new(
                            ExprKind::Inherited(name.map(|(name, _)| name)),
                            inherited_token.span.start..end,
                        )
                    });

            let identifier_expr =
                identifier().map(|(name, token)| Expr::new(ExprKind::Identifier(name), token.span));

            let integer = any()
                .filter(|token: &Token| matches!(token.kind, TokenKind::Integer(_)))
                .map(|token| {
                    let TokenKind::Integer(value) = token.kind else {
                        unreachable!()
                    };
                    Expr::new(ExprKind::Literal(Literal::Integer(value)), token.span)
                });

            let real = any()
                .filter(|token: &Token| matches!(token.kind, TokenKind::Real(_)))
                .map(|token| {
                    let TokenKind::Real(value) = token.kind else {
                        unreachable!()
                    };
                    Expr::new(ExprKind::Literal(Literal::Real(value)), token.span)
                });

            let string = any()
                .filter(|token: &Token| matches!(token.kind, TokenKind::String(_)))
                .map(|token| {
                    let TokenKind::String(value) = token.kind else {
                        unreachable!()
                    };
                    Expr::new(ExprKind::Literal(Literal::String(value)), token.span)
                });

            let parenthesized = symbol(TokenKind::LeftParen)
                .then(expression.clone())
                .then(symbol(TokenKind::RightParen))
                .map(|((open, mut expression), close)| {
                    expression.span = open.span.start..close.span.end;
                    expression
                });

            let set_element = expression
                .clone()
                .then(
                    symbol(TokenKind::DotDot)
                        .ignore_then(expression.clone())
                        .or_not(),
                )
                .map(|(low, high)| match high {
                    Some(high) => SetElement::Range { low, high },
                    None => SetElement::Value(low),
                });

            let set = symbol(TokenKind::LeftBracket)
                .then(
                    set_element
                        .separated_by(symbol(TokenKind::Comma))
                        .allow_trailing()
                        .collect::<Vec<_>>(),
                )
                .then(symbol(TokenKind::RightBracket))
                .map(|((open, elements), close)| {
                    Expr::new(ExprKind::Set(elements), open.span.start..close.span.end)
                });

            let atom = choice((
                boolean,
                nil,
                inherited,
                identifier_expr,
                integer,
                real,
                string,
                parenthesized,
                set,
            ))
            .labelled("Pascal expression");

            let expression_list = expression
                .clone()
                .separated_by(symbol(TokenKind::Comma))
                .collect::<Vec<_>>();

            let call = symbol(TokenKind::LeftParen)
                .then(expression_list.clone())
                .then(symbol(TokenKind::RightParen))
                .map(|((open, arguments), close)| Suffix::Call {
                    arguments,
                    modes: open.modes,
                    end: close.span.end,
                });

            let member = symbol(TokenKind::Dot)
                .ignore_then(identifier())
                .map(|(name, token)| Suffix::Member {
                    name,
                    end: token.span.end,
                });

            let index = symbol(TokenKind::LeftBracket)
                .ignore_then(
                    expression
                        .clone()
                        .separated_by(symbol(TokenKind::Comma))
                        .at_least(1)
                        .collect::<Vec<_>>(),
                )
                .then(symbol(TokenKind::RightBracket))
                .map(|(indices, close)| Suffix::Index {
                    indices,
                    range_checks: close.modes.range_checks,
                    end: close.span.end,
                });

            let dereference = symbol(TokenKind::Caret).map(|token| Suffix::Dereference {
                end: token.span.end,
            });

            let postfix = atom.foldl(
                choice((call, member, index, dereference)).repeated(),
                apply_suffix,
            );

            let prefixed = prefix_operator()
                .repeated()
                .collect::<Vec<_>>()
                .then(postfix)
                .map(|(operators, operand): (Vec<(Operator, Token)>, Expr)| {
                    operators
                        .into_iter()
                        .rev()
                        .fold(operand, |operand, operator| {
                            combine_prefix(operator, operand)
                        })
                });

            let multiplicative = prefixed.clone().foldl(
                multiplicative_operator().then(prefixed.clone()).repeated(),
                combine_binary,
            );

            let additive = multiplicative.clone().foldl(
                additive_operator().then(multiplicative.clone()).repeated(),
                combine_binary,
            );

            additive
                .clone()
                .then(comparison_operator().then(additive).or_not())
                .map(|(left, tail)| match tail {
                    Some(tail) => combine_binary(left, tail),
                    None => left,
                })
        },
    )
}

fn statement_parser<'a>() -> impl Parser<'a, &'a [Token], Statement, Extra<'a>> + Clone {
    expression_parser()
        .then(symbol(TokenKind::Assign).then(expression_parser()).or_not())
        .map(|(left, assignment)| match assignment {
            Some((operator_token, right)) => {
                let span = left.span.start..right.span.end;
                Statement::Assignment(Application::operator(
                    Operator::Assign,
                    vec![left, right],
                    ApplicationSyntax::Assignment,
                    operator_token.modes,
                    span,
                ))
            }
            None => Statement::Expression(left),
        })
}

fn source_span(segment: &[Token], token_span: std::ops::Range<usize>, eof: usize) -> Span {
    if segment.is_empty() {
        return eof..eof;
    }
    if token_span.start >= segment.len() {
        let end = segment.last().map_or(eof, |token| token.span.end);
        return end..end;
    }
    let start = segment
        .get(token_span.start)
        .map_or(eof, |token| token.span.start);
    let end = if token_span.end <= token_span.start {
        start
    } else {
        token_span
            .end
            .checked_sub(1)
            .and_then(|index| segment.get(index))
            .map_or(start, |token| token.span.end)
    };
    start..end
}

pub fn parse_tokens(tokens: &[Token], end_of_source: usize) -> ParseOutput {
    let mut statements = Vec::new();
    let mut diagnostics = Vec::new();
    let mut start = 0;

    for segment_end in (0..=tokens.len())
        .filter(|&index| index == tokens.len() || tokens[index].kind == TokenKind::Semicolon)
    {
        let segment = &tokens[start..segment_end];
        if !segment.is_empty() {
            let (statement, errors) = statement_parser()
                .then_ignore(end())
                .parse(segment)
                .into_output_errors();
            for error in errors {
                diagnostics.push(Diagnostic::new(
                    source_span(segment, error.span().into_range(), end_of_source),
                    format!("chumsky: {error}"),
                ));
            }
            statements.push(statement.unwrap_or_else(|| {
                let span = segment
                    .first()
                    .zip(segment.last())
                    .map_or(end_of_source..end_of_source, |(first, last)| {
                        first.span.start..last.span.end
                    });
                Statement::Error(span)
            }));
        }
        start = segment_end.saturating_add(1);
    }

    ParseOutput {
        statements,
        diagnostics,
    }
}

pub fn parse(source: &str) -> ParseOutput {
    let lexed = lex(source);
    let mut output = parse_tokens(&lexed.tokens, source.len());
    output.diagnostics.splice(0..0, lexed.diagnostics);
    output
}
