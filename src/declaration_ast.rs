use crate::{Expr, ModeSnapshot, Span, Token};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpannedName {
    pub spelling: String,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AggregateSyntaxKind {
    Record,
    PackedRecord,
    Object,
    Class,
    Interface,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoutineSyntaxKind {
    Procedure,
    Function,
    Constructor,
    Destructor,
    Operator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormalModeSyntax {
    Value,
    Const,
    Var,
    Out,
    ConstRef,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallingConventionSyntax {
    Pascal,
    Register,
    Cdecl,
    Stdcall,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormalParameterSyntax {
    pub names: Vec<SpannedName>,
    pub mode: FormalModeSyntax,
    pub ty: Option<TypeSyntax>,
    pub default: Option<Expr>,
    pub modes: ModeSnapshot,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeSyntax {
    pub kind: TypeSyntaxKind,
    pub span: Span,
    pub modes: ModeSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumMemberSyntax {
    pub name: SpannedName,
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantAlternativeSyntax {
    pub labels: Vec<Expr>,
    pub members: Vec<DeclarationSyntax>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantPartSyntax {
    pub selector_name: Option<SpannedName>,
    pub selector_type: Box<TypeSyntax>,
    pub alternatives: Vec<VariantAlternativeSyntax>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeSyntaxKind {
    Named(Vec<SpannedName>),
    /// A target-backend spelling carried by the RTL declaration. Semantic
    /// binding decides the Pascal type; the frontend must not interpret this
    /// string as a Pascal identifier.
    External {
        backend_name: String,
    },
    Pointer(Box<TypeSyntax>),
    Enumeration(Vec<EnumMemberSyntax>),
    Subrange {
        lower: Expr,
        upper: Expr,
    },
    Aggregate {
        kind: AggregateSyntaxKind,
        base: Option<Box<TypeSyntax>>,
        members: Vec<DeclarationSyntax>,
        variant: Option<Box<VariantPartSyntax>>,
    },
    Procedural {
        method_pointer: bool,
        parameters: Vec<FormalParameterSyntax>,
        result: Option<Box<TypeSyntax>>,
        calling_convention: CallingConventionSyntax,
    },
    ClassForward,
    Array {
        indices: Vec<TypeSyntax>,
        element: Option<Box<TypeSyntax>>,
        dynamic: bool,
    },
    Set {
        element: Option<Box<TypeSyntax>>,
    },
    Unsupported(Vec<Token>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDeclarationSyntax {
    pub name: SpannedName,
    pub ty: TypeSyntax,
    pub distinct: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueDeclarationSyntax {
    pub names: Vec<SpannedName>,
    pub ty: Option<TypeSyntax>,
    pub initializer: Option<Expr>,
    /// Raw backend-specific spelling from `external name '...'`.
    pub external_name: Option<String>,
    pub span: Span,
    pub modes: ModeSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyDeclarationSyntax {
    pub name: SpannedName,
    pub parameters: Vec<FormalParameterSyntax>,
    pub ty: Option<TypeSyntax>,
    pub read: Option<SpannedName>,
    pub write: Option<SpannedName>,
    pub is_default: bool,
    pub span: Span,
    pub modes: ModeSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoutineDeclarationSyntax {
    pub kind: RoutineSyntaxKind,
    pub qualifier: Vec<SpannedName>,
    pub name: SpannedName,
    pub parameters: Vec<FormalParameterSyntax>,
    pub result: Option<TypeSyntax>,
    pub body_declarations: Vec<DeclarationSyntax>,
    pub body_tokens: Vec<Token>,
    pub has_body: bool,
    pub is_forward: bool,
    pub is_external: bool,
    /// Raw backend-specific spelling from `external name '...'`. Lowering,
    /// rather than Pascal name lookup, owns its interpretation.
    pub external_name: Option<String>,
    pub overload: bool,
    pub class_method: bool,
    pub static_method: bool,
    pub virtual_method: bool,
    pub override_method: bool,
    pub abstract_method: bool,
    pub final_method: bool,
    pub reintroduce: bool,
    pub calling_convention: CallingConventionSyntax,
    pub span: Span,
    pub modes: ModeSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeclarationSyntax {
    Uses {
        units: Vec<SpannedName>,
        span: Span,
    },
    TypeSection {
        declarations: Vec<TypeDeclarationSyntax>,
        span: Span,
    },
    Variables(ValueDeclarationSyntax),
    Constants(ValueDeclarationSyntax),
    Routine(RoutineDeclarationSyntax),
    Property(PropertyDeclarationSyntax),
    Labels {
        names: Vec<SpannedName>,
        span: Span,
    },
    Visibility {
        name: SpannedName,
    },
    Unsupported {
        tokens: Vec<Token>,
        span: Span,
    },
}

impl DeclarationSyntax {
    pub fn span(&self) -> Span {
        match self {
            Self::Uses { span, .. }
            | Self::TypeSection { span, .. }
            | Self::Labels { span, .. }
            | Self::Unsupported { span, .. } => span.clone(),
            Self::Variables(declaration) | Self::Constants(declaration) => declaration.span.clone(),
            Self::Property(declaration) => declaration.span.clone(),
            Self::Routine(declaration) => declaration.span.clone(),
            Self::Visibility { name } => name.span.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedDeclarationSection {
    pub kind: crate::PascalSectionKind,
    pub declarations: Vec<DeclarationSyntax>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclarationParseOutput {
    pub sections: Vec<ParsedDeclarationSection>,
    pub diagnostics: Vec<crate::Diagnostic>,
    pub declaration_count: usize,
    pub unsupported_count: usize,
}
