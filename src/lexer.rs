use logos::Logos;

use crate::{
    ast::{Diagnostic, ModeSnapshot, SourceId, SourceSpan, Span},
    preprocessor::{DirectiveState, DirectiveStateId, PreprocessorOptions, preprocess},
};

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
pub(crate) enum RawToken {
    #[regex(r"\{\$[^}]*\}", parse_directive_body, priority = 100)]
    #[regex(r"\(\*\$[^*]*\*\)", parse_directive_body, priority = 100)]
    Directive(String),

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

fn parse_directive_body(lex: &mut logos::Lexer<'_, RawToken>) -> String {
    let slice = lex.slice();
    if let Some(body) = slice
        .strip_prefix("{$")
        .and_then(|body| body.strip_suffix('}'))
    {
        body.to_owned()
    } else if let Some(body) = slice
        .strip_prefix("(*$")
        .and_then(|body| body.strip_suffix("*)"))
    {
        body.to_owned()
    } else {
        unreachable!("directive regex guarantees one supported delimiter")
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
    /// Monotonic location in the preprocessed token stream.
    pub span: Span,
    /// Physical source location before include expansion.
    pub origin: SourceSpan,
    pub modes: ModeSnapshot,
    pub directive_state: DirectiveStateId,
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
    pub sources: Vec<SourceInfo>,
    pub dependencies: Vec<IncludeDependency>,
    pub directives: Vec<DirectiveEvent>,
    pub macro_expansions: Vec<MacroExpansion>,
    pub source_map: Vec<SourceMapEntry>,
    pub directive_states: Vec<DirectiveState>,
    pub final_directive_state: DirectiveStateId,
    pub logical_len: usize,
}

impl LexOutput {
    pub fn source(&self, source: SourceId) -> Option<&SourceInfo> {
        self.sources.get(source.as_u32() as usize)
    }

    pub fn directive_state(&self, state: DirectiveStateId) -> Option<&DirectiveState> {
        self.directive_states.get(state.as_u32() as usize)
    }

    pub fn physical_text(&self, span: &SourceSpan) -> Option<&str> {
        self.source(span.source)?.text.get(span.range.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceInfo {
    pub id: SourceId,
    pub name: String,
    pub text: String,
    pub byte_len: usize,
    pub line_starts: Vec<usize>,
    pub included_from: Option<SourceSpan>,
    pub synthetic: bool,
}

impl SourceInfo {
    pub fn line_column(&self, byte_offset: usize) -> Option<(usize, usize)> {
        if byte_offset > self.byte_len {
            return None;
        }
        let line = self
            .line_starts
            .partition_point(|start| *start <= byte_offset);
        let line_index = line.saturating_sub(1);
        Some((
            line_index + 1,
            byte_offset.saturating_sub(self.line_starts[line_index]) + 1,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncludeDependency {
    pub directive: SourceSpan,
    pub included: SourceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectiveEvent {
    pub name: String,
    pub origin: SourceSpan,
    pub active: bool,
    pub recognized: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacroExpansion {
    pub name: String,
    pub invocation: SourceSpan,
    pub expanded_source: SourceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceMapEntryKind {
    Token,
    Directive,
    MacroInvocation,
    Inactive,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceMapEntry {
    pub logical: Span,
    pub physical: SourceSpan,
    pub kind: SourceMapEntryKind,
}

pub(crate) fn lower_raw(raw: RawToken) -> Option<TokenKind> {
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RawLexeme {
    pub token: Result<RawToken, ()>,
    pub span: Span,
}

pub(crate) fn raw_lex(source: &str) -> Vec<RawLexeme> {
    RawToken::lexer(source)
        .spanned()
        .map(|(token, span)| RawLexeme { token, span })
        .collect()
}

pub fn lex(source: &str) -> LexOutput {
    lex_named("<memory>", source)
}

pub fn lex_named(source_name: &str, source: &str) -> LexOutput {
    preprocess(source_name, source, &PreprocessorOptions::default())
}
