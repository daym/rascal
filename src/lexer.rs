use logos::Logos;

use crate::ast::{Diagnostic, ModeSnapshot, Span};

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
enum RawToken {
    #[regex(r"\{\$[vVrRqQiIbB][+-]\}", parse_directive, priority = 3)]
    Directive(ModeDirective),

    #[regex(r"\{[^}]*\}", logos::skip, priority = 1)]
    #[regex(r"\(\*([^*]|\*+[^*)])*\*+\)", logos::skip)]
    #[regex(r"//[^\r\n]*", logos::skip, allow_greedy = true)]
    Comment,

    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", |lex| lex.slice().to_ascii_lowercase())]
    Identifier(String),

    #[regex(r"[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?", |lex| lex.slice().to_owned())]
    #[regex(r"[0-9]+[eE][+-]?[0-9]+", |lex| lex.slice().to_owned())]
    Real(String),

    #[regex(r"\$[0-9A-Fa-f]+", |lex| {
        i128::from_str_radix(&lex.slice()[1..], 16).map_err(|_| ())
    })]
    #[regex(r"%[01]+", |lex| {
        i128::from_str_radix(&lex.slice()[1..], 2).map_err(|_| ())
    })]
    #[regex(r"&[0-7]+", |lex| {
        i128::from_str_radix(&lex.slice()[1..], 8).map_err(|_| ())
    })]
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i128>().map_err(|_| ()))]
    Integer(i128),

    #[regex(r"'([^']|'')*'", parse_string)]
    #[regex(r"#[0-9]+", parse_decimal_character)]
    #[regex(r"#\$[0-9A-Fa-f]+", parse_hex_character)]
    String(String),

    #[token("@@")]
    AtAt,
    #[token(":=")]
    Assign,
    #[token("..")]
    DotDot,
    #[token("<>")]
    NotEqual,
    #[token("<=")]
    LessEqual,
    #[token(">=")]
    GreaterEqual,

    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token(".")]
    Dot,
    #[token("^")]
    Caret,
    #[token("@")]
    At,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("=")]
    Equal,
    #[token("<")]
    Less,
    #[token(">")]
    Greater,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModeDirective {
    VarString(bool),
    Range(bool),
    Overflow(bool),
    Io(bool),
    CompleteBoolean(bool),
}

fn parse_directive(lex: &mut logos::Lexer<'_, RawToken>) -> ModeDirective {
    let bytes = lex.slice().as_bytes();
    let enabled = bytes[3] == b'+';
    match bytes[2].to_ascii_lowercase() {
        b'v' => ModeDirective::VarString(enabled),
        b'r' => ModeDirective::Range(enabled),
        b'q' => ModeDirective::Overflow(enabled),
        b'i' => ModeDirective::Io(enabled),
        b'b' => ModeDirective::CompleteBoolean(enabled),
        _ => unreachable!("the directive regex admits only V, R, Q, I, or B"),
    }
}

fn parse_string(lex: &mut logos::Lexer<'_, RawToken>) -> String {
    let quoted = lex.slice();
    quoted[1..quoted.len() - 1].replace("''", "'")
}

fn character_string(value: u32) -> Result<String, ()> {
    char::from_u32(value)
        .map(|character| character.to_string())
        .ok_or(())
}

fn parse_decimal_character(lex: &mut logos::Lexer<'_, RawToken>) -> Result<String, ()> {
    lex.slice()[1..]
        .parse::<u32>()
        .map_err(|_| ())
        .and_then(character_string)
}

fn parse_hex_character(lex: &mut logos::Lexer<'_, RawToken>) -> Result<String, ()> {
    u32::from_str_radix(&lex.slice()[2..], 16)
        .map_err(|_| ())
        .and_then(character_string)
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Identifier(String),
    Integer(i128),
    Real(String),
    String(String),
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Comma,
    Colon,
    Semicolon,
    Dot,
    DotDot,
    Caret,
    At,
    AtAt,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Error,
}

impl TokenKind {
    pub fn describe(&self) -> String {
        match self {
            Self::Identifier(name) => format!("identifier `{name}`"),
            Self::Integer(value) => format!("integer `{value}`"),
            Self::Real(value) => format!("real `{value}`"),
            Self::String(_) => "string literal".to_owned(),
            Self::LeftParen => "`(`".to_owned(),
            Self::RightParen => "`)`".to_owned(),
            Self::LeftBracket => "`[`".to_owned(),
            Self::RightBracket => "`]`".to_owned(),
            Self::Comma => "`,`".to_owned(),
            Self::Colon => "`:`".to_owned(),
            Self::Semicolon => "`;`".to_owned(),
            Self::Dot => "`.`".to_owned(),
            Self::DotDot => "`..`".to_owned(),
            Self::Caret => "`^`".to_owned(),
            Self::At => "`@`".to_owned(),
            Self::AtAt => "`@@`".to_owned(),
            Self::Assign => "`:=`".to_owned(),
            Self::Plus => "`+`".to_owned(),
            Self::Minus => "`-`".to_owned(),
            Self::Star => "`*`".to_owned(),
            Self::Slash => "`/`".to_owned(),
            Self::Equal => "`=`".to_owned(),
            Self::NotEqual => "`<>`".to_owned(),
            Self::Less => "`<`".to_owned(),
            Self::Greater => "`>`".to_owned(),
            Self::LessEqual => "`<=`".to_owned(),
            Self::GreaterEqual => "`>=`".to_owned(),
            Self::Error => "invalid token".to_owned(),
        }
    }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.describe())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub modes: ModeSnapshot,
}

impl std::fmt::Display for Token {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(formatter)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

fn apply_directive(modes: &mut ModeSnapshot, directive: ModeDirective) {
    match directive {
        ModeDirective::VarString(enabled) => modes.var_string_checks = enabled,
        ModeDirective::Range(enabled) => modes.range_checks = enabled,
        ModeDirective::Overflow(enabled) => modes.overflow_checks = enabled,
        ModeDirective::Io(enabled) => modes.io_checks = enabled,
        ModeDirective::CompleteBoolean(enabled) => modes.complete_boolean_eval = enabled,
    }
}

fn lower_raw(raw: RawToken) -> Option<TokenKind> {
    Some(match raw {
        RawToken::Directive(_) | RawToken::Comment => return None,
        RawToken::Identifier(value) => TokenKind::Identifier(value),
        RawToken::Integer(value) => TokenKind::Integer(value),
        RawToken::Real(value) => TokenKind::Real(value),
        RawToken::String(value) => TokenKind::String(value),
        RawToken::LeftParen => TokenKind::LeftParen,
        RawToken::RightParen => TokenKind::RightParen,
        RawToken::LeftBracket => TokenKind::LeftBracket,
        RawToken::RightBracket => TokenKind::RightBracket,
        RawToken::Comma => TokenKind::Comma,
        RawToken::Colon => TokenKind::Colon,
        RawToken::Semicolon => TokenKind::Semicolon,
        RawToken::Dot => TokenKind::Dot,
        RawToken::DotDot => TokenKind::DotDot,
        RawToken::Caret => TokenKind::Caret,
        RawToken::At => TokenKind::At,
        RawToken::AtAt => TokenKind::AtAt,
        RawToken::Assign => TokenKind::Assign,
        RawToken::Plus => TokenKind::Plus,
        RawToken::Minus => TokenKind::Minus,
        RawToken::Star => TokenKind::Star,
        RawToken::Slash => TokenKind::Slash,
        RawToken::Equal => TokenKind::Equal,
        RawToken::NotEqual => TokenKind::NotEqual,
        RawToken::Less => TokenKind::Less,
        RawToken::Greater => TokenKind::Greater,
        RawToken::LessEqual => TokenKind::LessEqual,
        RawToken::GreaterEqual => TokenKind::GreaterEqual,
    })
}

pub fn lex(source: &str) -> LexOutput {
    let mut modes = ModeSnapshot::default();
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();

    for (raw, span) in RawToken::lexer(source).spanned() {
        match raw {
            Ok(RawToken::Directive(directive)) => apply_directive(&mut modes, directive),
            Ok(raw) => {
                if let Some(kind) = lower_raw(raw) {
                    tokens.push(Token { kind, span, modes });
                }
            }
            Err(()) => {
                diagnostics.push(Diagnostic::new(span.clone(), "invalid source token"));
                tokens.push(Token {
                    kind: TokenKind::Error,
                    span,
                    modes,
                });
            }
        }
    }

    LexOutput {
        tokens,
        diagnostics,
    }
}
