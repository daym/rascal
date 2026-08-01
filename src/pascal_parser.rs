use std::ops::Range;

use chumsky::{error::Rich, extra, prelude::*};

use crate::{
    Diagnostic, ModeSnapshot, Span, Token, TokenKind,
    lexer::lex,
    pascal_ast::{
        CstNode, Delimiter, PascalFile, PascalFileKind, PascalParseOutput, PascalSection,
        PascalSectionKind,
    },
};

type Extra<'a> = extra::Err<Rich<'a, Token>>;

fn symbol<'a>(expected: TokenKind) -> impl Parser<'a, &'a [Token], Token, Extra<'a>> + Clone {
    any().filter(move |token: &Token| token.kind == expected)
}

fn cst_node_parser<'a>() -> impl Parser<'a, &'a [Token], CstNode, Extra<'a>> + Clone {
    recursive(
        |node: Recursive<dyn Parser<'a, &'a [Token], CstNode, Extra<'a>>>| {
            let children = node.clone().repeated().collect::<Vec<_>>();
            let parentheses = symbol(TokenKind::LeftParen)
                .then(children.clone())
                .then(symbol(TokenKind::RightParen))
                .map(|((open, children), close)| CstNode::Group {
                    delimiter: Delimiter::Parentheses,
                    span: open.span.start..close.span.end,
                    open,
                    children,
                    close,
                });
            let brackets = symbol(TokenKind::LeftBracket)
                .then(children)
                .then(symbol(TokenKind::RightBracket))
                .map(|((open, children), close)| CstNode::Group {
                    delimiter: Delimiter::Brackets,
                    span: open.span.start..close.span.end,
                    open,
                    children,
                    close,
                });
            let token = any()
                .filter(|token: &Token| {
                    !matches!(
                        token.kind,
                        TokenKind::LeftParen
                            | TokenKind::RightParen
                            | TokenKind::LeftBracket
                            | TokenKind::RightBracket
                    )
                })
                .map(CstNode::Token);
            choice((parentheses, brackets, token))
        },
    )
}

fn token_index_span(segment: &[Token], span: Range<usize>, eof: usize) -> Span {
    if segment.is_empty() {
        return eof..eof;
    }
    if span.start >= segment.len() {
        let end = segment.last().map_or(eof, |token| token.span.end);
        return end..end;
    }
    let start = segment[span.start].span.start;
    let end = if span.end <= span.start {
        start
    } else {
        segment
            .get(span.end - 1)
            .map_or(start, |token| token.span.end)
    };
    start..end
}

fn keyword(node: &CstNode, expected: &str) -> bool {
    matches!(
        node.token().map(|token| &token.kind),
        Some(TokenKind::Identifier(name)) if name == expected
    )
}

fn symbol_node(node: &CstNode, expected: TokenKind) -> bool {
    node.token().is_some_and(|token| token.kind == expected)
}

fn identifier(node: &CstNode) -> Option<&str> {
    match node.token().map(|token| &token.kind) {
        Some(TokenKind::Identifier(name)) => Some(name),
        _ => None,
    }
}

fn node_range_span(nodes: &[CstNode], range: Range<usize>, fallback: usize) -> Span {
    nodes
        .get(range.start)
        .zip(range.end.checked_sub(1).and_then(|end| nodes.get(end)))
        .map_or(fallback..fallback, |(first, last)| {
            first.span().start..last.span().end
        })
}

fn section(
    kind: PascalSectionKind,
    nodes: &[CstNode],
    range: Range<usize>,
    fallback: usize,
) -> PascalSection {
    PascalSection {
        kind,
        span: node_range_span(nodes, range.clone(), fallback),
        nodes: range,
    }
}

fn next_is_keyword(nodes: &[CstNode], index: usize, words: &[&str]) -> bool {
    nodes
        .get(index + 1)
        .is_some_and(|node| words.iter().any(|word| keyword(node, word)))
}

fn opens_type_block(nodes: &[CstNode], index: usize) -> bool {
    if keyword(&nodes[index], "record") {
        return true;
    }
    if keyword(&nodes[index], "object") {
        return index == 0 || !keyword(&nodes[index - 1], "of");
    }
    if keyword(&nodes[index], "interface") {
        return !nodes
            .get(index + 1)
            .is_some_and(|node| symbol_node(node, TokenKind::Semicolon));
    }
    if !keyword(&nodes[index], "class") {
        return false;
    }
    if next_is_keyword(
        nodes,
        index,
        &[
            "of",
            "procedure",
            "function",
            "constructor",
            "destructor",
            "operator",
            "var",
        ],
    ) {
        return false;
    }
    if matches!(nodes.get(index + 1), Some(CstNode::Group { .. }))
        && nodes
            .get(index + 2)
            .is_some_and(|node| symbol_node(node, TokenKind::Semicolon))
    {
        return false;
    }
    !nodes
        .get(index + 1)
        .is_some_and(|node| symbol_node(node, TokenKind::Semicolon))
}

fn find_at_depth_zero(nodes: &[CstNode], start: usize, alternatives: &[&str]) -> Option<usize> {
    let mut depth = 0usize;
    for index in start..nodes.len() {
        let node = &nodes[index];
        if depth == 0 && alternatives.iter().any(|word| keyword(node, word)) {
            return Some(index);
        }
        if opens_type_block(nodes, index) || keyword(node, "begin") {
            depth += 1;
        } else if keyword(node, "end") && depth > 0 {
            depth -= 1;
        }
    }
    None
}

fn final_end_index(nodes: &[CstNode]) -> Option<usize> {
    nodes.len().checked_sub(2).filter(|index| {
        keyword(&nodes[*index], "end")
            && nodes
                .get(*index + 1)
                .is_some_and(|node| symbol_node(node, TokenKind::Dot))
    })
}

fn find_header_end(nodes: &[CstNode], start: usize) -> Option<usize> {
    nodes[start..]
        .iter()
        .position(|node| symbol_node(node, TokenKind::Semicolon))
        .map(|offset| start + offset + 1)
}

fn analyze_nodes(nodes: Vec<CstNode>, source_len: usize) -> PascalParseOutput {
    let mut diagnostics = Vec::new();
    let Some(first) = nodes.first() else {
        return PascalParseOutput {
            file: None,
            diagnostics: vec![Diagnostic::new(0..0, "empty Pascal source")],
        };
    };
    let modes = first
        .token()
        .map_or_else(ModeSnapshot::default, |token| token.modes);
    let (kind, name, header_end) = if keyword(first, "unit") {
        let name = nodes.get(1).and_then(identifier).map(str::to_owned);
        let end = find_header_end(&nodes, 1);
        (PascalFileKind::Unit, name, end)
    } else if keyword(first, "program") {
        let name = nodes.get(1).and_then(identifier).map(str::to_owned);
        let end = find_header_end(&nodes, 1);
        (PascalFileKind::Program, name, end)
    } else if keyword(first, "library") {
        let name = nodes.get(1).and_then(identifier).map(str::to_owned);
        let end = find_header_end(&nodes, 1);
        (PascalFileKind::Library, name, end)
    } else if keyword(first, "package") {
        let name = nodes.get(1).and_then(identifier).map(str::to_owned);
        let end = find_header_end(&nodes, 1);
        (PascalFileKind::Package, name, end)
    } else if keyword(first, "begin") {
        (PascalFileKind::BareProgram, None, Some(0))
    } else {
        diagnostics.push(Diagnostic::new(
            first.span(),
            "expected `unit`, `program`, `library`, `package`, or `begin`",
        ));
        (PascalFileKind::BareProgram, None, Some(0))
    };
    let header_end = header_end.unwrap_or_else(|| {
        diagnostics.push(Diagnostic::new(
            first.span(),
            "missing semicolon after Pascal file header",
        ));
        nodes.len().min(2)
    });
    if kind != PascalFileKind::BareProgram && name.is_none() {
        diagnostics.push(Diagnostic::new(first.span(), "missing Pascal file name"));
    }

    let final_end = final_end_index(&nodes).unwrap_or_else(|| {
        diagnostics.push(Diagnostic::new(
            nodes.last().map_or(source_len..source_len, CstNode::span),
            "Pascal file must end with `end.`",
        ));
        nodes.len()
    });
    let mut sections = Vec::new();

    match kind {
        PascalFileKind::Unit => {
            let interface = nodes
                .get(header_end)
                .filter(|node| keyword(node, "interface"))
                .map(|_| header_end);
            let Some(interface) = interface else {
                diagnostics.push(Diagnostic::new(
                    nodes
                        .get(header_end)
                        .map_or(source_len..source_len, CstNode::span),
                    "unit header must be followed by `interface`",
                ));
                return PascalParseOutput {
                    file: Some(PascalFile {
                        kind,
                        name,
                        modes,
                        header: 0..header_end,
                        sections,
                        nodes,
                        span: 0..source_len,
                    }),
                    diagnostics,
                };
            };
            let implementation = find_at_depth_zero(&nodes, interface + 1, &["implementation"]);
            let Some(implementation) = implementation else {
                diagnostics.push(Diagnostic::new(
                    nodes[interface].span(),
                    "unit is missing its `implementation` section",
                ));
                sections.push(section(
                    PascalSectionKind::Interface,
                    &nodes,
                    interface + 1..final_end,
                    source_len,
                ));
                return PascalParseOutput {
                    file: Some(PascalFile {
                        kind,
                        name,
                        modes,
                        header: 0..header_end,
                        sections,
                        nodes,
                        span: 0..source_len,
                    }),
                    diagnostics,
                };
            };
            sections.push(section(
                PascalSectionKind::Interface,
                &nodes,
                interface + 1..implementation,
                source_len,
            ));

            let tail = find_at_depth_zero(
                &nodes,
                implementation + 1,
                &["initialization", "finalization", "begin", "end"],
            )
            .unwrap_or(final_end);
            let tail = crate::declaration_parser::declaration_prefix_source_end(
                &nodes[implementation + 1..final_end],
                true,
            )
            .and_then(|source_end| {
                nodes[implementation + 1..final_end]
                    .iter()
                    .position(|node| node.span().start >= source_end)
                    .map(|offset| implementation + 1 + offset)
            })
            .unwrap_or(tail);
            sections.push(section(
                PascalSectionKind::Implementation,
                &nodes,
                implementation + 1..tail,
                source_len,
            ));
            if tail < final_end && keyword(&nodes[tail], "initialization") {
                let finalization = find_at_depth_zero(&nodes, tail + 1, &["finalization", "end"])
                    .unwrap_or(final_end);
                sections.push(section(
                    PascalSectionKind::Initialization,
                    &nodes,
                    tail + 1..finalization,
                    source_len,
                ));
                if finalization < final_end && keyword(&nodes[finalization], "finalization") {
                    sections.push(section(
                        PascalSectionKind::Finalization,
                        &nodes,
                        finalization + 1..final_end,
                        source_len,
                    ));
                }
            } else if tail < final_end && keyword(&nodes[tail], "finalization") {
                sections.push(section(
                    PascalSectionKind::Finalization,
                    &nodes,
                    tail + 1..final_end,
                    source_len,
                ));
            } else if tail < final_end && keyword(&nodes[tail], "begin") {
                sections.push(section(
                    PascalSectionKind::Body,
                    &nodes,
                    tail..final_end + 1,
                    source_len,
                ));
            }
        }
        PascalFileKind::Program | PascalFileKind::Library | PascalFileKind::BareProgram => {
            let fallback_body_start = if kind == PascalFileKind::BareProgram {
                0
            } else {
                find_at_depth_zero(&nodes, header_end, &["begin"]).unwrap_or(final_end)
            };
            let body_start = if kind == PascalFileKind::BareProgram {
                0
            } else {
                crate::declaration_parser::declaration_prefix_source_end(
                    &nodes[header_end..final_end],
                    true,
                )
                .and_then(|source_end| {
                    nodes[header_end..final_end]
                        .iter()
                        .position(|node| node.span().start >= source_end)
                        .map(|offset| header_end + offset)
                })
                .unwrap_or(fallback_body_start)
            };
            if header_end < body_start {
                sections.push(section(
                    PascalSectionKind::Declarations,
                    &nodes,
                    header_end..body_start,
                    source_len,
                ));
            }
            sections.push(section(
                PascalSectionKind::Body,
                &nodes,
                body_start..final_end.saturating_add(1).min(nodes.len()),
                source_len,
            ));
        }
        PascalFileKind::Package => {
            sections.push(section(
                PascalSectionKind::Declarations,
                &nodes,
                header_end..final_end,
                source_len,
            ));
        }
    }

    PascalParseOutput {
        file: Some(PascalFile {
            kind,
            name,
            modes,
            header: 0..header_end,
            sections,
            nodes,
            span: 0..source_len,
        }),
        diagnostics,
    }
}

pub fn parse_tokens(tokens: &[Token], source_len: usize) -> PascalParseOutput {
    let (nodes, errors) = cst_node_parser()
        .repeated()
        .collect::<Vec<_>>()
        .then_ignore(end())
        .parse(tokens)
        .into_output_errors();
    let mut output = nodes.map_or_else(
        || PascalParseOutput {
            file: None,
            diagnostics: Vec::new(),
        },
        |nodes| analyze_nodes(nodes, source_len),
    );
    output.diagnostics.splice(
        0..0,
        errors.into_iter().map(|error| {
            Diagnostic::new(
                token_index_span(tokens, error.span().into_range(), source_len),
                format!("chumsky: {error}"),
            )
        }),
    );
    output
}

pub fn parse(source: &str) -> PascalParseOutput {
    let lexed = lex(source);
    let mut output = parse_tokens(&lexed.tokens, source.len());
    output.diagnostics.splice(0..0, lexed.diagnostics);
    output
}
