use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Application, Callee, CaseLabel, Diagnostic, Expr, ExprKind, Literal, Operator, PascalFile,
    PascalFileKind, PascalSectionKind, Span, Statement, Token, TryContinuation, chumsky_parser,
    declaration_ast::{
        AggregateSyntaxKind, CallingConventionSyntax, DeclarationSyntax, FormalModeSyntax,
        FormalParameterSyntax, RoutineDeclarationSyntax, RoutineSyntaxKind, SpannedName,
        TypeDeclarationSyntax, TypeSyntax, TypeSyntaxKind, ValueDeclarationSyntax,
    },
    declaration_parser::{parse_file_declarations, section_tokens},
    pascal_parser,
};

use super::{
    ActualArgument, AggregateDefinition, AggregateKind, AliasType, ApplicationCandidate,
    ApplicationReceiver, ApplicationResolution, ApplicationResolver, ApplicationSelection,
    ArrayType, BindError, BoundApplicationTarget, BoundBody, BoundCaseArm, BoundCaseLabel,
    BoundExceptionHandler, BoundExpression, BoundExpressionKind, BoundStatement,
    BoundStatementKind, BoundTryContinuation, CallableFlavor, CallableType, CallingConvention,
    Capture, DeclarationMode, DeclarationState, DeclaredRoutine, EnvironmentId,
    EnvironmentRequirement, FieldLayout, FormalParameter, FrameKind, IncompleteReason,
    LookupBarrier, LookupEdge, LookupRequest, LookupResult, ModuleGraphError, ModuleId,
    ModulePhase, ModuleRegistry, NameId, NodeId, OpaqueType, ParameterMode, PointerType,
    PrimitiveKind, PrimitiveType, ReceiverId, RegionOwner, RoutineOwner, RoutineSignature,
    SemanticBinder, StorageLayout, SymbolCategory, SymbolFilter, SymbolId, SymbolKind, TypeOwner,
    TypeRef,
};

#[derive(Clone, Debug)]
pub struct BoundFile {
    pub source_name: String,
    pub pascal_name: Option<String>,
    pub kind: PascalFileKind,
    pub environment: EnvironmentId,
    pub declaration_count: usize,
    pub unsupported_declarations: usize,
}

#[derive(Debug)]
pub struct SemanticCompilation {
    pub binder: SemanticBinder,
    pub modules: ModuleRegistry,
    pub files: Vec<BoundFile>,
    pub bodies: Vec<BoundBody>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Copy, Debug)]
struct BuiltinTypes {
    integer: TypeRef,
    boolean: TypeRef,
    string: TypeRef,
    untyped_parameter: TypeRef,
}

struct ParsedInput {
    source_name: String,
    file: PascalFile,
    declarations: crate::declaration_ast::DeclarationParseOutput,
    body_tokens: Vec<Token>,
    module: Option<ModuleId>,
}

#[derive(Clone, Copy)]
struct ResolvedParameter {
    name: NameId,
    ty: TypeRef,
}

struct CompilationDriver {
    binder: SemanticBinder,
    modules: ModuleRegistry,
    module_names: BTreeMap<String, ModuleId>,
    diagnostics: Vec<Diagnostic>,
    bodies: Vec<BoundBody>,
    builtins: BuiltinTypes,
    system_module: ModuleId,
    system_exports: EnvironmentId,
    routine_forwards: BTreeMap<(super::RegionId, NameId), DeclaredRoutine>,
    next_anonymous_type: usize,
    next_receiver: usize,
    next_block: u32,
    loop_depth: u32,
}

impl CompilationDriver {
    fn new() -> Self {
        let mut binder = SemanticBinder::new();
        let builtins = install_builtins(&mut binder);
        install_system_callables(&mut binder, builtins);
        let root_region = binder
            .scopes
            .environment_region(binder.scopes.current_environment());
        let system_exports = binder.scopes.create_region_view(root_region, Vec::new());
        let system_name = binder.scopes.intern_name("System");
        let mut modules = ModuleRegistry::new();
        let system_module = modules.add_module(system_name, system_exports);
        let mut module_names = BTreeMap::new();
        module_names.insert("system".to_owned(), system_module);
        Self {
            binder,
            modules,
            module_names,
            diagnostics: Vec::new(),
            bodies: Vec::new(),
            builtins,
            system_module,
            system_exports,
            routine_forwards: BTreeMap::new(),
            next_anonymous_type: 0,
            next_receiver: 0,
            next_block: 0,
            loop_depth: 0,
        }
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
                DeclarationSyntax::Property(value) => self.bind_properties(value),
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
                    self.diagnostics.push(Diagnostic::new(
                        span.clone(),
                        "unsupported declaration retained by the CST-to-semantic boundary",
                    ));
                }
            }
        }
    }

    fn bind_type_declaration(&mut self, declaration: &TypeDeclarationSyntax) {
        let name = self.binder.scopes.intern_name(&declaration.name.spelling);
        match &declaration.ty.kind {
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
            TypeSyntaxKind::Aggregate {
                kind,
                base,
                members,
            } => self.bind_aggregate(name, *kind, base.as_deref(), members, declaration),
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
                };
                if let Err(error) = self.binder.define_type(name, implementation) {
                    self.bind_error(declaration.span.clone(), error);
                }
            }
            TypeSyntaxKind::Array { element, dynamic } => {
                let element = element
                    .as_deref()
                    .and_then(|element| self.resolve_type(element))
                    .unwrap_or(self.builtins.integer);
                let implementation = ArrayType {
                    element,
                    index: self.builtins.integer,
                    length: self.builtins.integer,
                    layout: (!dynamic).then_some(StorageLayout {
                        size: 0,
                        alignment: 1,
                    }),
                    resizable: *dynamic,
                    open: false,
                };
                if let Err(error) = self.binder.define_type(name, implementation) {
                    self.bind_error(declaration.span.clone(), error);
                }
            }
            TypeSyntaxKind::Set { .. } | TypeSyntaxKind::Unsupported(_) => {
                if matches!(declaration.ty.kind, TypeSyntaxKind::Unsupported(_)) {
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

    fn bind_aggregate(
        &mut self,
        name: NameId,
        syntax_kind: AggregateSyntaxKind,
        base_syntax: Option<&TypeSyntax>,
        members: &[DeclarationSyntax],
        declaration: &TypeDeclarationSyntax,
    ) {
        let base = base_syntax.and_then(|base| self.resolve_type(base));
        let kind = match syntax_kind {
            AggregateSyntaxKind::Record => AggregateKind::RegularRecord,
            AggregateSyntaxKind::PackedRecord => AggregateKind::PackedRecord,
            AggregateSyntaxKind::Object => AggregateKind::Object { base },
            AggregateSyntaxKind::Class | AggregateSyntaxKind::Interface => {
                AggregateKind::Class { base }
            }
        };
        let layout = if matches!(
            syntax_kind,
            AggregateSyntaxKind::Class | AggregateSyntaxKind::Interface
        ) {
            pointer_layout()
        } else {
            StorageLayout {
                size: 0,
                alignment: usize_alignment(),
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
        if let Err(error) = self.binder.end_aggregate(aggregate, None) {
            self.bind_error(declaration.span.clone(), error);
        }
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
        let field_layout = FieldLayout {
            byte_offset: aggregate.fields.len() as u64
                * self
                    .binder
                    .types
                    .storage_layout(ty)
                    .map_or(1, |layout| layout.size),
            bit_offset: 0,
            bit_width: None,
        };
        for name in &declaration.names {
            let name_id = self.binder.scopes.intern_name(&name.spelling);
            if let Err(error) = self
                .binder
                .declare_field(aggregate, name_id, ty, field_layout)
            {
                self.bind_error(name.span.clone(), error);
            }
        }
    }

    fn bind_values(&mut self, declaration: &ValueDeclarationSyntax, constant: bool) {
        let ty = declaration
            .ty
            .as_ref()
            .and_then(|syntax| self.resolve_type(syntax))
            .unwrap_or(self.builtins.integer);
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
            if let Err(error) = self.binder.scopes.declare(
                name_id,
                kind,
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            ) {
                self.bind_error(name.span.clone(), error.into());
            }
        }
    }

    fn bind_properties(&mut self, declaration: &ValueDeclarationSyntax) {
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
        for name in &declaration.names {
            let name_id = self.binder.scopes.intern_name(&name.spelling);
            if let Err(error) = self.binder.scopes.declare(
                name_id,
                SymbolKind::Property(ty),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            ) {
                self.bind_error(name.span.clone(), error.into());
            }
        }
    }

    fn bind_method(
        &mut self,
        routine: &RoutineDeclarationSyntax,
        aggregate: &mut AggregateDefinition,
    ) {
        let name = self.binder.scopes.intern_name(&routine.name.spelling);
        let (signature, parameters) = self.resolve_routine_signature(routine);
        let result = signature.result;
        let method = match self.binder.declare_method(
            aggregate,
            name,
            signature,
            routine_declaration_mode(routine),
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
        let name = self.binder.scopes.intern_name(&routine.name.spelling);
        let (signature, parameters) = self.resolve_routine_signature(routine);
        let result = signature.result;
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
        if routine.is_forward {
            self.routine_forwards.insert(key, declared);
        }
        if routine.has_body {
            self.bind_routine_body(routine, declared, &parameters, result);
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
        for parameter in parameters {
            if let Err(error) = self.binder.scopes.declare(
                parameter.name,
                SymbolKind::Variable(parameter.ty),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            ) {
                self.bind_error(routine.span.clone(), error.into());
            }
        }
        if let Some(result) = result {
            for spelling in ["result", routine.name.spelling.as_str()] {
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
        self.bind_body_tokens(Some(declared.ty), &routine.body_tokens);
        self.binder.end_routine_body(checkpoint);
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
            for name in &parameter.names {
                let name = self.binder.scopes.intern_name(&name.spelling);
                parameters.push(ResolvedParameter { name, ty });
                formals.push(FormalParameter {
                    mode,
                    ty,
                    has_default: parameter.has_default,
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
            for _ in &parameter.names {
                formals.push(FormalParameter {
                    mode,
                    ty,
                    has_default: parameter.has_default,
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
                kind: BoundStatementKind::Expression(self.bind_expression(expression, owner)),
            },
            Statement::Assignment(application) => BoundStatement {
                span: application.span.clone(),
                kind: BoundStatementKind::Assignment(self.bind_application(application, owner)),
            },
            Statement::Compound { statements, span } => BoundStatement {
                span: span.clone(),
                kind: BoundStatementKind::Compound(self.bind_scoped_statements(statements, owner)),
            },
            Statement::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let condition = self.bind_condition(condition, owner);
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
                span,
            } => BoundStatement {
                span: span.clone(),
                kind: BoundStatementKind::While {
                    condition: self.bind_condition(condition, owner),
                    body: Box::new(self.bind_loop_body(body, owner)),
                },
            },
            Statement::Repeat {
                body,
                condition,
                span,
            } => BoundStatement {
                span: span.clone(),
                kind: BoundStatementKind::Repeat {
                    body: self.bind_loop_statements(body, owner),
                    condition: self.bind_condition(condition, owner),
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
                let initial = self.bind_expression(initial, owner);
                let final_value = self.bind_expression(final_value, owner);
                if let Some(symbol) = control {
                    let control_type = self.binder.scopes.symbol(symbol).kind.ty();
                    for value in [&initial, &final_value] {
                        if control_type
                            .zip(value.ty)
                            .is_none_or(|(destination, source)| {
                                self.binder
                                    .types
                                    .value_conversion(destination, source)
                                    .is_none()
                            })
                        {
                            self.diagnostics.push(Diagnostic::new(
                                value.span.clone(),
                                "for-loop bound is not convertible to the control type",
                            ));
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
                if element.is_none() {
                    self.diagnostics.push(Diagnostic::new(
                        source.span.clone(),
                        "for-in source is not a sequence",
                    ));
                } else if let Some(symbol) = control {
                    let control_type = self.binder.scopes.symbol(symbol).kind.ty();
                    if control_type
                        .zip(element)
                        .is_none_or(|(destination, source)| {
                            self.binder
                                .types
                                .value_conversion(destination, source)
                                .is_none()
                        })
                    {
                        self.diagnostics.push(Diagnostic::new(
                            span.clone(),
                            "for-in element is not convertible to the control type",
                        ));
                    }
                }
                BoundStatement {
                    span: span.clone(),
                    kind: BoundStatementKind::ForIn {
                        control,
                        source,
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
                let arms = arms
                    .iter()
                    .map(|arm| BoundCaseArm {
                        labels: arm
                            .labels
                            .iter()
                            .map(|label| match label {
                                CaseLabel::Value(value) => {
                                    BoundCaseLabel::Value(self.bind_expression(value, owner))
                                }
                                CaseLabel::Range { low, high } => BoundCaseLabel::Range {
                                    low: self.bind_expression(low, owner),
                                    high: self.bind_expression(high, owner),
                                },
                            })
                            .collect(),
                        statement: self.bind_scoped_statement(&arm.statement, owner),
                        span: arm.span.clone(),
                    })
                    .collect();
                let otherwise = otherwise
                    .iter()
                    .map(|statement| self.bind_statement(statement, owner))
                    .collect();
                BoundStatement {
                    span: span.clone(),
                    kind: BoundStatementKind::Case {
                        selector,
                        arms,
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
            Statement::Exit { value, span } => {
                let value = value
                    .as_ref()
                    .map(|value| self.bind_expression(value, owner));
                let result = owner
                    .and_then(|owner| self.binder.types.callable(owner))
                    .and_then(|callable| callable.signature.result);
                match (result, value.as_ref().and_then(|value| value.ty)) {
                    (Some(destination), Some(source))
                        if self
                            .binder
                            .types
                            .value_conversion(destination, source)
                            .is_none() =>
                    {
                        self.diagnostics.push(Diagnostic::new(
                            span.clone(),
                            "exit value is not convertible to the function result",
                        ));
                    }
                    (None, Some(_)) => self.diagnostics.push(Diagnostic::new(
                        span.clone(),
                        "procedure exit cannot carry a result value",
                    )),
                    (Some(_), None) if value.is_some() => self.diagnostics.push(Diagnostic::new(
                        span.clone(),
                        "exit result has no semantic type",
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
                let initializer = initializer
                    .as_ref()
                    .map(|initializer| self.bind_expression(initializer, owner));
                let explicit_type = type_name
                    .as_ref()
                    .and_then(|path| self.resolve_type_path_strings(path, span.clone()));
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
                if let (Some(destination), Some(source)) = (
                    explicit_type,
                    initializer.as_ref().and_then(|initializer| initializer.ty),
                ) && self
                    .binder
                    .types
                    .value_conversion(destination, source)
                    .is_none()
                {
                    self.diagnostics.push(Diagnostic::new(
                        span.clone(),
                        "inline initializer is not convertible to its declared type",
                    ));
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

    fn bind_condition(&mut self, expression: &Expr, owner: Option<TypeRef>) -> BoundExpression {
        let condition = self.bind_expression(expression, owner);
        if condition.ty.is_none_or(|source| {
            self.binder
                .types
                .value_conversion(self.builtins.boolean, source)
                .is_none()
        }) {
            self.diagnostics.push(Diagnostic::new(
                expression.span.clone(),
                "condition is not Boolean-compatible",
            ));
        }
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
        match &expression.kind {
            ExprKind::Identifier(name) => {
                self.bind_identifier(name, expression.span.clone(), owner, false)
            }
            ExprKind::Inherited(name) => {
                let Some(name) = name else {
                    self.diagnostics.push(Diagnostic::new(
                        expression.span.clone(),
                        "bare `inherited` requires the current method binding",
                    ));
                    return bound_error(expression.span.clone());
                };
                self.bind_identifier(name, expression.span.clone(), owner, false)
            }
            ExprKind::Literal(literal) => BoundExpression {
                ty: match literal {
                    Literal::Integer(_) => Some(self.builtins.integer),
                    Literal::Boolean(_) => Some(self.builtins.boolean),
                    Literal::String(_) => Some(self.builtins.string),
                    Literal::Real(_) | Literal::Nil => None,
                },
                kind: BoundExpressionKind::Literal(literal.clone()),
                span: expression.span.clone(),
            },
            ExprKind::Application(application) => self.bind_application(application, owner),
            ExprKind::Member { base, member } => {
                let base = self.bind_expression(base, owner);
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
                let ty = self.binder.scopes.symbol(symbol).kind.ty();
                BoundExpression {
                    kind: BoundExpressionKind::Member {
                        base: Box::new(base),
                        symbol,
                    },
                    ty,
                    span: expression.span.clone(),
                }
            }
            ExprKind::Index { base, indices, .. } => {
                let base = self.bind_expression(base, owner);
                let indices = indices
                    .iter()
                    .map(|index| self.bind_expression(index, owner))
                    .collect();
                let ty = base
                    .ty
                    .and_then(|ty| self.binder.types.sequence_element_type(ty));
                if ty.is_none() {
                    self.diagnostics.push(Diagnostic::new(
                        expression.span.clone(),
                        "indexing requires a sequence type",
                    ));
                }
                BoundExpression {
                    kind: BoundExpressionKind::Index {
                        base: Box::new(base),
                        indices,
                    },
                    ty,
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
                    span: expression.span.clone(),
                }
            }
            ExprKind::Set(elements) => {
                let mut bound = Vec::new();
                for element in elements {
                    match element {
                        crate::SetElement::Value(value) => {
                            bound.push(self.bind_expression(value, owner));
                        }
                        crate::SetElement::Range { low, high } => {
                            bound.push(self.bind_expression(low, owner));
                            bound.push(self.bind_expression(high, owner));
                        }
                    }
                }
                BoundExpression {
                    kind: BoundExpressionKind::Set(bound),
                    ty: None,
                    span: expression.span.clone(),
                }
            }
            ExprKind::Error => bound_error(expression.span.clone()),
        }
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
            kind: BoundExpressionKind::Symbol { symbol, receiver },
            span,
        }
    }

    fn bind_application(
        &mut self,
        application: &Application,
        owner: Option<TypeRef>,
    ) -> BoundExpression {
        let operands = application
            .operands
            .iter()
            .map(|operand| self.bind_expression(operand, owner))
            .collect::<Vec<_>>();
        match &application.callee {
            Callee::Expression(callee) => {
                if let ExprKind::Identifier(name) = &callee.kind {
                    return self.bind_named_application(
                        name,
                        application.span.clone(),
                        operands,
                        owner,
                    );
                }
                let callee = self.bind_expression(callee, owner);
                let candidate = self.application_candidate_from_callee(&callee);
                let Some(candidate) = candidate else {
                    self.diagnostics.push(Diagnostic::new(
                        application.span.clone(),
                        "application operand is not callable",
                    ));
                    return BoundExpression {
                        kind: BoundExpressionKind::Application {
                            target: BoundApplicationTarget::Invalid,
                            callee: Some(Box::new(callee)),
                            operands,
                        },
                        ty: None,
                        span: application.span.clone(),
                    };
                };
                let target_kind = match candidate {
                    ApplicationCandidate::Routine { .. } => 0,
                    ApplicationCandidate::CallableValue { .. } => 1,
                    ApplicationCandidate::Conversion { .. } => unreachable!(),
                };
                let resolution = self.resolve_application(vec![candidate], &operands);
                self.report_application_resolution(
                    &resolution,
                    application.span.clone(),
                    "callable expression",
                );
                let result = resolution.result_type();
                let target = if target_kind == 0 {
                    BoundApplicationTarget::Routine { resolution }
                } else {
                    BoundApplicationTarget::CallableValue { resolution }
                };
                BoundExpression {
                    kind: BoundExpressionKind::Application {
                        target,
                        callee: Some(Box::new(callee)),
                        operands,
                    },
                    ty: result,
                    span: application.span.clone(),
                }
            }
            Callee::Operator(operator) => {
                self.bind_operator_application(*operator, application.span.clone(), operands)
            }
        }
    }

    fn bind_named_application(
        &mut self,
        spelling: &str,
        span: Span,
        operands: Vec<BoundExpression>,
        owner: Option<TypeRef>,
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
            return bound_application_error(span, operands);
        };
        let primary = result.primary[0].symbol;
        let primary_receiver = result.primary[0].receiver;
        let primary_kind = self.binder.scopes.symbol(primary).kind.clone();
        match primary_kind {
            SymbolKind::Type(destination) => {
                let resolution = self.resolve_application(
                    vec![ApplicationCandidate::Conversion { destination }],
                    &operands,
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
                    },
                    ty: result_type,
                    span,
                }
            }
            SymbolKind::Routine(_) => {
                let candidates = callable_candidates(&result, &self.binder, owner);
                let resolution = self.resolve_application(candidates, &operands);
                self.report_application_resolution(
                    &resolution,
                    span.clone(),
                    &format!("overload for `{spelling}`"),
                );
                let result_type = resolution.result_type();
                BoundExpression {
                    kind: BoundExpressionKind::Application {
                        target: BoundApplicationTarget::Routine { resolution },
                        callee: None,
                        operands,
                    },
                    ty: result_type,
                    span,
                }
            }
            SymbolKind::Variable(callable_type) | SymbolKind::Property(callable_type)
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
                    },
                    ty: result_type,
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
                bound_application_error(span, operands)
            }
        }
    }

    fn bind_operator_application(
        &mut self,
        operator: Operator,
        span: Span,
        operands: Vec<BoundExpression>,
    ) -> BoundExpression {
        let name = self.binder.scopes.intern_name(operator.spelling());
        let candidates = self
            .binder
            .scopes
            .lookup_symbol(
                self.binder.scopes.current_environment(),
                name,
                LookupRequest::ORDINARY,
            )
            .map_or_else(Vec::new, |result| {
                callable_candidates(&result, &self.binder, None)
            });
        let resolution = self.resolve_application(candidates, &operands);
        self.report_application_resolution(
            &resolution,
            span.clone(),
            &format!("`{}` operator", operator.spelling()),
        );
        let mut result_type = resolution.result_type();
        if operator == Operator::Assign {
            result_type = operands.first().and_then(|operand| operand.ty);
        }
        BoundExpression {
            kind: BoundExpressionKind::Application {
                target: BoundApplicationTarget::Operator {
                    operator,
                    resolution,
                },
                callee: None,
                operands,
            },
            ty: result_type,
            span,
        }
    }

    fn application_candidate_from_callee(
        &self,
        callee: &BoundExpression,
    ) -> Option<ApplicationCandidate> {
        let (symbol, lookup_receiver, explicit_receiver) = match &callee.kind {
            BoundExpressionKind::Symbol { symbol, receiver } => (Some(*symbol), *receiver, false),
            BoundExpressionKind::Member { symbol, .. } => (Some(*symbol), None, true),
            _ => (None, None, false),
        };
        let callable_type = callee.ty?;
        let callable = self.binder.types.callable(callable_type)?;
        if let Some(symbol) = symbol
            && matches!(
                self.binder.scopes.symbol(symbol).kind,
                SymbolKind::Routine(_)
            )
        {
            let receiver = if explicit_receiver {
                ApplicationReceiver::Explicit
            } else {
                declared_receiver(callable.flavor, lookup_receiver, false)
            };
            return Some(ApplicationCandidate::Routine {
                symbol,
                callable_type,
                receiver,
            });
        }
        Some(ApplicationCandidate::CallableValue {
            symbol,
            callable_type,
            receiver: ApplicationReceiver::CallableValue { lookup_receiver },
        })
    }

    fn resolve_application(
        &self,
        candidates: Vec<ApplicationCandidate>,
        operands: &[BoundExpression],
    ) -> ApplicationResolution {
        let actuals = operands
            .iter()
            .map(|operand| ActualArgument {
                ty: operand.ty,
                addressable: is_addressable(operand),
            })
            .collect::<Vec<_>>();
        ApplicationResolver::new(&self.binder.types, self.builtins.untyped_parameter)
            .resolve(candidates, &actuals)
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
            TypeSyntaxKind::Pointer(target) => {
                let target = self.resolve_type(target)?;
                Some(self.allocate_anonymous(PointerType {
                    target,
                    layout: pointer_layout(),
                }))
            }
            TypeSyntaxKind::Array { element, dynamic } => {
                let element = element
                    .as_deref()
                    .and_then(|syntax| self.resolve_type(syntax))?;
                Some(self.allocate_anonymous(ArrayType {
                    element,
                    index: self.builtins.integer,
                    length: self.builtins.integer,
                    layout: (!dynamic).then_some(StorageLayout {
                        size: 0,
                        alignment: 1,
                    }),
                    resizable: *dynamic,
                    open: false,
                }))
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
                }))
            }
            TypeSyntaxKind::Set { .. } | TypeSyntaxKind::Unsupported(_) => {
                Some(self.allocate_anonymous(opaque_type()))
            }
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

fn callable_candidates(
    result: &LookupResult,
    binder: &SemanticBinder,
    current_callable: Option<TypeRef>,
) -> Vec<ApplicationCandidate> {
    let mut seen = BTreeSet::new();
    result
        .primary
        .iter()
        .chain(result.shadowed.iter().flatten())
        .filter_map(|hit| {
            if !seen.insert(hit.symbol) {
                return None;
            }
            let SymbolKind::Routine(callable_type) = binder.scopes.symbol(hit.symbol).kind else {
                return None;
            };
            let flavor = binder.types.callable(callable_type)?.flavor;
            let implicit_self = current_callable
                .and_then(|current| binder.types.callable(current))
                .is_some_and(|current| matches!(current.owner, RoutineOwner::Type(_)));
            Some(ApplicationCandidate::Routine {
                symbol: hit.symbol,
                callable_type,
                receiver: declared_receiver(flavor, hit.receiver, implicit_self),
            })
        })
        .collect()
}

fn declared_receiver(
    flavor: CallableFlavor,
    lookup_receiver: Option<ReceiverId>,
    implicit_self: bool,
) -> ApplicationReceiver {
    match (flavor, lookup_receiver) {
        (_, Some(receiver)) => ApplicationReceiver::Lookup(receiver),
        (CallableFlavor::Nested, None) => ApplicationReceiver::StaticLink,
        (CallableFlavor::Method, None) if implicit_self => ApplicationReceiver::ImplicitSelf,
        (CallableFlavor::Routine | CallableFlavor::Method, None) => ApplicationReceiver::None,
    }
}

fn routine_declaration_mode(routine: &RoutineDeclarationSyntax) -> DeclarationMode {
    if routine.overload || routine.kind == RoutineSyntaxKind::Operator {
        DeclarationMode::Overload
    } else {
        DeclarationMode::Fresh
    }
}

fn semantic_calling_convention(syntax: CallingConventionSyntax) -> CallingConvention {
    match syntax {
        CallingConventionSyntax::Pascal => CallingConvention::Pascal,
        CallingConventionSyntax::Register => CallingConvention::Register,
        CallingConventionSyntax::Cdecl => CallingConvention::Cdecl,
        CallingConventionSyntax::Stdcall => CallingConvention::Stdcall,
    }
}

fn bound_error(span: Span) -> BoundExpression {
    BoundExpression {
        kind: BoundExpressionKind::Error,
        ty: None,
        span,
    }
}

fn bound_application_error(span: Span, operands: Vec<BoundExpression>) -> BoundExpression {
    BoundExpression {
        kind: BoundExpressionKind::Application {
            target: BoundApplicationTarget::Invalid,
            callee: None,
            operands,
        },
        ty: None,
        span,
    }
}

fn is_addressable(expression: &BoundExpression) -> bool {
    matches!(
        expression.kind,
        BoundExpressionKind::Symbol { .. }
            | BoundExpressionKind::Member { .. }
            | BoundExpressionKind::Index { .. }
            | BoundExpressionKind::Dereference(_)
    )
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

fn usize_alignment() -> u32 {
    8
}

fn opaque_type() -> OpaqueType {
    OpaqueType {
        layout: None,
        reference_type: false,
        managed_lifetime: false,
    }
}

fn install_system_callables(binder: &mut SemanticBinder, builtins: BuiltinTypes) {
    fn callable(
        binder: &mut SemanticBinder,
        spelling: &str,
        parameters: &[TypeRef],
        result: Option<TypeRef>,
    ) {
        let name = binder.scopes.intern_name(spelling);
        let ty = binder.types.allocate_complete(
            TypeOwner::Builtin,
            Some(name),
            binder.scopes.current_environment(),
            CallableType {
                owner: RoutineOwner::Module,
                flavor: CallableFlavor::Routine,
                signature: RoutineSignature {
                    parameters: parameters
                        .iter()
                        .map(|ty| FormalParameter {
                            mode: ParameterMode::Value,
                            ty: *ty,
                            has_default: false,
                        })
                        .collect(),
                    result,
                    calling_convention: CallingConvention::Pascal,
                },
                declaration_region: None,
                nested_routines: Vec::new(),
                local_types: Vec::new(),
                captures: Vec::new(),
                environment: EnvironmentRequirement::None,
                has_body: false,
            },
        );
        let symbol = binder
            .scopes
            .declare(
                name,
                SymbolKind::Routine(ty),
                DeclarationState::Complete,
                DeclarationMode::Overload,
            )
            .expect("System callable declarations are overload-compatible");
        binder
            .types
            .set_declared_in(ty, binder.scopes.symbol(symbol).declared_in);
    }

    for spelling in ["+", "-", "*", "/", "div", "mod", "shl", "shr"] {
        callable(
            binder,
            spelling,
            &[builtins.integer, builtins.integer],
            Some(builtins.integer),
        );
    }
    for spelling in ["+", "-"] {
        callable(
            binder,
            spelling,
            &[builtins.integer],
            Some(builtins.integer),
        );
    }
    for spelling in ["=", "<>", "<", ">", "<=", ">="] {
        callable(
            binder,
            spelling,
            &[builtins.integer, builtins.integer],
            Some(builtins.boolean),
        );
    }
    for spelling in ["and", "or", "xor"] {
        callable(
            binder,
            spelling,
            &[builtins.integer, builtins.integer],
            Some(builtins.integer),
        );
        callable(
            binder,
            spelling,
            &[builtins.boolean, builtins.boolean],
            Some(builtins.boolean),
        );
    }
    callable(binder, "not", &[builtins.boolean], Some(builtins.boolean));
    callable(binder, ":=", &[builtins.integer, builtins.integer], None);
    callable(binder, "high", &[builtins.integer], Some(builtins.integer));
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
    for name in ["longint", "cardinal"] {
        let _ = primitive(
            binder,
            name,
            PrimitiveKind::Integer {
                bits: 32,
                signed: name == "longint",
            },
            integer_layout,
        );
    }
    for (name, bits, signed) in [
        ("byte", 8, false),
        ("shortint", 8, true),
        ("word", 16, false),
        ("smallint", 16, true),
        ("int64", 64, true),
        ("qword", 64, false),
    ] {
        let bytes = u64::from(bits / 8);
        let _ = primitive(
            binder,
            name,
            PrimitiveKind::Integer { bits, signed },
            StorageLayout {
                size: bytes,
                alignment: u32::from(bits / 8),
            },
        );
    }
    let boolean = primitive(
        binder,
        "boolean",
        PrimitiveKind::Boolean,
        StorageLayout {
            size: 1,
            alignment: 1,
        },
    );
    let _ = primitive(
        binder,
        "char",
        PrimitiveKind::Character,
        StorageLayout {
            size: 1,
            alignment: 1,
        },
    );
    let string_name = binder.scopes.intern_name("string");
    let string = binder
        .define_type(string_name, opaque_type())
        .expect("builtin names are unique")
        .ty;
    for name in ["pointer", "ansistring", "widestring", "unicodeString"] {
        let name = binder.scopes.intern_name(name);
        let _ = binder
            .define_type(name, opaque_type())
            .expect("builtin names are unique");
    }
    let untyped_parameter = binder.types.allocate_complete(
        TypeOwner::Builtin,
        None,
        binder.scopes.current_environment(),
        opaque_type(),
    );
    BuiltinTypes {
        integer,
        boolean,
        string,
        untyped_parameter,
    }
}

pub fn bind_sources(sources: &[(&str, &str)]) -> SemanticCompilation {
    let mut driver = CompilationDriver::new();
    let mut inputs = Vec::new();
    for (source_name, source) in sources {
        let parsed = pascal_parser::parse(source);
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
        modules: driver.modules,
        files,
        bodies: driver.bodies,
        diagnostics: driver.diagnostics,
    }
}
