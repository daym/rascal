use crate::{Literal, Operator, Span};

use super::{EnvironmentId, ExplicitConversion, SymbolId, TypeRef, ValueConversion};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundBody {
    pub owner: Option<TypeRef>,
    pub environment: EnvironmentId,
    pub statements: Vec<BoundStatement>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundStatement {
    pub kind: BoundStatementKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundStatementKind {
    Expression(BoundExpression),
    Assignment(BoundExpression),
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundExpression {
    pub kind: BoundExpressionKind,
    pub ty: Option<TypeRef>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundExpressionKind {
    Symbol {
        symbol: SymbolId,
    },
    Literal(Literal),
    Application {
        target: BoundApplicationTarget,
        operands: Vec<BoundExpression>,
    },
    Member {
        base: Box<BoundExpression>,
        symbol: SymbolId,
    },
    Index {
        base: Box<BoundExpression>,
        indices: Vec<BoundExpression>,
    },
    Dereference(Box<BoundExpression>),
    Set(Vec<BoundExpression>),
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundApplicationTarget {
    Routine {
        candidates: Vec<SymbolId>,
        selected: Option<SymbolId>,
    },
    CallableValue {
        symbol: Option<SymbolId>,
        callable_type: TypeRef,
    },
    Conversion {
        destination: TypeRef,
        conversion: Option<ExplicitConversion>,
    },
    Operator {
        operator: Operator,
        candidates: Vec<SymbolId>,
        selected: Option<SymbolId>,
        operand_conversions: Vec<Option<ValueConversion>>,
    },
    Invalid,
}
