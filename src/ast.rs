use std::ops::Range;

pub type Span = Range<usize>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModeSnapshot {
    pub var_string_checks: bool,
    pub range_checks: bool,
    pub overflow_checks: bool,
    pub io_checks: bool,
    pub complete_boolean_eval: bool,
}

impl Default for ModeSnapshot {
    fn default() -> Self {
        Self {
            var_string_checks: true,
            range_checks: false,
            overflow_checks: false,
            io_checks: true,
            complete_boolean_eval: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Literal {
    Integer(i128),
    Real(String),
    String(String),
    Boolean(bool),
    Nil,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operator {
    Assign,
    Positive,
    Negative,
    Not,
    Address,
    ProcedureSlotAddress,
    Multiply,
    RealDivide,
    IntegerDivide,
    Modulo,
    And,
    ShiftLeft,
    ShiftRight,
    Add,
    Subtract,
    Or,
    Xor,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    In,
    Is,
    As,
}

impl Operator {
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Assign => ":=",
            Self::Positive | Self::Add => "+",
            Self::Negative | Self::Subtract => "-",
            Self::Not => "not",
            Self::Address => "@",
            Self::ProcedureSlotAddress => "@@",
            Self::Multiply => "*",
            Self::RealDivide => "/",
            Self::IntegerDivide => "div",
            Self::Modulo => "mod",
            Self::And => "and",
            Self::ShiftLeft => "shl",
            Self::ShiftRight => "shr",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::Equal => "=",
            Self::NotEqual => "<>",
            Self::Less => "<",
            Self::Greater => ">",
            Self::LessEqual => "<=",
            Self::GreaterEqual => ">=",
            Self::In => "in",
            Self::Is => "is",
            Self::As => "as",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationSyntax {
    Call,
    Prefix,
    Infix,
    Assignment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Callee {
    Expression(Box<Expr>),
    Operator(Operator),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Application {
    pub callee: Callee,
    pub operands: Vec<Expr>,
    pub syntax: ApplicationSyntax,
    pub modes: ModeSnapshot,
    pub span: Span,
}

impl Application {
    pub fn call(callee: Expr, operands: Vec<Expr>, modes: ModeSnapshot, end: usize) -> Self {
        let start = callee.span.start;
        Self {
            callee: Callee::Expression(Box::new(callee)),
            operands,
            syntax: ApplicationSyntax::Call,
            modes,
            span: start..end,
        }
    }

    pub fn operator(
        operator: Operator,
        operands: Vec<Expr>,
        syntax: ApplicationSyntax,
        modes: ModeSnapshot,
        span: Span,
    ) -> Self {
        Self {
            callee: Callee::Operator(operator),
            operands,
            syntax,
            modes,
            span,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetElement {
    Value(Expr),
    Range { low: Expr, high: Expr },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExprKind {
    Identifier(String),
    Inherited(Option<String>),
    Literal(Literal),
    Application(Application),
    Member {
        base: Box<Expr>,
        member: String,
    },
    Index {
        base: Box<Expr>,
        indices: Vec<Expr>,
        range_checks: bool,
        modes: ModeSnapshot,
    },
    Dereference(Box<Expr>),
    Set(Vec<SetElement>),
    Error,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn application(application: Application) -> Self {
        let span = application.span.clone();
        Self::new(ExprKind::Application(application), span)
    }

    pub fn error(span: Span) -> Self {
        Self::new(ExprKind::Error, span)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    Expression(Expr),
    Assignment(Application),
    Compound {
        statements: Vec<Statement>,
        span: Span,
    },
    If {
        condition: Expr,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
        modes: ModeSnapshot,
        span: Span,
    },
    While {
        condition: Expr,
        body: Box<Statement>,
        modes: ModeSnapshot,
        span: Span,
    },
    Repeat {
        body: Vec<Statement>,
        condition: Expr,
        modes: ModeSnapshot,
        span: Span,
    },
    For {
        control: String,
        initial: Expr,
        direction: ForDirection,
        final_value: Expr,
        body: Box<Statement>,
        span: Span,
        modes: ModeSnapshot,
    },
    ForIn {
        control: String,
        source: Expr,
        body: Box<Statement>,
        span: Span,
        modes: ModeSnapshot,
    },
    Case {
        selector: Expr,
        arms: Vec<CaseArm>,
        otherwise: Vec<Statement>,
        span: Span,
    },
    With {
        receivers: Vec<Expr>,
        body: Box<Statement>,
        span: Span,
    },
    Try {
        body: Vec<Statement>,
        continuation: TryContinuation,
        span: Span,
    },
    Raise {
        value: Option<Expr>,
        address: Option<Expr>,
        frame: Option<Expr>,
        span: Span,
    },
    Goto {
        label: String,
        span: Span,
    },
    Label {
        label: String,
        statement: Box<Statement>,
        span: Span,
    },
    Break(Span),
    Continue(Span),
    Exit {
        value: Option<Expr>,
        modes: ModeSnapshot,
        span: Span,
    },
    InlineVariable {
        names: Vec<String>,
        type_name: Option<Vec<String>>,
        initializer: Option<Expr>,
        modes: ModeSnapshot,
        span: Span,
    },
    Empty(Span),
    Error(Span),
}

impl Statement {
    pub fn span(&self) -> Span {
        match self {
            Self::Expression(expression) => expression.span.clone(),
            Self::Assignment(application) => application.span.clone(),
            Self::Compound { span, .. }
            | Self::If { span, .. }
            | Self::While { span, .. }
            | Self::Repeat { span, .. }
            | Self::For { span, .. }
            | Self::ForIn { span, .. }
            | Self::Case { span, .. }
            | Self::With { span, .. }
            | Self::Try { span, .. }
            | Self::Raise { span, .. }
            | Self::Goto { span, .. }
            | Self::Label { span, .. }
            | Self::Break(span)
            | Self::Continue(span)
            | Self::Exit { span, .. }
            | Self::InlineVariable { span, .. }
            | Self::Empty(span)
            | Self::Error(span) => span.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForDirection {
    To,
    DownTo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaseLabel {
    Value(Expr),
    Range { low: Expr, high: Expr },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseArm {
    pub labels: Vec<CaseLabel>,
    pub statement: Statement,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TryContinuation {
    Finally(Vec<Statement>),
    Except {
        handlers: Vec<ExceptionHandler>,
        otherwise: Vec<Statement>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExceptionHandler {
    pub variable: Option<String>,
    pub exception_type: String,
    pub body: Statement,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseOutput {
    pub statements: Vec<Statement>,
    pub diagnostics: Vec<Diagnostic>,
}
