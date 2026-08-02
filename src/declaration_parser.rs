use chumsky::{error::Rich, extra, prelude::*};

use crate::{
    CstNode, Diagnostic, PascalFile, PascalSectionKind, Span, Token, TokenKind,
    declaration_ast::{
        AggregateSyntaxKind, CallingConventionSyntax, DeclarationParseOutput, DeclarationSyntax,
        EnumMemberSyntax, FormalModeSyntax, FormalParameterSyntax, ParsedDeclarationSection,
        PropertyDeclarationSyntax, RoutineDeclarationSyntax, RoutineSyntaxKind, SpannedName,
        TypeDeclarationSyntax, TypeSyntax, TypeSyntaxKind, ValueDeclarationSyntax,
        VariantAlternativeSyntax, VariantPartSyntax,
    },
};

type Extra<'a> = extra::Err<Rich<'a, Token>>;

fn symbol<'a>(expected: TokenKind) -> impl Parser<'a, &'a [Token], Token, Extra<'a>> + Clone {
    any().filter(move |token: &Token| token.kind == expected)
}

fn keyword<'a>(expected: &'static str) -> impl Parser<'a, &'a [Token], Token, Extra<'a>> + Clone {
    any().filter(
        move |token: &Token| matches!(&token.kind, TokenKind::Identifier(name) if name == expected),
    )
}

fn identifier<'a>() -> impl Parser<'a, &'a [Token], SpannedName, Extra<'a>> + Clone {
    any()
        .filter(|token: &Token| matches!(token.kind, TokenKind::Identifier(_)))
        .map(|token| {
            let TokenKind::Identifier(spelling) = token.kind else {
                unreachable!("identifier parser filtered its input")
            };
            SpannedName {
                spelling,
                span: token.span,
            }
        })
}

fn is_keyword(token: &Token, expected: &str) -> bool {
    matches!(&token.kind, TokenKind::Identifier(name) if name == expected)
}

fn token_span(tokens: &[Token], fallback: usize) -> Span {
    tokens
        .first()
        .zip(tokens.last())
        .map_or(fallback..fallback, |(first, last)| {
            first.span.start..last.span.end
        })
}

fn flatten_node(node: &CstNode, output: &mut Vec<Token>) {
    match node {
        CstNode::Token(token) => output.push(token.clone()),
        CstNode::Group {
            open,
            children,
            close,
            ..
        } => {
            output.push(open.clone());
            for child in children {
                flatten_node(child, output);
            }
            output.push(close.clone());
        }
    }
}

fn flatten_range(file: &PascalFile, range: std::ops::Range<usize>) -> Vec<Token> {
    let mut tokens = Vec::new();
    for node in &file.nodes[range] {
        flatten_node(node, &mut tokens);
    }
    tokens
}

pub fn section_tokens(file: &PascalFile, kind: PascalSectionKind) -> Option<Vec<Token>> {
    file.sections
        .iter()
        .find(|section| section.kind == kind)
        .map(|section| flatten_range(file, section.nodes.clone()))
}

fn opens_aggregate(tokens: &[Token], index: usize) -> bool {
    if is_keyword(&tokens[index], "record")
        || is_keyword(&tokens[index], "object")
        || is_keyword(&tokens[index], "interface")
        || is_keyword(&tokens[index], "dispinterface")
    {
        return !tokens
            .get(index + 1)
            .is_some_and(|token| token.kind == TokenKind::Semicolon);
    }
    if !is_keyword(&tokens[index], "class") {
        return false;
    }
    let starts_type_declaration = index > 0 && tokens[index - 1].kind == TokenKind::Equal;
    if tokens
        .get(index + 1)
        .is_some_and(|token| token.kind == TokenKind::Semicolon)
        || tokens.get(index + 1).is_some_and(|token| {
            is_keyword(token, "of")
                || !starts_type_declaration
                    && [
                        "procedure",
                        "function",
                        "constructor",
                        "destructor",
                        "operator",
                        "var",
                    ]
                    .iter()
                    .any(|word| is_keyword(token, word))
        })
    {
        return false;
    }
    true
}

fn declaration_semicolon(tokens: &[Token], start: usize) -> usize {
    let mut aggregate_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    for index in start..tokens.len() {
        match tokens[index].kind {
            TokenKind::LeftParen => paren_depth += 1,
            TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::LeftBracket => bracket_depth += 1,
            TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
            TokenKind::Semicolon
                if aggregate_depth == 0 && paren_depth == 0 && bracket_depth == 0 =>
            {
                return index;
            }
            _ if paren_depth == 0 && bracket_depth == 0 && opens_aggregate(tokens, index) => {
                aggregate_depth += 1;
            }
            _ if paren_depth == 0
                && bracket_depth == 0
                && is_keyword(&tokens[index], "end")
                && aggregate_depth > 0 =>
            {
                aggregate_depth -= 1;
            }
            _ => {}
        }
    }
    tokens.len()
}

fn first_semicolon(tokens: &[Token], start: usize) -> usize {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        match token.kind {
            TokenKind::LeftParen => paren_depth += 1,
            TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::LeftBracket => bracket_depth += 1,
            TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
            TokenKind::Semicolon if paren_depth == 0 && bracket_depth == 0 => return index,
            _ => {}
        }
    }
    tokens.len()
}

fn matching_routine_end(tokens: &[Token], begin: usize) -> usize {
    let mut end_depth = 0usize;
    let mut repeat_depth = 0usize;
    for index in begin..tokens.len() {
        if ["begin", "case", "try", "asm"]
            .iter()
            .any(|word| is_keyword(&tokens[index], word))
        {
            end_depth += 1;
        } else if is_keyword(&tokens[index], "repeat") {
            repeat_depth += 1;
        } else if is_keyword(&tokens[index], "until") && repeat_depth > 0 {
            repeat_depth -= 1;
        } else if is_keyword(&tokens[index], "end") && end_depth > 0 {
            end_depth -= 1;
            if end_depth == 0 && repeat_depth == 0 {
                return first_semicolon(tokens, index + 1);
            }
        }
    }
    tokens.len()
}

fn parse_with<'a, O>(
    parser: impl Parser<'a, &'a [Token], O, Extra<'a>>,
    tokens: &'a [Token],
) -> (Option<O>, Vec<Diagnostic>) {
    let (output, errors) = parser.then_ignore(end()).parse(tokens).into_output_errors();
    let fallback = tokens.last().map_or(0, |token| token.span.end);
    let diagnostics = errors
        .into_iter()
        .map(|error| {
            let range = error.span().into_range();
            let span = if range.start < tokens.len() {
                let start = tokens[range.start].span.start;
                let end = range
                    .end
                    .checked_sub(1)
                    .and_then(|index| tokens.get(index))
                    .map_or(start, |token| token.span.end);
                start..end
            } else {
                fallback..fallback
            };
            Diagnostic::new(span, format!("declaration grammar: {error}"))
        })
        .collect();
    (output, diagnostics)
}

fn parse_uses(tokens: &[Token]) -> (Option<DeclarationSyntax>, Vec<Diagnostic>) {
    let parser = keyword("uses")
        .ignore_then(
            identifier()
                .separated_by(symbol(TokenKind::Comma))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then_ignore(symbol(TokenKind::Semicolon));
    let span = token_span(tokens, 0);
    let (names, diagnostics) = parse_with(parser, tokens);
    (
        names.map(|units| DeclarationSyntax::Uses { units, span }),
        diagnostics,
    )
}

fn named_type(tokens: &[Token]) -> Option<TypeSyntax> {
    let modes = tokens.first()?.modes;
    let span = token_span(tokens, tokens[0].span.start);
    let parser = identifier()
        .separated_by(symbol(TokenKind::Dot))
        .at_least(1)
        .collect::<Vec<_>>();
    let (names, errors) = parse_with(parser, tokens);
    errors.is_empty().then_some(TypeSyntax {
        kind: TypeSyntaxKind::Named(names?),
        span,
        modes,
    })
}

fn parenthesized_base(tokens: &[Token], start: usize) -> Option<TypeSyntax> {
    if tokens.get(start)?.kind != TokenKind::LeftParen {
        return None;
    }
    let end = tokens[start + 1..]
        .iter()
        .position(|token| token.kind == TokenKind::RightParen)?
        + start
        + 1;
    named_type(&tokens[start + 1..end])
}

fn split_top_level(tokens: &[Token], delimiter: TokenKind) -> Vec<&[Token]> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LeftParen => paren_depth += 1,
            TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::LeftBracket => bracket_depth += 1,
            TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
            _ if token.kind == delimiter && paren_depth == 0 && bracket_depth == 0 => {
                parts.push(&tokens[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&tokens[start..]);
    parts
}

fn top_level_position(tokens: &[Token], expected: TokenKind) -> Option<usize> {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LeftParen => paren_depth += 1,
            TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::LeftBracket => bracket_depth += 1,
            TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
            _ if token.kind == expected && paren_depth == 0 && bracket_depth == 0 => {
                return Some(index);
            }
            _ => {}
        }
    }
    None
}

fn expression_syntax(tokens: &[Token]) -> Option<crate::Expr> {
    let fallback = tokens.last().map_or(0, |token| token.span.end);
    crate::chumsky_parser::parse_expression_tokens(tokens, fallback).0
}

fn aggregate_end(tokens: &[Token], opening: usize) -> Option<usize> {
    let mut depth = 0usize;
    for index in opening..tokens.len() {
        if index == opening || opens_aggregate(tokens, index) {
            depth += 1;
        } else if is_keyword(&tokens[index], "end") && depth > 0 {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn variant_case_position(tokens: &[Token]) -> Option<usize> {
    let mut aggregate_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        if paren_depth == 0
            && bracket_depth == 0
            && aggregate_depth == 0
            && is_keyword(token, "case")
        {
            return Some(index);
        }
        match token.kind {
            TokenKind::LeftParen => paren_depth += 1,
            TokenKind::RightParen => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::LeftBracket => bracket_depth += 1,
            TokenKind::RightBracket => bracket_depth = bracket_depth.saturating_sub(1),
            _ if paren_depth == 0 && bracket_depth == 0 && opens_aggregate(tokens, index) => {
                aggregate_depth += 1;
            }
            _ if paren_depth == 0
                && bracket_depth == 0
                && aggregate_depth > 0
                && is_keyword(token, "end") =>
            {
                aggregate_depth -= 1;
            }
            _ => {}
        }
    }
    None
}

fn parse_variant_part(tokens: &[Token]) -> Option<VariantPartSyntax> {
    let case = tokens.first()?;
    if !is_keyword(case, "case") {
        return None;
    }
    let of = tokens.iter().position(|token| is_keyword(token, "of"))?;
    let selector = &tokens[1..of];
    let colon = top_level_position(selector, TokenKind::Colon);
    let (selector_name, selector_type) = if let Some(colon) = colon {
        let name = selector.first().and_then(|token| {
            let TokenKind::Identifier(spelling) = &token.kind else {
                return None;
            };
            Some(SpannedName {
                spelling: spelling.clone(),
                span: token.span.clone(),
            })
        });
        (
            name,
            parse_type_syntax(
                selector
                    .get(colon + 1..)
                    .filter(|tokens| !tokens.is_empty())?,
            ),
        )
    } else {
        (None, parse_type_syntax(selector))
    };
    let mut alternatives = Vec::new();
    let mut cursor = of + 1;
    while cursor < tokens.len() {
        while tokens
            .get(cursor)
            .is_some_and(|token| token.kind == TokenKind::Semicolon)
        {
            cursor += 1;
        }
        if cursor >= tokens.len() {
            break;
        }
        let label_colon = top_level_position(&tokens[cursor..], TokenKind::Colon)? + cursor;
        let labels = split_top_level(&tokens[cursor..label_colon], TokenKind::Comma)
            .into_iter()
            .map(expression_syntax)
            .collect::<Option<Vec<_>>>()?;
        let open = label_colon + 1;
        if !tokens
            .get(open)
            .is_some_and(|token| token.kind == TokenKind::LeftParen)
        {
            return None;
        }
        let mut depth = 0usize;
        let mut close = None;
        for (index, token) in tokens.iter().enumerate().skip(open) {
            match token.kind {
                TokenKind::LeftParen => depth += 1,
                TokenKind::RightParen => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        close = Some(index);
                        break;
                    }
                }
                _ => {}
            }
        }
        let close = close?;
        let mut scanner = DeclarationScanner::new(&tokens[open + 1..close], false, true);
        let members = scanner.parse_all();
        alternatives.push(VariantAlternativeSyntax {
            labels,
            members,
            span: tokens[cursor].span.start..tokens[close].span.end,
        });
        cursor = close + 1;
    }
    Some(VariantPartSyntax {
        selector_name,
        selector_type: Box::new(selector_type),
        alternatives,
        span: token_span(tokens, case.span.start),
    })
}

fn parse_type_syntax(tokens: &[Token]) -> TypeSyntax {
    let fallback = tokens.first().map_or(0, |token| token.span.start);
    let span = token_span(tokens, fallback);
    let modes = tokens
        .first()
        .map_or_else(crate::ModeSnapshot::default, |token| token.modes);
    let distinct_offset = usize::from(
        tokens
            .first()
            .is_some_and(|token| is_keyword(token, "type")),
    );
    let tokens = &tokens[distinct_offset..];

    if tokens.len() == 1 && is_keyword(&tokens[0], "class") {
        return TypeSyntax {
            kind: TypeSyntaxKind::ClassForward,
            span,
            modes,
        };
    }
    if tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::Caret)
    {
        return TypeSyntax {
            kind: TypeSyntaxKind::Pointer(Box::new(parse_type_syntax(&tokens[1..]))),
            span,
            modes,
        };
    }
    if tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::LeftParen)
        && tokens
            .last()
            .is_some_and(|token| token.kind == TokenKind::RightParen)
    {
        let mut members = Vec::new();
        for part in split_top_level(&tokens[1..tokens.len() - 1], TokenKind::Comma) {
            let Some(name_token) = part.first() else {
                continue;
            };
            let TokenKind::Identifier(spelling) = &name_token.kind else {
                return TypeSyntax {
                    kind: TypeSyntaxKind::Unsupported(tokens.to_vec()),
                    span,
                    modes,
                };
            };
            let equal = part.iter().position(|token| token.kind == TokenKind::Equal);
            let value = equal.and_then(|equal| expression_syntax(&part[equal + 1..]));
            members.push(EnumMemberSyntax {
                name: SpannedName {
                    spelling: spelling.clone(),
                    span: name_token.span.clone(),
                },
                value,
                span: token_span(part, name_token.span.start),
            });
        }
        return TypeSyntax {
            kind: TypeSyntaxKind::Enumeration(members),
            span,
            modes,
        };
    }
    if let Some(range) = top_level_position(tokens, TokenKind::DotDot)
        && let (Some(lower), Some(upper)) = (
            expression_syntax(&tokens[..range]),
            expression_syntax(&tokens[range + 1..]),
        )
    {
        return TypeSyntax {
            kind: TypeSyntaxKind::Subrange { lower, upper },
            span,
            modes,
        };
    }
    if tokens
        .first()
        .is_some_and(|token| is_keyword(token, "procedure") || is_keyword(token, "function"))
    {
        let (parameters, result) = routine_signature_syntax(tokens);
        let method_pointer = tokens
            .windows(2)
            .any(|window| is_keyword(&window[0], "of") && is_keyword(&window[1], "object"));
        let calling_convention = if tokens.iter().any(|token| is_keyword(token, "cdecl")) {
            CallingConventionSyntax::Cdecl
        } else if tokens.iter().any(|token| is_keyword(token, "stdcall")) {
            CallingConventionSyntax::Stdcall
        } else if tokens.iter().any(|token| is_keyword(token, "register")) {
            CallingConventionSyntax::Register
        } else {
            CallingConventionSyntax::Pascal
        };
        return TypeSyntax {
            kind: TypeSyntaxKind::Procedural {
                method_pointer,
                parameters,
                result: result.map(Box::new),
                calling_convention,
            },
            span,
            modes,
        };
    }
    if tokens
        .first()
        .is_some_and(|token| is_keyword(token, "array"))
    {
        let of = tokens.iter().position(|token| is_keyword(token, "of"));
        let indices = tokens
            .iter()
            .position(|token| token.kind == TokenKind::LeftBracket)
            .and_then(|open| {
                tokens[open + 1..]
                    .iter()
                    .position(|token| token.kind == TokenKind::RightBracket)
                    .map(|offset| (open, open + 1 + offset))
            })
            .map_or_else(Vec::new, |(open, close)| {
                split_top_level(&tokens[open + 1..close], TokenKind::Comma)
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .map(parse_type_syntax)
                    .collect()
            });
        let element = of
            .filter(|index| index + 1 < tokens.len())
            .map(|index| Box::new(parse_type_syntax(&tokens[index + 1..])));
        let dynamic = !tokens
            .iter()
            .any(|token| token.kind == TokenKind::LeftBracket);
        return TypeSyntax {
            kind: TypeSyntaxKind::Array {
                indices,
                element,
                dynamic,
            },
            span,
            modes,
        };
    }
    if tokens.first().is_some_and(|token| is_keyword(token, "set")) {
        let of = tokens.iter().position(|token| is_keyword(token, "of"));
        let element = of
            .filter(|index| index + 1 < tokens.len())
            .map(|index| Box::new(parse_type_syntax(&tokens[index + 1..])));
        return TypeSyntax {
            kind: TypeSyntaxKind::Set { element },
            span,
            modes,
        };
    }

    let (kind, opening) = if tokens.len() >= 2
        && is_keyword(&tokens[0], "packed")
        && is_keyword(&tokens[1], "record")
    {
        (Some(AggregateSyntaxKind::PackedRecord), 1)
    } else if tokens
        .first()
        .is_some_and(|token| is_keyword(token, "record"))
    {
        (Some(AggregateSyntaxKind::Record), 0)
    } else if tokens
        .first()
        .is_some_and(|token| is_keyword(token, "object"))
    {
        (Some(AggregateSyntaxKind::Object), 0)
    } else if tokens
        .first()
        .is_some_and(|token| is_keyword(token, "class"))
    {
        (Some(AggregateSyntaxKind::Class), 0)
    } else if tokens
        .first()
        .is_some_and(|token| is_keyword(token, "interface") || is_keyword(token, "dispinterface"))
    {
        (Some(AggregateSyntaxKind::Interface), 0)
    } else {
        (None, 0)
    };
    if let Some(kind) = kind
        && let Some(end) = aggregate_end(tokens, opening)
    {
        let after_keyword = opening + 1;
        let base = parenthesized_base(tokens, after_keyword).map(Box::new);
        let member_start = if tokens
            .get(after_keyword)
            .is_some_and(|token| token.kind == TokenKind::LeftParen)
        {
            tokens[after_keyword + 1..]
                .iter()
                .position(|token| token.kind == TokenKind::RightParen)
                .map_or(after_keyword, |offset| after_keyword + offset + 2)
        } else {
            after_keyword
        };
        let member_tokens = &tokens[member_start..end];
        let variant_start = matches!(
            kind,
            AggregateSyntaxKind::Record | AggregateSyntaxKind::PackedRecord
        )
        .then(|| variant_case_position(member_tokens))
        .flatten();
        let ordinary_tokens = variant_start.map_or(member_tokens, |variant_start| {
            &member_tokens[..variant_start]
        });
        let mut scanner = DeclarationScanner::new(ordinary_tokens, false, true);
        let members = scanner.parse_all();
        let variant = variant_start
            .and_then(|variant_start| parse_variant_part(&member_tokens[variant_start..]))
            .map(Box::new);
        return TypeSyntax {
            kind: TypeSyntaxKind::Aggregate {
                kind,
                base,
                members,
                variant,
            },
            span,
            modes,
        };
    }

    named_type(tokens).unwrap_or_else(|| TypeSyntax {
        kind: TypeSyntaxKind::Unsupported(tokens.to_vec()),
        span,
        modes,
    })
}

fn parse_type_declaration(tokens: &[Token]) -> (Option<TypeDeclarationSyntax>, Vec<Diagnostic>) {
    let parser = identifier()
        .then_ignore(symbol(TokenKind::Equal))
        .then(any().repeated().collect::<Vec<_>>());
    let span = token_span(tokens, 0);
    let (parsed, diagnostics) = parse_with(parser, tokens);
    (
        parsed.map(|(name, mut specification)| {
            if specification
                .last()
                .is_some_and(|token| token.kind == TokenKind::Semicolon)
            {
                specification.pop();
            }
            let distinct = specification
                .first()
                .is_some_and(|token| is_keyword(token, "type"));
            TypeDeclarationSyntax {
                name,
                ty: parse_type_syntax(&specification),
                distinct,
                span,
            }
        }),
        diagnostics,
    )
}

fn parse_value_declaration(tokens: &[Token], constant: bool) -> Option<ValueDeclarationSyntax> {
    let separator = if constant {
        tokens
            .iter()
            .position(|token| token.kind == TokenKind::Colon || token.kind == TokenKind::Equal)?
    } else {
        tokens
            .iter()
            .position(|token| token.kind == TokenKind::Colon)?
    };
    let names_parser = identifier()
        .separated_by(symbol(TokenKind::Comma))
        .at_least(1)
        .collect::<Vec<_>>();
    let (names, errors) = parse_with(names_parser, &tokens[..separator]);
    if !errors.is_empty() {
        return None;
    }
    let declaration_end = if tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Semicolon)
    {
        tokens.len().saturating_sub(1)
    } else {
        tokens.len()
    };
    let type_end = tokens[separator + 1..]
        .iter()
        .position(|token| {
            token.kind == TokenKind::Equal
                || token.kind == TokenKind::Assign
                || is_keyword(token, "absolute")
                || is_keyword(token, "read")
                || is_keyword(token, "write")
                || is_keyword(token, "stored")
                || is_keyword(token, "default")
                || is_keyword(token, "nodefault")
                || is_keyword(token, "implements")
        })
        .map_or(declaration_end, |offset| separator + 1 + offset);
    let ty = (tokens[separator].kind == TokenKind::Colon && separator + 1 < type_end)
        .then(|| parse_type_syntax(&tokens[separator + 1..type_end]));
    let initializer_start = if tokens[separator].kind == TokenKind::Equal {
        Some(separator + 1)
    } else {
        tokens[separator + 1..]
            .iter()
            .position(|token| token.kind == TokenKind::Equal || token.kind == TokenKind::Assign)
            .map(|offset| separator + offset + 2)
    };
    let initializer = initializer_start.and_then(|start| {
        let end = tokens
            .iter()
            .rposition(|token| token.kind != TokenKind::Semicolon)
            .map_or(start, |last| last + 1);
        (start < end)
            .then(|| expression_syntax(&tokens[start..end]))
            .flatten()
    });
    Some(ValueDeclarationSyntax {
        names: names?,
        ty,
        initializer,
        span: token_span(tokens, 0),
        modes: tokens
            .first()
            .map_or_else(crate::ModeSnapshot::default, |token| token.modes),
    })
}

fn routine_kind(token: &Token) -> Option<RoutineSyntaxKind> {
    [
        ("procedure", RoutineSyntaxKind::Procedure),
        ("function", RoutineSyntaxKind::Function),
        ("constructor", RoutineSyntaxKind::Constructor),
        ("destructor", RoutineSyntaxKind::Destructor),
        ("operator", RoutineSyntaxKind::Operator),
    ]
    .into_iter()
    .find_map(|(word, kind)| is_keyword(token, word).then_some(kind))
}

fn routine_name(tokens: &[Token], kind: RoutineSyntaxKind) -> Option<SpannedName> {
    if kind == RoutineSyntaxKind::Operator {
        let token = tokens.get(1)?;
        let spelling = match &token.kind {
            TokenKind::Identifier(spelling) => spelling.clone(),
            TokenKind::Assign => ":=".to_owned(),
            TokenKind::Plus => "+".to_owned(),
            TokenKind::Minus => "-".to_owned(),
            TokenKind::Star => "*".to_owned(),
            TokenKind::Slash => "/".to_owned(),
            TokenKind::Equal => "=".to_owned(),
            TokenKind::NotEqual => "<>".to_owned(),
            TokenKind::Less => "<".to_owned(),
            TokenKind::Greater => ">".to_owned(),
            TokenKind::LessEqual => "<=".to_owned(),
            TokenKind::GreaterEqual => ">=".to_owned(),
            _ => return None,
        };
        return Some(SpannedName {
            spelling,
            span: token.span.clone(),
        });
    }
    let header_end = first_semicolon(tokens, 0).min(tokens.len());
    let end = tokens[1..header_end]
        .iter()
        .position(|token| {
            matches!(
                token.kind,
                TokenKind::LeftParen | TokenKind::Colon | TokenKind::Semicolon
            )
        })
        .map_or(header_end, |offset| offset + 1);
    tokens[1..end].iter().rev().find_map(|token| {
        if let TokenKind::Identifier(spelling) = &token.kind {
            Some(SpannedName {
                spelling: spelling.clone(),
                span: token.span.clone(),
            })
        } else {
            None
        }
    })
}

fn formal_mode(token: &Token) -> Option<FormalModeSyntax> {
    [
        ("const", FormalModeSyntax::Const),
        ("var", FormalModeSyntax::Var),
        ("out", FormalModeSyntax::Out),
        ("constref", FormalModeSyntax::ConstRef),
    ]
    .into_iter()
    .find_map(|(word, mode)| is_keyword(token, word).then_some(mode))
}

fn parse_formal_parameter(tokens: &[Token]) -> Option<FormalParameterSyntax> {
    let (mode, tokens) = tokens
        .first()
        .and_then(formal_mode)
        .map_or((FormalModeSyntax::Value, tokens), |mode| {
            (mode, &tokens[1..])
        });
    let colon = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Colon);
    let equal = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Equal);
    let names_end = colon.or(equal).unwrap_or(tokens.len());
    let names_parser = identifier()
        .separated_by(symbol(TokenKind::Comma))
        .at_least(1)
        .collect::<Vec<_>>();
    let (names, errors) = parse_with(names_parser, &tokens[..names_end]);
    if !errors.is_empty() {
        return None;
    }
    let ty = colon.and_then(|colon| {
        let type_end = equal.unwrap_or(tokens.len());
        (colon + 1 < type_end).then(|| parse_type_syntax(&tokens[colon + 1..type_end]))
    });
    let default = equal.and_then(|equal| expression_syntax(&tokens[equal + 1..]));
    Some(FormalParameterSyntax {
        names: names?,
        mode,
        ty,
        default,
        modes: tokens.first()?.modes,
        span: token_span(tokens, tokens.first()?.span.start),
    })
}

fn matching_delimiter(
    tokens: &[Token],
    open: usize,
    left: TokenKind,
    right: TokenKind,
) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        if token.kind == left {
            depth += 1;
        } else if token.kind == right {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn parse_formal_parameter_list(tokens: &[Token]) -> Vec<FormalParameterSyntax> {
    let mut parameters = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for index in 0..=tokens.len() {
        let separator = match tokens.get(index).map(|token| &token.kind) {
            Some(TokenKind::LeftParen | TokenKind::LeftBracket) => {
                depth += 1;
                false
            }
            Some(TokenKind::RightParen | TokenKind::RightBracket) => {
                depth = depth.saturating_sub(1);
                false
            }
            Some(TokenKind::Semicolon) => depth == 0,
            None => true,
            _ => false,
        };
        if separator {
            if start < index
                && let Some(parameter) = parse_formal_parameter(&tokens[start..index])
            {
                parameters.push(parameter);
            }
            start = index + 1;
        }
    }
    parameters
}

fn routine_signature_syntax(tokens: &[Token]) -> (Vec<FormalParameterSyntax>, Option<TypeSyntax>) {
    let header_end = first_semicolon(tokens, 0).min(tokens.len());
    let open = tokens[..header_end]
        .iter()
        .position(|token| token.kind == TokenKind::LeftParen);
    let close = open.and_then(|open| {
        let mut depth = 0usize;
        for (index, token) in tokens.iter().enumerate().take(header_end).skip(open) {
            match token.kind {
                TokenKind::LeftParen => depth += 1,
                TokenKind::RightParen => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(index);
                    }
                }
                _ => {}
            }
        }
        None
    });
    let mut parameters = Vec::new();
    if let (Some(open), Some(close)) = (open, close) {
        let mut start = open + 1;
        let mut depth = 0usize;
        for index in open + 1..=close {
            match tokens[index].kind {
                TokenKind::LeftParen | TokenKind::LeftBracket => depth += 1,
                TokenKind::RightParen | TokenKind::RightBracket if depth > 0 => depth -= 1,
                TokenKind::Semicolon if depth == 0 => {
                    if let Some(parameter) = parse_formal_parameter(&tokens[start..index]) {
                        parameters.push(parameter);
                    }
                    start = index + 1;
                }
                TokenKind::RightParen if depth == 0 => {
                    if start < index
                        && let Some(parameter) = parse_formal_parameter(&tokens[start..index])
                    {
                        parameters.push(parameter);
                    }
                }
                _ => {}
            }
        }
    }

    let result_start = close.map_or_else(
        || {
            tokens[1..header_end]
                .iter()
                .position(|token| token.kind == TokenKind::Colon)
                .map(|offset| offset + 2)
        },
        |close| {
            tokens[close + 1..header_end]
                .iter()
                .position(|token| token.kind == TokenKind::Colon)
                .map(|offset| close + offset + 2)
        },
    );
    let result = result_start
        .filter(|start| *start < header_end)
        .map(|start| parse_type_syntax(&tokens[start..header_end]));
    (parameters, result)
}

fn is_declaration_start(token: &Token) -> bool {
    matches!(
        &token.kind,
        TokenKind::Identifier(name)
            if matches!(
                name.as_str(),
                "uses"
                    | "type"
                    | "var"
                    | "threadvar"
                    | "const"
                    | "resourcestring"
                    | "label"
                    | "procedure"
                    | "function"
                    | "constructor"
                    | "destructor"
                    | "operator"
            )
    )
}

struct DeclarationScanner<'a> {
    tokens: &'a [Token],
    index: usize,
    allow_bodies: bool,
    members: bool,
    diagnostics: Vec<Diagnostic>,
    unsupported: usize,
}

impl<'a> DeclarationScanner<'a> {
    const fn new(tokens: &'a [Token], allow_bodies: bool, members: bool) -> Self {
        Self {
            tokens,
            index: 0,
            allow_bodies,
            members,
            diagnostics: Vec::new(),
            unsupported: 0,
        }
    }

    fn parse_all(&mut self) -> Vec<DeclarationSyntax> {
        let mut declarations = Vec::new();
        while self.index < self.tokens.len() {
            if is_keyword(&self.tokens[self.index], "begin")
                || is_keyword(&self.tokens[self.index], "end")
                || is_keyword(&self.tokens[self.index], "initialization")
                || is_keyword(&self.tokens[self.index], "finalization")
            {
                break;
            }
            if is_keyword(&self.tokens[self.index], "uses") {
                let end = first_semicolon(self.tokens, self.index);
                let end_exclusive = (end + 1).min(self.tokens.len());
                let (declaration, diagnostics) =
                    parse_uses(&self.tokens[self.index..end_exclusive]);
                self.diagnostics.extend(diagnostics);
                if let Some(declaration) = declaration {
                    declarations.push(declaration);
                }
                self.index = end_exclusive;
            } else if is_keyword(&self.tokens[self.index], "type") {
                declarations.push(self.parse_type_section());
            } else if is_keyword(&self.tokens[self.index], "var")
                || is_keyword(&self.tokens[self.index], "threadvar")
                || is_keyword(&self.tokens[self.index], "class")
                    && self
                        .tokens
                        .get(self.index + 1)
                        .is_some_and(|token| is_keyword(token, "var"))
            {
                self.index += usize::from(is_keyword(&self.tokens[self.index], "class")) + 1;
                self.parse_value_section(false, &mut declarations);
            } else if is_keyword(&self.tokens[self.index], "const")
                || is_keyword(&self.tokens[self.index], "resourcestring")
            {
                self.index += 1;
                self.parse_value_section(true, &mut declarations);
            } else if is_keyword(&self.tokens[self.index], "label") {
                declarations.push(self.parse_labels());
            } else if routine_kind(&self.tokens[self.index]).is_some() {
                declarations.push(self.parse_routine());
            } else if self.members
                && is_keyword(&self.tokens[self.index], "class")
                && self
                    .tokens
                    .get(self.index + 1)
                    .and_then(routine_kind)
                    .is_some()
            {
                self.index += 1;
                declarations.push(self.parse_routine());
            } else if self.members
                && ["private", "protected", "public", "published", "strict"]
                    .iter()
                    .any(|word| is_keyword(&self.tokens[self.index], word))
            {
                let token = &self.tokens[self.index];
                let name = match &token.kind {
                    TokenKind::Identifier(spelling) => SpannedName {
                        spelling: spelling.clone(),
                        span: token.span.clone(),
                    },
                    _ => unreachable!(),
                };
                declarations.push(DeclarationSyntax::Visibility { name });
                self.index += 1;
            } else if self.members && is_keyword(&self.tokens[self.index], "property") {
                declarations.push(self.parse_property());
            } else if self.members
                && matches!(self.tokens[self.index].kind, TokenKind::Identifier(_))
            {
                let end = declaration_semicolon(self.tokens, self.index);
                let end_exclusive = (end + 1).min(self.tokens.len());
                if let Some(value) =
                    parse_value_declaration(&self.tokens[self.index..end_exclusive], false)
                {
                    declarations.push(DeclarationSyntax::Variables(value));
                } else {
                    declarations.push(self.unsupported_to(end_exclusive));
                }
                self.index = end_exclusive;
            } else if self.tokens[self.index].kind == TokenKind::Semicolon {
                self.index += 1;
            } else {
                let end = declaration_semicolon(self.tokens, self.index);
                let end_exclusive = (end + 1).min(self.tokens.len()).max(self.index + 1);
                declarations.push(self.unsupported_to(end_exclusive));
                self.index = end_exclusive;
            }
        }
        declarations
    }

    fn parse_type_section(&mut self) -> DeclarationSyntax {
        let start = self.index;
        self.index += 1;
        let mut types = Vec::new();
        while self.index + 1 < self.tokens.len()
            && matches!(self.tokens[self.index].kind, TokenKind::Identifier(_))
            && self.tokens[self.index + 1].kind == TokenKind::Equal
        {
            let end = declaration_semicolon(self.tokens, self.index);
            let end_exclusive = (end + 1).min(self.tokens.len());
            let (declaration, diagnostics) =
                parse_type_declaration(&self.tokens[self.index..end_exclusive]);
            self.diagnostics.extend(diagnostics);
            if let Some(declaration) = declaration {
                types.push(declaration);
            }
            self.index = end_exclusive;
        }
        DeclarationSyntax::TypeSection {
            span: token_span(
                &self.tokens[start..self.index],
                self.tokens[start].span.start,
            ),
            declarations: types,
        }
    }

    fn parse_value_section(&mut self, constant: bool, declarations: &mut Vec<DeclarationSyntax>) {
        while self.index < self.tokens.len()
            && matches!(self.tokens[self.index].kind, TokenKind::Identifier(_))
            && !is_declaration_start(&self.tokens[self.index])
            && !is_keyword(&self.tokens[self.index], "begin")
            && !is_keyword(&self.tokens[self.index], "end")
        {
            let end = declaration_semicolon(self.tokens, self.index);
            let end_exclusive = (end + 1).min(self.tokens.len());
            if let Some(value) =
                parse_value_declaration(&self.tokens[self.index..end_exclusive], constant)
            {
                declarations.push(if constant {
                    DeclarationSyntax::Constants(value)
                } else {
                    DeclarationSyntax::Variables(value)
                });
            } else {
                declarations.push(self.unsupported_to(end_exclusive));
            }
            self.index = end_exclusive;
        }
    }

    fn parse_labels(&mut self) -> DeclarationSyntax {
        let start = self.index;
        let end = first_semicolon(self.tokens, start);
        let end_exclusive = (end + 1).min(self.tokens.len());
        let parser = keyword("label")
            .ignore_then(
                identifier()
                    .separated_by(symbol(TokenKind::Comma))
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .then_ignore(symbol(TokenKind::Semicolon));
        let (names, diagnostics) = parse_with(parser, &self.tokens[start..end_exclusive]);
        self.diagnostics.extend(diagnostics);
        self.index = end_exclusive;
        DeclarationSyntax::Labels {
            names: names.unwrap_or_default(),
            span: token_span(
                &self.tokens[start..end_exclusive],
                self.tokens[start].span.start,
            ),
        }
    }

    fn parse_routine(&mut self) -> DeclarationSyntax {
        let start = self.index;
        let kind = routine_kind(&self.tokens[start]).expect("caller checked routine keyword");
        let header_end = first_semicolon(self.tokens, start);
        let mut cursor = (header_end + 1).min(self.tokens.len());
        let mut is_forward = false;
        let mut overload = kind == RoutineSyntaxKind::Operator;
        let mut calling_convention = CallingConventionSyntax::Pascal;
        while cursor < self.tokens.len()
            && [
                "forward", "external", "overload", "inline", "cdecl", "stdcall", "register",
                "pascal",
            ]
            .iter()
            .any(|word| is_keyword(&self.tokens[cursor], word))
        {
            is_forward |= is_keyword(&self.tokens[cursor], "forward")
                || is_keyword(&self.tokens[cursor], "external");
            overload |= is_keyword(&self.tokens[cursor], "overload");
            calling_convention = if is_keyword(&self.tokens[cursor], "cdecl") {
                CallingConventionSyntax::Cdecl
            } else if is_keyword(&self.tokens[cursor], "stdcall") {
                CallingConventionSyntax::Stdcall
            } else if is_keyword(&self.tokens[cursor], "register") {
                CallingConventionSyntax::Register
            } else if is_keyword(&self.tokens[cursor], "pascal") {
                CallingConventionSyntax::Pascal
            } else {
                calling_convention
            };
            cursor = (first_semicolon(self.tokens, cursor) + 1).min(self.tokens.len());
        }

        let mut body_declarations = Vec::new();
        let mut body_tokens = Vec::new();
        let mut has_body = false;
        if self.allow_bodies && !is_forward {
            let mut nested = DeclarationScanner::new(&self.tokens[cursor..], true, false);
            body_declarations = nested.parse_all();
            let nested_consumed = nested.index;
            self.diagnostics.extend(nested.diagnostics);
            self.unsupported += nested.unsupported;
            cursor += nested_consumed;
            if cursor < self.tokens.len() && is_keyword(&self.tokens[cursor], "begin") {
                let body_begin = cursor;
                let body_end = matching_routine_end(self.tokens, cursor);
                let final_end = body_end.saturating_sub(1);
                if body_begin < final_end && final_end <= self.tokens.len() {
                    body_tokens = self.tokens[body_begin + 1..final_end].to_vec();
                }
                cursor = (body_end + 1).min(self.tokens.len());
                has_body = true;
            }
        }
        let end = cursor.max(header_end + 1).min(self.tokens.len());
        let name = routine_name(
            &self.tokens[start..=header_end.min(self.tokens.len() - 1)],
            kind,
        )
        .unwrap_or_else(|| SpannedName {
            spelling: "<missing>".to_owned(),
            span: self.tokens[start].span.clone(),
        });
        let (parameters, result) =
            routine_signature_syntax(&self.tokens[start..=header_end.min(self.tokens.len() - 1)]);
        self.index = end;
        DeclarationSyntax::Routine(RoutineDeclarationSyntax {
            kind,
            name,
            parameters,
            result,
            body_declarations,
            body_tokens,
            has_body,
            is_forward,
            overload,
            calling_convention,
            span: token_span(&self.tokens[start..end], self.tokens[start].span.start),
            modes: self.tokens[start].modes,
        })
    }

    fn parse_property(&mut self) -> DeclarationSyntax {
        let start = self.index;
        let declaration_end = declaration_semicolon(self.tokens, start);
        let header_end = declaration_end.min(self.tokens.len());
        let header = &self.tokens[start + 1..header_end];
        let name = header
            .first()
            .and_then(|token| match &token.kind {
                TokenKind::Identifier(spelling) => Some(SpannedName {
                    spelling: spelling.clone(),
                    span: token.span.clone(),
                }),
                _ => None,
            })
            .unwrap_or_else(|| SpannedName {
                spelling: "<missing>".to_owned(),
                span: self.tokens[start].span.clone(),
            });

        let bracket_open = header
            .iter()
            .position(|token| token.kind == TokenKind::LeftBracket);
        let bracket_close = bracket_open.and_then(|open| {
            matching_delimiter(
                header,
                open,
                TokenKind::LeftBracket,
                TokenKind::RightBracket,
            )
        });
        let parameters = bracket_open
            .zip(bracket_close)
            .map_or_else(Vec::new, |(open, close)| {
                parse_formal_parameter_list(&header[open + 1..close])
            });

        let type_start = bracket_close.map_or(1, |close| close + 1);
        let colon = header
            .iter()
            .enumerate()
            .skip(type_start)
            .find_map(|(index, token)| (token.kind == TokenKind::Colon).then_some(index));
        let type_end = colon.map(|colon| {
            header
                .iter()
                .enumerate()
                .skip(colon + 1)
                .find_map(|(index, token)| {
                    [
                        "read",
                        "write",
                        "stored",
                        "default",
                        "nodefault",
                        "implements",
                    ]
                    .iter()
                    .any(|word| is_keyword(token, word))
                    .then_some(index)
                })
                .unwrap_or(header.len())
        });
        let ty = colon.zip(type_end).and_then(|(colon, end)| {
            (colon + 1 < end).then(|| parse_type_syntax(&header[colon + 1..end]))
        });

        let accessor = |keyword: &str| {
            header
                .iter()
                .position(|token| is_keyword(token, keyword))
                .and_then(|index| header.get(index + 1))
                .and_then(|token| match &token.kind {
                    TokenKind::Identifier(spelling) => Some(SpannedName {
                        spelling: spelling.clone(),
                        span: token.span.clone(),
                    }),
                    _ => None,
                })
        };
        let read = accessor("read");
        let write = accessor("write");

        let mut end_exclusive = (declaration_end + 1).min(self.tokens.len());
        let mut is_default = header.iter().any(|token| is_keyword(token, "default"));
        if end_exclusive < self.tokens.len()
            && (is_keyword(&self.tokens[end_exclusive], "default")
                || is_keyword(&self.tokens[end_exclusive], "nodefault"))
        {
            is_default = is_keyword(&self.tokens[end_exclusive], "default");
            end_exclusive =
                (declaration_semicolon(self.tokens, end_exclusive) + 1).min(self.tokens.len());
        }
        let span = token_span(
            &self.tokens[start..end_exclusive],
            self.tokens[start].span.start,
        );
        self.index = end_exclusive;
        DeclarationSyntax::Property(PropertyDeclarationSyntax {
            name,
            parameters,
            ty,
            read,
            write,
            is_default,
            span,
            modes: self.tokens[start].modes,
        })
    }

    fn unsupported_to(&mut self, end: usize) -> DeclarationSyntax {
        self.unsupported += 1;
        let tokens = self.tokens[self.index..end].to_vec();
        let span = token_span(&tokens, self.tokens[self.index].span.start);
        DeclarationSyntax::Unsupported { tokens, span }
    }
}

pub(crate) fn declaration_prefix_source_end(
    nodes: &[CstNode],
    allow_bodies: bool,
) -> Option<usize> {
    let mut tokens = Vec::new();
    for node in nodes {
        flatten_node(node, &mut tokens);
    }
    let mut scanner = DeclarationScanner::new(&tokens, allow_bodies, false);
    let _ = scanner.parse_all();
    tokens
        .get(scanner.index)
        .map(|token| token.span.start)
        .or_else(|| nodes.last().map(|node| node.span().end))
}

fn count_declarations(declarations: &[DeclarationSyntax]) -> (usize, usize) {
    let mut total = 0;
    let mut unsupported = 0;
    for declaration in declarations {
        total += 1;
        match declaration {
            DeclarationSyntax::TypeSection { declarations, .. } => {
                for declaration in declarations {
                    if let TypeSyntaxKind::Aggregate {
                        members, variant, ..
                    } = &declaration.ty.kind
                    {
                        let nested = count_declarations(members);
                        total += nested.0;
                        unsupported += nested.1;
                        if let Some(variant) = variant {
                            for alternative in &variant.alternatives {
                                let nested = count_declarations(&alternative.members);
                                total += nested.0;
                                unsupported += nested.1;
                            }
                        }
                    }
                    if matches!(declaration.ty.kind, TypeSyntaxKind::Unsupported(_)) {
                        unsupported += 1;
                    }
                }
            }
            DeclarationSyntax::Routine(routine) => {
                let nested = count_declarations(&routine.body_declarations);
                total += nested.0;
                unsupported += nested.1;
            }
            DeclarationSyntax::Unsupported { .. } => unsupported += 1,
            _ => {}
        }
    }
    (total, unsupported)
}

pub fn parse_file_declarations(file: &PascalFile) -> DeclarationParseOutput {
    let mut sections = Vec::new();
    let mut diagnostics = Vec::new();
    let mut declaration_count = 0;
    let mut unsupported_count = 0;
    for section in &file.sections {
        if !matches!(
            section.kind,
            PascalSectionKind::Interface
                | PascalSectionKind::Implementation
                | PascalSectionKind::Declarations
        ) {
            continue;
        }
        let tokens = flatten_range(file, section.nodes.clone());
        let allow_bodies = section.kind != PascalSectionKind::Interface;
        let mut scanner = DeclarationScanner::new(&tokens, allow_bodies, false);
        let declarations = scanner.parse_all();
        let counts = count_declarations(&declarations);
        declaration_count += counts.0;
        unsupported_count += counts.1;
        diagnostics.extend(scanner.diagnostics);
        sections.push(ParsedDeclarationSection {
            kind: section.kind,
            declarations,
            span: section.span.clone(),
        });
    }
    DeclarationParseOutput {
        sections,
        diagnostics,
        declaration_count,
        unsupported_count,
    }
}
