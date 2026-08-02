use crate::{ForDirection, Literal, ModeSnapshot, Operator, Span};

use super::{
    ApplicationResolution, ConversionResolution, EnvironmentId, ReceiverId, SymbolId, TypeRef,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticUse {
    Value,
    MutablePlace,
    AssignmentTarget,
    Condition,
    Address,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpressionCategory {
    Storage { mutable: bool },
    Property { readable: bool, writable: bool },
    Value,
    Temporary,
    Error,
}

impl ExpressionCategory {
    pub const fn is_addressable(self) -> bool {
        matches!(self, Self::Storage { .. })
    }

    pub const fn is_mutable_storage(self) -> bool {
        matches!(self, Self::Storage { mutable: true })
    }

    pub const fn is_assignment_target(self) -> bool {
        matches!(
            self,
            Self::Storage { mutable: true } | Self::Property { writable: true, .. }
        )
    }
}

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
    Assignment(BoundAssignment),
    Compound(Vec<BoundStatement>),
    If {
        condition: BoundExpression,
        then_branch: Box<BoundStatement>,
        else_branch: Option<Box<BoundStatement>>,
    },
    While {
        condition: BoundExpression,
        body: Box<BoundStatement>,
    },
    Repeat {
        body: Vec<BoundStatement>,
        condition: BoundExpression,
    },
    For {
        control: Option<SymbolId>,
        initial: BoundExpression,
        direction: ForDirection,
        final_value: BoundExpression,
        body: Box<BoundStatement>,
        modes: ModeSnapshot,
    },
    ForIn {
        control: Option<SymbolId>,
        source: BoundExpression,
        element_conversion: Option<ConversionResolution>,
        body: Box<BoundStatement>,
        modes: ModeSnapshot,
    },
    Case {
        selector: BoundExpression,
        arms: Vec<BoundCaseArm>,
        otherwise: Vec<BoundStatement>,
    },
    With {
        receivers: Vec<BoundExpression>,
        body: Box<BoundStatement>,
    },
    Try {
        body: Vec<BoundStatement>,
        continuation: BoundTryContinuation,
    },
    Raise {
        value: Option<BoundExpression>,
        address: Option<BoundExpression>,
        frame: Option<BoundExpression>,
    },
    Goto {
        label: Option<SymbolId>,
    },
    Label {
        label: Option<SymbolId>,
        statement: Box<BoundStatement>,
    },
    Break,
    Continue,
    Exit(Option<BoundExpression>),
    InlineVariable {
        symbols: Vec<SymbolId>,
        initializer: Option<BoundExpression>,
        modes: ModeSnapshot,
    },
    Empty,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundAssignment {
    pub target: BoundExpression,
    pub source: BoundExpression,
    pub conversion: Option<ConversionResolution>,
    pub modes: ModeSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundCaseArm {
    pub labels: Vec<BoundCaseLabel>,
    pub statement: BoundStatement,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundCaseLabel {
    Value(BoundExpression),
    Range {
        low: BoundExpression,
        high: BoundExpression,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundTryContinuation {
    Finally(Vec<BoundStatement>),
    Except {
        handlers: Vec<BoundExceptionHandler>,
        otherwise: Vec<BoundStatement>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundExceptionHandler {
    pub variable: Option<SymbolId>,
    pub exception_type: Option<TypeRef>,
    pub body: BoundStatement,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundExpression {
    pub kind: BoundExpressionKind,
    pub ty: Option<TypeRef>,
    pub category: ExpressionCategory,
    pub semantic_use: SemanticUse,
    pub conversion: Option<ConversionResolution>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyAccessKind {
    Read,
    Write,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundPropertyBinding {
    pub kind: PropertyAccessKind,
    pub resolution: ApplicationResolution,
    pub accessor_symbols: Vec<SymbolId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundExpressionKind {
    Symbol {
        symbol: SymbolId,
        receiver: Option<ReceiverId>,
    },
    Literal(Literal),
    Application {
        target: BoundApplicationTarget,
        callee: Option<Box<BoundExpression>>,
        operands: Vec<BoundExpression>,
        modes: ModeSnapshot,
    },
    Member {
        base: Box<BoundExpression>,
        symbol: SymbolId,
    },
    TypeIdentifier {
        symbol: SymbolId,
        instance_type: TypeRef,
    },
    Inherited {
        symbols: Vec<SymbolId>,
        forward_parameters: bool,
    },
    Property {
        base: Option<Box<BoundExpression>>,
        lookup_receiver: Option<ReceiverId>,
        symbol: SymbolId,
        indices: Vec<BoundExpression>,
        binding: Option<Box<BoundPropertyBinding>>,
    },
    RoutineDesignator {
        routine: Box<BoundExpression>,
        symbol: SymbolId,
    },
    ProcedureCode(Box<BoundExpression>),
    Address(Box<BoundExpression>),
    Index {
        base: Box<BoundExpression>,
        indices: Vec<BoundExpression>,
    },
    Dereference(Box<BoundExpression>),
    Set(Vec<BoundSetElement>),
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundSetElement {
    Value(BoundExpression),
    Range {
        low: BoundExpression,
        high: BoundExpression,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundApplicationTarget {
    Routine {
        resolution: ApplicationResolution,
    },
    CallableValue {
        resolution: ApplicationResolution,
    },
    Conversion {
        destination: TypeRef,
        resolution: ApplicationResolution,
    },
    Operator {
        operator: Operator,
        resolution: ApplicationResolution,
    },
    Invalid,
}
