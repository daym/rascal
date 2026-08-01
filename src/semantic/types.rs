use std::{any::Any, cell::RefCell, collections::BTreeSet, fmt::Debug};

use super::{
    ids::{DeclId, EnvironmentId, NameId, NodeId, RegionId, StorageId, SymbolId, TypeRef},
    scope::SymbolCategory,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageLayout {
    pub size: u64,
    pub alignment: u32,
}

#[derive(Clone, Debug)]
pub struct OpaqueType {
    pub layout: Option<StorageLayout>,
    pub reference_type: bool,
    pub managed_lifetime: bool,
}

impl PascalType for OpaqueType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn storage_layout(&self, _query: TypeQuery<'_>) -> Option<StorageLayout> {
        self.layout
    }

    fn is_reference_type(&self, _query: TypeQuery<'_>) -> bool {
        self.reference_type
    }

    fn has_managed_lifetime(&self, _query: TypeQuery<'_>) -> bool {
        self.managed_lifetime
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversionRank {
    Exact,
    Subtype,
    Widening,
    Compatible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeCheck {
    None,
    TargetPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueConversionOperation {
    Identity,
    IntegerWiden,
    ClassUpcast,
    InterfaceUpcast,
    StringConvert,
    ArrayConvert,
    Callable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueConversion {
    pub rank: ConversionRank,
    pub operation: ValueConversionOperation,
    pub range_check: RangeCheck,
}

impl ValueConversion {
    pub const fn identity() -> Self {
        Self {
            rank: ConversionRank::Exact,
            operation: ValueConversionOperation::Identity,
            range_check: RangeCheck::None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplicitConversion {
    Value(ValueConversion),
    IntegerTruncate {
        destination: TypeRef,
        source: TypeRef,
    },
    PointerCrossing {
        destination: TypeRef,
        source: TypeRef,
    },
    RelatedDowncast {
        destination: TypeRef,
        source: TypeRef,
    },
    RepresentationOverlay {
        destination: TypeRef,
        source: TypeRef,
        size: u64,
        writable_requires_addressable_source: bool,
    },
}

#[derive(Clone, Copy)]
pub struct TypeQuery<'a> {
    pub types: &'a TypeRegistry,
    pub this: TypeRef,
}

pub trait PascalType: Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn storage_layout(&self, _query: TypeQuery<'_>) -> Option<StorageLayout> {
        None
    }

    fn is_reference_type(&self, _query: TypeQuery<'_>) -> bool {
        false
    }

    fn has_managed_lifetime(&self, _query: TypeQuery<'_>) -> bool {
        false
    }

    fn value_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ValueConversion> {
        (query.this == source).then(ValueConversion::identity)
    }

    /// Returns one predefined direct edge for explicit `ThisType(value)`
    /// syntax. Implementations inspect only `source` and this destination.
    /// The binder must not compose this result with another conversion.
    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        self.value_conversion_from(query, source)
            .map(ExplicitConversion::Value)
    }

    fn is_subtype_of(&self, query: TypeQuery<'_>, target: TypeRef) -> bool {
        query.this == target
    }

    fn same_formal_contract_as(&self, query: TypeQuery<'_>, other: TypeRef) -> bool {
        query.this == other
    }

    fn sequence_element_type(&self, _query: TypeQuery<'_>) -> Option<TypeRef> {
        None
    }

    fn array_element_type(&self, _query: TypeQuery<'_>) -> Option<TypeRef> {
        None
    }

    fn sequence_index_type(&self, _query: TypeQuery<'_>) -> Option<TypeRef> {
        None
    }

    fn sequence_length_type(&self, _query: TypeQuery<'_>) -> Option<TypeRef> {
        None
    }

    fn sequence_is_resizable(&self, _query: TypeQuery<'_>) -> bool {
        false
    }

    fn member_environment(&self, _query: TypeQuery<'_>) -> Option<EnvironmentId> {
        None
    }

    fn project_field(&self, _query: TypeQuery<'_>, _base: Place, _name: NameId) -> Option<Place> {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeOwner {
    Builtin,
    Declaration(DeclId),
    Routine(TypeRef),
    Type(TypeRef),
    Anonymous(NodeId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IncompleteReason {
    PointerForward,
    ClassForward,
    RoutineForward,
    AggregateDefinition,
}

#[derive(Debug)]
pub enum TypeState {
    Incomplete(IncompleteReason),
    Defining,
    Complete(Box<dyn PascalType>),
    Error,
}

#[derive(Debug)]
pub struct TypeEntry {
    pub owner: TypeOwner,
    pub name: Option<NameId>,
    pub declared_in: EnvironmentId,
    pub state: TypeState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeRegistryError {
    AlreadyComplete(TypeRef),
    NotCallable(TypeRef),
}

#[derive(Debug, Default)]
pub struct TypeRegistry {
    entries: Vec<TypeEntry>,
    subtype_active: RefCell<BTreeSet<(TypeRef, TypeRef)>>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allocate_incomplete(
        &mut self,
        owner: TypeOwner,
        name: Option<NameId>,
        declared_in: EnvironmentId,
        reason: IncompleteReason,
    ) -> TypeRef {
        self.allocate_entry(owner, name, declared_in, TypeState::Incomplete(reason))
    }

    pub fn allocate_complete(
        &mut self,
        owner: TypeOwner,
        name: Option<NameId>,
        declared_in: EnvironmentId,
        implementation: impl PascalType + 'static,
    ) -> TypeRef {
        self.allocate_entry(
            owner,
            name,
            declared_in,
            TypeState::Complete(Box::new(implementation)),
        )
    }

    pub fn begin_definition(&mut self, ty: TypeRef) -> Result<(), TypeRegistryError> {
        match self.entries[ty.index()].state {
            TypeState::Complete(_) => Err(TypeRegistryError::AlreadyComplete(ty)),
            TypeState::Incomplete(_) | TypeState::Defining | TypeState::Error => {
                self.entries[ty.index()].state = TypeState::Defining;
                Ok(())
            }
        }
    }

    pub fn complete(
        &mut self,
        ty: TypeRef,
        implementation: impl PascalType + 'static,
    ) -> Result<(), TypeRegistryError> {
        if matches!(self.entries[ty.index()].state, TypeState::Complete(_)) {
            return Err(TypeRegistryError::AlreadyComplete(ty));
        }
        self.entries[ty.index()].state = TypeState::Complete(Box::new(implementation));
        Ok(())
    }

    pub fn mark_error(&mut self, ty: TypeRef) {
        self.entries[ty.index()].state = TypeState::Error;
    }

    pub fn entry(&self, ty: TypeRef) -> &TypeEntry {
        &self.entries[ty.index()]
    }

    pub fn set_declared_in(&mut self, ty: TypeRef, environment: EnvironmentId) {
        self.entries[ty.index()].declared_in = environment;
    }

    pub fn query(&self, ty: TypeRef) -> TypeQuery<'_> {
        TypeQuery {
            types: self,
            this: ty,
        }
    }

    pub fn storage_layout(&self, ty: TypeRef) -> Option<StorageLayout> {
        self.implementation(ty)?.storage_layout(self.query(ty))
    }

    pub fn is_reference_type(&self, ty: TypeRef) -> bool {
        self.implementation(ty)
            .is_some_and(|implementation| implementation.is_reference_type(self.query(ty)))
    }

    pub fn has_managed_lifetime(&self, ty: TypeRef) -> bool {
        self.implementation(ty)
            .is_some_and(|implementation| implementation.has_managed_lifetime(self.query(ty)))
    }

    pub fn value_conversion(
        &self,
        destination: TypeRef,
        source: TypeRef,
    ) -> Option<ValueConversion> {
        self.implementation(destination)?
            .value_conversion_from(self.query(destination), source)
    }

    pub fn predefined_explicit_conversion(
        &self,
        destination: TypeRef,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        self.implementation(destination)?
            .predefined_explicit_conversion_from(self.query(destination), source)
    }

    pub fn is_subtype(&self, source: TypeRef, target: TypeRef) -> bool {
        if source == target {
            return true;
        }
        if !self.subtype_active.borrow_mut().insert((source, target)) {
            return false;
        }
        let result = self
            .implementation(source)
            .is_some_and(|implementation| implementation.is_subtype_of(self.query(source), target));
        self.subtype_active.borrow_mut().remove(&(source, target));
        result
    }

    pub fn same_formal_contract(&self, left: TypeRef, right: TypeRef) -> bool {
        left == right
            || self.implementation(left).is_some_and(|implementation| {
                implementation.same_formal_contract_as(self.query(left), right)
            })
    }

    pub fn sequence_element_type(&self, ty: TypeRef) -> Option<TypeRef> {
        self.implementation(ty)?
            .sequence_element_type(self.query(ty))
    }

    pub fn array_element_type(&self, ty: TypeRef) -> Option<TypeRef> {
        self.implementation(ty)?.array_element_type(self.query(ty))
    }

    pub fn sequence_index_type(&self, ty: TypeRef) -> Option<TypeRef> {
        self.implementation(ty)?.sequence_index_type(self.query(ty))
    }

    pub fn sequence_length_type(&self, ty: TypeRef) -> Option<TypeRef> {
        self.implementation(ty)?
            .sequence_length_type(self.query(ty))
    }

    pub fn sequence_is_resizable(&self, ty: TypeRef) -> bool {
        self.implementation(ty)
            .is_some_and(|implementation| implementation.sequence_is_resizable(self.query(ty)))
    }

    pub fn member_environment(&self, ty: TypeRef) -> Option<EnvironmentId> {
        self.implementation(ty)?.member_environment(self.query(ty))
    }

    pub fn project_field(&self, ty: TypeRef, base: Place, name: NameId) -> Option<Place> {
        self.implementation(ty)?
            .project_field(self.query(ty), base, name)
    }

    pub fn add_nested_routine(
        &mut self,
        parent: TypeRef,
        nested: TypeRef,
    ) -> Result<(), TypeRegistryError> {
        let Some(callable) = self
            .implementation_mut(parent)
            .and_then(|implementation| implementation.as_any_mut().downcast_mut::<CallableType>())
        else {
            return Err(TypeRegistryError::NotCallable(parent));
        };
        callable.nested_routines.push(nested);
        Ok(())
    }

    pub fn add_local_type(
        &mut self,
        parent: TypeRef,
        local: TypeRef,
    ) -> Result<(), TypeRegistryError> {
        let Some(callable) = self
            .implementation_mut(parent)
            .and_then(|implementation| implementation.as_any_mut().downcast_mut::<CallableType>())
        else {
            return Err(TypeRegistryError::NotCallable(parent));
        };
        callable.local_types.push(local);
        Ok(())
    }

    pub fn set_callable_declaration_region(
        &mut self,
        callable: TypeRef,
        region: RegionId,
    ) -> Result<(), TypeRegistryError> {
        let Some(callable) = self
            .implementation_mut(callable)
            .and_then(|implementation| implementation.as_any_mut().downcast_mut::<CallableType>())
        else {
            return Err(TypeRegistryError::NotCallable(callable));
        };
        callable.declaration_region = Some(region);
        Ok(())
    }

    pub fn add_capture(
        &mut self,
        callable: TypeRef,
        capture: Capture,
    ) -> Result<(), TypeRegistryError> {
        let Some(callable) = self
            .implementation_mut(callable)
            .and_then(|implementation| implementation.as_any_mut().downcast_mut::<CallableType>())
        else {
            return Err(TypeRegistryError::NotCallable(callable));
        };
        if !callable
            .captures
            .iter()
            .any(|existing| existing.symbol == capture.symbol)
        {
            callable.captures.push(capture);
        }
        Ok(())
    }

    pub fn set_callable_has_body(
        &mut self,
        callable: TypeRef,
        has_body: bool,
    ) -> Result<(), TypeRegistryError> {
        let Some(callable) = self
            .implementation_mut(callable)
            .and_then(|implementation| implementation.as_any_mut().downcast_mut::<CallableType>())
        else {
            return Err(TypeRegistryError::NotCallable(callable));
        };
        callable.has_body = has_body;
        Ok(())
    }

    pub fn callable(&self, ty: TypeRef) -> Option<&CallableType> {
        self.implementation(ty)?
            .as_any()
            .downcast_ref::<CallableType>()
    }

    fn allocate_entry(
        &mut self,
        owner: TypeOwner,
        name: Option<NameId>,
        declared_in: EnvironmentId,
        state: TypeState,
    ) -> TypeRef {
        let ty = TypeRef::from_index(self.entries.len());
        self.entries.push(TypeEntry {
            owner,
            name,
            declared_in,
            state,
        });
        ty
    }

    fn implementation(&self, ty: TypeRef) -> Option<&dyn PascalType> {
        match &self.entries.get(ty.index())?.state {
            TypeState::Complete(implementation) => Some(implementation.as_ref()),
            TypeState::Incomplete(_) | TypeState::Defining | TypeState::Error => None,
        }
    }

    fn implementation_mut(&mut self, ty: TypeRef) -> Option<&mut dyn PascalType> {
        match &mut self.entries.get_mut(ty.index())?.state {
            TypeState::Complete(implementation) => Some(implementation.as_mut()),
            TypeState::Incomplete(_) | TypeState::Defining | TypeState::Error => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitiveKind {
    Integer { bits: u16, signed: bool },
    Real { bits: u16 },
    Boolean,
    Character,
}

#[derive(Clone, Debug)]
pub struct PrimitiveType {
    pub kind: PrimitiveKind,
    pub layout: StorageLayout,
}

impl PascalType for PrimitiveType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn storage_layout(&self, _query: TypeQuery<'_>) -> Option<StorageLayout> {
        Some(self.layout)
    }

    fn value_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ValueConversion> {
        if query.this == source {
            return Some(ValueConversion::identity());
        }
        let source = query
            .types
            .implementation(source)?
            .as_any()
            .downcast_ref::<PrimitiveType>()?;
        match (self.kind, source.kind) {
            (
                PrimitiveKind::Integer {
                    bits: destination_bits,
                    signed: destination_signed,
                },
                PrimitiveKind::Integer {
                    bits: source_bits,
                    signed: source_signed,
                },
            ) if destination_bits >= source_bits
                && (destination_signed == source_signed
                    || destination_signed && destination_bits > source_bits) =>
            {
                Some(ValueConversion {
                    rank: ConversionRank::Widening,
                    operation: ValueConversionOperation::IntegerWiden,
                    range_check: RangeCheck::None,
                })
            }
            _ => None,
        }
    }

    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        if let Some(implicit) = self.value_conversion_from(query, source) {
            return Some(ExplicitConversion::Value(implicit));
        }
        let source_type = query
            .types
            .implementation(source)?
            .as_any()
            .downcast_ref::<PrimitiveType>()?;
        match (self.kind, source_type.kind) {
            (PrimitiveKind::Integer { .. }, PrimitiveKind::Integer { .. }) => {
                Some(ExplicitConversion::IntegerTruncate {
                    destination: query.this,
                    source,
                })
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AliasType {
    pub target: TypeRef,
    pub nominal: bool,
}

impl PascalType for AliasType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn storage_layout(&self, query: TypeQuery<'_>) -> Option<StorageLayout> {
        query.types.storage_layout(self.target)
    }

    fn is_reference_type(&self, query: TypeQuery<'_>) -> bool {
        !self.nominal && query.types.is_reference_type(self.target)
    }

    fn has_managed_lifetime(&self, query: TypeQuery<'_>) -> bool {
        query.types.has_managed_lifetime(self.target)
    }

    fn value_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ValueConversion> {
        if query.this == source {
            Some(ValueConversion::identity())
        } else if self.nominal {
            None
        } else {
            query.types.value_conversion(self.target, source)
        }
    }

    fn is_subtype_of(&self, query: TypeQuery<'_>, target: TypeRef) -> bool {
        query.this == target || (!self.nominal && query.types.is_subtype(self.target, target))
    }

    fn member_environment(&self, query: TypeQuery<'_>) -> Option<EnvironmentId> {
        (!self.nominal)
            .then(|| query.types.member_environment(self.target))
            .flatten()
    }
}

#[derive(Clone, Debug)]
pub struct PointerType {
    pub target: TypeRef,
    pub layout: StorageLayout,
}

impl PascalType for PointerType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn storage_layout(&self, _query: TypeQuery<'_>) -> Option<StorageLayout> {
        Some(self.layout)
    }

    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        if query.this == source {
            return Some(ExplicitConversion::Value(ValueConversion::identity()));
        }
        query
            .types
            .implementation(source)?
            .as_any()
            .is::<PointerType>()
            .then_some(ExplicitConversion::PointerCrossing {
                destination: query.this,
                source,
            })
    }
}

#[derive(Clone, Debug)]
pub struct ClassType {
    pub aggregate: Option<AggregateShape>,
    pub base: Option<TypeRef>,
    pub interfaces: Vec<TypeRef>,
    pub methods: Vec<TypeRef>,
    pub pointer_layout: StorageLayout,
}

impl PascalType for ClassType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn storage_layout(&self, _query: TypeQuery<'_>) -> Option<StorageLayout> {
        Some(self.pointer_layout)
    }

    fn is_reference_type(&self, _query: TypeQuery<'_>) -> bool {
        true
    }

    fn member_environment(&self, _query: TypeQuery<'_>) -> Option<EnvironmentId> {
        self.aggregate
            .as_ref()
            .map(|aggregate| aggregate.member_environment)
    }

    fn is_subtype_of(&self, query: TypeQuery<'_>, target: TypeRef) -> bool {
        query.this == target
            || self
                .base
                .is_some_and(|base| query.types.is_subtype(base, target))
            || self
                .interfaces
                .iter()
                .any(|interface| query.types.is_subtype(*interface, target))
    }

    fn value_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ValueConversion> {
        if source == query.this {
            Some(ValueConversion::identity())
        } else if query.types.is_subtype(source, query.this) {
            Some(ValueConversion {
                rank: ConversionRank::Subtype,
                operation: ValueConversionOperation::ClassUpcast,
                range_check: RangeCheck::None,
            })
        } else {
            None
        }
    }

    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        if let Some(implicit) = self.value_conversion_from(query, source) {
            return Some(ExplicitConversion::Value(implicit));
        }
        query
            .types
            .is_subtype(query.this, source)
            .then_some(ExplicitConversion::RelatedDowncast {
                destination: query.this,
                source,
            })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterMode {
    Value,
    Const,
    Var,
    Out,
    ConstRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormalParameter {
    pub mode: ParameterMode,
    pub ty: TypeRef,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallingConvention {
    Pascal,
    Register,
    Cdecl,
    Stdcall,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoutineSignature {
    pub parameters: Vec<FormalParameter>,
    pub result: Option<TypeRef>,
    pub calling_convention: CallingConvention,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallableFlavor {
    Routine,
    Method,
    Nested,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoutineOwner {
    Unit,
    Type(TypeRef),
    Routine(TypeRef),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvironmentRequirement {
    None,
    StaticLink { lexical_parent: TypeRef },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capture {
    pub symbol: SymbolId,
    pub lexical_depth: u32,
}

#[derive(Clone, Debug)]
pub struct CallableType {
    pub owner: RoutineOwner,
    pub flavor: CallableFlavor,
    pub signature: RoutineSignature,
    pub declaration_region: Option<RegionId>,
    pub nested_routines: Vec<TypeRef>,
    pub local_types: Vec<TypeRef>,
    pub captures: Vec<Capture>,
    pub environment: EnvironmentRequirement,
    pub has_body: bool,
}

impl CallableType {
    fn compatible_signature(&self, query: TypeQuery<'_>, other: &Self) -> bool {
        self.signature.calling_convention == other.signature.calling_convention
            && self.signature.parameters.len() == other.signature.parameters.len()
            && self
                .signature
                .parameters
                .iter()
                .zip(&other.signature.parameters)
                .all(|(left, right)| {
                    left.mode == right.mode && query.types.same_formal_contract(left.ty, right.ty)
                })
            && match (self.signature.result, other.signature.result) {
                (Some(left), Some(right)) => query.types.same_formal_contract(left, right),
                (None, None) => true,
                _ => false,
            }
    }
}

impl PascalType for CallableType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn value_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ValueConversion> {
        if query.this == source {
            return Some(ValueConversion::identity());
        }
        if self.flavor == CallableFlavor::Nested {
            return None;
        }
        let source = query.types.callable(source)?;
        (source.flavor == self.flavor
            && source.flavor != CallableFlavor::Nested
            && self.compatible_signature(query, source))
        .then_some(ValueConversion {
            rank: ConversionRank::Compatible,
            operation: ValueConversionOperation::Callable,
            range_check: RangeCheck::None,
        })
    }

    fn same_formal_contract_as(&self, query: TypeQuery<'_>, other: TypeRef) -> bool {
        let Some(other) = query.types.callable(other) else {
            return false;
        };
        self.flavor == other.flavor
            && self.flavor != CallableFlavor::Nested
            && self.compatible_signature(query, other)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldLayout {
    pub byte_offset: u64,
    pub bit_offset: u8,
    pub bit_width: Option<u16>,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: NameId,
    pub ty: TypeRef,
    pub layout: FieldLayout,
}

#[derive(Clone, Debug)]
pub struct AggregateShape {
    pub member_environment: EnvironmentId,
    pub fields: Vec<Field>,
}

impl AggregateShape {
    fn field(&self, name: NameId) -> Option<&Field> {
        self.fields.iter().find(|field| field.name == name)
    }
}

#[derive(Clone, Debug)]
pub struct VariantAlternative {
    pub labels: Vec<i128>,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug)]
pub struct VariantPart {
    pub selector: Option<Field>,
    pub alternatives: Vec<VariantAlternative>,
    pub byte_offset: u64,
    pub byte_size: u64,
    pub alignment: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageBase {
    Symbol(SymbolId),
    Temporary(StorageId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccessKind {
    Aligned,
    Unaligned,
    BitPacked { bit_offset: u8, bit_width: u16 },
    VariantPayload,
    RepresentationOverlay,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub ty: TypeRef,
    pub base: StorageBase,
    pub byte_offset: u64,
    pub access: AccessKind,
    pub mutable: bool,
}

fn aggregate_managed(query: TypeQuery<'_>, aggregate: &AggregateShape) -> bool {
    aggregate
        .fields
        .iter()
        .any(|field| query.types.has_managed_lifetime(field.ty))
}

fn project_regular_field(base: Place, field: &Field) -> Place {
    Place {
        ty: field.ty,
        base: base.base,
        byte_offset: base.byte_offset + field.layout.byte_offset,
        access: if let Some(bit_width) = field.layout.bit_width {
            AccessKind::BitPacked {
                bit_offset: field.layout.bit_offset,
                bit_width,
            }
        } else {
            AccessKind::Aligned
        },
        mutable: base.mutable,
    }
}

fn project_packed_field(base: Place, field: &Field) -> Place {
    Place {
        ty: field.ty,
        base: base.base,
        byte_offset: base.byte_offset + field.layout.byte_offset,
        access: match field.layout.bit_width {
            Some(bit_width) => AccessKind::BitPacked {
                bit_offset: field.layout.bit_offset,
                bit_width,
            },
            None => AccessKind::Unaligned,
        },
        mutable: base.mutable,
    }
}

#[derive(Clone, Debug)]
pub struct RegularRecordType {
    pub aggregate: AggregateShape,
    pub variant: Option<VariantPart>,
    pub layout: StorageLayout,
}

impl PascalType for RegularRecordType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn storage_layout(&self, _query: TypeQuery<'_>) -> Option<StorageLayout> {
        Some(self.layout)
    }

    fn has_managed_lifetime(&self, query: TypeQuery<'_>) -> bool {
        aggregate_managed(query, &self.aggregate)
    }

    fn member_environment(&self, _query: TypeQuery<'_>) -> Option<EnvironmentId> {
        Some(self.aggregate.member_environment)
    }

    fn project_field(&self, _query: TypeQuery<'_>, base: Place, name: NameId) -> Option<Place> {
        self.aggregate
            .field(name)
            .map(|field| project_regular_field(base, field))
    }
}

#[derive(Clone, Debug)]
pub struct PackedRecordType {
    pub aggregate: AggregateShape,
    pub variant: Option<VariantPart>,
    pub layout: StorageLayout,
}

impl PascalType for PackedRecordType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn storage_layout(&self, _query: TypeQuery<'_>) -> Option<StorageLayout> {
        Some(self.layout)
    }

    fn has_managed_lifetime(&self, query: TypeQuery<'_>) -> bool {
        aggregate_managed(query, &self.aggregate)
    }

    fn member_environment(&self, _query: TypeQuery<'_>) -> Option<EnvironmentId> {
        Some(self.aggregate.member_environment)
    }

    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        if query.this == source {
            return Some(ExplicitConversion::Value(ValueConversion::identity()));
        }
        let source_layout = query.types.storage_layout(source)?;
        (source_layout.size == self.layout.size).then_some(
            ExplicitConversion::RepresentationOverlay {
                destination: query.this,
                source,
                size: self.layout.size,
                writable_requires_addressable_source: true,
            },
        )
    }

    fn project_field(&self, _query: TypeQuery<'_>, base: Place, name: NameId) -> Option<Place> {
        self.aggregate
            .field(name)
            .map(|field| project_packed_field(base, field))
    }
}

#[derive(Clone, Debug)]
pub struct ObjectType {
    pub aggregate: AggregateShape,
    pub base: Option<TypeRef>,
    pub methods: Vec<TypeRef>,
    pub layout: StorageLayout,
}

impl PascalType for ObjectType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn storage_layout(&self, _query: TypeQuery<'_>) -> Option<StorageLayout> {
        Some(self.layout)
    }

    fn has_managed_lifetime(&self, query: TypeQuery<'_>) -> bool {
        aggregate_managed(query, &self.aggregate)
    }

    fn member_environment(&self, _query: TypeQuery<'_>) -> Option<EnvironmentId> {
        Some(self.aggregate.member_environment)
    }

    fn is_subtype_of(&self, query: TypeQuery<'_>, target: TypeRef) -> bool {
        query.this == target
            || self
                .base
                .is_some_and(|base| query.types.is_subtype(base, target))
    }

    fn project_field(&self, _query: TypeQuery<'_>, base: Place, name: NameId) -> Option<Place> {
        self.aggregate
            .field(name)
            .map(|field| project_regular_field(base, field))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringKind {
    Short,
    Ansi,
    Wide,
    Unicode,
}

#[derive(Clone, Debug)]
pub struct StringType {
    pub kind: StringKind,
    pub element: TypeRef,
    pub index: TypeRef,
    pub length: TypeRef,
    pub layout: StorageLayout,
}

impl PascalType for StringType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn storage_layout(&self, _query: TypeQuery<'_>) -> Option<StorageLayout> {
        Some(self.layout)
    }

    fn has_managed_lifetime(&self, _query: TypeQuery<'_>) -> bool {
        self.kind != StringKind::Short
    }

    fn sequence_element_type(&self, _query: TypeQuery<'_>) -> Option<TypeRef> {
        Some(self.element)
    }

    fn sequence_index_type(&self, _query: TypeQuery<'_>) -> Option<TypeRef> {
        Some(self.index)
    }

    fn sequence_length_type(&self, _query: TypeQuery<'_>) -> Option<TypeRef> {
        Some(self.length)
    }

    fn sequence_is_resizable(&self, _query: TypeQuery<'_>) -> bool {
        true
    }
}

#[derive(Clone, Debug)]
pub struct ArrayType {
    pub element: TypeRef,
    pub index: TypeRef,
    pub length: TypeRef,
    pub layout: Option<StorageLayout>,
    pub resizable: bool,
    pub open: bool,
}

impl PascalType for ArrayType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn storage_layout(&self, _query: TypeQuery<'_>) -> Option<StorageLayout> {
        self.layout
    }

    fn has_managed_lifetime(&self, query: TypeQuery<'_>) -> bool {
        self.resizable || query.types.has_managed_lifetime(self.element)
    }

    fn same_formal_contract_as(&self, query: TypeQuery<'_>, other: TypeRef) -> bool {
        if query.this == other {
            return true;
        }
        let Some(other) = query
            .types
            .implementation(other)
            .and_then(|implementation| implementation.as_any().downcast_ref::<ArrayType>())
        else {
            return false;
        };
        self.open && other.open && self.element == other.element
    }

    fn sequence_element_type(&self, _query: TypeQuery<'_>) -> Option<TypeRef> {
        Some(self.element)
    }

    fn array_element_type(&self, _query: TypeQuery<'_>) -> Option<TypeRef> {
        Some(self.element)
    }

    fn sequence_index_type(&self, _query: TypeQuery<'_>) -> Option<TypeRef> {
        Some(self.index)
    }

    fn sequence_length_type(&self, _query: TypeQuery<'_>) -> Option<TypeRef> {
        Some(self.length)
    }

    fn sequence_is_resizable(&self, _query: TypeQuery<'_>) -> bool {
        self.resizable
    }
}

pub const fn symbol_category_for_callable(flavor: CallableFlavor) -> SymbolCategory {
    match flavor {
        CallableFlavor::Routine | CallableFlavor::Method | CallableFlavor::Nested => {
            SymbolCategory::Routine
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> EnvironmentId {
        EnvironmentId(0)
    }

    fn i32_type(types: &mut TypeRegistry) -> TypeRef {
        types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            PrimitiveType {
                kind: PrimitiveKind::Integer {
                    bits: 32,
                    signed: true,
                },
                layout: StorageLayout {
                    size: 4,
                    alignment: 4,
                },
            },
        )
    }

    fn callable(types: &mut TypeRegistry, flavor: CallableFlavor, parameter: TypeRef) -> TypeRef {
        types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            CallableType {
                owner: RoutineOwner::Unit,
                flavor,
                signature: RoutineSignature {
                    parameters: vec![FormalParameter {
                        mode: ParameterMode::Value,
                        ty: parameter,
                    }],
                    result: None,
                    calling_convention: CallingConvention::Pascal,
                },
                declaration_region: None,
                nested_routines: Vec::new(),
                local_types: Vec::new(),
                captures: Vec::new(),
                environment: EnvironmentRequirement::None,
                has_body: false,
            },
        )
    }

    #[test]
    fn routine_method_and_nested_callable_families_do_not_cross() {
        let mut types = TypeRegistry::new();
        let integer = i32_type(&mut types);
        let routine = callable(&mut types, CallableFlavor::Routine, integer);
        let routine_2 = callable(&mut types, CallableFlavor::Routine, integer);
        let method = callable(&mut types, CallableFlavor::Method, integer);
        let nested = callable(&mut types, CallableFlavor::Nested, integer);

        assert!(types.value_conversion(routine, routine_2).is_some());
        assert!(types.value_conversion(routine, method).is_none());
        assert!(types.value_conversion(method, routine).is_none());
        assert!(types.value_conversion(routine, nested).is_none());
        assert!(types.value_conversion(nested, routine).is_none());
    }

    #[test]
    fn class_subtyping_and_downcasts_are_type_owned_relations() {
        let mut types = TypeRegistry::new();
        let base = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            ClassType {
                aggregate: None,
                base: None,
                interfaces: Vec::new(),
                methods: Vec::new(),
                pointer_layout: StorageLayout {
                    size: 8,
                    alignment: 8,
                },
            },
        );
        let child = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            ClassType {
                aggregate: None,
                base: Some(base),
                interfaces: Vec::new(),
                methods: Vec::new(),
                pointer_layout: StorageLayout {
                    size: 8,
                    alignment: 8,
                },
            },
        );

        assert!(types.is_subtype(child, base));
        assert!(types.value_conversion(base, child).is_some());
        assert_eq!(
            types.predefined_explicit_conversion(child, base),
            Some(ExplicitConversion::RelatedDowncast {
                destination: child,
                source: base,
            })
        );
    }

    #[test]
    fn only_packed_record_offers_primitive_representation_overlay() {
        let mut types = TypeRegistry::new();
        let integer = i32_type(&mut types);
        let regular = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            RegularRecordType {
                aggregate: AggregateShape {
                    member_environment: env(),
                    fields: Vec::new(),
                },
                variant: None,
                layout: StorageLayout {
                    size: 4,
                    alignment: 4,
                },
            },
        );
        let packed = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            PackedRecordType {
                aggregate: AggregateShape {
                    member_environment: env(),
                    fields: Vec::new(),
                },
                variant: None,
                layout: StorageLayout {
                    size: 4,
                    alignment: 1,
                },
            },
        );

        assert!(
            types
                .predefined_explicit_conversion(regular, integer)
                .is_none()
        );
        assert!(matches!(
            types.predefined_explicit_conversion(packed, integer),
            Some(ExplicitConversion::RepresentationOverlay { .. })
        ));
    }

    #[test]
    fn strings_are_sequences_but_not_arrays() {
        let mut types = TypeRegistry::new();
        let integer = i32_type(&mut types);
        let character = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            PrimitiveType {
                kind: PrimitiveKind::Character,
                layout: StorageLayout {
                    size: 1,
                    alignment: 1,
                },
            },
        );
        let string = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            StringType {
                kind: StringKind::Ansi,
                element: character,
                index: integer,
                length: integer,
                layout: StorageLayout {
                    size: 8,
                    alignment: 8,
                },
            },
        );

        assert_eq!(types.sequence_element_type(string), Some(character));
        assert_eq!(types.array_element_type(string), None);
        assert!(types.sequence_is_resizable(string));
        assert!(types.has_managed_lifetime(string));
    }
}
