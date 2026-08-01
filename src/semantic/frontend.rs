use std::collections::BTreeMap;

use crate::{
    Diagnostic, PascalFile, PascalFileKind, PascalSectionKind, Span,
    declaration_ast::{
        AggregateSyntaxKind, DeclarationSyntax, RoutineDeclarationSyntax, SpannedName,
        TypeDeclarationSyntax, TypeSyntax, TypeSyntaxKind, ValueDeclarationSyntax,
    },
    declaration_parser::parse_file_declarations,
    pascal_parser,
};

use super::{
    AggregateDefinition, AggregateKind, AliasType, ArrayType, BindError, CallableFlavor,
    CallableType, CallingConvention, DeclarationMode, DeclarationState, DeclaredRoutine,
    EnvironmentId, EnvironmentRequirement, FieldLayout, FrameKind, IncompleteReason, LookupEdge,
    LookupRequest, NameId, NodeId, OpaqueType, PointerType, PrimitiveKind, PrimitiveType,
    RegionOwner, RoutineOwner, RoutineSignature, SemanticBinder, StorageLayout, SymbolKind,
    TypeOwner, TypeRef, UnitGraphError, UnitId, UnitPhase, UnitRegistry,
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
    pub units: UnitRegistry,
    pub files: Vec<BoundFile>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Copy, Debug)]
struct BuiltinTypes {
    integer: TypeRef,
}

struct ParsedInput {
    source_name: String,
    file: PascalFile,
    declarations: crate::declaration_ast::DeclarationParseOutput,
    unit: Option<UnitId>,
}

struct CompilationDriver {
    binder: SemanticBinder,
    units: UnitRegistry,
    unit_names: BTreeMap<String, UnitId>,
    diagnostics: Vec<Diagnostic>,
    builtins: BuiltinTypes,
    system_unit: UnitId,
    system_exports: EnvironmentId,
    routine_forwards: BTreeMap<(super::RegionId, NameId), DeclaredRoutine>,
    next_anonymous_type: usize,
}

impl CompilationDriver {
    fn new() -> Self {
        let mut binder = SemanticBinder::new();
        let builtins = install_builtins(&mut binder);
        let root_region = binder
            .scopes
            .environment_region(binder.scopes.current_environment());
        let system_exports = binder.scopes.create_region_view(root_region, Vec::new());
        let system_name = binder.scopes.intern_name("System");
        let mut units = UnitRegistry::new();
        let system_unit = units.add_unit(system_name, system_exports);
        let mut unit_names = BTreeMap::new();
        unit_names.insert("system".to_owned(), system_unit);
        Self {
            binder,
            units,
            unit_names,
            diagnostics: Vec::new(),
            builtins,
            system_unit,
            system_exports,
            routine_forwards: BTreeMap::new(),
            next_anonymous_type: 0,
        }
    }

    fn register_units(&mut self, inputs: &mut [ParsedInput]) {
        for input in inputs {
            if input.file.kind != PascalFileKind::Unit {
                continue;
            }
            let Some(name) = input.file.name.as_deref() else {
                continue;
            };
            if self.unit_names.contains_key(name) {
                self.diagnostics.push(Diagnostic::new(
                    input.file.span.clone(),
                    format!("duplicate unit `{name}`"),
                ));
                continue;
            }
            let predicted = UnitId::from_index(self.units_len());
            let (_, exports) = self
                .binder
                .scopes
                .create_detached_region(RegionOwner::Unit(predicted), Vec::new());
            let name_id = self.binder.scopes.intern_name(name);
            let unit = self.units.add_unit(name_id, exports);
            debug_assert_eq!(unit, predicted);
            self.unit_names.insert(name.to_owned(), unit);
            input.unit = Some(unit);
        }
    }

    fn units_len(&self) -> usize {
        self.unit_names.len()
    }

    fn configure_uses(&mut self, inputs: &[ParsedInput]) {
        for input in inputs {
            let Some(unit) = input.unit else {
                continue;
            };
            for section in &input.declarations.sections {
                let phase = match section.kind {
                    PascalSectionKind::Interface => UnitPhase::Interface,
                    PascalSectionKind::Implementation => UnitPhase::Implementation,
                    _ => continue,
                };
                let uses = self.resolve_uses(&section.declarations);
                self.units.set_uses(unit, phase, uses);
            }
        }
    }

    fn resolve_uses(&mut self, declarations: &[DeclarationSyntax]) -> Vec<UnitId> {
        let mut result = Vec::new();
        for declaration in declarations {
            let DeclarationSyntax::Uses { units, .. } = declaration else {
                continue;
            };
            for unit in units {
                if let Some(resolved) = self.unit_names.get(&unit.spelling) {
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

    fn bind_unit_interfaces(&mut self, inputs: &[ParsedInput]) {
        let order = match self.units.interface_order() {
            Ok(order) => order,
            Err(UnitGraphError::InterfaceCycle { cycle }) => {
                self.diagnostics.push(Diagnostic::new(
                    0..0,
                    format!("interface uses cycle: {cycle:?}"),
                ));
                return;
            }
        };
        for unit in order {
            if unit == self.system_unit {
                continue;
            }
            let Some(input) = inputs.iter().find(|input| input.unit == Some(unit)) else {
                continue;
            };
            let local = self.units.unit(unit).interface_exports;
            let lookup = self.units.interface_lookup_environment(
                &mut self.binder.scopes,
                unit,
                local,
                Some((self.system_unit, self.system_exports)),
            );
            self.binder.scopes.select_environment(lookup);
            if let Some(section) = input
                .declarations
                .sections
                .iter()
                .find(|section| section.kind == PascalSectionKind::Interface)
            {
                self.bind_declarations(&section.declarations, RoutineOwner::Unit, None);
            }
            let region = self
                .binder
                .scopes
                .environment_region(self.binder.scopes.current_environment());
            let exports = self.binder.scopes.create_region_view(region, Vec::new());
            self.units.set_interface_exports(unit, exports);
        }
    }

    fn bind_remaining_files(&mut self, inputs: &[ParsedInput], files: &mut Vec<BoundFile>) {
        for input in inputs {
            let environment = if let Some(unit) = input.unit {
                let (_, local) = self
                    .binder
                    .scopes
                    .create_detached_region(RegionOwner::Unit(unit), Vec::new());
                let lookup = self.units.implementation_lookup_environment(
                    &mut self.binder.scopes,
                    unit,
                    local,
                    Some((self.system_unit, self.system_exports)),
                );
                self.binder.scopes.select_environment(lookup);
                if let Some(section) = input
                    .declarations
                    .sections
                    .iter()
                    .find(|section| section.kind == PascalSectionKind::Implementation)
                {
                    self.bind_declarations(&section.declarations, RoutineOwner::Unit, None);
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
        layers.extend(
            uses.iter()
                .rev()
                .map(|unit| LookupEdge::import(self.units.unit(*unit).interface_exports, *unit)),
        );
        layers.push(LookupEdge::system(self.system_exports, self.system_unit));
        let lookup = self.binder.scopes.create_lookup_environment(region, layers);
        self.binder.scopes.select_environment(lookup);
        if let Some(section) = declarations {
            self.bind_declarations(&section.declarations, RoutineOwner::Unit, None);
        }
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
            TypeSyntaxKind::Procedural { method_pointer } => {
                let flavor = if *method_pointer {
                    CallableFlavor::Method
                } else {
                    CallableFlavor::Routine
                };
                let implementation = CallableType {
                    owner: RoutineOwner::Unit,
                    flavor,
                    signature: empty_signature(),
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
        let method = match self.binder.declare_method(
            aggregate,
            name,
            empty_signature(),
            DeclarationMode::Overload,
        ) {
            Ok(method) => method,
            Err(error) => {
                self.bind_error(routine.span.clone(), error);
                return;
            }
        };
        if routine.has_body {
            self.bind_routine_body(routine, method);
        }
    }

    fn bind_routine(&mut self, routine: &RoutineDeclarationSyntax, owner: RoutineOwner) {
        let name = self.binder.scopes.intern_name(&routine.name.spelling);
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
                    .declare_routine(name, empty_signature(), owner, DeclarationMode::Overload)
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
            self.bind_routine_body(routine, declared);
        }
    }

    fn bind_routine_body(&mut self, routine: &RoutineDeclarationSyntax, declared: DeclaredRoutine) {
        let checkpoint = match self.binder.begin_routine_body(declared) {
            Ok(checkpoint) => checkpoint,
            Err(error) => {
                self.bind_error(routine.span.clone(), error);
                return;
            }
        };
        self.bind_declarations(
            &routine.body_declarations,
            RoutineOwner::Routine(declared.ty),
            None,
        );
        self.binder.end_routine_body(checkpoint);
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
            TypeSyntaxKind::Procedural { method_pointer } => {
                let flavor = if *method_pointer {
                    CallableFlavor::Method
                } else {
                    CallableFlavor::Routine
                };
                Some(self.allocate_anonymous(CallableType {
                    owner: RoutineOwner::Unit,
                    flavor,
                    signature: empty_signature(),
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

fn empty_signature() -> RoutineSignature {
    RoutineSignature {
        parameters: Vec::new(),
        result: None,
        calling_convention: CallingConvention::Pascal,
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
    let _ = primitive(
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
    for name in [
        "pointer",
        "string",
        "ansistring",
        "widestring",
        "unicodeString",
    ] {
        let name = binder.scopes.intern_name(name);
        let _ = binder
            .define_type(name, opaque_type())
            .expect("builtin names are unique");
    }
    BuiltinTypes { integer }
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
            file,
            declarations,
            unit: None,
        });
    }
    driver.register_units(&mut inputs);
    driver.configure_uses(&inputs);
    driver.bind_unit_interfaces(&inputs);
    let mut files = Vec::new();
    driver.bind_remaining_files(&inputs, &mut files);
    SemanticCompilation {
        binder: driver.binder,
        units: driver.units,
        files,
        diagnostics: driver.diagnostics,
    }
}
