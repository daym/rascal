use std::ops::Range;

use crate::{Diagnostic, ModeSnapshot, Span, Token};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PascalFileKind {
    Unit,
    Program,
    Library,
    Package,
    BareProgram,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delimiter {
    Parentheses,
    Brackets,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CstNode {
    Token(Token),
    Group {
        delimiter: Delimiter,
        open: Token,
        children: Vec<CstNode>,
        close: Token,
        span: Span,
    },
}

impl CstNode {
    pub fn span(&self) -> Span {
        match self {
            Self::Token(token) => token.span.clone(),
            Self::Group { span, .. } => span.clone(),
        }
    }

    pub fn token(&self) -> Option<&Token> {
        match self {
            Self::Token(token) => Some(token),
            Self::Group { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PascalSectionKind {
    Interface,
    Implementation,
    Declarations,
    Initialization,
    Finalization,
    Body,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PascalSection {
    pub kind: PascalSectionKind,
    pub nodes: Range<usize>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PascalFile {
    pub kind: PascalFileKind,
    pub name: Option<String>,
    pub modes: ModeSnapshot,
    pub header: Range<usize>,
    pub sections: Vec<PascalSection>,
    pub nodes: Vec<CstNode>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PascalParseOutput {
    pub file: Option<PascalFile>,
    pub diagnostics: Vec<Diagnostic>,
}
