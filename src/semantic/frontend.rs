use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

use crate::{
    Application, Callee, CaseLabel, Diagnostic, Expr, ExprKind, Literal, Operator,
    OperatorInvocation, PascalFile, PascalFileKind, PascalSectionKind, Span, Statement, Token,
    TryContinuation, chumsky_parser,
    declaration_ast::{
        AggregateSyntaxKind, CallingConventionSyntax, DeclarationSyntax, EnumMemberSyntax,
        FormalModeSyntax, FormalParameterSyntax, PropertyDeclarationSyntax,
        RoutineDeclarationSyntax, RoutineSyntaxKind, SpannedName, TypeDeclarationSyntax,
        TypeSyntax, TypeSyntaxKind, ValueDeclarationSyntax,
    },
    declaration_parser::{parse_file_declarations, section_tokens},
    operator_declaration_spec, operator_declaration_specs, operator_invocation_identifier,
    pascal_parser,
};

use super::{
    ActualArgument, ActualArgumentForm, AggregateDefinition, AggregateKind, AliasType,
    ApplicationCandidate, ApplicationReceiver, ApplicationResolution, ApplicationResolver,
    ApplicationSelection, ArrayType, BindError, BoundApplicationTarget, BoundAssignment, BoundBody,
    BoundCaseArm, BoundCaseLabel, BoundExceptionHandler, BoundExpression, BoundExpressionKind,
    BoundPropertyBinding, BoundSetElement, BoundStatement, BoundStatementKind,
    BoundTryContinuation, BuiltinActual, BuiltinContract, BuiltinFamilyDecl, BuiltinOperandForm,
    BuiltinRegistry, BuiltinTypeContext, CallableFlavor, CallableType, CallingConvention, Capture,
    ConstantEntry, ConstantEvaluator, ConstantValue, ConversionResolution, ConversionResolver,
    ConversionSelection, DeclarationMode, DeclarationState, DeclaredRoutine, EnumMember, EnumType,
    EnvironmentId, EnvironmentRequirement, ExpressionCategory, FieldLayout, FormalParameter,
    FrameKind, IncompleteReason, LookupBarrier, LookupEdge, LookupRequest, LookupResult,
    MetadataQuery, MethodDispatch, MethodMetadata, ModuleGraphError, ModuleId, ModulePhase,
    ModuleRegistry, NameId, NilType, NodeId, NumericOperation, OpaqueType, OrdinalDomain,
    OrdinalOperation, ParameterMode, PointerType, PrimitiveKind, PrimitiveType, PropertyAccessKind,
    PropertyAccessor, PropertySymbol, RawMethodType, ReceiverId, RegionOwner, RoutineOwner,
    RoutineSignature, SemanticBinder, SemanticUse, SetType, StepOperation, StorageLayout,
    StringKind, StringLiteralType, StringType, SubrangeType, SymbolCategory, SymbolFilter,
    SymbolId, SymbolKind, TypeOwner, TypeRef, UnitType, UntypedPointerType, VariantAlternative,
    VariantPart,
};

#[derive(Clone, Debug)]
pub struct BoundFile {
    pub source_name: String,
    pub pascal_name: Option<String>,
    pub kind: PascalFileKind,
    pub environment: EnvironmentId,
    pub declaration_count: usize,
    pub unsupported_declarations: usize,
    pub final_directive_state: crate::DirectiveState,
}

#[derive(Debug)]
pub struct SemanticCompilation {
    pub binder: SemanticBinder,
    pub builtin_families: BuiltinRegistry,
    pub modules: ModuleRegistry,
    pub files: Vec<BoundFile>,
    pub bodies: Vec<BoundBody>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Copy, Debug)]
struct BuiltinTypes {
    integer: TypeRef,
    long_integer: TypeRef,
    real: TypeRef,
    boolean: TypeRef,
    character: TypeRef,
    byte: TypeRef,
    word: TypeRef,
    size_unsigned: TypeRef,
    nil: TypeRef,
    untyped_parameter: TypeRef,
}

struct ParsedInput {
    source_name: String,
    file: PascalFile,
    declarations: crate::declaration_ast::DeclarationParseOutput,
    body_tokens: Vec<Token>,
    final_directive_state: crate::DirectiveState,
    module: Option<ModuleId>,
}

fn system_declarations() -> &'static crate::declaration_ast::DeclarationParseOutput {
    static DECLARATIONS: OnceLock<crate::declaration_ast::DeclarationParseOutput> = OnceLock::new();
    DECLARATIONS.get_or_init(|| {
        let lexed = crate::preprocess(
            "rtl/system.pp",
            include_str!("../../rtl/system.pp"),
            &crate::PreprocessorOptions::default(),
        );
        assert!(
            lexed.diagnostics.is_empty(),
            "bundled rtl/system.pp must preprocess cleanly: {:#?}",
            lexed.diagnostics
        );
        let parsed = pascal_parser::parse_tokens(&lexed.tokens, lexed.logical_len);
        assert!(
            parsed.diagnostics.is_empty(),
            "bundled rtl/system.pp must parse cleanly: {:#?}",
            parsed.diagnostics
        );
        let file = parsed
            .file
            .expect("bundled rtl/system.pp must produce a unit syntax tree");
        let declarations = parse_file_declarations(&file);
        assert!(
            declarations.diagnostics.is_empty(),
            "bundled rtl/system.pp declarations must parse cleanly: {:#?}",
            declarations.diagnostics
        );
        declarations
    })
}

#[derive(Clone, Copy)]
struct ResolvedParameter {
    name: NameId,
    ty: TypeRef,
}

#[derive(Clone, Debug)]
struct ActiveRoutine {
    ty: TypeRef,
    parameters: Vec<SymbolId>,
}

struct CompilationDriver {
    binder: SemanticBinder,
    builtin_families: BuiltinRegistry,
    modules: ModuleRegistry,
    module_names: BTreeMap<String, ModuleId>,
    diagnostics: Vec<Diagnostic>,
    bodies: Vec<BoundBody>,
    builtins: BuiltinTypes,
    intrinsic_types: EnvironmentId,
    binding_system: bool,
    system_module: ModuleId,
    system_exports: EnvironmentId,
    routine_forwards: BTreeMap<(super::RegionId, NameId), DeclaredRoutine>,
    next_anonymous_type: usize,
    next_receiver: usize,
    next_block: u32,
    loop_depth: u32,
    active_routines: Vec<ActiveRoutine>,
}

impl CompilationDriver {
    fn new() -> Self {
        let mut binder = SemanticBinder::new();
        let (_, intrinsic_entry) = binder
            .scopes
            .create_detached_region(RegionOwner::Block(u32::MAX), Vec::new());
        binder.scopes.select_environment(intrinsic_entry);
        let builtins = install_builtins(&mut binder);
        let intrinsic_types = binder.scopes.current_environment();
        let builtin_families = BuiltinRegistry::default();
        let system_name = binder.scopes.intern_name("System");
        let mut modules = ModuleRegistry::new();
        let predicted = ModuleId::from_index(0);
        let (_, system_local) = binder
            .scopes
            .create_detached_region(RegionOwner::Module(predicted), Vec::new());
        let system_module = modules.add_module(system_name, system_local);
        debug_assert_eq!(system_module, predicted);
        let mut module_names = BTreeMap::new();
        module_names.insert("system".to_owned(), system_module);
        let mut driver = Self {
            binder,
            builtin_families,
            modules,
            module_names,
            diagnostics: Vec::new(),
            bodies: Vec::new(),
            builtins,
            intrinsic_types,
            binding_system: false,
            system_module,
            system_exports: system_local,
            routine_forwards: BTreeMap::new(),
            next_anonymous_type: 0,
            next_receiver: 0,
            next_block: 0,
            loop_depth: 0,
            active_routines: Vec::new(),
        };
        driver.bind_system_rtl(system_local);
        driver
    }

    fn bind_system_rtl(&mut self, local: EnvironmentId) {
        let declarations = system_declarations();
        self.binder.scopes.select_environment(local);
        self.binding_system = true;
        if let Some(interface) = declarations
            .sections
            .iter()
            .find(|section| section.kind == PascalSectionKind::Interface)
        {
            self.bind_declarations(&interface.declarations, RoutineOwner::Module, None);
        }
        self.binding_system = false;
        let region = self.binder.scopes.environment_region(local);
        let exports = self.binder.scopes.create_region_view(region, Vec::new());
        self.modules
            .set_interface_exports(self.system_module, exports);
        self.system_exports = exports;
        self.builtins.integer = self.system_declared_type(exports, "integer");
        self.builtins.long_integer = self.system_declared_type(exports, "longint");
        self.builtins.real = self.system_declared_type(exports, "real");
        self.builtins.boolean = self.system_declared_type(exports, "boolean");
        self.builtins.character = self.system_declared_type(exports, "char");
        self.builtins.byte = self.system_declared_type(exports, "byte");
        self.builtins.word = self.system_declared_type(exports, "word");
        self.builtins.size_unsigned = self.system_declared_type(exports, "sizeuint");
    }

    fn system_declared_type(&mut self, exports: EnvironmentId, spelling: &str) -> TypeRef {
        let name = self.binder.scopes.intern_name(spelling);
        let lookup = self
            .binder
            .scopes
            .lookup_symbol(exports, name, LookupRequest::REQUIRED_TYPE)
            .unwrap_or_else(|| panic!("rtl/system.pp must declare `{spelling}`"));
        let SymbolKind::Type(ty) = self.binder.scopes.symbol(lookup.primary[0].symbol).kind else {
            panic!("rtl/system.pp `{spelling}` must be a type declaration");
        };
        ty
    }

    fn register_modules(&mut self, inputs: &mut [ParsedInput]) {
        for input in inputs {
            if input.file.kind != PascalFileKind::Unit {
                continue;
            }
            let Some(name) = input.file.name.as_deref() else {
                continue;
            };
            if self.module_names.contains_key(name) {
                self.diagnostics.push(Diagnostic::new(
                    input.file.span.clone(),
                    format!("duplicate unit `{name}`"),
                ));
                continue;
            }
            let predicted = ModuleId::from_index(self.modules_len());
            let (_, exports) = self
                .binder
                .scopes
                .create_detached_region(RegionOwner::Module(predicted), Vec::new());
            let name_id = self.binder.scopes.intern_name(name);
            let module = self.modules.add_module(name_id, exports);
            debug_assert_eq!(module, predicted);
            self.module_names.insert(name.to_owned(), module);
            input.module = Some(module);
        }
    }

    fn modules_len(&self) -> usize {
        self.module_names.len()
    }

    fn configure_uses(&mut self, inputs: &[ParsedInput]) {
        for input in inputs {
            let Some(module) = input.module else {
                continue;
            };
            for section in &input.declarations.sections {
                let phase = match section.kind {
                    PascalSectionKind::Interface => ModulePhase::Interface,
                    PascalSectionKind::Implementation => ModulePhase::Implementation,
                    _ => continue,
                };
                let uses = self.resolve_uses(&section.declarations);
                self.modules.set_uses(module, phase, uses);
            }
        }
    }

    fn resolve_uses(&mut self, declarations: &[DeclarationSyntax]) -> Vec<ModuleId> {
        let mut result = Vec::new();
        for declaration in declarations {
            let DeclarationSyntax::Uses { units, .. } = declaration else {
                continue;
            };
            for unit in units {
                if let Some(resolved) = self.module_names.get(&unit.spelling) {
                    result.push(*resolved);
                } else {
                    self.diagnostics.push(Diagnostic::new(
                        unit.span.clone(),
                        format!("unknown unit `{}`", unit.spelling),
                    ));
                }
            }
        }
        result
    }

    fn bind_module_interfaces(&mut self, inputs: &[ParsedInput]) {
        let order = match self.modules.interface_order() {
            Ok(order) => order,
            Err(ModuleGraphError::InterfaceCycle { cycle }) => {
                self.diagnostics.push(Diagnostic::new(
                    0..0,
                    format!("interface uses cycle: {cycle:?}"),
                ));
                return;
            }
        };
        for module in order {
            if module == self.system_module {
                continue;
            }
            let Some(input) = inputs.iter().find(|input| input.module == Some(module)) else {
                continue;
            };
            let local = self.modules.module(module).interface_exports;
            let lookup = self.modules.interface_lookup_environment(
                &mut self.binder.scopes,
                module,
                local,
                Some((self.system_module, self.system_exports)),
            );
            self.binder.scopes.select_environment(lookup);
            if let Some(section) = input
                .declarations
                .sections
                .iter()
                .find(|section| section.kind == PascalSectionKind::Interface)
            {
                self.bind_declarations(&section.declarations, RoutineOwner::Module, None);
            }
            let region = self
                .binder
                .scopes
                .environment_region(self.binder.scopes.current_environment());
            let exports = self.binder.scopes.create_region_view(region, Vec::new());
            self.modules.set_interface_exports(module, exports);
        }
    }

    fn bind_remaining_files(&mut self, inputs: &[ParsedInput], files: &mut Vec<BoundFile>) {
        for input in inputs {
            let environment = if let Some(module) = input.module {
                let (_, local) = self
                    .binder
                    .scopes
                    .create_detached_region(RegionOwner::Module(module), Vec::new());
                let lookup = self.modules.implementation_lookup_environment(
                    &mut self.binder.scopes,
                    module,
                    local,
                    Some((self.system_module, self.system_exports)),
                );
                self.binder.scopes.select_environment(lookup);
                if let Some(section) = input
                    .declarations
                    .sections
                    .iter()
                    .find(|section| section.kind == PascalSectionKind::Implementation)
                {
                    self.bind_declarations(&section.declarations, RoutineOwner::Module, None);
                }
                self.binder.scopes.current_environment()
            } else {
                self.bind_program(input)
            };
            files.push(BoundFile {
                source_name: input.source_name.clone(),
                pascal_name: input.file.name.clone(),
                kind: input.file.kind,
                environment,
                declaration_count: input.declarations.declaration_count,
                unsupported_declarations: input.declarations.unsupported_count,
                final_directive_state: input.final_directive_state.clone(),
            });
        }
    }

    fn bind_program(&mut self, input: &ParsedInput) -> EnvironmentId {
        let (_, local) = self
            .binder
            .scopes
            .create_detached_region(RegionOwner::Block(0), Vec::new());
        let region = self.binder.scopes.environment_region(local);
        let declarations = input
            .declarations
            .sections
            .iter()
            .find(|section| section.kind == PascalSectionKind::Declarations);
        let uses =
            declarations.map_or_else(Vec::new, |section| self.resolve_uses(&section.declarations));
        let mut layers = vec![LookupEdge::lexical_parent(local)];
        layers.extend(uses.iter().rev().map(|module| {
            LookupEdge::import(self.modules.module(*module).interface_exports, *module)
        }));
        layers.push(LookupEdge::system(self.system_exports, self.system_module));
        let lookup = self.binder.scopes.create_lookup_environment(region, layers);
        self.binder.scopes.select_environment(lookup);
        if let Some(section) = declarations {
            self.bind_declarations(&section.declarations, RoutineOwner::Module, None);
        }
        self.bind_body_tokens(None, &input.body_tokens);
        self.binder.scopes.current_environment()
    }

    fn bind_declarations(
        &mut self,
        declarations: &[DeclarationSyntax],
        routine_owner: RoutineOwner,
        mut aggregate: Option<&mut AggregateDefinition>,
    ) {
        for declaration in declarations {
            match declaration {
                DeclarationSyntax::Uses { .. } | DeclarationSyntax::Visibility { .. } => {}
                DeclarationSyntax::TypeSection { declarations, .. } => {
                    if let Err(error) = self.binder.begin_type_section() {
                        self.bind_error(declaration.span(), error);
                        continue;
                    }
                    for declaration in declarations {
                        self.bind_type_declaration(declaration);
                    }
                    match self.binder.end_type_section() {
                        Ok(unresolved) => {
                            for unresolved in unresolved {
                                self.diagnostics.push(Diagnostic::new(
                                    declaration.span(),
                                    format!(
                                        "unresolved pointer-forward type `{}` at end of type section",
                                        self.binder.scopes.names().spelling(unresolved.name)
                                    ),
                                ));
                            }
                        }
                        Err(error) => self.bind_error(declaration.span(), error),
                    }
                }
                DeclarationSyntax::Variables(value) => {
                    if let Some(aggregate) = aggregate.as_deref_mut() {
                        self.bind_fields(value, aggregate);
                    } else {
                        self.bind_values(value, false);
                    }
                }
                DeclarationSyntax::Constants(value) => self.bind_values(value, true),
                DeclarationSyntax::Property(value) => {
                    self.bind_properties(value, aggregate.as_deref_mut())
                }
                DeclarationSyntax::Labels { names, span } => {
                    for name in names {
                        let name_id = self.binder.scopes.intern_name(&name.spelling);
                        if let Err(error) = self.binder.scopes.declare(
                            name_id,
                            SymbolKind::Label,
                            DeclarationState::Complete,
                            DeclarationMode::Fresh,
                        ) {
                            self.bind_error(span.clone(), error.into());
                        }
                    }
                }
                DeclarationSyntax::Routine(routine) => {
                    if let Some(aggregate) = aggregate.as_deref_mut() {
                        self.bind_method(routine, aggregate);
                    } else {
                        self.bind_routine(routine, routine_owner);
                    }
                }
                DeclarationSyntax::Unsupported { span, .. } => {
                    if !self.binding_system {
                        self.diagnostics.push(Diagnostic::new(
                            span.clone(),
                            "unsupported declaration retained by the CST-to-semantic boundary",
                        ));
                    }
                }
            }
        }
    }

    fn bind_type_declaration(&mut self, declaration: &TypeDeclarationSyntax) {
        let name = self.binder.scopes.intern_name(&declaration.name.spelling);
        match &declaration.ty.kind {
            TypeSyntaxKind::External { .. } => {
                let target = self.system_external_type(name);
                if let Err(error) = self.binder.define_type(
                    name,
                    AliasType {
                        target,
                        nominal: false,
                    },
                ) {
                    self.bind_error(declaration.span.clone(), error);
                }
            }
            TypeSyntaxKind::ClassForward => {
                if let Err(error) = self
                    .binder
                    .declare_explicit_type_forward(name, IncompleteReason::ClassForward)
                {
                    self.bind_error(declaration.span.clone(), error);
                }
            }
            TypeSyntaxKind::Pointer(target) => {
                if let TypeSyntaxKind::Named(path) = &target.kind
                    && path.len() == 1
                {
                    let target_name = self.binder.scopes.intern_name(&path[0].spelling);
                    if let Err(error) =
                        self.binder
                            .declare_pointer_type(name, target_name, pointer_layout())
                    {
                        self.bind_error(declaration.span.clone(), error);
                    }
                    return;
                }
                let Some(target) = self.resolve_type(target) else {
                    self.define_error_type(name, declaration.span.clone());
                    return;
                };
                if let Err(error) = self.binder.define_type(
                    name,
                    PointerType {
                        target,
                        layout: pointer_layout(),
                    },
                ) {
                    self.bind_error(declaration.span.clone(), error);
                }
            }
            TypeSyntaxKind::Enumeration(members) => {
                self.bind_enumeration(name, members, declaration);
            }
            TypeSyntaxKind::Subrange { lower, upper } => {
                self.bind_subrange(name, lower, upper, declaration);
            }
            TypeSyntaxKind::Aggregate {
                kind,
                base,
                members,
                variant,
            } => self.bind_aggregate(
                name,
                *kind,
                base.as_deref(),
                members,
                variant.as_deref(),
                declaration,
            ),
            TypeSyntaxKind::Named(_) => {
                let Some(target) = self.resolve_type(&declaration.ty) else {
                    self.define_error_type(name, declaration.span.clone());
                    return;
                };
                if let Err(error) = self.binder.define_type(
                    name,
                    AliasType {
                        target,
                        nominal: declaration.distinct,
                    },
                ) {
                    self.bind_error(declaration.span.clone(), error);
                }
            }
            TypeSyntaxKind::Procedural {
                method_pointer,
                parameters,
                result,
                calling_convention,
            } => {
                let flavor = if *method_pointer {
                    CallableFlavor::Method
                } else {
                    CallableFlavor::Routine
                };
                let signature = self.resolve_procedural_signature(
                    parameters,
                    result.as_deref(),
                    *calling_convention,
                );
                let implementation = CallableType {
                    owner: RoutineOwner::Module,
                    flavor,
                    signature,
                    declaration_region: None,
                    nested_routines: Vec::new(),
                    local_types: Vec::new(),
                    captures: Vec::new(),
                    environment: EnvironmentRequirement::None,
                    has_body: false,
                    method: None,
                    overload: false,
                };
                if let Err(error) = self.binder.define_type(name, implementation) {
                    self.bind_error(declaration.span.clone(), error);
                }
            }
            TypeSyntaxKind::Array {
                indices,
                element,
                dynamic,
            } => {
                let Some(element) = element
                    .as_deref()
                    .and_then(|element| self.resolve_type(element))
                else {
                    self.define_error_type(name, declaration.span.clone());
                    return;
                };
                let Some(implementation) =
                    self.build_array_implementation(indices, element, *dynamic)
                else {
                    self.define_error_type(name, declaration.span.clone());
                    return;
                };
                if let Err(error) = self.binder.define_type(name, implementation) {
                    self.bind_error(declaration.span.clone(), error);
                }
            }
            TypeSyntaxKind::Set { element } => {
                let Some(element) = element
                    .as_deref()
                    .and_then(|element| self.resolve_type(element))
                else {
                    self.define_error_type(name, declaration.span.clone());
                    return;
                };
                let Some(domain) = self.binder.types.ordinal_domain(element) else {
                    self.diagnostics.push(Diagnostic::new(
                        declaration.ty.span.clone(),
                        "set element type must be ordinal",
                    ));
                    self.define_error_type(name, declaration.span.clone());
                    return;
                };
                let bytes = domain
                    .cardinality()
                    .and_then(|bits| bits.checked_add(7))
                    .map(|bits| bits / 8)
                    .and_then(|bytes| u64::try_from(bytes).ok())
                    .unwrap_or(0);
                if let Err(error) = self.binder.define_type(
                    name,
                    SetType {
                        element,
                        domain,
                        layout: StorageLayout {
                            size: bytes,
                            alignment: 1,
                        },
                    },
                ) {
                    self.bind_error(declaration.span.clone(), error);
                }
            }
            TypeSyntaxKind::Unsupported(_) => {
                if !self.binding_system {
                    self.diagnostics.push(Diagnostic::new(
                        declaration.ty.span.clone(),
                        "unsupported type syntax bound as an opaque error type",
                    ));
                }
                if let Err(error) = self.binder.define_type(name, opaque_type()) {
                    self.bind_error(declaration.span.clone(), error);
                }
            }
        }
    }

    fn bind_enumeration(
        &mut self,
        name: NameId,
        syntax_members: &[EnumMemberSyntax],
        declaration: &TypeDeclarationSyntax,
    ) {
        let declared = match self.binder.define_type(
            name,
            EnumType {
                members: Vec::new(),
                domain: OrdinalDomain { lower: 0, upper: 0 },
                layout: StorageLayout {
                    size: 4,
                    alignment: 4,
                },
            },
        ) {
            Ok(declared) => declared,
            Err(error) => {
                self.bind_error(declaration.span.clone(), error);
                return;
            }
        };
        let mut members = Vec::new();
        let mut previous = None;
        for syntax in syntax_members {
            let value = if let Some(expression) = &syntax.value {
                let Some(entry) = self.evaluate_constant_expression_with_modes(
                    expression,
                    None,
                    declaration.ty.modes,
                ) else {
                    continue;
                };
                let Some(value) = entry.value.ordinal() else {
                    self.diagnostics.push(Diagnostic::new(
                        syntax.span.clone(),
                        "explicit enum value must be an ordinal constant",
                    ));
                    continue;
                };
                value
            } else {
                previous.map_or(0, |value: i128| value.saturating_add(1))
            };
            if previous.is_some_and(|previous| value <= previous) {
                self.diagnostics.push(Diagnostic::new(
                    syntax.span.clone(),
                    "explicit enum values must ascend",
                ));
                continue;
            }
            let member_name = self.binder.scopes.intern_name(&syntax.name.spelling);
            match self.binder.scopes.declare(
                member_name,
                SymbolKind::Constant(declared.ty),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            ) {
                Ok(symbol) => {
                    self.binder.constants.insert(
                        symbol,
                        ConstantEntry {
                            ty: declared.ty,
                            value: ConstantValue::Integer(value),
                        },
                    );
                    members.push(EnumMember {
                        name: member_name,
                        value,
                    });
                    previous = Some(value);
                }
                Err(error) => self.bind_error(syntax.span.clone(), error.into()),
            }
        }
        let domain = members.first().zip(members.last()).map_or(
            OrdinalDomain { lower: 0, upper: 0 },
            |(first, last)| OrdinalDomain {
                lower: first.value,
                upper: last.value,
            },
        );
        if let Err(error) = self
            .binder
            .types
            .set_enum_members(declared.ty, members, domain)
        {
            self.bind_error(declaration.span.clone(), error.into());
        }
    }

    fn bind_subrange(
        &mut self,
        name: NameId,
        lower: &Expr,
        upper: &Expr,
        declaration: &TypeDeclarationSyntax,
    ) {
        let Some(lower) =
            self.evaluate_constant_expression_with_modes(lower, None, declaration.ty.modes)
        else {
            self.define_error_type(name, declaration.span.clone());
            return;
        };
        let Some(upper) = self.evaluate_constant_expression_with_modes(
            upper,
            Some(lower.ty),
            declaration.ty.modes,
        ) else {
            self.define_error_type(name, declaration.span.clone());
            return;
        };
        let (Some(lower_value), Some(upper_value)) = (lower.value.ordinal(), upper.value.ordinal())
        else {
            self.diagnostics.push(Diagnostic::new(
                declaration.span.clone(),
                "subrange bounds must be ordinal constants",
            ));
            self.define_error_type(name, declaration.span.clone());
            return;
        };
        let lower_base = self.binder.types.ordinal_base_type(lower.ty);
        let upper_base = self.binder.types.ordinal_base_type(upper.ty);
        if lower_base.is_none() || lower_base != upper_base || lower_value > upper_value {
            self.diagnostics.push(Diagnostic::new(
                declaration.span.clone(),
                "subrange bounds require one compatible ordinal base and ascending values",
            ));
            self.define_error_type(name, declaration.span.clone());
            return;
        }
        let base = lower_base.unwrap();
        let layout = self
            .binder
            .types
            .storage_layout(base)
            .unwrap_or(StorageLayout {
                size: 4,
                alignment: 4,
            });
        if let Err(error) = self.binder.define_type(
            name,
            SubrangeType {
                base,
                domain: OrdinalDomain {
                    lower: lower_value,
                    upper: upper_value,
                },
                layout,
            },
        ) {
            self.bind_error(declaration.span.clone(), error);
        }
    }

    fn build_array_implementation(
        &mut self,
        indices: &[TypeSyntax],
        element: TypeRef,
        dynamic: bool,
    ) -> Option<ArrayType> {
        if dynamic {
            return Some(ArrayType {
                element,
                index: self.builtins.integer,
                length: self.builtins.integer,
                layout: None,
                resizable: true,
                open: false,
            });
        }
        let resolved = indices
            .iter()
            .map(|index| self.resolve_type(index))
            .collect::<Option<Vec<_>>>()?;
        if resolved.is_empty() {
            return None;
        }
        let mut current_element = element;
        for (position, index) in resolved.iter().enumerate().rev() {
            let cardinality = self.binder.types.ordinal_domain(*index)?.cardinality()?;
            let element_layout = self.binder.types.storage_layout(current_element)?;
            let size = u64::try_from(cardinality)
                .ok()?
                .checked_mul(element_layout.size)?;
            let implementation = ArrayType {
                element: current_element,
                index: *index,
                length: self.builtins.integer,
                layout: Some(StorageLayout {
                    size,
                    alignment: element_layout.alignment,
                }),
                resizable: false,
                open: false,
            };
            if position == 0 {
                return Some(implementation);
            }
            current_element = self.allocate_anonymous(implementation);
        }
        None
    }

    fn bind_aggregate(
        &mut self,
        name: NameId,
        syntax_kind: AggregateSyntaxKind,
        base_syntax: Option<&TypeSyntax>,
        members: &[DeclarationSyntax],
        variant: Option<&crate::declaration_ast::VariantPartSyntax>,
        declaration: &TypeDeclarationSyntax,
    ) {
        let base = base_syntax.and_then(|base| self.resolve_type(base));
        let kind = match syntax_kind {
            AggregateSyntaxKind::Record => AggregateKind::RegularRecord,
            AggregateSyntaxKind::PackedRecord => AggregateKind::PackedRecord,
            AggregateSyntaxKind::Object => AggregateKind::Object { base },
            AggregateSyntaxKind::Class => AggregateKind::Class { base },
            AggregateSyntaxKind::Interface => AggregateKind::Interface { base },
        };
        let layout = if matches!(
            syntax_kind,
            AggregateSyntaxKind::Class | AggregateSyntaxKind::Interface
        ) {
            pointer_layout()
        } else {
            StorageLayout {
                size: 0,
                alignment: 1,
            }
        };
        let mut aggregate = match self.binder.begin_aggregate(name, kind, layout) {
            Ok(aggregate) => aggregate,
            Err(error) => {
                self.bind_error(declaration.span.clone(), error);
                return;
            }
        };
        self.bind_declarations(
            members,
            RoutineOwner::Type(aggregate.declared.ty),
            Some(&mut aggregate),
        );
        let variant = variant.and_then(|variant| self.bind_variant_part(variant, &mut aggregate));
        if matches!(
            aggregate.kind,
            AggregateKind::RegularRecord | AggregateKind::Object { .. }
        ) {
            aggregate.layout.size = align_up(aggregate.layout.size, aggregate.layout.alignment);
        }
        if let Err(error) = self.binder.end_aggregate(aggregate, variant) {
            self.bind_error(declaration.span.clone(), error);
        }
    }

    fn bind_variant_part(
        &mut self,
        syntax: &crate::declaration_ast::VariantPartSyntax,
        aggregate: &mut AggregateDefinition,
    ) -> Option<VariantPart> {
        let selector_type = self.resolve_type(&syntax.selector_type)?;
        let domain = self.binder.types.ordinal_domain(selector_type);
        if domain.is_none() {
            self.diagnostics.push(Diagnostic::new(
                syntax.selector_type.span.clone(),
                "variant selector type must be ordinal",
            ));
            return None;
        }
        let selector = syntax.selector_name.as_ref().and_then(|selector| {
            let before = aggregate.fields.len();
            let declaration = ValueDeclarationSyntax {
                names: vec![selector.clone()],
                ty: Some((*syntax.selector_type).clone()),
                initializer: None,
                external_name: None,
                span: selector.span.clone(),
                modes: syntax.selector_type.modes,
            };
            self.bind_fields(&declaration, aggregate);
            aggregate.fields.get(before).cloned()
        });
        let payload_offset = aggregate.layout.size;
        let mut alternatives = Vec::new();
        let mut maximum_end = payload_offset;
        let mut labels_seen = BTreeSet::new();
        for alternative in &syntax.alternatives {
            let mut labels = Vec::new();
            for label in &alternative.labels {
                let Some(entry) = self.evaluate_constant_expression_with_modes(
                    label,
                    Some(selector_type),
                    syntax.selector_type.modes,
                ) else {
                    continue;
                };
                let Some(value) = entry.value.ordinal() else {
                    self.diagnostics.push(Diagnostic::new(
                        label.span.clone(),
                        "variant label must be an ordinal constant",
                    ));
                    continue;
                };
                if !domain.is_some_and(|domain| domain.contains(value)) {
                    self.diagnostics.push(Diagnostic::new(
                        label.span.clone(),
                        "variant label is outside the selector domain",
                    ));
                    continue;
                }
                if !labels_seen.insert(value) {
                    self.diagnostics.push(Diagnostic::new(
                        label.span.clone(),
                        "duplicate variant label",
                    ));
                    continue;
                }
                labels.push(value);
            }
            aggregate.layout.size = payload_offset;
            let field_start = aggregate.fields.len();
            self.bind_declarations(
                &alternative.members,
                RoutineOwner::Type(aggregate.declared.ty),
                Some(aggregate),
            );
            maximum_end = maximum_end.max(aggregate.layout.size);
            alternatives.push(VariantAlternative {
                labels,
                fields: aggregate.fields[field_start..].to_vec(),
            });
        }
        aggregate.layout.size = maximum_end;
        Some(VariantPart {
            selector,
            alternatives,
            byte_offset: payload_offset,
            byte_size: maximum_end.saturating_sub(payload_offset),
            alignment: aggregate.layout.alignment,
        })
    }

    fn bind_fields(
        &mut self,
        declaration: &ValueDeclarationSyntax,
        aggregate: &mut AggregateDefinition,
    ) {
        let Some(ty) = declaration
            .ty
            .as_ref()
            .and_then(|syntax| self.resolve_type(syntax))
        else {
            self.diagnostics.push(Diagnostic::new(
                declaration.span.clone(),
                "field type could not be resolved",
            ));
            return;
        };
        let storage = self
            .binder
            .types
            .storage_layout(ty)
            .unwrap_or(StorageLayout {
                size: 0,
                alignment: 1,
            });
        for name in &declaration.names {
            let byte_offset = match aggregate.kind {
                AggregateKind::PackedRecord => aggregate.layout.size,
                AggregateKind::RegularRecord | AggregateKind::Object { .. } => {
                    align_up(aggregate.layout.size, storage.alignment)
                }
                AggregateKind::Class { .. } | AggregateKind::Interface { .. } => {
                    aggregate.fields.last().map_or(0, |field| {
                        field.layout.byte_offset
                            + self
                                .binder
                                .types
                                .storage_layout(field.ty)
                                .map_or(0, |layout| layout.size)
                    })
                }
            };
            let field_layout = FieldLayout {
                byte_offset,
                bit_offset: 0,
                bit_width: None,
            };
            let name_id = self.binder.scopes.intern_name(&name.spelling);
            if let Err(error) = self
                .binder
                .declare_field(aggregate, name_id, ty, field_layout)
            {
                self.bind_error(name.span.clone(), error);
            } else if !matches!(
                aggregate.kind,
                AggregateKind::Class { .. } | AggregateKind::Interface { .. }
            ) {
                aggregate.layout.size = byte_offset.saturating_add(storage.size);
                if !matches!(aggregate.kind, AggregateKind::PackedRecord) {
                    aggregate.layout.alignment = aggregate.layout.alignment.max(storage.alignment);
                }
            }
        }
    }

    fn bind_values(&mut self, declaration: &ValueDeclarationSyntax, constant: bool) {
        let explicit_type = declaration
            .ty
            .as_ref()
            .and_then(|syntax| self.resolve_type(syntax));
        let mut initializer = declaration.initializer.as_ref().map(|initializer| {
            self.bind_expression_with_expected(initializer, None, explicit_type)
        });
        let constant_conversion_valid = if constant
            && let (Some(destination), Some(initializer)) = (explicit_type, initializer.as_mut())
        {
            initializer.ty.is_none()
                || self
                    .apply_implicit_conversion(
                        initializer,
                        destination,
                        declaration.modes,
                        "constant is not convertible to its declared type",
                    )
                    .is_some_and(|resolution| {
                        matches!(resolution.selection, ConversionSelection::Selected { .. })
                    })
        } else {
            true
        };
        let constant_entry = if constant && constant_conversion_valid {
            initializer.as_ref().and_then(|initializer| {
                let evaluator = ConstantEvaluator::new(&self.binder.constants, &self.binder.types);
                match evaluator.evaluate_with_modes(initializer, explicit_type, declaration.modes) {
                    Ok(entry) => Some(entry),
                    Err(error) => {
                        self.diagnostics.push(Diagnostic::new(
                            initializer.span.clone(),
                            format!("constant expression: {error:?}"),
                        ));
                        None
                    }
                }
            })
        } else {
            None
        };
        let ty = explicit_type
            .or_else(|| constant_entry.as_ref().map(|entry| entry.ty))
            .or_else(|| initializer.as_ref().and_then(|initializer| initializer.ty))
            .unwrap_or(self.builtins.integer);
        if constant && initializer.is_none() {
            self.diagnostics.push(Diagnostic::new(
                declaration.span.clone(),
                "constant declaration requires an initializer",
            ));
        }
        if !constant && let Some(initializer) = initializer.as_mut() {
            self.apply_implicit_conversion(
                initializer,
                ty,
                declaration.modes,
                "variable initializer is not convertible to its declared type",
            );
        }
        self.binder.scopes.extend_environment(if constant {
            FrameKind::ConstSection
        } else {
            FrameKind::VarSection
        });
        for name in &declaration.names {
            let name_id = self.binder.scopes.intern_name(&name.spelling);
            let kind = if constant {
                SymbolKind::Constant(ty)
            } else {
                SymbolKind::Variable(ty)
            };
            match self.binder.scopes.declare(
                name_id,
                kind,
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            ) {
                Ok(symbol) => {
                    if let Some(entry) = constant_entry.clone() {
                        self.binder.constants.insert(symbol, entry);
                    }
                }
                Err(error) => self.bind_error(name.span.clone(), error.into()),
            }
        }
    }

    fn evaluate_constant_expression_with_modes(
        &mut self,
        expression: &Expr,
        expected: Option<TypeRef>,
        modes: crate::ModeSnapshot,
    ) -> Option<ConstantEntry> {
        let mut bound = self.bind_expression(expression, None);
        if let Some(expected) = expected
            && bound.ty.is_some()
        {
            let resolution = self.apply_implicit_conversion(
                &mut bound,
                expected,
                modes,
                "constant is not convertible to its declared type",
            )?;
            if !matches!(resolution.selection, ConversionSelection::Selected { .. }) {
                return None;
            }
        }
        let evaluator = ConstantEvaluator::new(&self.binder.constants, &self.binder.types);
        match evaluator.evaluate_with_modes(&bound, expected, modes) {
            Ok(entry) => Some(entry),
            Err(error) => {
                self.diagnostics.push(Diagnostic::new(
                    expression.span.clone(),
                    format!("constant expression: {error:?}"),
                ));
                None
            }
        }
    }

    fn evaluate_bound_ordinal(
        &mut self,
        expression: &BoundExpression,
        expected: Option<TypeRef>,
    ) -> Option<i128> {
        let evaluator = ConstantEvaluator::new(&self.binder.constants, &self.binder.types);
        match evaluator.evaluate(expression, expected) {
            Ok(entry) => entry.value.ordinal().or_else(|| {
                self.diagnostics.push(Diagnostic::new(
                    expression.span.clone(),
                    "case label must be an ordinal constant",
                ));
                None
            }),
            Err(error) => {
                self.diagnostics.push(Diagnostic::new(
                    expression.span.clone(),
                    format!("case label is not constant: {error:?}"),
                ));
                None
            }
        }
    }

    fn check_case_interval(
        &mut self,
        low: i128,
        high: i128,
        span: Span,
        occupied: &mut Vec<(i128, i128)>,
    ) {
        if occupied
            .iter()
            .any(|(existing_low, existing_high)| low <= *existing_high && *existing_low <= high)
        {
            self.diagnostics
                .push(Diagnostic::new(span, "duplicate or overlapping case label"));
        } else {
            occupied.push((low, high));
        }
    }

    fn bind_properties(
        &mut self,
        declaration: &PropertyDeclarationSyntax,
        mut aggregate: Option<&mut AggregateDefinition>,
    ) {
        let Some(ty) = declaration
            .ty
            .as_ref()
            .and_then(|syntax| self.resolve_type(syntax))
        else {
            self.diagnostics.push(Diagnostic::new(
                declaration.span.clone(),
                "property type could not be resolved",
            ));
            return;
        };
        let parameters = self
            .resolve_procedural_signature(
                &declaration.parameters,
                None,
                CallingConventionSyntax::Pascal,
            )
            .parameters;
        if parameters
            .iter()
            .any(|parameter| matches!(parameter.mode, ParameterMode::Var | ParameterMode::Out))
        {
            self.diagnostics.push(Diagnostic::new(
                declaration.span.clone(),
                "property index parameters cannot be var or out",
            ));
        }
        let read = declaration
            .read
            .as_ref()
            .map(|name| self.bind_property_accessor(name));
        let write = declaration
            .write
            .as_ref()
            .map(|name| self.bind_property_accessor(name));
        let contract = |result, parameters: Vec<FormalParameter>| CallableType {
            owner: aggregate
                .as_ref()
                .map_or(RoutineOwner::Module, |aggregate| {
                    RoutineOwner::Type(aggregate.declared.ty)
                }),
            flavor: CallableFlavor::Routine,
            signature: RoutineSignature {
                parameters,
                result,
                calling_convention: CallingConvention::Pascal,
            },
            declaration_region: None,
            nested_routines: Vec::new(),
            local_types: Vec::new(),
            captures: Vec::new(),
            environment: EnvironmentRequirement::None,
            has_body: false,
            method: None,
            overload: false,
        };
        let read_contract = read
            .as_ref()
            .map(|_| self.allocate_anonymous(contract(Some(ty), parameters.clone())));
        let write_contract = write.as_ref().map(|_| {
            let mut write_parameters = parameters.clone();
            write_parameters.push(FormalParameter {
                mode: ParameterMode::Value,
                ty,
                default: None,
            });
            self.allocate_anonymous(contract(None, write_parameters))
        });
        let name_id = self.binder.scopes.intern_name(&declaration.name.spelling);
        let symbol = match self.binder.scopes.declare(
            name_id,
            SymbolKind::Property(PropertySymbol {
                ty,
                parameters,
                read,
                write,
                read_contract,
                write_contract,
                is_default: declaration.is_default,
            }),
            DeclarationState::Complete,
            DeclarationMode::Fresh,
        ) {
            Ok(symbol) => symbol,
            Err(error) => {
                self.bind_error(declaration.name.span.clone(), error.into());
                return;
            }
        };
        if declaration.is_default
            && let Some(aggregate) = aggregate.as_mut()
            && aggregate.default_property.replace(symbol).is_some()
        {
            self.diagnostics.push(Diagnostic::new(
                declaration.span.clone(),
                "aggregate has more than one default property",
            ));
        }
    }

    fn bind_property_accessor(&mut self, name: &SpannedName) -> PropertyAccessor {
        let name_id = self.binder.scopes.intern_name(&name.spelling);
        let symbols = self
            .binder
            .scopes
            .lookup_symbol(
                self.binder.scopes.current_environment(),
                name_id,
                LookupRequest::ORDINARY,
            )
            .map_or_else(Vec::new, |lookup| {
                lookup
                    .primary
                    .into_iter()
                    .filter_map(|hit| {
                        matches!(
                            self.binder.scopes.symbol(hit.symbol).kind,
                            SymbolKind::Variable(_) | SymbolKind::Routine(_)
                        )
                        .then_some(hit.symbol)
                    })
                    .collect()
            });
        PropertyAccessor {
            name: name_id,
            symbols,
        }
    }

    fn bind_method(
        &mut self,
        routine: &RoutineDeclarationSyntax,
        aggregate: &mut AggregateDefinition,
    ) {
        let (signature, parameters) = self.resolve_routine_signature(routine);
        let Some(name) = self.routine_name_id(routine, &signature) else {
            return;
        };
        let flavor = if routine.class_method {
            CallableFlavor::ClassMethod
        } else {
            CallableFlavor::Method
        };
        let inherited = self.inherited_method_matches(aggregate, name, &signature, flavor);
        let metadata = self.method_metadata(routine, aggregate, &inherited);
        let result = signature.result;
        let method = match self.binder.declare_method(
            aggregate,
            name,
            signature,
            routine_declaration_mode(routine),
            flavor,
            metadata,
        ) {
            Ok(method) => method,
            Err(error) => {
                self.bind_error(routine.span.clone(), error);
                return;
            }
        };
        if routine.has_body {
            self.bind_routine_body(routine, method, &parameters, result);
        }
    }

    fn bind_routine(&mut self, routine: &RoutineDeclarationSyntax, owner: RoutineOwner) {
        let (signature, parameters) = self.resolve_routine_signature(routine);
        let Some(name) = self.routine_name_id(routine, &signature) else {
            return;
        };
        let result = signature.result;
        if !routine.qualifier.is_empty() {
            self.bind_qualified_method_implementation(routine, signature, parameters, name, result);
            return;
        }
        let region = self
            .binder
            .scopes
            .environment_region(self.binder.scopes.current_environment());
        let key = (region, name);
        let declared = if routine.has_body {
            self.routine_forwards.remove(&key)
        } else {
            None
        }
        .map_or_else(
            || {
                self.binder
                    .declare_routine(name, signature, owner, routine_declaration_mode(routine))
                    .map_err(|error| {
                        self.bind_error(routine.span.clone(), error);
                    })
                    .ok()
            },
            Some,
        );
        let Some(declared) = declared else {
            return;
        };
        if self.binding_system {
            self.attach_system_builtin(routine, declared);
        }
        if routine.is_forward && !routine.is_external {
            self.routine_forwards.insert(key, declared);
        }
        if routine.has_body {
            self.bind_routine_body(routine, declared, &parameters, result);
        }
    }

    fn attach_system_builtin(
        &mut self,
        routine: &RoutineDeclarationSyntax,
        declared: DeclaredRoutine,
    ) {
        let canonical_name = self
            .binder
            .scopes
            .names()
            .spelling(self.binder.scopes.symbol(declared.symbol).name);
        let generic = routine
            .parameters
            .iter()
            .any(|parameter| parameter.ty.is_none());
        let contract = if routine.kind == RoutineSyntaxKind::Operator {
            system_operator_contract(canonical_name)
        } else {
            match canonical_name {
                "low" => Some(BuiltinContract::Metadata(MetadataQuery::Low)),
                "high" => Some(BuiltinContract::Metadata(MetadataQuery::High)),
                "sizeof" => Some(BuiltinContract::Metadata(MetadataQuery::SizeOf)),
                "length" if generic => Some(BuiltinContract::Metadata(MetadataQuery::Length)),
                "odd" => Some(BuiltinContract::Ordinal(OrdinalOperation::Odd)),
                "ord" => Some(BuiltinContract::Ordinal(OrdinalOperation::Ord)),
                "chr" => Some(BuiltinContract::Ordinal(OrdinalOperation::Chr)),
                "pred" => Some(BuiltinContract::Ordinal(OrdinalOperation::Pred)),
                "succ" => Some(BuiltinContract::Ordinal(OrdinalOperation::Succ)),
                "abs" => Some(BuiltinContract::Numeric(NumericOperation::Abs)),
                "sqr" => Some(BuiltinContract::Numeric(NumericOperation::Sqr)),
                "inc" => Some(BuiltinContract::StepMutation(StepOperation::Increment)),
                "dec" => Some(BuiltinContract::StepMutation(StepOperation::Decrement)),
                _ => None,
            }
        };
        let Some(contract) = contract else {
            return;
        };
        let declared_signature = (!generic).then(|| {
            self.binder
                .types
                .callable(declared.ty)
                .expect("declared routine has a callable type")
                .signature
                .clone()
        });
        self.builtin_families.attach(
            declared.symbol,
            BuiltinFamilyDecl {
                contract,
                declared_signature,
            },
        );
    }

    fn inherited_method_matches(
        &self,
        aggregate: &AggregateDefinition,
        name: NameId,
        signature: &RoutineSignature,
        flavor: CallableFlavor,
    ) -> Vec<(SymbolId, TypeRef, MethodMetadata)> {
        let Some(environment) = SemanticBinder::inherited_environment(aggregate) else {
            return Vec::new();
        };
        let Some(lookup) =
            self.binder
                .scopes
                .lookup_symbol(environment, name, LookupRequest::ORDINARY)
        else {
            return Vec::new();
        };
        let mut seen = BTreeSet::new();
        lookup
            .primary
            .iter()
            .chain(lookup.shadowed.iter().flatten())
            .filter_map(|hit| {
                if !seen.insert(hit.symbol) {
                    return None;
                }
                let SymbolKind::Routine(callable_type) = self.binder.scopes.symbol(hit.symbol).kind
                else {
                    return None;
                };
                let callable = self.binder.types.callable(callable_type)?;
                let metadata = callable.method?;
                (callable.flavor == flavor
                    && callable
                        .signature_equivalent(self.binder.types.query(callable_type), signature))
                .then_some((hit.symbol, callable_type, metadata))
            })
            .collect()
    }

    fn method_metadata(
        &mut self,
        routine: &RoutineDeclarationSyntax,
        aggregate: &mut AggregateDefinition,
        inherited: &[(SymbolId, TypeRef, MethodMetadata)],
    ) -> MethodMetadata {
        let next_slot = |aggregate: &mut AggregateDefinition| {
            let slot = aggregate.next_virtual_slot;
            aggregate.next_virtual_slot = aggregate.next_virtual_slot.saturating_add(1);
            slot
        };
        let virtual_ancestor = inherited
            .iter()
            .find(|(_, _, metadata)| metadata.virtual_slot().is_some())
            .copied();
        let exact_ancestor = inherited.first().map(|(symbol, _, _)| *symbol);
        let dispatch = match aggregate.kind {
            AggregateKind::Class { .. } => {
                if routine.static_method {
                    if routine.virtual_method || routine.override_method {
                        self.diagnostics.push(Diagnostic::new(
                            routine.span.clone(),
                            "a static class method cannot be virtual or override",
                        ));
                    }
                    MethodDispatch::Static
                } else if routine.override_method {
                    match virtual_ancestor {
                        Some((symbol, _, metadata)) => {
                            if metadata.final_method {
                                self.diagnostics.push(Diagnostic::new(
                                    routine.span.clone(),
                                    "cannot override a final method",
                                ));
                            }
                            MethodDispatch::Virtual {
                                slot: metadata.virtual_slot().unwrap(),
                                overridden: Some(symbol),
                            }
                        }
                        None => {
                            self.diagnostics.push(Diagnostic::new(
                                routine.span.clone(),
                                "override has no matching inherited virtual method",
                            ));
                            MethodDispatch::Virtual {
                                slot: next_slot(aggregate),
                                overridden: None,
                            }
                        }
                    }
                } else if routine.virtual_method {
                    if virtual_ancestor.is_some() {
                        self.diagnostics.push(Diagnostic::new(
                            routine.span.clone(),
                            "a matching inherited virtual method must use `override`",
                        ));
                    }
                    MethodDispatch::Virtual {
                        slot: next_slot(aggregate),
                        overridden: None,
                    }
                } else {
                    if virtual_ancestor.is_some() {
                        self.diagnostics.push(Diagnostic::new(
                            routine.span.clone(),
                            "a matching inherited virtual method must use `override`",
                        ));
                    }
                    MethodDispatch::NonVirtual
                }
            }
            AggregateKind::Object { .. } => {
                if routine.class_method
                    || routine.static_method
                    || routine.override_method
                    || routine.abstract_method
                    || routine.final_method
                    || routine.reintroduce
                {
                    self.diagnostics.push(Diagnostic::new(
                        routine.span.clone(),
                        "old-style object methods support only the `virtual` dispatch modifier",
                    ));
                }
                if routine.virtual_method {
                    match virtual_ancestor {
                        Some((symbol, _, metadata)) => MethodDispatch::Virtual {
                            slot: metadata.virtual_slot().unwrap(),
                            overridden: Some(symbol),
                        },
                        None => MethodDispatch::Virtual {
                            slot: next_slot(aggregate),
                            overridden: None,
                        },
                    }
                } else {
                    if virtual_ancestor.is_some() {
                        self.diagnostics.push(Diagnostic::new(
                            routine.span.clone(),
                            "a matching inherited virtual object method must be declared `virtual`",
                        ));
                    }
                    MethodDispatch::NonVirtual
                }
            }
            AggregateKind::Interface { .. } => MethodDispatch::Virtual {
                slot: virtual_ancestor
                    .and_then(|(_, _, metadata)| metadata.virtual_slot())
                    .unwrap_or_else(|| next_slot(aggregate)),
                overridden: virtual_ancestor.map(|(symbol, _, _)| symbol),
            },
            AggregateKind::RegularRecord | AggregateKind::PackedRecord => {
                if routine.virtual_method || routine.override_method || routine.abstract_method {
                    self.diagnostics.push(Diagnostic::new(
                        routine.span.clone(),
                        "record methods cannot be virtual, abstract, or override",
                    ));
                }
                if routine.static_method || routine.class_method {
                    MethodDispatch::Static
                } else {
                    MethodDispatch::NonVirtual
                }
            }
        };
        MethodMetadata {
            dispatch,
            ancestor: exact_ancestor,
            abstract_method: routine.abstract_method
                || matches!(aggregate.kind, AggregateKind::Interface { .. }),
            final_method: routine.final_method,
            reintroduce: routine.reintroduce,
        }
    }

    fn bind_qualified_method_implementation(
        &mut self,
        routine: &RoutineDeclarationSyntax,
        signature: RoutineSignature,
        parameters: Vec<ResolvedParameter>,
        name: NameId,
        result: Option<TypeRef>,
    ) {
        let Some(owner) = self.resolve_named_type(&routine.qualifier, routine.span.clone()) else {
            return;
        };
        let Some(environment) = self.binder.types.member_environment(owner) else {
            self.diagnostics.push(Diagnostic::new(
                routine.span.clone(),
                "qualified routine owner has no member environment",
            ));
            return;
        };
        let Some(lookup) =
            self.binder
                .scopes
                .lookup_symbol(environment, name, LookupRequest::ORDINARY)
        else {
            self.diagnostics.push(Diagnostic::new(
                routine.span.clone(),
                "qualified implementation has no matching member declaration",
            ));
            return;
        };
        let expected_flavor = if routine.class_method {
            CallableFlavor::ClassMethod
        } else {
            CallableFlavor::Method
        };
        let matches = lookup
            .primary
            .iter()
            .filter_map(|hit| {
                let SymbolKind::Routine(callable_type) = self.binder.scopes.symbol(hit.symbol).kind
                else {
                    return None;
                };
                let callable = self.binder.types.callable(callable_type)?;
                (callable.owner == RoutineOwner::Type(owner)
                    && callable.flavor == expected_flavor
                    && callable
                        .signature_equivalent(self.binder.types.query(callable_type), &signature))
                .then_some(DeclaredRoutine {
                    symbol: hit.symbol,
                    ty: callable_type,
                    lexical_parent_environment: self.binder.scopes.symbol(hit.symbol).declared_in,
                })
            })
            .collect::<Vec<_>>();
        let [declared] = matches.as_slice() else {
            self.diagnostics.push(Diagnostic::new(
                routine.span.clone(),
                if matches.is_empty() {
                    "qualified implementation has no exact member signature"
                } else {
                    "qualified implementation matches more than one member declaration"
                },
            ));
            return;
        };
        if self
            .binder
            .types
            .callable(declared.ty)
            .is_some_and(|callable| callable.has_body)
        {
            self.diagnostics.push(Diagnostic::new(
                routine.span.clone(),
                "member declaration already has an implementation body",
            ));
            return;
        }
        if routine.has_body {
            self.bind_routine_body(routine, *declared, &parameters, result);
        }
    }

    fn bind_routine_body(
        &mut self,
        routine: &RoutineDeclarationSyntax,
        declared: DeclaredRoutine,
        parameters: &[ResolvedParameter],
        result: Option<TypeRef>,
    ) {
        let checkpoint = match self.binder.begin_routine_body(declared) {
            Ok(checkpoint) => checkpoint,
            Err(error) => {
                self.bind_error(routine.span.clone(), error);
                return;
            }
        };
        self.binder.scopes.extend_environment(FrameKind::VarSection);
        let mut parameter_symbols = Vec::new();
        for parameter in parameters {
            match self.binder.scopes.declare(
                parameter.name,
                SymbolKind::Variable(parameter.ty),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            ) {
                Ok(symbol) => parameter_symbols.push(symbol),
                Err(error) => self.bind_error(routine.span.clone(), error.into()),
            }
        }
        if let Some(result) = result {
            let result_names = if routine.kind == RoutineSyntaxKind::Operator {
                vec!["result"]
            } else {
                vec!["result", routine.name.spelling.as_str()]
            };
            for spelling in result_names {
                let name = self.binder.scopes.intern_name(spelling);
                if let Err(error) = self.binder.scopes.declare(
                    name,
                    SymbolKind::Variable(result),
                    DeclarationState::Complete,
                    DeclarationMode::Fresh,
                ) {
                    self.bind_error(routine.span.clone(), error.into());
                }
            }
        }
        if let Some(callable) = self.binder.types.callable(declared.ty)
            && let RoutineOwner::Type(owner) = callable.owner
        {
            let self_name = self.binder.scopes.intern_name("self");
            if let Err(error) = self.binder.scopes.declare(
                self_name,
                SymbolKind::Variable(owner),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            ) {
                self.bind_error(routine.span.clone(), error.into());
            }
        }
        self.bind_declarations(
            &routine.body_declarations,
            RoutineOwner::Routine(declared.ty),
            None,
        );
        self.active_routines.push(ActiveRoutine {
            ty: declared.ty,
            parameters: parameter_symbols,
        });
        self.bind_body_tokens(Some(declared.ty), &routine.body_tokens);
        self.active_routines.pop();
        self.binder.end_routine_body(checkpoint);
    }

    fn routine_name_id(
        &mut self,
        routine: &RoutineDeclarationSyntax,
        signature: &RoutineSignature,
    ) -> Option<NameId> {
        if routine.kind != RoutineSyntaxKind::Operator {
            return Some(self.binder.scopes.intern_name(&routine.name.spelling));
        }
        let arity = signature.parameters.len();
        let logical_operands = !signature.parameters.is_empty()
            && signature.parameters.iter().all(|parameter| {
                self.binder.types.canonical_type(parameter.ty)
                    == self.binder.types.canonical_type(self.builtins.boolean)
            });
        let implicit_conversion = operator_declaration_specs(&routine.name.spelling, arity)
            .any(|spec| spec.invocation == OperatorInvocation::ImplicitConversion);
        let checks_enabled = if implicit_conversion {
            routine.modes.range_checks
        } else {
            routine.modes.overflow_checks
        };
        let Some(spec) = operator_declaration_spec(
            &routine.name.spelling,
            arity,
            checks_enabled,
            logical_operands,
        ) else {
            self.diagnostics.push(Diagnostic::new(
                routine.name.span.clone(),
                format!(
                    "unknown or incompatible operator declaration `{}` with {arity} parameter(s)",
                    routine.name.spelling
                ),
            ));
            return None;
        };
        Some(self.binder.scopes.intern_name(spec.pascal_identifier))
    }

    fn resolve_routine_signature(
        &mut self,
        routine: &RoutineDeclarationSyntax,
    ) -> (RoutineSignature, Vec<ResolvedParameter>) {
        let mut parameters = Vec::new();
        let mut formals = Vec::new();
        for parameter in &routine.parameters {
            let ty = parameter
                .ty
                .as_ref()
                .and_then(|syntax| self.resolve_type(syntax))
                .unwrap_or(self.builtins.untyped_parameter);
            let mode = match parameter.mode {
                FormalModeSyntax::Value => ParameterMode::Value,
                FormalModeSyntax::Const => ParameterMode::Const,
                FormalModeSyntax::Var => ParameterMode::Var,
                FormalModeSyntax::Out => ParameterMode::Out,
                FormalModeSyntax::ConstRef => ParameterMode::ConstRef,
            };
            let default = parameter
                .default
                .as_ref()
                .and_then(|default| {
                    self.evaluate_constant_expression_with_modes(default, Some(ty), parameter.modes)
                })
                .map(|entry| entry.value);
            for name in &parameter.names {
                let name = self.binder.scopes.intern_name(&name.spelling);
                parameters.push(ResolvedParameter { name, ty });
                formals.push(FormalParameter {
                    mode,
                    ty,
                    default: default.clone(),
                });
            }
        }
        let result = routine
            .result
            .as_ref()
            .and_then(|result| self.resolve_type(result));
        (
            RoutineSignature {
                parameters: formals,
                result,
                calling_convention: semantic_calling_convention(routine.calling_convention),
            },
            parameters,
        )
    }

    fn resolve_procedural_signature(
        &mut self,
        parameters: &[FormalParameterSyntax],
        result: Option<&TypeSyntax>,
        calling_convention: CallingConventionSyntax,
    ) -> RoutineSignature {
        let mut formals = Vec::new();
        for parameter in parameters {
            let ty = parameter
                .ty
                .as_ref()
                .and_then(|syntax| self.resolve_type(syntax))
                .unwrap_or(self.builtins.untyped_parameter);
            let mode = match parameter.mode {
                FormalModeSyntax::Value => ParameterMode::Value,
                FormalModeSyntax::Const => ParameterMode::Const,
                FormalModeSyntax::Var => ParameterMode::Var,
                FormalModeSyntax::Out => ParameterMode::Out,
                FormalModeSyntax::ConstRef => ParameterMode::ConstRef,
            };
            let default = parameter
                .default
                .as_ref()
                .and_then(|default| {
                    self.evaluate_constant_expression_with_modes(default, Some(ty), parameter.modes)
                })
                .map(|entry| entry.value);
            for _ in &parameter.names {
                formals.push(FormalParameter {
                    mode,
                    ty,
                    default: default.clone(),
                });
            }
        }
        RoutineSignature {
            parameters: formals,
            result: result.and_then(|result| self.resolve_type(result)),
            calling_convention: semantic_calling_convention(calling_convention),
        }
    }

    fn bind_body_tokens(&mut self, owner: Option<TypeRef>, tokens: &[Token]) {
        let environment = self.binder.scopes.current_environment();
        let fallback = tokens.last().map_or(0, |token| token.span.end);
        let parsed = chumsky_parser::parse_tokens(tokens, fallback);
        self.diagnostics.extend(parsed.diagnostics);
        let statements = parsed
            .statements
            .iter()
            .map(|statement| self.bind_statement(statement, owner))
            .collect();
        let span = tokens
            .first()
            .zip(tokens.last())
            .map_or(fallback..fallback, |(first, last)| {
                first.span.start..last.span.end
            });
        self.bodies.push(BoundBody {
            owner,
            environment,
            statements,
            span,
        });
    }

    fn bind_statement(&mut self, statement: &Statement, owner: Option<TypeRef>) -> BoundStatement {
        match statement {
            Statement::Expression(expression) => BoundStatement {
                span: expression.span.clone(),
                kind: BoundStatementKind::Expression(
                    self.bind_statement_expression(expression, owner),
                ),
            },
            Statement::Assignment(application) => BoundStatement {
                span: application.span.clone(),
                kind: BoundStatementKind::Assignment(self.bind_assignment(application, owner)),
            },
            Statement::Compound { statements, span } => BoundStatement {
                span: span.clone(),
                kind: BoundStatementKind::Compound(self.bind_scoped_statements(statements, owner)),
            },
            Statement::If {
                condition,
                then_branch,
                else_branch,
                modes,
                span,
            } => {
                let condition = self.bind_condition(condition, owner, *modes);
                let then_branch = Box::new(self.bind_scoped_statement(then_branch, owner));
                let else_branch = else_branch
                    .as_deref()
                    .map(|branch| Box::new(self.bind_scoped_statement(branch, owner)));
                BoundStatement {
                    span: span.clone(),
                    kind: BoundStatementKind::If {
                        condition,
                        then_branch,
                        else_branch,
                    },
                }
            }
            Statement::While {
                condition,
                body,
                modes,
                span,
            } => BoundStatement {
                span: span.clone(),
                kind: BoundStatementKind::While {
                    condition: self.bind_condition(condition, owner, *modes),
                    body: Box::new(self.bind_loop_body(body, owner)),
                },
            },
            Statement::Repeat {
                body,
                condition,
                modes,
                span,
            } => BoundStatement {
                span: span.clone(),
                kind: BoundStatementKind::Repeat {
                    body: self.bind_loop_statements(body, owner),
                    condition: self.bind_condition(condition, owner, *modes),
                },
            },
            Statement::For {
                control,
                initial,
                direction,
                final_value,
                body,
                span,
                modes,
            } => {
                let control = self.lookup_control_variable(control, span.clone());
                let mut initial = self.bind_expression(initial, owner);
                let mut final_value = self.bind_expression(final_value, owner);
                if let Some(symbol) = control {
                    let control_type = self.binder.scopes.symbol(symbol).kind.ty();
                    if let Some(destination) = control_type {
                        for value in [&mut initial, &mut final_value] {
                            self.apply_implicit_conversion(
                                value,
                                destination,
                                *modes,
                                "for-loop bound is not convertible to the control type",
                            );
                        }
                    }
                }
                BoundStatement {
                    span: span.clone(),
                    kind: BoundStatementKind::For {
                        control,
                        initial,
                        direction: *direction,
                        final_value,
                        body: Box::new(self.bind_loop_body(body, owner)),
                        modes: *modes,
                    },
                }
            }
            Statement::ForIn {
                control,
                source,
                body,
                span,
                modes,
            } => {
                let control = self.lookup_control_variable(control, span.clone());
                let source = self.bind_expression(source, owner);
                let element = source
                    .ty
                    .and_then(|ty| self.binder.types.sequence_element_type(ty));
                let mut element_conversion = None;
                if element.is_none() {
                    self.diagnostics.push(Diagnostic::new(
                        source.span.clone(),
                        "for-in source is not a sequence",
                    ));
                } else if let Some(symbol) = control {
                    let control_type = self.binder.scopes.symbol(symbol).kind.ty();
                    if let Some((destination, source)) = control_type.zip(element) {
                        let resolution = ConversionResolver::new(
                            &self.binder.types,
                            &self.binder.scopes,
                            self.binder.scopes.current_environment(),
                            *modes,
                        )
                        .resolve_implicit(destination, source);
                        if !matches!(resolution.selection, ConversionSelection::Selected { .. }) {
                            self.diagnostics.push(Diagnostic::new(
                                span.clone(),
                                "for-in element is not convertible to the control type",
                            ));
                        }
                        element_conversion = Some(resolution);
                    }
                }
                BoundStatement {
                    span: span.clone(),
                    kind: BoundStatementKind::ForIn {
                        control,
                        source,
                        element_conversion,
                        body: Box::new(self.bind_loop_body(body, owner)),
                        modes: *modes,
                    },
                }
            }
            Statement::Case {
                selector,
                arms,
                otherwise,
                span,
            } => {
                let selector = self.bind_expression(selector, owner);
                let selector_type = selector.ty;
                if selector_type
                    .and_then(|ty| self.binder.types.ordinal_domain(ty))
                    .is_none()
                {
                    self.diagnostics.push(Diagnostic::new(
                        selector.span.clone(),
                        "case selector must have an ordinal type",
                    ));
                }
                let mut occupied = Vec::<(i128, i128)>::new();
                let mut bound_arms = Vec::new();
                for arm in arms {
                    let mut labels = Vec::new();
                    for label in &arm.labels {
                        match label {
                            CaseLabel::Value(value) => {
                                let bound = self.bind_expression(value, owner);
                                if let Some(value) =
                                    self.evaluate_bound_ordinal(&bound, selector_type)
                                {
                                    self.check_case_interval(
                                        value,
                                        value,
                                        bound.span.clone(),
                                        &mut occupied,
                                    );
                                }
                                labels.push(BoundCaseLabel::Value(bound));
                            }
                            CaseLabel::Range { low, high } => {
                                let low = self.bind_expression(low, owner);
                                let high = self.bind_expression(high, owner);
                                let values = self
                                    .evaluate_bound_ordinal(&low, selector_type)
                                    .zip(self.evaluate_bound_ordinal(&high, selector_type));
                                if let Some((low_value, high_value)) = values {
                                    if low_value > high_value {
                                        self.diagnostics.push(Diagnostic::new(
                                            low.span.start..high.span.end,
                                            "case label range is reversed",
                                        ));
                                    } else {
                                        self.check_case_interval(
                                            low_value,
                                            high_value,
                                            low.span.start..high.span.end,
                                            &mut occupied,
                                        );
                                    }
                                }
                                labels.push(BoundCaseLabel::Range { low, high });
                            }
                        }
                    }
                    bound_arms.push(BoundCaseArm {
                        labels,
                        statement: self.bind_scoped_statement(&arm.statement, owner),
                        span: arm.span.clone(),
                    });
                }
                let otherwise = otherwise
                    .iter()
                    .map(|statement| self.bind_statement(statement, owner))
                    .collect();
                BoundStatement {
                    span: span.clone(),
                    kind: BoundStatementKind::Case {
                        selector,
                        arms: bound_arms,
                        otherwise,
                    },
                }
            }
            Statement::With {
                receivers,
                body,
                span,
            } => {
                let receivers = receivers
                    .iter()
                    .map(|receiver| self.bind_expression(receiver, owner))
                    .collect::<Vec<_>>();
                let mut edges = Vec::new();
                for receiver in &receivers {
                    let Some(environment) = receiver
                        .ty
                        .and_then(|ty| self.binder.types.member_environment(ty))
                    else {
                        self.diagnostics.push(Diagnostic::new(
                            receiver.span.clone(),
                            "with receiver has no members",
                        ));
                        continue;
                    };
                    let receiver_id = ReceiverId::from_index(self.next_receiver);
                    self.next_receiver += 1;
                    edges.push(LookupEdge::with_receiver(environment, receiver_id));
                }
                edges.reverse();
                let checkpoint = self.binder.scopes.push_overlay(edges);
                let body = Box::new(self.bind_scoped_statement(body, owner));
                self.binder.scopes.restore_environment(checkpoint);
                BoundStatement {
                    span: span.clone(),
                    kind: BoundStatementKind::With { receivers, body },
                }
            }
            Statement::Try {
                body,
                continuation,
                span,
            } => {
                let body = body
                    .iter()
                    .map(|statement| self.bind_statement(statement, owner))
                    .collect();
                let continuation = match continuation {
                    TryContinuation::Finally(statements) => BoundTryContinuation::Finally(
                        statements
                            .iter()
                            .map(|statement| self.bind_statement(statement, owner))
                            .collect(),
                    ),
                    TryContinuation::Except {
                        handlers,
                        otherwise,
                    } => BoundTryContinuation::Except {
                        handlers: handlers
                            .iter()
                            .map(|handler| self.bind_exception_handler(handler, owner))
                            .collect(),
                        otherwise: otherwise
                            .iter()
                            .map(|statement| self.bind_statement(statement, owner))
                            .collect(),
                    },
                };
                BoundStatement {
                    span: span.clone(),
                    kind: BoundStatementKind::Try { body, continuation },
                }
            }
            Statement::Raise {
                value,
                address,
                frame,
                span,
            } => BoundStatement {
                span: span.clone(),
                kind: BoundStatementKind::Raise {
                    value: value
                        .as_ref()
                        .map(|value| self.bind_expression(value, owner)),
                    address: address
                        .as_ref()
                        .map(|address| self.bind_expression(address, owner)),
                    frame: frame
                        .as_ref()
                        .map(|frame| self.bind_expression(frame, owner)),
                },
            },
            Statement::Goto { label, span } => BoundStatement {
                span: span.clone(),
                kind: BoundStatementKind::Goto {
                    label: self.lookup_label(label, span.clone()),
                },
            },
            Statement::Label {
                label,
                statement,
                span,
            } => BoundStatement {
                span: span.clone(),
                kind: BoundStatementKind::Label {
                    label: self.lookup_label(label, span.clone()),
                    statement: Box::new(self.bind_statement(statement, owner)),
                },
            },
            Statement::Break(span) => {
                if self.loop_depth == 0 {
                    self.diagnostics
                        .push(Diagnostic::new(span.clone(), "`break` used outside a loop"));
                }
                BoundStatement {
                    span: span.clone(),
                    kind: BoundStatementKind::Break,
                }
            }
            Statement::Continue(span) => {
                if self.loop_depth == 0 {
                    self.diagnostics.push(Diagnostic::new(
                        span.clone(),
                        "`continue` used outside a loop",
                    ));
                }
                BoundStatement {
                    span: span.clone(),
                    kind: BoundStatementKind::Continue,
                }
            }
            Statement::Exit { value, modes, span } => {
                let result = owner
                    .and_then(|owner| self.binder.types.callable(owner))
                    .and_then(|callable| callable.signature.result);
                let mut value = value
                    .as_ref()
                    .map(|value| self.bind_expression_with_expected(value, owner, result));
                match (result, value.as_mut()) {
                    (Some(destination), Some(value)) => {
                        self.apply_implicit_conversion(
                            value,
                            destination,
                            *modes,
                            "exit value is not convertible to the function result",
                        );
                    }
                    (None, Some(_)) => self.diagnostics.push(Diagnostic::new(
                        span.clone(),
                        "procedure exit cannot carry a result value",
                    )),
                    _ => {}
                }
                BoundStatement {
                    span: span.clone(),
                    kind: BoundStatementKind::Exit(value),
                }
            }
            Statement::InlineVariable {
                names,
                type_name,
                initializer,
                modes,
                span,
            } => {
                let explicit_type = type_name
                    .as_ref()
                    .and_then(|path| self.resolve_type_path_strings(path, span.clone()));
                let mut initializer = initializer.as_ref().map(|initializer| {
                    self.bind_expression_with_expected(initializer, owner, explicit_type)
                });
                let ty = explicit_type
                    .or_else(|| initializer.as_ref().and_then(|initializer| initializer.ty));
                let Some(ty) = ty else {
                    self.diagnostics.push(Diagnostic::new(
                        span.clone(),
                        "inline variable requires a resolvable type or initializer",
                    ));
                    return BoundStatement {
                        span: span.clone(),
                        kind: BoundStatementKind::InlineVariable {
                            symbols: Vec::new(),
                            initializer,
                            modes: *modes,
                        },
                    };
                };
                if let (Some(destination), Some(initializer)) =
                    (explicit_type, initializer.as_mut())
                {
                    self.apply_implicit_conversion(
                        initializer,
                        destination,
                        *modes,
                        "inline initializer is not convertible to its declared type",
                    );
                }
                let block = self.next_block;
                self.next_block += 1;
                let _ = self.binder.scopes.enter_region(RegionOwner::Block(block));
                let mut symbols = Vec::new();
                for spelling in names {
                    let name = self.binder.scopes.intern_name(spelling);
                    match self.binder.scopes.declare(
                        name,
                        SymbolKind::Variable(ty),
                        DeclarationState::Complete,
                        DeclarationMode::Fresh,
                    ) {
                        Ok(symbol) => symbols.push(symbol),
                        Err(error) => self.bind_error(span.clone(), error.into()),
                    }
                }
                BoundStatement {
                    span: span.clone(),
                    kind: BoundStatementKind::InlineVariable {
                        symbols,
                        initializer,
                        modes: *modes,
                    },
                }
            }
            Statement::Empty(span) => BoundStatement {
                span: span.clone(),
                kind: BoundStatementKind::Empty,
            },
            Statement::Error(span) => BoundStatement {
                span: span.clone(),
                kind: BoundStatementKind::Error,
            },
        }
    }

    fn bind_loop_body(&mut self, statement: &Statement, owner: Option<TypeRef>) -> BoundStatement {
        self.loop_depth += 1;
        let result = self.bind_scoped_statement(statement, owner);
        self.loop_depth -= 1;
        result
    }

    fn bind_loop_statements(
        &mut self,
        statements: &[Statement],
        owner: Option<TypeRef>,
    ) -> Vec<BoundStatement> {
        self.loop_depth += 1;
        let result = self.bind_scoped_statements(statements, owner);
        self.loop_depth -= 1;
        result
    }

    fn bind_scoped_statement(
        &mut self,
        statement: &Statement,
        owner: Option<TypeRef>,
    ) -> BoundStatement {
        let block = self.next_block;
        self.next_block += 1;
        let (_, checkpoint) = self.binder.scopes.enter_region(RegionOwner::Block(block));
        let result = self.bind_statement(statement, owner);
        self.binder.scopes.exit_region(checkpoint);
        result
    }

    fn bind_scoped_statements(
        &mut self,
        statements: &[Statement],
        owner: Option<TypeRef>,
    ) -> Vec<BoundStatement> {
        let block = self.next_block;
        self.next_block += 1;
        let (_, checkpoint) = self.binder.scopes.enter_region(RegionOwner::Block(block));
        let result = statements
            .iter()
            .map(|statement| self.bind_statement(statement, owner))
            .collect();
        self.binder.scopes.exit_region(checkpoint);
        result
    }

    fn bind_condition(
        &mut self,
        expression: &Expr,
        owner: Option<TypeRef>,
        modes: crate::ModeSnapshot,
    ) -> BoundExpression {
        let mut condition = self.bind_expression_for(expression, owner, SemanticUse::Condition);
        self.apply_implicit_conversion(
            &mut condition,
            self.builtins.boolean,
            modes,
            "condition is not Boolean-compatible",
        );
        condition
    }

    fn lookup_control_variable(&mut self, spelling: &str, span: Span) -> Option<SymbolId> {
        let name = self.binder.scopes.intern_name(spelling);
        let result = self.binder.scopes.lookup_symbol(
            self.binder.scopes.current_environment(),
            name,
            LookupRequest {
                accepted: SymbolFilter::Category(SymbolCategory::Variable),
                barrier: LookupBarrier::AnyDeclaration,
            },
        );
        let symbol = result.and_then(|result| result.primary.first().map(|hit| hit.symbol));
        if symbol.is_none()
            || !symbol.is_some_and(|symbol| {
                matches!(
                    self.binder.scopes.symbol(symbol).kind,
                    SymbolKind::Variable(_)
                )
            })
        {
            self.diagnostics.push(Diagnostic::new(
                span,
                format!("for-loop control `{spelling}` is not a variable"),
            ));
            return None;
        }
        symbol
    }

    fn lookup_label(&mut self, spelling: &str, span: Span) -> Option<SymbolId> {
        let name = self.binder.scopes.intern_name(spelling);
        let result = self.binder.scopes.lookup_symbol(
            self.binder.scopes.current_environment(),
            name,
            LookupRequest {
                accepted: SymbolFilter::Category(SymbolCategory::Label),
                barrier: LookupBarrier::AcceptedDeclaration,
            },
        );
        let symbol = result.and_then(|result| result.primary.first().map(|hit| hit.symbol));
        if symbol.is_none() {
            self.diagnostics
                .push(Diagnostic::new(span, format!("unknown label `{spelling}`")));
        }
        symbol
    }

    fn bind_exception_handler(
        &mut self,
        handler: &crate::ExceptionHandler,
        owner: Option<TypeRef>,
    ) -> BoundExceptionHandler {
        let exception_name = self.binder.scopes.intern_name(&handler.exception_type);
        let exception_type = self
            .binder
            .scopes
            .lookup_symbol(
                self.binder.scopes.current_environment(),
                exception_name,
                LookupRequest::REQUIRED_TYPE,
            )
            .and_then(|result| {
                self.binder
                    .scopes
                    .symbol(result.primary[0].symbol)
                    .kind
                    .ty()
            });
        if exception_type.is_none() {
            self.diagnostics.push(Diagnostic::new(
                handler.span.clone(),
                format!("unknown exception type `{}`", handler.exception_type),
            ));
        }

        let block = self.next_block;
        self.next_block += 1;
        let (_, checkpoint) = self.binder.scopes.enter_region(RegionOwner::Block(block));
        let variable = handler.variable.as_ref().and_then(|variable| {
            let ty = exception_type?;
            let name = self.binder.scopes.intern_name(variable);
            self.binder
                .scopes
                .declare(
                    name,
                    SymbolKind::Variable(ty),
                    DeclarationState::Complete,
                    DeclarationMode::Fresh,
                )
                .map_err(|error| self.bind_error(handler.span.clone(), error.into()))
                .ok()
        });
        let body = self.bind_statement(&handler.body, owner);
        self.binder.scopes.exit_region(checkpoint);
        BoundExceptionHandler {
            variable,
            exception_type,
            body,
            span: handler.span.clone(),
        }
    }

    fn bind_expression(&mut self, expression: &Expr, owner: Option<TypeRef>) -> BoundExpression {
        self.bind_expression_for(expression, owner, SemanticUse::Value)
    }

    fn bind_expression_with_expected(
        &mut self,
        expression: &Expr,
        owner: Option<TypeRef>,
        _expected: Option<TypeRef>,
    ) -> BoundExpression {
        self.bind_expression(expression, owner)
    }

    fn bind_expression_for(
        &mut self,
        expression: &Expr,
        owner: Option<TypeRef>,
        semantic_use: SemanticUse,
    ) -> BoundExpression {
        let mut bound = self.bind_expression_raw(expression, owner);
        bound.semantic_use = semantic_use;
        bound = self.bind_property_use(bound, semantic_use, expression_modes(expression));
        if matches!(semantic_use, SemanticUse::Value | SemanticUse::Condition) {
            bound = self.bind_implicit_zero_argument_call(
                bound,
                owner,
                expression_modes(expression),
                false,
            );
            bound = self.reject_bare_routine_designator(bound);
        }
        if bound.category == ExpressionCategory::Error {
            return bound;
        }
        let accepted = match semantic_use {
            SemanticUse::Value | SemanticUse::Condition => !matches!(
                bound.category,
                ExpressionCategory::Property {
                    readable: false,
                    ..
                }
            ),
            SemanticUse::MutablePlace => bound.category.is_mutable_storage(),
            SemanticUse::AssignmentTarget => bound.category.is_assignment_target(),
            SemanticUse::Address => bound.category.is_addressable(),
        };
        if !accepted {
            let description = match semantic_use {
                SemanticUse::Value => "expression does not produce a readable value",
                SemanticUse::MutablePlace => "expression is not mutable storage",
                SemanticUse::AssignmentTarget => "left side of assignment is not writable",
                SemanticUse::Condition => "condition does not produce a readable value",
                SemanticUse::Address => "address operand is not storage",
            };
            self.diagnostics
                .push(Diagnostic::new(expression.span.clone(), description));
        }
        bound
    }

    fn bind_statement_expression(
        &mut self,
        expression: &Expr,
        owner: Option<TypeRef>,
    ) -> BoundExpression {
        let mut bound = self.bind_expression_raw(expression, owner);
        bound.semantic_use = SemanticUse::Value;
        bound = self.bind_property_use(bound, SemanticUse::Value, expression_modes(expression));
        bound =
            self.bind_implicit_zero_argument_call(bound, owner, expression_modes(expression), true);
        self.reject_bare_routine_designator(bound)
    }

    fn reject_bare_routine_designator(
        &mut self,
        mut expression: BoundExpression,
    ) -> BoundExpression {
        let routine = match &expression.kind {
            BoundExpressionKind::Symbol { symbol, .. }
            | BoundExpressionKind::Member { symbol, .. } => matches!(
                self.binder.scopes.symbol(*symbol).kind,
                SymbolKind::Routine(_)
            ),
            _ => false,
        };
        if routine {
            self.diagnostics.push(Diagnostic::new(
                expression.span.clone(),
                "a procedural value requires explicit `@Routine` syntax",
            ));
            expression.ty = None;
            expression.category = ExpressionCategory::Error;
        }
        expression
    }

    fn bind_implicit_zero_argument_call(
        &mut self,
        expression: BoundExpression,
        owner: Option<TypeRef>,
        modes: crate::ModeSnapshot,
        include_procedure: bool,
    ) -> BoundExpression {
        if matches!(expression.kind, BoundExpressionKind::Application { .. }) {
            return expression;
        }
        let forwarded_operands = match &expression.kind {
            BoundExpressionKind::Inherited {
                forward_parameters: true,
                ..
            } => self
                .active_routines
                .last()
                .map(|active| {
                    active
                        .parameters
                        .iter()
                        .map(|symbol| {
                            let kind = self.binder.scopes.symbol(*symbol).kind.clone();
                            BoundExpression {
                                kind: BoundExpressionKind::Symbol {
                                    symbol: *symbol,
                                    receiver: None,
                                },
                                ty: kind.ty(),
                                category: expression_category_for_symbol(&kind),
                                semantic_use: SemanticUse::Value,
                                conversion: None,
                                span: expression.span.clone(),
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let candidates = match &expression.kind {
            BoundExpressionKind::Symbol { symbol, .. } => {
                let symbol_info = self.binder.scopes.symbol(*symbol);
                if !matches!(symbol_info.kind, SymbolKind::Routine(_)) {
                    return expression;
                }
                self.binder
                    .scopes
                    .lookup_symbol(
                        self.binder.scopes.current_environment(),
                        symbol_info.name,
                        LookupRequest::ORDINARY,
                    )
                    .map_or_else(Vec::new, |lookup| {
                        self.callable_candidates(&lookup, &forwarded_operands, modes, owner)
                    })
            }
            BoundExpressionKind::Member { .. } | BoundExpressionKind::Inherited { .. } => {
                self.application_candidates_from_callee(&expression)
            }
            _ => return expression,
        };
        let has_forwarded_operands = !forwarded_operands.is_empty();
        let candidates = candidates
            .into_iter()
            .filter(|candidate| {
                candidate
                    .callable_type()
                    .and_then(|ty| self.binder.types.callable(ty))
                    .is_some_and(|callable| {
                        (include_procedure || callable.signature.result.is_some())
                            && (has_forwarded_operands
                                || callable
                                    .signature
                                    .parameters
                                    .iter()
                                    .all(|parameter| parameter.default.is_some()))
                    })
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return expression;
        }
        let resolution = self.resolve_application(candidates, &forwarded_operands, modes);
        self.report_application_resolution(
            &resolution,
            expression.span.clone(),
            "implicit zero-argument call",
        );
        let ty = resolution.result_type();
        let span = expression.span.clone();
        BoundExpression {
            kind: BoundExpressionKind::Application {
                target: BoundApplicationTarget::Routine { resolution },
                callee: Some(Box::new(expression)),
                operands: forwarded_operands,
                modes,
            },
            ty,
            category: ExpressionCategory::Temporary,
            semantic_use: SemanticUse::Value,
            conversion: None,
            span,
        }
    }

    fn bind_property_use(
        &mut self,
        mut expression: BoundExpression,
        semantic_use: SemanticUse,
        modes: crate::ModeSnapshot,
    ) -> BoundExpression {
        let (symbol, lookup_receiver, explicit_receiver, indices) = match &expression.kind {
            BoundExpressionKind::Property {
                base,
                symbol,
                lookup_receiver,
                indices,
                ..
            } => (*symbol, *lookup_receiver, base.is_some(), indices.clone()),
            _ => return expression,
        };
        let SymbolKind::Property(property) = self.binder.scopes.symbol(symbol).kind.clone() else {
            return expression;
        };
        if !matches!(semantic_use, SemanticUse::Value | SemanticUse::Condition) {
            return expression;
        }
        let Some(callable_type) = property.read_contract else {
            return expression;
        };
        let (candidates, accessor_symbols) = self.property_accessor_candidates(
            symbol,
            &property,
            PropertyAccessKind::Read,
            callable_type,
            lookup_receiver,
            explicit_receiver,
        );
        let resolution = self.resolve_application(candidates, &indices, modes);
        self.report_application_resolution(&resolution, expression.span.clone(), "property read");
        if let BoundExpressionKind::Property { binding, .. } = &mut expression.kind {
            *binding = Some(Box::new(BoundPropertyBinding {
                kind: PropertyAccessKind::Read,
                resolution,
                accessor_symbols,
            }));
        }
        expression
    }

    fn bind_property_write(
        &mut self,
        target: &mut BoundExpression,
        source: &mut BoundExpression,
        modes: crate::ModeSnapshot,
    ) -> Option<ConversionResolution> {
        let (symbol, lookup_receiver, explicit_receiver, mut operands) = match &target.kind {
            BoundExpressionKind::Property {
                base,
                symbol,
                lookup_receiver,
                indices,
                ..
            } => (*symbol, *lookup_receiver, base.is_some(), indices.clone()),
            _ => return None,
        };
        let SymbolKind::Property(property) = self.binder.scopes.symbol(symbol).kind.clone() else {
            return None;
        };
        let callable_type = property.write_contract?;
        operands.push(source.clone());
        let (candidates, accessor_symbols) = self.property_accessor_candidates(
            symbol,
            &property,
            PropertyAccessKind::Write,
            callable_type,
            lookup_receiver,
            explicit_receiver,
        );
        let resolution = self.resolve_application(candidates, &operands, modes);
        self.report_application_resolution(&resolution, target.span.clone(), "property write");
        let conversion = resolution
            .selected_attempt()
            .and_then(|attempt| attempt.arguments.last())
            .and_then(|argument| match argument.conversion.as_ref()? {
                super::ArgumentConversion::Implicit(conversion)
                | super::ArgumentConversion::Explicit(conversion) => Some(conversion.clone()),
                super::ArgumentConversion::Storage(_) => None,
            });
        source.conversion = conversion.clone();
        if let BoundExpressionKind::Property { binding, .. } = &mut target.kind {
            *binding = Some(Box::new(BoundPropertyBinding {
                kind: PropertyAccessKind::Write,
                resolution,
                accessor_symbols,
            }));
        }
        conversion
    }

    fn property_accessor_candidates(
        &self,
        property_symbol: SymbolId,
        property: &PropertySymbol,
        access: PropertyAccessKind,
        fallback_contract: TypeRef,
        lookup_receiver: Option<ReceiverId>,
        explicit_receiver: bool,
    ) -> (Vec<ApplicationCandidate>, Vec<SymbolId>) {
        let accessor = match access {
            PropertyAccessKind::Read => property.read.as_ref(),
            PropertyAccessKind::Write => property.write.as_ref(),
        };
        let Some(accessor) = accessor else {
            return (Vec::new(), Vec::new());
        };
        let mut symbols = Vec::new();
        let mut candidates = Vec::new();
        let mut field_accessor = false;
        for symbol in &accessor.symbols {
            match self.binder.scopes.symbol(*symbol).kind {
                SymbolKind::Routine(callable_type)
                    if self.property_accessor_signature_matches(
                        property,
                        access,
                        callable_type,
                    ) =>
                {
                    let Some(callable) = self.binder.types.callable(callable_type) else {
                        continue;
                    };
                    let receiver = match callable.flavor {
                        CallableFlavor::Routine => ApplicationReceiver::None,
                        CallableFlavor::Nested => ApplicationReceiver::StaticLink,
                        CallableFlavor::Method if explicit_receiver => {
                            ApplicationReceiver::Explicit
                        }
                        CallableFlavor::Method => lookup_receiver.map_or(
                            ApplicationReceiver::ImplicitSelf,
                            ApplicationReceiver::Lookup,
                        ),
                        CallableFlavor::ClassMethod if explicit_receiver => {
                            ApplicationReceiver::Explicit
                        }
                        CallableFlavor::ClassMethod => lookup_receiver.map_or(
                            ApplicationReceiver::ImplicitSelf,
                            ApplicationReceiver::Lookup,
                        ),
                    };
                    symbols.push(*symbol);
                    candidates.push(ApplicationCandidate::Routine {
                        symbol: *symbol,
                        callable_type,
                        receiver,
                    });
                }
                SymbolKind::Variable(field_type)
                    if property.parameters.is_empty()
                        && self
                            .binder
                            .types
                            .same_formal_contract(field_type, property.ty) =>
                {
                    symbols.push(*symbol);
                    field_accessor = true;
                }
                _ => {}
            }
        }
        if field_accessor || accessor.symbols.is_empty() {
            candidates.push(ApplicationCandidate::CallableValue {
                symbol: Some(property_symbol),
                callable_type: fallback_contract,
                receiver: ApplicationReceiver::CallableValue { lookup_receiver },
            });
        }
        (candidates, symbols)
    }

    fn property_accessor_signature_matches(
        &self,
        property: &PropertySymbol,
        access: PropertyAccessKind,
        callable_type: TypeRef,
    ) -> bool {
        let Some(callable) = self.binder.types.callable(callable_type) else {
            return false;
        };
        let expected_count =
            property.parameters.len() + usize::from(access == PropertyAccessKind::Write);
        if callable.signature.parameters.len() != expected_count {
            return false;
        }
        let indexes_match = callable
            .signature
            .parameters
            .iter()
            .zip(&property.parameters)
            .all(|(actual, declared)| {
                actual.mode == declared.mode
                    && self
                        .binder
                        .types
                        .same_formal_contract(actual.ty, declared.ty)
            });
        if !indexes_match {
            return false;
        }
        match access {
            PropertyAccessKind::Read => callable
                .signature
                .result
                .is_some_and(|result| self.binder.types.same_formal_contract(result, property.ty)),
            PropertyAccessKind::Write => {
                callable.signature.result.is_none()
                    && callable.signature.parameters.last().is_some_and(|value| {
                        value.mode == ParameterMode::Value
                            && self
                                .binder
                                .types
                                .same_formal_contract(value.ty, property.ty)
                    })
            }
        }
    }

    fn bind_expression_raw(
        &mut self,
        expression: &Expr,
        owner: Option<TypeRef>,
    ) -> BoundExpression {
        match &expression.kind {
            ExprKind::Identifier(name) => {
                self.bind_identifier(name, expression.span.clone(), owner, false)
            }
            ExprKind::Inherited(name) => {
                self.bind_inherited_expression(name.as_deref(), expression.span.clone())
            }
            ExprKind::Literal(literal) => BoundExpression {
                ty: match literal {
                    Literal::Integer(_) => Some(self.builtins.integer),
                    Literal::Boolean(_) => Some(self.builtins.boolean),
                    Literal::String(value) if value.chars().count() == 1 => {
                        Some(self.builtins.character)
                    }
                    Literal::String(value) => Some(self.allocate_anonymous(StringLiteralType {
                        element: self.builtins.character,
                        index: self.builtins.integer,
                        length: self.builtins.integer,
                        character_count: u32::try_from(value.chars().count()).unwrap_or(u32::MAX),
                    })),
                    Literal::Real(_) => Some(self.builtins.real),
                    Literal::Nil => Some(self.builtins.nil),
                },
                kind: BoundExpressionKind::Literal(literal.clone()),
                category: ExpressionCategory::Value,
                semantic_use: SemanticUse::Value,
                conversion: None,
                span: expression.span.clone(),
            },
            ExprKind::Application(application) => self.bind_application(application, owner),
            ExprKind::Member { base, member } => {
                let base = self.bind_member_base(base, owner);
                let Some(base_type) = base.ty else {
                    return bound_error(expression.span.clone());
                };
                let Some(environment) = self.binder.types.member_environment(base_type) else {
                    self.diagnostics.push(Diagnostic::new(
                        expression.span.clone(),
                        "member access on a type without members",
                    ));
                    return bound_error(expression.span.clone());
                };
                let name = self.binder.scopes.intern_name(member);
                let Some(result) =
                    self.binder
                        .scopes
                        .lookup_symbol(environment, name, LookupRequest::ORDINARY)
                else {
                    self.diagnostics.push(Diagnostic::new(
                        expression.span.clone(),
                        format!("unknown member `{member}`"),
                    ));
                    return bound_error(expression.span.clone());
                };
                let symbol = result.primary[0].symbol;
                let symbol_kind = self.binder.scopes.symbol(symbol).kind.clone();
                let ty = symbol_kind.ty();
                BoundExpression {
                    kind: if matches!(symbol_kind, SymbolKind::Property(_)) {
                        BoundExpressionKind::Property {
                            base: Some(Box::new(base)),
                            lookup_receiver: None,
                            symbol,
                            indices: Vec::new(),
                            binding: None,
                        }
                    } else {
                        BoundExpressionKind::Member {
                            base: Box::new(base),
                            symbol,
                        }
                    },
                    ty,
                    category: expression_category_for_symbol(&symbol_kind),
                    semantic_use: SemanticUse::Value,
                    conversion: None,
                    span: expression.span.clone(),
                }
            }
            ExprKind::Index { base, indices, .. } => {
                let mut base = self.bind_expression_raw(base, owner);
                let mut indices = indices
                    .iter()
                    .map(|index| self.bind_expression(index, owner))
                    .collect::<Vec<_>>();
                let indexed_property = match &base.kind {
                    BoundExpressionKind::Property { symbol, .. } => {
                        matches!(
                            &self.binder.scopes.symbol(*symbol).kind,
                            SymbolKind::Property(property) if !property.parameters.is_empty()
                        )
                    }
                    _ => false,
                };
                if indexed_property {
                    if let BoundExpressionKind::Property {
                        indices: property_indices,
                        ..
                    } = &mut base.kind
                    {
                        property_indices.append(&mut indices);
                    }
                    base.span = expression.span.clone();
                    return base;
                }
                base =
                    self.bind_property_use(base, SemanticUse::Value, expression_modes(expression));
                let ty = base
                    .ty
                    .and_then(|ty| self.binder.types.sequence_element_type(ty));
                if ty.is_none()
                    && let Some(base_type) = base.ty
                    && let Some(symbol) = self.binder.types.default_property(base_type)
                {
                    let symbol_kind = self.binder.scopes.symbol(symbol).kind.clone();
                    return BoundExpression {
                        kind: BoundExpressionKind::Property {
                            base: Some(Box::new(base)),
                            lookup_receiver: None,
                            symbol,
                            indices,
                            binding: None,
                        },
                        ty: symbol_kind.ty(),
                        category: expression_category_for_symbol(&symbol_kind),
                        semantic_use: SemanticUse::Value,
                        conversion: None,
                        span: expression.span.clone(),
                    };
                }
                if ty.is_none() {
                    self.diagnostics.push(Diagnostic::new(
                        expression.span.clone(),
                        "indexing requires a sequence type or a default property",
                    ));
                }
                let category = if base.category.is_mutable_storage() {
                    ExpressionCategory::Storage { mutable: true }
                } else {
                    ExpressionCategory::Temporary
                };
                BoundExpression {
                    kind: BoundExpressionKind::Index {
                        base: Box::new(base),
                        indices,
                    },
                    ty,
                    category,
                    semantic_use: SemanticUse::Value,
                    conversion: None,
                    span: expression.span.clone(),
                }
            }
            ExprKind::Dereference(base) => {
                let base = self.bind_expression(base, owner);
                let ty = base.ty.and_then(|ty| self.binder.types.pointer_target(ty));
                if ty.is_none() {
                    self.diagnostics.push(Diagnostic::new(
                        expression.span.clone(),
                        "dereference requires a pointer type",
                    ));
                }
                BoundExpression {
                    kind: BoundExpressionKind::Dereference(Box::new(base)),
                    ty,
                    category: ExpressionCategory::Storage { mutable: true },
                    semantic_use: SemanticUse::Value,
                    conversion: None,
                    span: expression.span.clone(),
                }
            }
            ExprKind::Set(elements) => {
                let mut bound = Vec::new();
                for element in elements {
                    match element {
                        crate::SetElement::Value(value) => {
                            bound.push(BoundSetElement::Value(self.bind_expression(value, owner)));
                        }
                        crate::SetElement::Range { low, high } => {
                            bound.push(BoundSetElement::Range {
                                low: self.bind_expression(low, owner),
                                high: self.bind_expression(high, owner),
                            });
                        }
                    }
                }
                BoundExpression {
                    kind: BoundExpressionKind::Set(bound),
                    ty: None,
                    category: ExpressionCategory::Temporary,
                    semantic_use: SemanticUse::Value,
                    conversion: None,
                    span: expression.span.clone(),
                }
            }
            ExprKind::Error => bound_error(expression.span.clone()),
        }
    }

    fn bind_inherited_expression(&mut self, spelling: Option<&str>, span: Span) -> BoundExpression {
        let Some(active_type) = self.active_routines.last().map(|active| active.ty) else {
            self.diagnostics.push(Diagnostic::new(
                span.clone(),
                "`inherited` is only valid in a method body",
            ));
            return bound_error(span);
        };
        let Some(current) = self.binder.types.callable(active_type).cloned() else {
            return bound_error(span);
        };
        let RoutineOwner::Type(owner_type) = current.owner else {
            self.diagnostics.push(Diagnostic::new(
                span.clone(),
                "`inherited` is only valid in a method body",
            ));
            return bound_error(span);
        };

        let (symbols, forward_parameters) = if let Some(spelling) = spelling {
            let Some(base_type) = self.binder.types.base_type(owner_type) else {
                self.diagnostics.push(Diagnostic::new(
                    span.clone(),
                    "method owner has no inherited base type",
                ));
                return bound_error(span);
            };
            let Some(environment) = self.binder.types.member_environment(base_type) else {
                return bound_error(span);
            };
            let name = self.binder.scopes.intern_name(spelling);
            let Some(lookup) =
                self.binder
                    .scopes
                    .lookup_symbol(environment, name, LookupRequest::ORDINARY)
            else {
                self.diagnostics.push(Diagnostic::new(
                    span.clone(),
                    format!("no inherited member named `{spelling}`"),
                ));
                return bound_error(span);
            };
            let include_shadowed = lookup.primary.iter().any(|hit| {
                let SymbolKind::Routine(callable_type) = self.binder.scopes.symbol(hit.symbol).kind
                else {
                    return false;
                };
                self.binder
                    .types
                    .callable(callable_type)
                    .is_some_and(|callable| callable.overload)
            });
            let mut symbols = lookup
                .primary
                .iter()
                .filter_map(|hit| {
                    matches!(
                        self.binder.scopes.symbol(hit.symbol).kind,
                        SymbolKind::Routine(_)
                    )
                    .then_some(hit.symbol)
                })
                .collect::<Vec<_>>();
            if include_shadowed {
                symbols.extend(lookup.shadowed.iter().flatten().filter_map(|hit| {
                    matches!(
                        self.binder.scopes.symbol(hit.symbol).kind,
                        SymbolKind::Routine(_)
                    )
                    .then_some(hit.symbol)
                }));
            }
            symbols.sort_unstable();
            symbols.dedup();
            (symbols, false)
        } else {
            let Some(ancestor) = current.method.and_then(MethodMetadata::ancestor) else {
                self.diagnostics.push(Diagnostic::new(
                    span.clone(),
                    "current method has no exact inherited declaration",
                ));
                return bound_error(span);
            };
            (vec![ancestor], true)
        };
        let Some(callable_type) = symbols.iter().find_map(|symbol| {
            let SymbolKind::Routine(callable_type) = self.binder.scopes.symbol(*symbol).kind else {
                return None;
            };
            Some(callable_type)
        }) else {
            self.diagnostics.push(Diagnostic::new(
                span.clone(),
                "inherited member is not callable",
            ));
            return bound_error(span);
        };
        BoundExpression {
            kind: BoundExpressionKind::Inherited {
                symbols,
                forward_parameters,
            },
            ty: Some(callable_type),
            category: ExpressionCategory::Value,
            semantic_use: SemanticUse::Value,
            conversion: None,
            span,
        }
    }

    fn bind_member_base(&mut self, expression: &Expr, owner: Option<TypeRef>) -> BoundExpression {
        if let ExprKind::Identifier(spelling) = &expression.kind {
            let name = self.binder.scopes.intern_name(spelling);
            if let Some(lookup) = self.binder.scopes.lookup_symbol(
                self.binder.scopes.current_environment(),
                name,
                LookupRequest::ORDINARY,
            ) {
                let symbol = lookup.primary[0].symbol;
                if let SymbolKind::Type(instance_type) = self.binder.scopes.symbol(symbol).kind
                    && self.binder.types.is_class_type(instance_type)
                {
                    return BoundExpression {
                        kind: BoundExpressionKind::TypeIdentifier {
                            symbol,
                            instance_type,
                        },
                        ty: Some(instance_type),
                        category: ExpressionCategory::Value,
                        semantic_use: SemanticUse::Value,
                        conversion: None,
                        span: expression.span.clone(),
                    };
                }
            }
        }
        self.bind_expression(expression, owner)
    }

    fn bind_identifier(
        &mut self,
        spelling: &str,
        span: Span,
        owner: Option<TypeRef>,
        permit_type: bool,
    ) -> BoundExpression {
        let name = self.binder.scopes.intern_name(spelling);
        let Some(result) = self.binder.scopes.lookup_symbol(
            self.binder.scopes.current_environment(),
            name,
            LookupRequest::ORDINARY,
        ) else {
            self.diagnostics.push(Diagnostic::new(
                span.clone(),
                format!("unknown identifier `{spelling}`"),
            ));
            return bound_error(span);
        };
        let symbol = result.primary[0].symbol;
        let receiver = result.primary[0].receiver;
        let kind = self.binder.scopes.symbol(symbol).kind.clone();
        if matches!(kind, SymbolKind::Type(_)) && !permit_type {
            self.diagnostics.push(Diagnostic::new(
                span.clone(),
                format!("type `{spelling}` is not a value"),
            ));
        }
        self.note_capture(owner, symbol);
        BoundExpression {
            ty: kind.ty(),
            kind: if matches!(kind, SymbolKind::Property(_)) {
                BoundExpressionKind::Property {
                    base: None,
                    lookup_receiver: receiver,
                    symbol,
                    indices: Vec::new(),
                    binding: None,
                }
            } else {
                BoundExpressionKind::Symbol { symbol, receiver }
            },
            category: expression_category_for_symbol(&kind),
            semantic_use: SemanticUse::Value,
            conversion: None,
            span,
        }
    }

    fn bind_assignment(
        &mut self,
        application: &Application,
        owner: Option<TypeRef>,
    ) -> BoundAssignment {
        let mut operands = application.operands.iter();
        let mut target = operands.next().map_or_else(
            || bound_error(application.span.clone()),
            |target| self.bind_expression_for(target, owner, SemanticUse::AssignmentTarget),
        );
        let mut source = operands.next().map_or_else(
            || bound_error(application.span.clone()),
            |source| self.bind_expression_for(source, owner, SemanticUse::Value),
        );
        if operands.next().is_some() || application.operands.len() != 2 {
            self.diagnostics.push(Diagnostic::new(
                application.span.clone(),
                "assignment requires exactly one target and one source",
            ));
        }
        let conversion = match (target.ty, source.ty) {
            (Some(_), Some(_))
                if matches!(target.kind, BoundExpressionKind::Property { .. })
                    && target.category.is_assignment_target() =>
            {
                self.bind_property_write(&mut target, &mut source, application.modes)
            }
            (Some(destination), Some(_)) if target.category.is_assignment_target() => self
                .apply_implicit_conversion(
                    &mut source,
                    destination,
                    application.modes,
                    "assignment source is not convertible to the target type",
                ),
            _ => None,
        };
        target.semantic_use = SemanticUse::AssignmentTarget;
        BoundAssignment {
            target,
            source,
            conversion,
            modes: application.modes,
        }
    }

    fn bind_application(
        &mut self,
        application: &Application,
        owner: Option<TypeRef>,
    ) -> BoundExpression {
        if let Callee::Expression(callee) = &application.callee
            && let ExprKind::Identifier(name) = &callee.kind
        {
            return self.bind_named_application(
                name,
                application.span.clone(),
                &application.operands,
                owner,
                application.modes,
            );
        }
        let address_operator = matches!(
            application.callee,
            Callee::Operator(Operator::Address | Operator::ProcedureSlotAddress)
        );
        let operands = application
            .operands
            .iter()
            .map(|operand| {
                if address_operator {
                    let mut bound = self.bind_expression_raw(operand, owner);
                    bound.semantic_use = SemanticUse::Address;
                    bound
                } else {
                    self.bind_expression_for(operand, owner, SemanticUse::Value)
                }
            })
            .collect::<Vec<_>>();
        match &application.callee {
            Callee::Expression(callee) => {
                let mut callee = self.bind_expression_raw(callee, owner);
                callee = self.bind_property_use(callee, SemanticUse::Value, application.modes);
                let candidates = self.application_candidates_from_callee(&callee);
                if candidates.is_empty() {
                    self.diagnostics.push(Diagnostic::new(
                        application.span.clone(),
                        "application operand is not callable",
                    ));
                    return BoundExpression {
                        kind: BoundExpressionKind::Application {
                            target: BoundApplicationTarget::Invalid,
                            callee: Some(Box::new(callee)),
                            operands,
                            modes: application.modes,
                        },
                        ty: None,
                        category: ExpressionCategory::Error,
                        semantic_use: SemanticUse::Value,
                        conversion: None,
                        span: application.span.clone(),
                    };
                };
                let routine_target = candidates
                    .iter()
                    .all(|candidate| matches!(candidate, ApplicationCandidate::Routine { .. }));
                let resolution = self.resolve_application(candidates, &operands, application.modes);
                self.report_application_resolution(
                    &resolution,
                    application.span.clone(),
                    "callable expression",
                );
                let result = resolution.result_type();
                let target = if routine_target {
                    BoundApplicationTarget::Routine { resolution }
                } else {
                    BoundApplicationTarget::CallableValue { resolution }
                };
                BoundExpression {
                    kind: BoundExpressionKind::Application {
                        target,
                        callee: Some(Box::new(callee)),
                        operands,
                        modes: application.modes,
                    },
                    ty: result,
                    category: ExpressionCategory::Temporary,
                    semantic_use: SemanticUse::Value,
                    conversion: None,
                    span: application.span.clone(),
                }
            }
            Callee::Operator(operator) => self.bind_operator_application(
                *operator,
                application.span.clone(),
                operands,
                application.modes,
            ),
        }
    }

    fn bind_named_application(
        &mut self,
        spelling: &str,
        span: Span,
        operand_syntax: &[Expr],
        owner: Option<TypeRef>,
        modes: crate::ModeSnapshot,
    ) -> BoundExpression {
        let name = self.binder.scopes.intern_name(spelling);
        let Some(result) = self.binder.scopes.lookup_symbol(
            self.binder.scopes.current_environment(),
            name,
            LookupRequest::ORDINARY,
        ) else {
            self.diagnostics.push(Diagnostic::new(
                span.clone(),
                format!("unknown application name `{spelling}`"),
            ));
            let operands = operand_syntax
                .iter()
                .map(|operand| self.bind_expression_for(operand, owner, SemanticUse::Value))
                .collect();
            return bound_application_error(span, operands, modes);
        };
        let type_operand_permissions = (0..operand_syntax.len())
            .map(|index| {
                result
                    .primary
                    .iter()
                    .chain(result.shadowed.iter().flatten())
                    .any(|hit| {
                        let Some(family) = self.builtin_families.family_for_symbol(hit.symbol)
                        else {
                            return false;
                        };
                        self.builtin_families
                            .get(family)
                            .contract
                            .permits_type_operand(index)
                    })
            })
            .collect::<Vec<_>>();
        let operands = operand_syntax
            .iter()
            .enumerate()
            .map(|(index, operand)| {
                if type_operand_permissions[index]
                    && let ExprKind::Identifier(type_name) = &operand.kind
                {
                    let name = self.binder.scopes.intern_name(type_name);
                    if let Some(type_lookup) = self.binder.scopes.lookup_symbol(
                        self.binder.scopes.current_environment(),
                        name,
                        LookupRequest::ORDINARY,
                    ) {
                        let symbol = type_lookup.primary[0].symbol;
                        if let SymbolKind::Type(represented_type) =
                            self.binder.scopes.symbol(symbol).kind
                        {
                            return BoundExpression {
                                kind: BoundExpressionKind::TypeOperand {
                                    symbol,
                                    represented_type,
                                },
                                ty: Some(represented_type),
                                category: ExpressionCategory::Value,
                                semantic_use: SemanticUse::Value,
                                conversion: None,
                                span: operand.span.clone(),
                            };
                        }
                    }
                }
                self.bind_expression_for(operand, owner, SemanticUse::Value)
            })
            .collect::<Vec<_>>();
        let primary = result.primary[0].symbol;
        let primary_receiver = result.primary[0].receiver;
        let primary_kind = self.binder.scopes.symbol(primary).kind.clone();
        let builtin_target = result.primary.iter().any(|hit| {
            self.builtin_families
                .family_for_symbol(hit.symbol)
                .is_some()
        });
        match primary_kind {
            SymbolKind::Type(destination) => {
                let resolution = self.resolve_application(
                    vec![ApplicationCandidate::Conversion { destination }],
                    &operands,
                    modes,
                );
                self.report_application_resolution(
                    &resolution,
                    span.clone(),
                    &format!("direct conversion to `{spelling}`"),
                );
                let result_type = resolution.result_type();
                BoundExpression {
                    kind: BoundExpressionKind::Application {
                        target: BoundApplicationTarget::Conversion {
                            destination,
                            resolution,
                        },
                        callee: None,
                        operands,
                        modes,
                    },
                    ty: result_type,
                    category: ExpressionCategory::Temporary,
                    semantic_use: SemanticUse::Value,
                    conversion: None,
                    span,
                }
            }
            SymbolKind::Routine(_) => {
                let candidates = self.callable_candidates(&result, &operands, modes, owner);
                let resolution = self.resolve_application(candidates, &operands, modes);
                self.report_application_resolution(
                    &resolution,
                    span.clone(),
                    &format!("overload for `{spelling}`"),
                );
                let result_type = resolution.result_type();
                BoundExpression {
                    kind: BoundExpressionKind::Application {
                        target: if builtin_target {
                            BoundApplicationTarget::Builtin { resolution }
                        } else {
                            BoundApplicationTarget::Routine { resolution }
                        },
                        callee: None,
                        operands,
                        modes,
                    },
                    ty: result_type,
                    category: ExpressionCategory::Temporary,
                    semantic_use: SemanticUse::Value,
                    conversion: None,
                    span,
                }
            }
            SymbolKind::Variable(callable_type)
                if self.binder.types.callable(callable_type).is_some() =>
            {
                self.note_capture(owner, primary);
                let resolution = self.resolve_application(
                    vec![ApplicationCandidate::CallableValue {
                        symbol: Some(primary),
                        callable_type,
                        receiver: ApplicationReceiver::CallableValue {
                            lookup_receiver: primary_receiver,
                        },
                    }],
                    &operands,
                    modes,
                );
                self.report_application_resolution(
                    &resolution,
                    span.clone(),
                    &format!("procedural value `{spelling}`"),
                );
                let result_type = resolution.result_type();
                BoundExpression {
                    kind: BoundExpressionKind::Application {
                        target: BoundApplicationTarget::CallableValue { resolution },
                        callee: None,
                        operands,
                        modes,
                    },
                    ty: result_type,
                    category: ExpressionCategory::Temporary,
                    semantic_use: SemanticUse::Value,
                    conversion: None,
                    span,
                }
            }
            SymbolKind::Property(property)
                if property.readable() && self.binder.types.callable(property.ty).is_some() =>
            {
                self.note_capture(owner, primary);
                let property_expression = BoundExpression {
                    kind: BoundExpressionKind::Property {
                        base: None,
                        lookup_receiver: primary_receiver,
                        symbol: primary,
                        indices: Vec::new(),
                        binding: None,
                    },
                    ty: Some(property.ty),
                    category: expression_category_for_symbol(&SymbolKind::Property(
                        property.clone(),
                    )),
                    semantic_use: SemanticUse::Value,
                    conversion: None,
                    span: span.clone(),
                };
                let property_expression =
                    self.bind_property_use(property_expression, SemanticUse::Value, modes);
                let resolution = self.resolve_application(
                    vec![ApplicationCandidate::CallableValue {
                        symbol: None,
                        callable_type: property.ty,
                        receiver: ApplicationReceiver::CallableValue {
                            lookup_receiver: None,
                        },
                    }],
                    &operands,
                    modes,
                );
                self.report_application_resolution(
                    &resolution,
                    span.clone(),
                    &format!("procedural property `{spelling}`"),
                );
                let result_type = resolution.result_type();
                BoundExpression {
                    kind: BoundExpressionKind::Application {
                        target: BoundApplicationTarget::CallableValue { resolution },
                        callee: Some(Box::new(property_expression)),
                        operands,
                        modes,
                    },
                    ty: result_type,
                    category: ExpressionCategory::Temporary,
                    semantic_use: SemanticUse::Value,
                    conversion: None,
                    span,
                }
            }
            _ => {
                self.diagnostics.push(Diagnostic::new(
                    span.clone(),
                    format!(
                        "nearest declaration `{spelling}` is not callable and blocks outer declarations"
                    ),
                ));
                bound_application_error(span, operands, modes)
            }
        }
    }

    fn bind_operator_application(
        &mut self,
        operator: Operator,
        span: Span,
        operands: Vec<BoundExpression>,
        modes: crate::ModeSnapshot,
    ) -> BoundExpression {
        if matches!(operator, Operator::Address | Operator::ProcedureSlotAddress) {
            return self.bind_address_application(operator, span, operands, modes);
        }
        if operator == Operator::Assign {
            self.diagnostics.push(Diagnostic::new(
                span.clone(),
                "assignment is a store operation, not an operator application",
            ));
            return bound_application_error(span, operands, modes);
        }
        let name = {
            let invocation = match operator {
                Operator::Positive | Operator::Negative | Operator::Not => {
                    Some(OperatorInvocation::UnaryToken)
                }
                Operator::Multiply
                | Operator::RealDivide
                | Operator::IntegerDivide
                | Operator::Modulo
                | Operator::And
                | Operator::ShiftLeft
                | Operator::ShiftRight
                | Operator::Add
                | Operator::Subtract
                | Operator::Or
                | Operator::Xor
                | Operator::Equal
                | Operator::NotEqual
                | Operator::Less
                | Operator::Greater
                | Operator::LessEqual
                | Operator::GreaterEqual
                | Operator::In => Some(OperatorInvocation::BinaryToken),
                Operator::Assign
                | Operator::Address
                | Operator::ProcedureSlotAddress
                | Operator::Is
                | Operator::As => None,
            };
            let logical_operands = !operands.is_empty()
                && operands.iter().all(|operand| {
                    operand.ty.is_some_and(|ty| {
                        self.binder.types.canonical_type(ty)
                            == self.binder.types.canonical_type(self.builtins.boolean)
                    })
                });
            let Some(identifier) = invocation.and_then(|invocation| {
                operator_invocation_identifier(
                    invocation,
                    operator.spelling(),
                    operands.len(),
                    modes.overflow_checks,
                    logical_operands,
                )
            }) else {
                self.diagnostics.push(Diagnostic::new(
                    span.clone(),
                    format!(
                        "operator `{}` has no canonical invocation identity",
                        operator.spelling()
                    ),
                ));
                return bound_application_error(span, operands, modes);
            };
            self.binder.scopes.intern_name(identifier)
        };
        let candidates = self
            .binder
            .scopes
            .lookup_symbol(
                self.binder.scopes.current_environment(),
                name,
                LookupRequest::ORDINARY,
            )
            .map_or_else(Vec::new, |result| {
                self.callable_candidates(&result, &operands, modes, None)
            });
        let resolution = self.resolve_application(candidates, &operands, modes);
        self.report_application_resolution(
            &resolution,
            span.clone(),
            &format!("`{}` operator", operator.spelling()),
        );
        let result_type = resolution.result_type();
        BoundExpression {
            kind: BoundExpressionKind::Application {
                target: BoundApplicationTarget::Operator {
                    operator,
                    resolution,
                },
                callee: None,
                operands,
                modes,
            },
            ty: result_type,
            category: ExpressionCategory::Temporary,
            semantic_use: SemanticUse::Value,
            conversion: None,
            span,
        }
    }

    fn bind_address_application(
        &mut self,
        operator: Operator,
        span: Span,
        mut operands: Vec<BoundExpression>,
        modes: crate::ModeSnapshot,
    ) -> BoundExpression {
        if operands.len() != 1 {
            self.diagnostics.push(Diagnostic::new(
                span.clone(),
                "address syntax requires exactly one operand",
            ));
            return bound_application_error(span, operands, modes);
        }
        let operand = operands.pop().unwrap();
        let routine_symbol = match &operand.kind {
            BoundExpressionKind::Symbol { symbol, .. }
            | BoundExpressionKind::Member { symbol, .. }
                if matches!(
                    self.binder.scopes.symbol(*symbol).kind,
                    SymbolKind::Routine(_)
                ) =>
            {
                Some(*symbol)
            }
            _ => None,
        };
        if let Some(symbol) = routine_symbol {
            if operator == Operator::ProcedureSlotAddress {
                self.diagnostics.push(Diagnostic::new(
                    span.clone(),
                    "`@@` requires procedural-variable storage, not a routine declaration",
                ));
                return bound_application_error(span, vec![operand], modes);
            }
            let ty = self.binder.scopes.symbol(symbol).kind.ty();
            if ty
                .and_then(|ty| self.binder.types.callable(ty))
                .is_some_and(|callable| callable.flavor == CallableFlavor::Nested)
            {
                self.diagnostics.push(Diagnostic::new(
                    span.clone(),
                    "nested routines cannot be converted to procedural values",
                ));
                return bound_application_error(span, vec![operand], modes);
            }
            return BoundExpression {
                kind: BoundExpressionKind::RoutineDesignator {
                    routine: Box::new(operand),
                    symbol,
                },
                ty,
                category: ExpressionCategory::Value,
                semantic_use: SemanticUse::Value,
                conversion: None,
                span,
            };
        }

        let Some(operand_type) = operand.ty else {
            self.diagnostics.push(Diagnostic::new(
                span.clone(),
                "address operand has no semantic type",
            ));
            return bound_application_error(span, vec![operand], modes);
        };
        if operator == Operator::Address
            && operand.category.is_addressable()
            && self.binder.types.callable(operand_type).is_some()
        {
            return BoundExpression {
                kind: BoundExpressionKind::ProcedureCode(Box::new(operand)),
                ty: Some(operand_type),
                category: ExpressionCategory::Value,
                semantic_use: SemanticUse::Value,
                conversion: None,
                span,
            };
        }
        if !operand.category.is_addressable() {
            self.diagnostics.push(Diagnostic::new(
                span.clone(),
                "address operand is not storage",
            ));
            return bound_application_error(span, vec![operand], modes);
        }
        if operator == Operator::ProcedureSlotAddress
            && self.binder.types.callable(operand_type).is_none()
        {
            self.diagnostics.push(Diagnostic::new(
                span.clone(),
                "`@@` requires procedural-variable storage",
            ));
            return bound_application_error(span, vec![operand], modes);
        }
        let pointer = self.allocate_anonymous(PointerType {
            target: operand_type,
            layout: pointer_layout(),
        });
        BoundExpression {
            kind: BoundExpressionKind::Address(Box::new(operand)),
            ty: Some(pointer),
            category: ExpressionCategory::Value,
            semantic_use: SemanticUse::Value,
            conversion: None,
            span,
        }
    }

    fn application_candidates_from_callee(
        &self,
        callee: &BoundExpression,
    ) -> Vec<ApplicationCandidate> {
        match &callee.kind {
            BoundExpressionKind::Member { base, symbol } => {
                let Some(base_type) = base.ty else {
                    return Vec::new();
                };
                let Some(environment) = self.binder.types.member_environment(base_type) else {
                    return Vec::new();
                };
                let name = self.binder.scopes.symbol(*symbol).name;
                let Some(lookup) =
                    self.binder
                        .scopes
                        .lookup_symbol(environment, name, LookupRequest::ORDINARY)
                else {
                    return Vec::new();
                };
                let include_inherited = lookup.primary.iter().any(|hit| {
                    let SymbolKind::Routine(callable_type) =
                        self.binder.scopes.symbol(hit.symbol).kind
                    else {
                        return false;
                    };
                    self.binder
                        .types
                        .callable(callable_type)
                        .is_some_and(|callable| callable.overload)
                });
                let class_identifier =
                    matches!(&base.kind, BoundExpressionKind::TypeIdentifier { .. });
                let instance_type = match &base.kind {
                    BoundExpressionKind::TypeIdentifier { instance_type, .. } => {
                        Some(*instance_type)
                    }
                    _ => None,
                };
                let mut seen = BTreeSet::new();
                return lookup
                    .primary
                    .iter()
                    .chain(
                        include_inherited
                            .then_some(lookup.shadowed.iter().flatten())
                            .into_iter()
                            .flatten(),
                    )
                    .filter_map(|hit| {
                        if !seen.insert(hit.symbol) {
                            return None;
                        }
                        let SymbolKind::Routine(callable_type) =
                            self.binder.scopes.symbol(hit.symbol).kind
                        else {
                            return None;
                        };
                        self.binder.types.callable(callable_type)?;
                        let receiver = if class_identifier {
                            ApplicationReceiver::ClassIdentifier(instance_type.unwrap())
                        } else {
                            ApplicationReceiver::Explicit
                        };
                        Some(ApplicationCandidate::Routine {
                            symbol: hit.symbol,
                            callable_type,
                            receiver,
                        })
                    })
                    .collect();
            }
            BoundExpressionKind::Inherited { symbols, .. } => {
                return symbols
                    .iter()
                    .filter_map(|symbol| {
                        let SymbolKind::Routine(callable_type) =
                            self.binder.scopes.symbol(*symbol).kind
                        else {
                            return None;
                        };
                        Some(ApplicationCandidate::Routine {
                            symbol: *symbol,
                            callable_type,
                            receiver: ApplicationReceiver::Inherited,
                        })
                    })
                    .collect();
            }
            BoundExpressionKind::RoutineDesignator { routine, .. } => {
                return self.application_candidates_from_callee(routine);
            }
            _ => {}
        }
        let (symbol, lookup_receiver) = match &callee.kind {
            BoundExpressionKind::Symbol { symbol, receiver } => (Some(*symbol), *receiver),
            _ => (None, None),
        };
        let Some(callable_type) = callee.ty else {
            return Vec::new();
        };
        let Some(callable) = self.binder.types.callable(callable_type) else {
            return Vec::new();
        };
        if let Some(symbol) = symbol
            && matches!(
                self.binder.scopes.symbol(symbol).kind,
                SymbolKind::Routine(_)
            )
        {
            return vec![ApplicationCandidate::Routine {
                symbol,
                callable_type,
                receiver: declared_receiver(callable.flavor, lookup_receiver, false),
            }];
        }
        vec![ApplicationCandidate::CallableValue {
            symbol,
            callable_type,
            receiver: ApplicationReceiver::CallableValue { lookup_receiver },
        }]
    }

    fn callable_candidates(
        &self,
        result: &LookupResult,
        operands: &[BoundExpression],
        modes: crate::ModeSnapshot,
        current_callable: Option<TypeRef>,
    ) -> Vec<ApplicationCandidate> {
        let actuals = operands
            .iter()
            .map(|operand| BuiltinActual {
                form: if matches!(operand.kind, BoundExpressionKind::TypeOperand { .. }) {
                    BuiltinOperandForm::Type
                } else {
                    BuiltinOperandForm::Value
                },
                ty: operand.ty,
                addressable: operand.category.is_addressable(),
            })
            .collect::<Vec<_>>();
        let builtin_types = BuiltinTypeContext {
            integer: self.builtins.integer,
            long_integer: self.builtins.long_integer,
            real: self.builtins.real,
            boolean: self.builtins.boolean,
            character: self.builtins.character,
            byte: self.builtins.byte,
            word: self.builtins.word,
            size_unsigned: self.builtins.size_unsigned,
        };
        let mut seen = BTreeSet::new();
        result
            .primary
            .iter()
            .chain(result.shadowed.iter().flatten())
            .filter_map(|hit| {
                if !seen.insert(hit.symbol) {
                    return None;
                }
                match self.binder.scopes.symbol(hit.symbol).kind {
                    SymbolKind::Routine(callable_type) => {
                        if let Some(family) = self.builtin_families.family_for_symbol(hit.symbol) {
                            let instantiation = self.builtin_families.get(family).instantiate(
                                &actuals,
                                &self.binder.types,
                                builtin_types,
                                modes,
                            );
                            return Some(ApplicationCandidate::Builtin {
                                symbol: hit.symbol,
                                family,
                                instantiation,
                            });
                        }
                        let flavor = self.binder.types.callable(callable_type)?.flavor;
                        let implicit_self = current_callable
                            .and_then(|current| self.binder.types.callable(current))
                            .is_some_and(|current| matches!(current.owner, RoutineOwner::Type(_)));
                        Some(ApplicationCandidate::Routine {
                            symbol: hit.symbol,
                            callable_type,
                            receiver: declared_receiver(flavor, hit.receiver, implicit_self),
                        })
                    }
                    _ => None,
                }
            })
            .collect()
    }

    fn resolve_application(
        &self,
        candidates: Vec<ApplicationCandidate>,
        operands: &[BoundExpression],
        modes: crate::ModeSnapshot,
    ) -> ApplicationResolution {
        let actuals = operands
            .iter()
            .map(|operand| ActualArgument {
                form: if matches!(operand.kind, BoundExpressionKind::TypeOperand { .. }) {
                    ActualArgumentForm::Type
                } else {
                    ActualArgumentForm::Value
                },
                ty: operand.ty,
                addressable: operand.category.is_addressable(),
            })
            .collect::<Vec<_>>();
        let conversions = ConversionResolver::new(
            &self.binder.types,
            &self.binder.scopes,
            self.binder.scopes.current_environment(),
            modes,
        );
        ApplicationResolver::new(
            &self.binder.types,
            self.builtins.untyped_parameter,
            &conversions,
        )
        .resolve(candidates, &actuals)
    }

    fn apply_implicit_conversion(
        &mut self,
        expression: &mut BoundExpression,
        destination: TypeRef,
        modes: crate::ModeSnapshot,
        diagnostic: &str,
    ) -> Option<ConversionResolution> {
        let source = expression.ty?;
        let resolution = ConversionResolver::new(
            &self.binder.types,
            &self.binder.scopes,
            self.binder.scopes.current_environment(),
            modes,
        )
        .resolve_implicit(destination, source);
        match &resolution.selection {
            ConversionSelection::Selected { .. } => {
                expression.ty = Some(destination);
                expression.conversion = Some(resolution.clone());
            }
            ConversionSelection::Ambiguous { attempts } => {
                self.diagnostics.push(Diagnostic::new(
                    expression.span.clone(),
                    format!(
                        "{diagnostic}: {} custom conversions are ambiguous",
                        attempts.len()
                    ),
                ));
            }
            ConversionSelection::NoViable => {
                self.diagnostics
                    .push(Diagnostic::new(expression.span.clone(), diagnostic));
            }
        }
        Some(resolution)
    }

    fn report_application_resolution(
        &mut self,
        resolution: &ApplicationResolution,
        span: Span,
        description: &str,
    ) {
        match &resolution.selection {
            ApplicationSelection::Selected { .. } => {}
            ApplicationSelection::Ambiguous { attempts } => {
                self.diagnostics.push(Diagnostic::new(
                    span,
                    format!(
                        "ambiguous {description}: {} viable candidates are incomparable",
                        attempts.len()
                    ),
                ));
            }
            ApplicationSelection::NoViable => {
                self.diagnostics
                    .push(Diagnostic::new(span, format!("no viable {description}")));
            }
        }
    }

    fn note_capture(&mut self, owner: Option<TypeRef>, symbol: SymbolId) {
        let Some(owner) = owner else {
            return;
        };
        let symbol_info = self.binder.scopes.symbol(symbol);
        let current_region = self
            .binder
            .scopes
            .environment_region(self.binder.scopes.current_environment());
        if symbol_info.region == current_region
            || !matches!(
                symbol_info.kind,
                SymbolKind::Variable(_) | SymbolKind::Property(_)
            )
        {
            return;
        }
        let _ = self.binder.types.add_capture(
            owner,
            Capture {
                symbol,
                lexical_depth: 1,
            },
        );
    }

    fn resolve_type(&mut self, syntax: &TypeSyntax) -> Option<TypeRef> {
        match &syntax.kind {
            TypeSyntaxKind::Named(path) => self.resolve_named_type(path, syntax.span.clone()),
            TypeSyntaxKind::External { .. } => Some(self.allocate_anonymous(opaque_type())),
            TypeSyntaxKind::Pointer(target) => {
                let target = self.resolve_type(target)?;
                Some(self.allocate_anonymous(PointerType {
                    target,
                    layout: pointer_layout(),
                }))
            }
            TypeSyntaxKind::Enumeration(members) => {
                let mut semantic_members = Vec::new();
                let mut previous = None;
                for member in members {
                    let value = if let Some(value) = &member.value {
                        self.evaluate_constant_expression_with_modes(value, None, syntax.modes)?
                            .value
                            .ordinal()?
                    } else {
                        previous.map_or(0, |value: i128| value.saturating_add(1))
                    };
                    let name = self.binder.scopes.intern_name(&member.name.spelling);
                    semantic_members.push(EnumMember { name, value });
                    previous = Some(value);
                }
                let domain = semantic_members
                    .first()
                    .zip(semantic_members.last())
                    .map_or(OrdinalDomain { lower: 0, upper: 0 }, |(first, last)| {
                        OrdinalDomain {
                            lower: first.value,
                            upper: last.value,
                        }
                    });
                Some(self.allocate_anonymous(EnumType {
                    members: semantic_members,
                    domain,
                    layout: StorageLayout {
                        size: 4,
                        alignment: 4,
                    },
                }))
            }
            TypeSyntaxKind::Subrange { lower, upper } => {
                let lower =
                    self.evaluate_constant_expression_with_modes(lower, None, syntax.modes)?;
                let upper = self.evaluate_constant_expression_with_modes(
                    upper,
                    Some(lower.ty),
                    syntax.modes,
                )?;
                let lower_value = lower.value.ordinal()?;
                let upper_value = upper.value.ordinal()?;
                let base = self.binder.types.ordinal_base_type(lower.ty)?;
                if self.binder.types.ordinal_base_type(upper.ty) != Some(base)
                    || lower_value > upper_value
                {
                    return None;
                }
                let layout = self.binder.types.storage_layout(base)?;
                Some(self.allocate_anonymous(SubrangeType {
                    base,
                    domain: OrdinalDomain {
                        lower: lower_value,
                        upper: upper_value,
                    },
                    layout,
                }))
            }
            TypeSyntaxKind::Array {
                indices,
                element,
                dynamic,
            } => {
                let element = element
                    .as_deref()
                    .and_then(|syntax| self.resolve_type(syntax))?;
                let implementation = self.build_array_implementation(indices, element, *dynamic)?;
                Some(self.allocate_anonymous(implementation))
            }
            TypeSyntaxKind::Procedural {
                method_pointer,
                parameters,
                result,
                calling_convention,
            } => {
                let flavor = if *method_pointer {
                    CallableFlavor::Method
                } else {
                    CallableFlavor::Routine
                };
                let signature = self.resolve_procedural_signature(
                    parameters,
                    result.as_deref(),
                    *calling_convention,
                );
                Some(self.allocate_anonymous(CallableType {
                    owner: RoutineOwner::Module,
                    flavor,
                    signature,
                    declaration_region: None,
                    nested_routines: Vec::new(),
                    local_types: Vec::new(),
                    captures: Vec::new(),
                    environment: EnvironmentRequirement::None,
                    has_body: false,
                    method: None,
                    overload: false,
                }))
            }
            TypeSyntaxKind::Set { element } => {
                let element = element
                    .as_deref()
                    .and_then(|syntax| self.resolve_type(syntax))?;
                let domain = self.binder.types.ordinal_domain(element)?;
                let size = domain
                    .cardinality()?
                    .checked_add(7)?
                    .checked_div(8)
                    .and_then(|size| u64::try_from(size).ok())?;
                Some(self.allocate_anonymous(SetType {
                    element,
                    domain,
                    layout: StorageLayout { size, alignment: 1 },
                }))
            }
            TypeSyntaxKind::Unsupported(_) => Some(self.allocate_anonymous(opaque_type())),
            TypeSyntaxKind::Aggregate { .. } | TypeSyntaxKind::ClassForward => {
                self.diagnostics.push(Diagnostic::new(
                    syntax.span.clone(),
                    "anonymous aggregate/forward type is not supported here",
                ));
                None
            }
        }
    }

    fn resolve_named_type(&mut self, path: &[SpannedName], span: Span) -> Option<TypeRef> {
        let first = path.first()?;
        let first_name = self.binder.scopes.intern_name(&first.spelling);
        let lookup = self.binder.scopes.lookup_symbol(
            self.binder.scopes.current_environment(),
            first_name,
            LookupRequest::REQUIRED_TYPE,
        );
        let Some(lookup) = lookup else {
            self.diagnostics.push(Diagnostic::new(
                span,
                format!("unknown type `{}`", first.spelling),
            ));
            return None;
        };
        let SymbolKind::Type(mut ty) = self.binder.scopes.symbol(lookup.primary[0].symbol).kind
        else {
            return None;
        };
        for member in &path[1..] {
            let Some(environment) = self.binder.types.member_environment(ty) else {
                self.diagnostics.push(Diagnostic::new(
                    member.span.clone(),
                    format!("type has no nested member `{}`", member.spelling),
                ));
                return None;
            };
            let member_name = self.binder.scopes.intern_name(&member.spelling);
            let Some(lookup) = self.binder.scopes.lookup_symbol(
                environment,
                member_name,
                LookupRequest::REQUIRED_TYPE,
            ) else {
                self.diagnostics.push(Diagnostic::new(
                    member.span.clone(),
                    format!("unknown nested type `{}`", member.spelling),
                ));
                return None;
            };
            let SymbolKind::Type(member_type) =
                self.binder.scopes.symbol(lookup.primary[0].symbol).kind
            else {
                return None;
            };
            ty = member_type;
        }
        Some(ty)
    }

    fn resolve_type_path_strings(&mut self, path: &[String], span: Span) -> Option<TypeRef> {
        let path = path
            .iter()
            .map(|spelling| SpannedName {
                spelling: spelling.clone(),
                span: span.clone(),
            })
            .collect::<Vec<_>>();
        self.resolve_named_type(&path, span)
    }

    fn allocate_anonymous(&mut self, implementation: impl super::PascalType + 'static) -> TypeRef {
        let owner = TypeOwner::Anonymous(NodeId::from_index(self.next_anonymous_type));
        self.next_anonymous_type += 1;
        self.binder.types.allocate_complete(
            owner,
            None,
            self.binder.scopes.current_environment(),
            implementation,
        )
    }

    fn system_external_type(&mut self, name: NameId) -> TypeRef {
        if let Some(lookup) = self.binder.scopes.lookup_symbol(
            self.intrinsic_types,
            name,
            LookupRequest::REQUIRED_TYPE,
        ) && let SymbolKind::Type(ty) = self.binder.scopes.symbol(lookup.primary[0].symbol).kind
        {
            return ty;
        }
        match self.binder.scopes.names().spelling(name) {
            "single" => self.allocate_anonymous(PrimitiveType {
                kind: PrimitiveKind::Real { bits: 32 },
                layout: StorageLayout {
                    size: 4,
                    alignment: 4,
                },
            }),
            "double" => self.allocate_anonymous(PrimitiveType {
                kind: PrimitiveKind::Real { bits: 64 },
                layout: StorageLayout {
                    size: 8,
                    alignment: 8,
                },
            }),
            "extended" => self.allocate_anonymous(PrimitiveType {
                kind: PrimitiveKind::Real { bits: 80 },
                layout: StorageLayout {
                    size: 16,
                    alignment: 16,
                },
            }),
            _ => self.allocate_anonymous(opaque_type()),
        }
    }

    fn define_error_type(&mut self, name: NameId, span: Span) {
        if let Err(error) = self.binder.define_type(name, opaque_type()) {
            self.bind_error(span, error);
        }
    }

    fn bind_error(&mut self, span: Span, error: BindError) {
        self.diagnostics.push(Diagnostic::new(
            span,
            format!("semantic binding: {error:?}"),
        ));
    }
}

fn declared_receiver(
    flavor: CallableFlavor,
    lookup_receiver: Option<ReceiverId>,
    implicit_self: bool,
) -> ApplicationReceiver {
    match (flavor, lookup_receiver) {
        (_, Some(receiver)) => ApplicationReceiver::Lookup(receiver),
        (CallableFlavor::Nested, None) => ApplicationReceiver::StaticLink,
        (CallableFlavor::Method | CallableFlavor::ClassMethod, None) if implicit_self => {
            ApplicationReceiver::ImplicitSelf
        }
        (CallableFlavor::Routine | CallableFlavor::Method | CallableFlavor::ClassMethod, None) => {
            ApplicationReceiver::None
        }
    }
}

fn routine_declaration_mode(routine: &RoutineDeclarationSyntax) -> DeclarationMode {
    if routine.overload || routine.kind == RoutineSyntaxKind::Operator {
        DeclarationMode::Overload
    } else {
        DeclarationMode::Fresh
    }
}

fn system_operator_contract(canonical_name: &str) -> Option<BuiltinContract> {
    let operator = match canonical_name {
        "&op_checkedaddition" | "&op_addition" => Operator::Add,
        "&op_checkedsubtraction" | "&op_subtraction" => Operator::Subtract,
        "&op_checkedmultiply" | "&op_multiply" => Operator::Multiply,
        "&op_checkedintdivide" | "&op_intdivide" => Operator::IntegerDivide,
        "&op_division" => Operator::RealDivide,
        "&op_modulus" => Operator::Modulo,
        "&op_leftshift" => Operator::ShiftLeft,
        "&op_rightshift" => Operator::ShiftRight,
        "&op_checkedunarynegation" | "&op_unarynegation" => Operator::Negative,
        "&op_unaryplus" => Operator::Positive,
        "&op_logicalnot" => Operator::Not,
        "&op_equality" => Operator::Equal,
        "&op_inequality" => Operator::NotEqual,
        "&op_greaterthan" => Operator::Greater,
        "&op_greaterthanorequal" => Operator::GreaterEqual,
        "&op_lessthan" => Operator::Less,
        "&op_lessthanorequal" => Operator::LessEqual,
        "&op_logicaland" | "&op_bitwiseand" => Operator::And,
        "&op_logicalor" | "&op_bitwiseor" => Operator::Or,
        "&op_logicalxor" | "&op_bitwisexor" => Operator::Xor,
        "&op_in" => Operator::In,
        // Implicit/explicit declarations are conversions, not assignment.
        "&op_checkedimplicit" | "&op_implicit" | "&op_explicit" => return None,
        _ => return None,
    };
    Some(BuiltinContract::Operator(operator))
}

fn semantic_calling_convention(syntax: CallingConventionSyntax) -> CallingConvention {
    match syntax {
        CallingConventionSyntax::Pascal => CallingConvention::Pascal,
        CallingConventionSyntax::Register => CallingConvention::Register,
        CallingConventionSyntax::Cdecl => CallingConvention::Cdecl,
        CallingConventionSyntax::Stdcall => CallingConvention::Stdcall,
    }
}

fn expression_modes(expression: &Expr) -> crate::ModeSnapshot {
    match &expression.kind {
        ExprKind::Application(application) => application.modes,
        ExprKind::Index { modes, .. } => *modes,
        _ => crate::ModeSnapshot::default(),
    }
}

fn expression_category_for_symbol(kind: &SymbolKind) -> ExpressionCategory {
    match kind {
        SymbolKind::Variable(_) => ExpressionCategory::Storage { mutable: true },
        SymbolKind::Property(property) => ExpressionCategory::Property {
            readable: property.readable(),
            writable: property.writable(),
        },
        SymbolKind::Type(_)
        | SymbolKind::Routine(_)
        | SymbolKind::Constant(_)
        | SymbolKind::Label => ExpressionCategory::Value,
    }
}

fn bound_error(span: Span) -> BoundExpression {
    BoundExpression {
        kind: BoundExpressionKind::Error,
        ty: None,
        category: ExpressionCategory::Error,
        semantic_use: SemanticUse::Value,
        conversion: None,
        span,
    }
}

fn bound_application_error(
    span: Span,
    operands: Vec<BoundExpression>,
    modes: crate::ModeSnapshot,
) -> BoundExpression {
    BoundExpression {
        kind: BoundExpressionKind::Application {
            target: BoundApplicationTarget::Invalid,
            callee: None,
            operands,
            modes,
        },
        ty: None,
        category: ExpressionCategory::Error,
        semantic_use: SemanticUse::Value,
        conversion: None,
        span,
    }
}

fn strip_compound_body(mut tokens: Vec<Token>) -> Vec<Token> {
    if tokens.first().is_some_and(
        |token| matches!(&token.kind, crate::TokenKind::Identifier(name) if name == "begin"),
    ) {
        tokens.remove(0);
    }
    if tokens.last().is_some_and(
        |token| matches!(&token.kind, crate::TokenKind::Identifier(name) if name == "end"),
    ) {
        tokens.pop();
    }
    tokens
}

fn pointer_layout() -> StorageLayout {
    StorageLayout {
        size: 8,
        alignment: 8,
    }
}

fn align_up(offset: u64, alignment: u32) -> u64 {
    let alignment = u64::from(alignment.max(1));
    offset
        .checked_add(alignment - 1)
        .map_or(offset, |value| value / alignment * alignment)
}

fn opaque_type() -> OpaqueType {
    OpaqueType {
        layout: None,
        reference_type: false,
        managed_lifetime: false,
    }
}

fn install_builtins(binder: &mut SemanticBinder) -> BuiltinTypes {
    fn primitive(
        binder: &mut SemanticBinder,
        name: &str,
        kind: PrimitiveKind,
        layout: StorageLayout,
    ) -> TypeRef {
        let name = binder.scopes.intern_name(name);
        let ty = binder.types.allocate_complete(
            TypeOwner::Builtin,
            Some(name),
            binder.scopes.current_environment(),
            PrimitiveType { kind, layout },
        );
        let symbol = binder
            .scopes
            .declare(
                name,
                SymbolKind::Type(ty),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            )
            .expect("builtin names are unique");
        binder
            .types
            .set_declared_in(ty, binder.scopes.symbol(symbol).declared_in);
        ty
    }

    fn alias(binder: &mut SemanticBinder, name: &str, target: TypeRef) -> TypeRef {
        let name = binder.scopes.intern_name(name);
        binder
            .define_type(
                name,
                AliasType {
                    target,
                    nominal: false,
                },
            )
            .expect("builtin names are unique")
            .ty
    }

    let integer_layout = StorageLayout {
        size: 4,
        alignment: 4,
    };
    let integer = primitive(
        binder,
        "integer",
        PrimitiveKind::Integer {
            bits: 32,
            signed: true,
        },
        integer_layout,
    );
    let long_integer = primitive(
        binder,
        "longint",
        PrimitiveKind::Integer {
            bits: 32,
            signed: true,
        },
        integer_layout,
    );
    let cardinal = primitive(
        binder,
        "cardinal",
        PrimitiveKind::Integer {
            bits: 32,
            signed: false,
        },
        integer_layout,
    );
    let mut byte = None;
    let mut word = None;
    let mut int64 = None;
    let mut qword = None;
    for (name, bits, signed) in [
        ("byte", 8, false),
        ("shortint", 8, true),
        ("word", 16, false),
        ("smallint", 16, true),
        ("int64", 64, true),
        ("qword", 64, false),
    ] {
        let bytes = u64::from(bits / 8);
        let ty = primitive(
            binder,
            name,
            PrimitiveKind::Integer { bits, signed },
            StorageLayout {
                size: bytes,
                alignment: u32::from(bits / 8),
            },
        );
        match name {
            "byte" => byte = Some(ty),
            "word" => word = Some(ty),
            "int64" => int64 = Some(ty),
            "qword" => qword = Some(ty),
            _ => {}
        }
    }
    let byte = byte.unwrap();
    let word = word.unwrap();
    let int64 = int64.unwrap();
    let qword = qword.unwrap();
    let _ = alias(binder, "longword", cardinal);
    let _ = alias(binder, "dword", cardinal);
    let _ = alias(binder, "sizeint", int64);
    let size_unsigned = alias(binder, "sizeuint", qword);
    let _ = alias(binder, "ptrint", int64);
    let _ = alias(binder, "ptruint", qword);
    let boolean = primitive(
        binder,
        "boolean",
        PrimitiveKind::Boolean,
        StorageLayout {
            size: 1,
            alignment: 1,
        },
    );
    let character = primitive(
        binder,
        "char",
        PrimitiveKind::Character,
        StorageLayout {
            size: 1,
            alignment: 1,
        },
    );
    let wide_character = primitive(
        binder,
        "widechar",
        PrimitiveKind::WideCharacter { bits: 16 },
        StorageLayout {
            size: 2,
            alignment: 2,
        },
    );
    let real = primitive(
        binder,
        "real",
        PrimitiveKind::Real { bits: 64 },
        StorageLayout {
            size: 8,
            alignment: 8,
        },
    );
    let string_name = binder.scopes.intern_name("string");
    let _string = binder
        .define_type(
            string_name,
            StringType {
                kind: StringKind::Short,
                capacity: Some(255),
                element: character,
                index: integer,
                length: integer,
                layout: StorageLayout {
                    size: 256,
                    alignment: 1,
                },
            },
        )
        .expect("builtin names are unique");
    let short_string_name = binder.scopes.intern_name("shortstring");
    let _ = binder
        .define_type(
            short_string_name,
            AliasType {
                target: _string.ty,
                nominal: false,
            },
        )
        .expect("builtin names are unique");
    let pointer_name = binder.scopes.intern_name("pointer");
    let _pointer = binder
        .define_type(
            pointer_name,
            UntypedPointerType {
                layout: pointer_layout(),
            },
        )
        .expect("builtin names are unique");
    for (name, kind, element) in [
        ("ansistring", StringKind::Ansi, character),
        ("utf8string", StringKind::Utf8, character),
        ("widestring", StringKind::Wide, wide_character),
        ("unicodestring", StringKind::Unicode, wide_character),
    ] {
        let name = binder.scopes.intern_name(name);
        let _ = binder
            .define_type(
                name,
                StringType {
                    kind,
                    capacity: None,
                    element,
                    index: integer,
                    length: integer,
                    layout: pointer_layout(),
                },
            )
            .expect("builtin names are unique");
    }
    let tmethod = binder.scopes.intern_name("tmethod");
    let _ = binder
        .define_type(
            tmethod,
            RawMethodType {
                layout: StorageLayout {
                    size: 16,
                    alignment: 8,
                },
            },
        )
        .expect("builtin names are unique");
    let nil = binder.types.allocate_complete(
        TypeOwner::Builtin,
        None,
        binder.scopes.current_environment(),
        NilType,
    );
    let untyped_parameter = binder.types.allocate_complete(
        TypeOwner::Builtin,
        None,
        binder.scopes.current_environment(),
        opaque_type(),
    );
    let _unit = binder.types.allocate_complete(
        TypeOwner::Builtin,
        None,
        binder.scopes.current_environment(),
        UnitType,
    );
    BuiltinTypes {
        integer,
        long_integer,
        real,
        boolean,
        character,
        byte,
        word,
        size_unsigned,
        nil,
        untyped_parameter,
    }
}

pub fn bind_sources(sources: &[(&str, &str)]) -> SemanticCompilation {
    bind_sources_with_options(sources, &crate::PreprocessorOptions::default())
}

pub fn bind_sources_with_options(
    sources: &[(&str, &str)],
    preprocessor_options: &crate::PreprocessorOptions,
) -> SemanticCompilation {
    let mut driver = CompilationDriver::new();
    let mut inputs = Vec::new();
    for (source_name, source) in sources {
        let lexed = crate::preprocess(source_name, source, preprocessor_options);
        let final_directive_state = lexed
            .directive_state(lexed.final_directive_state)
            .cloned()
            .unwrap_or_default();
        let mut parsed = pascal_parser::parse_tokens(&lexed.tokens, lexed.logical_len);
        parsed.diagnostics.splice(0..0, lexed.diagnostics);
        driver.diagnostics.extend(parsed.diagnostics);
        let Some(file) = parsed.file else {
            continue;
        };
        let declarations = parse_file_declarations(&file);
        driver
            .diagnostics
            .extend(declarations.diagnostics.iter().cloned());
        inputs.push(ParsedInput {
            source_name: (*source_name).to_owned(),
            body_tokens: section_tokens(&file, PascalSectionKind::Body)
                .map(strip_compound_body)
                .unwrap_or_default(),
            final_directive_state,
            file,
            declarations,
            module: None,
        });
    }
    driver.register_modules(&mut inputs);
    driver.configure_uses(&inputs);
    driver.bind_module_interfaces(&inputs);
    let mut files = Vec::new();
    driver.bind_remaining_files(&inputs, &mut files);
    SemanticCompilation {
        binder: driver.binder,
        builtin_families: driver.builtin_families,
        modules: driver.modules,
        files,
        bodies: driver.bodies,
        diagnostics: driver.diagnostics,
    }
}
