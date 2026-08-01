use std::collections::BTreeMap;

use super::{
    AggregateShape, CallableFlavor, CallableType, DeclId, DeclarationMode, DeclarationState,
    DeclareError, EnvironmentCheckpoint, EnvironmentId, EnvironmentRequirement, Field, FieldLayout,
    FrameKind, IncompleteReason, LookupEdge, LookupRequest, NameId, ObjectType, PackedRecordType,
    PascalType, PointerType, RegionOwner, RegularRecordType, RoutineOwner, RoutineSignature,
    ScopeGraph, StorageLayout, SymbolId, SymbolKind, TypeOwner, TypeRef, TypeRegistry,
    TypeRegistryError, TypeSectionId, VariantPart,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeclaredType {
    pub symbol: SymbolId,
    pub ty: TypeRef,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnresolvedTypeForward {
    pub name: NameId,
    pub symbol: SymbolId,
    pub ty: TypeRef,
    pub section: TypeSectionId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeclaredRoutine {
    pub symbol: SymbolId,
    pub ty: TypeRef,
    /// Exact environment after the header symbol became visible.
    pub lexical_parent_environment: EnvironmentId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoutineBodyCheckpoint {
    routine: TypeRef,
    selected_environment: EnvironmentCheckpoint,
    body_environment: EnvironmentCheckpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AggregateKind {
    RegularRecord,
    PackedRecord,
    Object { base: Option<TypeRef> },
}

#[derive(Clone, Debug)]
pub struct AggregateDefinition {
    pub declared: DeclaredType,
    pub kind: AggregateKind,
    pub layout: StorageLayout,
    pub fields: Vec<Field>,
    pub methods: Vec<TypeRef>,
    member_region: super::RegionId,
    selected_environment: EnvironmentCheckpoint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindError {
    TypeSectionAlreadyActive,
    NoActiveTypeSection,
    MissingMemberEnvironment(TypeRef),
    VariantPartOnObject,
    Declare(DeclareError),
    TypeRegistry(TypeRegistryError),
}

impl From<DeclareError> for BindError {
    fn from(error: DeclareError) -> Self {
        Self::Declare(error)
    }
}

impl From<TypeRegistryError> for BindError {
    fn from(error: TypeRegistryError) -> Self {
        Self::TypeRegistry(error)
    }
}

#[derive(Clone, Copy, Debug)]
struct PendingType {
    symbol: SymbolId,
    ty: TypeRef,
}

#[derive(Clone, Debug)]
struct ActiveTypeSection {
    id: TypeSectionId,
    pending: BTreeMap<NameId, PendingType>,
}

#[derive(Debug)]
pub struct SemanticBinder {
    pub scopes: ScopeGraph,
    pub types: TypeRegistry,
    active_type_section: Option<ActiveTypeSection>,
    next_type_section: usize,
    next_declaration: usize,
}

impl Default for SemanticBinder {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticBinder {
    pub fn new() -> Self {
        Self {
            scopes: ScopeGraph::new(),
            types: TypeRegistry::new(),
            active_type_section: None,
            next_type_section: 0,
            next_declaration: 0,
        }
    }

    pub fn begin_type_section(&mut self) -> Result<TypeSectionId, BindError> {
        if self.active_type_section.is_some() {
            return Err(BindError::TypeSectionAlreadyActive);
        }
        let id = TypeSectionId::from_index(self.next_type_section);
        self.next_type_section += 1;
        self.scopes.extend_environment(FrameKind::TypeSection(id));
        self.active_type_section = Some(ActiveTypeSection {
            id,
            pending: BTreeMap::new(),
        });
        Ok(id)
    }

    pub fn end_type_section(&mut self) -> Result<Vec<UnresolvedTypeForward>, BindError> {
        let Some(section) = self.active_type_section.take() else {
            return Err(BindError::NoActiveTypeSection);
        };
        let unresolved = section
            .pending
            .into_iter()
            .map(|(name, pending)| {
                self.scopes
                    .set_symbol_state(pending.symbol, DeclarationState::Error);
                self.types.mark_error(pending.ty);
                UnresolvedTypeForward {
                    name,
                    symbol: pending.symbol,
                    ty: pending.ty,
                    section: section.id,
                }
            })
            .collect();
        Ok(unresolved)
    }

    pub fn define_type(
        &mut self,
        name: NameId,
        implementation: impl PascalType + 'static,
    ) -> Result<DeclaredType, BindError> {
        if let Some(pending) = self
            .active_type_section
            .as_mut()
            .and_then(|section| section.pending.remove(&name))
        {
            self.types.begin_definition(pending.ty)?;
            self.types.complete(pending.ty, implementation)?;
            self.scopes
                .complete_symbol(pending.symbol, SymbolKind::Type(pending.ty));
            return Ok(DeclaredType {
                symbol: pending.symbol,
                ty: pending.ty,
            });
        }

        let owner = self.type_owner();
        let declared_in = self.scopes.current_environment();
        let ty = self
            .types
            .allocate_complete(owner, Some(name), declared_in, implementation);
        let symbol = match self.scopes.declare(
            name,
            SymbolKind::Type(ty),
            DeclarationState::Complete,
            DeclarationMode::Fresh,
        ) {
            Ok(symbol) => symbol,
            Err(error) => {
                self.types.mark_error(ty);
                return Err(error.into());
            }
        };
        self.types
            .set_declared_in(ty, self.scopes.symbol(symbol).declared_in);
        self.register_type_owner(owner, ty)?;
        Ok(DeclaredType { symbol, ty })
    }

    pub fn declare_pointer_type(
        &mut self,
        name: NameId,
        target_name: NameId,
        pointer_layout: StorageLayout,
    ) -> Result<(DeclaredType, TypeRef), BindError> {
        let target = self.resolve_pointer_target(target_name)?;
        let declared = self.define_type(
            name,
            PointerType {
                target,
                layout: pointer_layout,
            },
        )?;
        Ok((declared, target))
    }

    pub fn declare_routine(
        &mut self,
        name: NameId,
        signature: RoutineSignature,
        owner: RoutineOwner,
        mode: DeclarationMode,
    ) -> Result<DeclaredRoutine, BindError> {
        let (flavor, environment, type_owner) = match owner {
            RoutineOwner::Unit => (
                CallableFlavor::Routine,
                EnvironmentRequirement::None,
                self.fresh_declaration_owner(),
            ),
            RoutineOwner::Type(parent) => (
                CallableFlavor::Method,
                EnvironmentRequirement::None,
                TypeOwner::Type(parent),
            ),
            RoutineOwner::Routine(parent) => (
                CallableFlavor::Nested,
                EnvironmentRequirement::StaticLink {
                    lexical_parent: parent,
                },
                TypeOwner::Routine(parent),
            ),
        };
        let declared_in = self.scopes.current_environment();
        let ty = self.types.allocate_complete(
            type_owner,
            Some(name),
            declared_in,
            CallableType {
                owner,
                flavor,
                signature,
                declaration_region: None,
                nested_routines: Vec::new(),
                local_types: Vec::new(),
                captures: Vec::new(),
                environment,
                has_body: false,
            },
        );
        let symbol = match self.scopes.declare(
            name,
            SymbolKind::Routine(ty),
            DeclarationState::Complete,
            mode,
        ) {
            Ok(symbol) => symbol,
            Err(error) => {
                self.types.mark_error(ty);
                return Err(error.into());
            }
        };
        let lexical_parent_environment = self.scopes.current_environment();
        self.types
            .set_declared_in(ty, self.scopes.symbol(symbol).declared_in);
        if let RoutineOwner::Routine(parent) = owner {
            self.types.add_nested_routine(parent, ty)?;
        }
        Ok(DeclaredRoutine {
            symbol,
            ty,
            lexical_parent_environment,
        })
    }

    pub fn begin_aggregate(
        &mut self,
        name: NameId,
        kind: AggregateKind,
        layout: StorageLayout,
    ) -> Result<AggregateDefinition, BindError> {
        let declared =
            self.begin_named_type_definition(name, IncompleteReason::AggregateDefinition)?;
        let outer_environment = self.scopes.current_environment();
        let mut fallbacks = Vec::new();
        if let AggregateKind::Object { base: Some(base) } = kind {
            let inherited = self
                .types
                .member_environment(base)
                .ok_or(BindError::MissingMemberEnvironment(base))?;
            fallbacks.push(LookupEdge::inherited_members(inherited));
        }
        fallbacks.push(LookupEdge::lexical_parent(outer_environment));
        let (member_region, member_environment) = self
            .scopes
            .create_detached_region(RegionOwner::Type(declared.ty), fallbacks);
        let selected_environment = self.scopes.select_environment(member_environment);
        Ok(AggregateDefinition {
            declared,
            kind,
            layout,
            fields: Vec::new(),
            methods: Vec::new(),
            member_region,
            selected_environment,
        })
    }

    pub fn declare_field(
        &mut self,
        aggregate: &mut AggregateDefinition,
        name: NameId,
        ty: TypeRef,
        layout: FieldLayout,
    ) -> Result<SymbolId, BindError> {
        let symbol = self.scopes.declare(
            name,
            SymbolKind::Variable(ty),
            DeclarationState::Complete,
            DeclarationMode::Fresh,
        )?;
        aggregate.fields.push(Field { name, ty, layout });
        Ok(symbol)
    }

    pub fn declare_method(
        &mut self,
        aggregate: &mut AggregateDefinition,
        name: NameId,
        signature: RoutineSignature,
        mode: DeclarationMode,
    ) -> Result<DeclaredRoutine, BindError> {
        let method = self.declare_routine(
            name,
            signature,
            RoutineOwner::Type(aggregate.declared.ty),
            mode,
        )?;
        aggregate.methods.push(method.ty);
        Ok(method)
    }

    pub fn end_aggregate(
        &mut self,
        aggregate: AggregateDefinition,
        variant: Option<VariantPart>,
    ) -> Result<DeclaredType, BindError> {
        if matches!(aggregate.kind, AggregateKind::Object { .. }) && variant.is_some() {
            self.types.mark_error(aggregate.declared.ty);
            self.scopes
                .set_symbol_state(aggregate.declared.symbol, DeclarationState::Error);
            self.scopes
                .restore_environment(aggregate.selected_environment);
            return Err(BindError::VariantPartOnObject);
        }

        debug_assert_eq!(
            self.scopes
                .environment_region(self.scopes.current_environment()),
            aggregate.member_region
        );
        let shape = AggregateShape {
            member_environment: self.scopes.current_environment(),
            fields: aggregate.fields,
        };
        let implementation = match aggregate.kind {
            AggregateKind::RegularRecord => AggregateImplementation::Regular(RegularRecordType {
                aggregate: shape,
                variant,
                layout: aggregate.layout,
            }),
            AggregateKind::PackedRecord => AggregateImplementation::Packed(PackedRecordType {
                aggregate: shape,
                variant,
                layout: aggregate.layout,
            }),
            AggregateKind::Object { base } => AggregateImplementation::Object(ObjectType {
                aggregate: shape,
                base,
                methods: aggregate.methods,
                layout: aggregate.layout,
            }),
        };
        let result = implementation.complete(&mut self.types, aggregate.declared.ty);
        self.scopes
            .restore_environment(aggregate.selected_environment);
        result?;
        self.scopes.complete_symbol(
            aggregate.declared.symbol,
            SymbolKind::Type(aggregate.declared.ty),
        );
        Ok(aggregate.declared)
    }

    pub fn begin_routine_body(
        &mut self,
        routine: DeclaredRoutine,
    ) -> Result<RoutineBodyCheckpoint, BindError> {
        let selected_environment = self
            .scopes
            .select_environment(routine.lexical_parent_environment);
        let (region, body_environment) = self.scopes.enter_region(RegionOwner::Routine(routine.ty));
        self.types
            .set_callable_declaration_region(routine.ty, region)?;
        self.types.set_callable_has_body(routine.ty, true)?;
        Ok(RoutineBodyCheckpoint {
            routine: routine.ty,
            selected_environment,
            body_environment,
        })
    }

    pub fn end_routine_body(&mut self, checkpoint: RoutineBodyCheckpoint) {
        let _ = checkpoint.routine;
        self.scopes.exit_region(checkpoint.body_environment);
        self.scopes
            .restore_environment(checkpoint.selected_environment);
    }

    fn resolve_pointer_target(&mut self, name: NameId) -> Result<TypeRef, BindError> {
        if let Some(result) = self.scopes.lookup_symbol(
            self.scopes.current_environment(),
            name,
            LookupRequest::REQUIRED_TYPE,
        ) && let SymbolKind::Type(ty) = self.scopes.symbol(result.primary[0].symbol).kind
        {
            return Ok(ty);
        }

        let Some(section) = self.active_type_section.as_ref() else {
            return Err(BindError::NoActiveTypeSection);
        };
        let section_id = section.id;
        let owner = self.type_owner();
        let declared_in = self.scopes.current_environment();
        let ty = self.types.allocate_incomplete(
            owner,
            Some(name),
            declared_in,
            IncompleteReason::PointerForward,
        );
        let symbol = match self.scopes.declare(
            name,
            SymbolKind::Type(ty),
            DeclarationState::Defining,
            DeclarationMode::Fresh,
        ) {
            Ok(symbol) => symbol,
            Err(error) => {
                self.types.mark_error(ty);
                return Err(error.into());
            }
        };
        self.types
            .set_declared_in(ty, self.scopes.symbol(symbol).declared_in);
        self.register_type_owner(owner, ty)?;
        self.active_type_section
            .as_mut()
            .expect("type section checked above")
            .pending
            .insert(name, PendingType { symbol, ty });
        debug_assert_eq!(
            self.active_type_section.as_ref().map(|active| active.id),
            Some(section_id)
        );
        Ok(ty)
    }

    fn begin_named_type_definition(
        &mut self,
        name: NameId,
        reason: IncompleteReason,
    ) -> Result<DeclaredType, BindError> {
        if let Some(pending) = self
            .active_type_section
            .as_mut()
            .and_then(|section| section.pending.remove(&name))
        {
            self.types.begin_definition(pending.ty)?;
            return Ok(DeclaredType {
                symbol: pending.symbol,
                ty: pending.ty,
            });
        }

        let owner = self.type_owner();
        let ty = self.types.allocate_incomplete(
            owner,
            Some(name),
            self.scopes.current_environment(),
            reason,
        );
        let symbol = match self.scopes.declare(
            name,
            SymbolKind::Type(ty),
            DeclarationState::Defining,
            DeclarationMode::Fresh,
        ) {
            Ok(symbol) => symbol,
            Err(error) => {
                self.types.mark_error(ty);
                return Err(error.into());
            }
        };
        self.types
            .set_declared_in(ty, self.scopes.symbol(symbol).declared_in);
        self.register_type_owner(owner, ty)?;
        self.types.begin_definition(ty)?;
        Ok(DeclaredType { symbol, ty })
    }

    fn type_owner(&mut self) -> TypeOwner {
        let region = self
            .scopes
            .environment_region(self.scopes.current_environment());
        match self.scopes.region_owner(region) {
            RegionOwner::Routine(parent) => TypeOwner::Routine(parent),
            RegionOwner::Type(parent) => TypeOwner::Type(parent),
            RegionOwner::Root | RegionOwner::Unit(_) | RegionOwner::Block(_) => {
                self.fresh_declaration_owner()
            }
        }
    }

    fn fresh_declaration_owner(&mut self) -> TypeOwner {
        let declaration = DeclId::from_index(self.next_declaration);
        self.next_declaration += 1;
        TypeOwner::Declaration(declaration)
    }

    fn register_type_owner(&mut self, owner: TypeOwner, ty: TypeRef) -> Result<(), BindError> {
        if let TypeOwner::Routine(parent) = owner {
            self.types.add_local_type(parent, ty)?;
        }
        Ok(())
    }
}

enum AggregateImplementation {
    Regular(RegularRecordType),
    Packed(PackedRecordType),
    Object(ObjectType),
}

impl AggregateImplementation {
    fn complete(self, types: &mut TypeRegistry, ty: TypeRef) -> Result<(), TypeRegistryError> {
        match self {
            Self::Regular(implementation) => types.complete(ty, implementation),
            Self::Packed(implementation) => types.complete(ty, implementation),
            Self::Object(implementation) => types.complete(ty, implementation),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{
        CallingConvention, PrimitiveKind, PrimitiveType, SymbolCategory, SymbolFilter,
    };

    fn integer_type() -> PrimitiveType {
        PrimitiveType {
            kind: PrimitiveKind::Integer {
                bits: 32,
                signed: true,
            },
            layout: StorageLayout {
                size: 4,
                alignment: 4,
            },
        }
    }

    fn empty_signature() -> RoutineSignature {
        RoutineSignature {
            parameters: Vec::new(),
            result: None,
            calling_convention: CallingConvention::Pascal,
        }
    }

    #[test]
    fn pointer_forward_is_completed_only_in_its_type_section() {
        let mut binder = SemanticBinder::new();
        let pfoo = binder.scopes.intern_name("PFoo");
        let foo = binder.scopes.intern_name("TFoo");
        let layout = StorageLayout {
            size: 8,
            alignment: 8,
        };

        binder.begin_type_section().unwrap();
        let (_, forward) = binder.declare_pointer_type(pfoo, foo, layout).unwrap();
        let completed = binder.define_type(foo, integer_type()).unwrap();
        assert_eq!(forward, completed.ty);
        assert!(binder.end_type_section().unwrap().is_empty());
        assert_eq!(binder.types.storage_layout(forward).unwrap().size, 4);
    }

    #[test]
    fn unresolved_pointer_forward_cannot_leak_into_a_later_section() {
        let mut binder = SemanticBinder::new();
        let pfoo = binder.scopes.intern_name("PFoo");
        let foo = binder.scopes.intern_name("TFoo");
        let layout = StorageLayout {
            size: 8,
            alignment: 8,
        };

        let first = binder.begin_type_section().unwrap();
        binder.declare_pointer_type(pfoo, foo, layout).unwrap();
        let unresolved = binder.end_type_section().unwrap();
        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0].section, first);

        binder.begin_type_section().unwrap();
        assert!(matches!(
            binder.define_type(foo, integer_type()),
            Err(BindError::Declare(DeclareError::Duplicate { .. }))
        ));
    }

    #[test]
    fn nested_routine_owns_types_and_retains_its_declaration_snapshot() {
        let mut binder = SemanticBinder::new();
        let parent_name = binder.scopes.intern_name("Parent");
        let parent = binder
            .declare_routine(
                parent_name,
                empty_signature(),
                RoutineOwner::Unit,
                DeclarationMode::Fresh,
            )
            .unwrap();
        let parent_body = binder.begin_routine_body(parent).unwrap();

        binder.scopes.extend_environment(FrameKind::VarSection);
        let before_name = binder.scopes.intern_name("Before");
        let dummy_type = binder.types.allocate_complete(
            TypeOwner::Builtin,
            None,
            binder.scopes.current_environment(),
            integer_type(),
        );
        binder
            .scopes
            .declare(
                before_name,
                SymbolKind::Variable(dummy_type),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            )
            .unwrap();

        let nested_name = binder.scopes.intern_name("Nested");
        let nested = binder
            .declare_routine(
                nested_name,
                empty_signature(),
                RoutineOwner::Routine(parent.ty),
                DeclarationMode::Fresh,
            )
            .unwrap();

        binder.scopes.extend_environment(FrameKind::VarSection);
        let after_name = binder.scopes.intern_name("After");
        binder
            .scopes
            .declare(
                after_name,
                SymbolKind::Variable(dummy_type),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            )
            .unwrap();

        assert!(
            binder
                .scopes
                .lookup_symbol(
                    nested.lexical_parent_environment,
                    before_name,
                    LookupRequest::ORDINARY,
                )
                .is_some()
        );
        assert!(
            binder
                .scopes
                .lookup_symbol(
                    nested.lexical_parent_environment,
                    nested_name,
                    LookupRequest::ORDINARY,
                )
                .is_some()
        );
        assert!(
            binder
                .scopes
                .lookup_symbol(
                    nested.lexical_parent_environment,
                    after_name,
                    LookupRequest::ORDINARY,
                )
                .is_none()
        );

        let nested_body = binder.begin_routine_body(nested).unwrap();
        binder.begin_type_section().unwrap();
        let local_name = binder.scopes.intern_name("TLocal");
        let local = binder.define_type(local_name, integer_type()).unwrap();
        binder.end_type_section().unwrap();
        let nested_type = binder.types.callable(nested.ty).unwrap();
        assert_eq!(nested_type.owner, RoutineOwner::Routine(parent.ty));
        assert_eq!(nested_type.flavor, CallableFlavor::Nested);
        assert_eq!(nested_type.local_types, vec![local.ty]);
        assert_eq!(
            nested_type.environment,
            EnvironmentRequirement::StaticLink {
                lexical_parent: parent.ty
            }
        );

        let exact_variable = LookupRequest {
            accepted: SymbolFilter::Category(SymbolCategory::Variable),
            barrier: super::super::LookupBarrier::AcceptedDeclaration,
        };
        assert!(
            binder
                .scopes
                .lookup_symbol(
                    nested.lexical_parent_environment,
                    before_name,
                    exact_variable,
                )
                .is_some()
        );

        binder.end_routine_body(nested_body);
        binder.end_routine_body(parent_body);
    }

    #[test]
    fn object_members_link_to_the_base_and_type_lookup_skips_fields() {
        let mut binder = SemanticBinder::new();
        let x = binder.scopes.intern_name("X");
        let outer_x = binder.define_type(x, integer_type()).unwrap();
        let integer = outer_x.ty;
        let layout = StorageLayout {
            size: 4,
            alignment: 4,
        };
        let field_layout = FieldLayout {
            byte_offset: 0,
            bit_offset: 0,
            bit_width: None,
        };

        let base_name = binder.scopes.intern_name("TBase");
        let mut base = binder
            .begin_aggregate(base_name, AggregateKind::Object { base: None }, layout)
            .unwrap();
        let base_x = binder
            .declare_field(&mut base, x, integer, field_layout)
            .unwrap();
        let base_only = binder.scopes.intern_name("BaseOnly");
        binder
            .declare_field(&mut base, base_only, integer, field_layout)
            .unwrap();
        let base = binder.end_aggregate(base, None).unwrap();

        let derived_name = binder.scopes.intern_name("TDerived");
        let mut derived = binder
            .begin_aggregate(
                derived_name,
                AggregateKind::Object {
                    base: Some(base.ty),
                },
                layout,
            )
            .unwrap();
        let member_environment = binder.scopes.current_environment();

        let ordinary = binder
            .scopes
            .lookup_symbol(member_environment, x, LookupRequest::ORDINARY)
            .unwrap();
        assert_eq!(ordinary.primary[0].symbol, base_x);

        let required_type = binder
            .scopes
            .lookup_symbol(member_environment, x, LookupRequest::REQUIRED_TYPE)
            .unwrap();
        assert_eq!(required_type.primary[0].symbol, outer_x.symbol);

        let derived_field = binder
            .declare_field(&mut derived, base_only, integer, field_layout)
            .unwrap();
        let ordinary = binder
            .scopes
            .lookup_symbol(
                binder.scopes.current_environment(),
                base_only,
                LookupRequest::ORDINARY,
            )
            .unwrap();
        assert_eq!(ordinary.primary[0].symbol, derived_field);

        let nested_name = binder.scopes.intern_name("TNested");
        let nested = binder
            .begin_aggregate(nested_name, AggregateKind::RegularRecord, layout)
            .unwrap();
        let nested = binder.end_aggregate(nested, None).unwrap();
        assert_eq!(
            binder.types.entry(nested.ty).owner,
            TypeOwner::Type(derived.declared.ty)
        );

        let derived = binder.end_aggregate(derived, None).unwrap();
        assert!(binder.types.is_subtype(derived.ty, base.ty));
        assert!(!binder.types.is_subtype(base.ty, derived.ty));
    }
}
