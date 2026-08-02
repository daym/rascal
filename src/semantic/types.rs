use std::{any::Any, cell::RefCell, collections::BTreeSet, fmt::Debug};

use super::{
    constants::ConstantValue,
    ids::{DeclId, EnvironmentId, NameId, NodeId, RegionId, StorageId, SymbolId, TypeRef},
    scope::SymbolCategory,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageLayout {
    pub size: u64,
    pub alignment: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrdinalDomain {
    pub lower: i128,
    pub upper: i128,
}

impl OrdinalDomain {
    pub const fn contains(self, value: i128) -> bool {
        value >= self.lower && value <= self.upper
    }

    pub fn cardinality(self) -> Option<u128> {
        let width = self.upper.checked_sub(self.lower)?;
        u128::try_from(width).ok()?.checked_add(1)
    }
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
    Operator,
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
    IntegerNarrow,
    IntegerToReal,
    RealWiden,
    RealToComp,
    BooleanNormalize,
    OrdinalCast,
    CharacterCast,
    ClassUpcast,
    InterfaceUpcast,
    NullValue,
    PointerErase,
    ObjectPointerUpcast,
    SetConvert,
    StringConvert,
    StringFromCharacter,
    StringFromPointer,
    StringBorrow,
    ArrayConvert,
    Callable,
    UntypedStorage,
    CustomOperator {
        symbol: SymbolId,
        callable_type: TypeRef,
        input: Box<ValueConversion>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueConversion {
    pub rank: ConversionRank,
    pub operation: ValueConversionOperation,
    pub range_check: RangeCheck,
}

fn null_conversion() -> ValueConversion {
    ValueConversion {
        rank: ConversionRank::Exact,
        operation: ValueConversionOperation::NullValue,
        range_check: RangeCheck::None,
    }
}

fn is_character_type(types: &TypeRegistry, ty: TypeRef) -> bool {
    matches!(
        types
            .implementation(types.canonical_type(ty))
            .and_then(|implementation| implementation.as_any().downcast_ref::<PrimitiveType>()),
        Some(PrimitiveType {
            kind: PrimitiveKind::Character | PrimitiveKind::WideCharacter { .. },
            ..
        })
    )
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
    RealNarrow {
        destination: TypeRef,
        source: TypeRef,
    },
    OrdinalCast {
        destination: TypeRef,
        source: TypeRef,
    },
    ProcedureAdapter {
        destination: TypeRef,
        source: TypeRef,
    },
    ProcedurePointerCrossing {
        destination: TypeRef,
        source: TypeRef,
    },
    CustomOperator {
        symbol: SymbolId,
        callable_type: TypeRef,
        input: ValueConversion,
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

    fn ordinal_domain(&self, _query: TypeQuery<'_>) -> Option<OrdinalDomain> {
        None
    }

    fn ordinal_base_type(&self, query: TypeQuery<'_>) -> Option<TypeRef> {
        self.ordinal_domain(query).map(|_| query.this)
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

    fn default_property(&self, _query: TypeQuery<'_>) -> Option<SymbolId> {
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
    NotEnumeration(TypeRef),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcedurePointerTargetAbi {
    pub function_pointer_object_pointer_round_trip: bool,
}

impl Default for ProcedurePointerTargetAbi {
    fn default() -> Self {
        Self {
            function_pointer_object_pointer_round_trip: matches!(
                std::env::consts::ARCH,
                "x86" | "x86_64" | "arm" | "aarch64" | "powerpc" | "powerpc64"
            ),
        }
    }
}

#[derive(Debug, Default)]
pub struct TypeRegistry {
    entries: Vec<TypeEntry>,
    subtype_active: RefCell<BTreeSet<(TypeRef, TypeRef)>>,
    explicit_conversion_active: RefCell<BTreeSet<(TypeRef, TypeRef)>>,
    procedure_pointer_abi: ProcedurePointerTargetAbi,
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

    pub const fn procedure_pointer_abi(&self) -> ProcedurePointerTargetAbi {
        self.procedure_pointer_abi
    }

    pub fn set_procedure_pointer_abi(&mut self, abi: ProcedurePointerTargetAbi) {
        self.procedure_pointer_abi = abi;
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
        if self.canonical_type(destination) == self.canonical_type(source) {
            return Some(ValueConversion::identity());
        }
        self.implementation(destination)?
            .value_conversion_from(self.query(destination), source)
    }

    pub fn canonical_type(&self, mut ty: TypeRef) -> TypeRef {
        let mut visited = BTreeSet::new();
        while visited.insert(ty) {
            let Some(alias) = self
                .implementation(ty)
                .and_then(|implementation| implementation.as_any().downcast_ref::<AliasType>())
            else {
                break;
            };
            if alias.nominal {
                break;
            }
            ty = alias.target;
        }
        ty
    }

    pub fn predefined_explicit_conversion(
        &self,
        destination: TypeRef,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        if self.canonical_type(destination) == self.canonical_type(source) {
            return Some(ExplicitConversion::Value(ValueConversion::identity()));
        }
        let implementation = self.implementation(destination)?;
        if !self
            .explicit_conversion_active
            .borrow_mut()
            .insert((destination, source))
        {
            return None;
        }
        let result =
            implementation.predefined_explicit_conversion_from(self.query(destination), source);
        self.explicit_conversion_active
            .borrow_mut()
            .remove(&(destination, source));
        result
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

    pub fn ordinal_domain(&self, ty: TypeRef) -> Option<OrdinalDomain> {
        self.implementation(ty)?.ordinal_domain(self.query(ty))
    }

    pub fn ordinal_base_type(&self, ty: TypeRef) -> Option<TypeRef> {
        self.implementation(ty)?.ordinal_base_type(self.query(ty))
    }

    pub fn sequence_element_type(&self, ty: TypeRef) -> Option<TypeRef> {
        self.implementation(ty)?
            .sequence_element_type(self.query(ty))
    }

    pub fn array_element_type(&self, ty: TypeRef) -> Option<TypeRef> {
        self.implementation(ty)?.array_element_type(self.query(ty))
    }

    pub fn set_element_type(&self, ty: TypeRef) -> Option<TypeRef> {
        self.implementation(ty)?
            .as_any()
            .downcast_ref::<SetType>()
            .map(|set| set.element)
    }

    pub fn variant_part(&self, ty: TypeRef) -> Option<&VariantPart> {
        let implementation = self.implementation(ty)?;
        if let Some(record) = implementation.as_any().downcast_ref::<RegularRecordType>() {
            return record.variant.as_ref();
        }
        implementation
            .as_any()
            .downcast_ref::<PackedRecordType>()
            .and_then(|record| record.variant.as_ref())
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

    pub fn default_property(&self, ty: TypeRef) -> Option<SymbolId> {
        self.implementation(ty)?.default_property(self.query(ty))
    }

    pub fn pointer_target(&self, ty: TypeRef) -> Option<TypeRef> {
        self.implementation(ty)?
            .as_any()
            .downcast_ref::<PointerType>()
            .map(|pointer| pointer.target)
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

    pub fn set_enum_members(
        &mut self,
        ty: TypeRef,
        members: Vec<EnumMember>,
        domain: OrdinalDomain,
    ) -> Result<(), TypeRegistryError> {
        let Some(enumeration) = self
            .implementation_mut(ty)
            .and_then(|implementation| implementation.as_any_mut().downcast_mut::<EnumType>())
        else {
            return Err(TypeRegistryError::NotEnumeration(ty));
        };
        enumeration.members = members;
        enumeration.domain = domain;
        Ok(())
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
    WideCharacter { bits: u16 },
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
        let source_type = query.types.canonical_type(source);
        let source_implementation = query.types.implementation(source_type)?;
        if let Some(source) = source_implementation
            .as_any()
            .downcast_ref::<PrimitiveType>()
        {
            return match (self.kind, source.kind) {
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
                (PrimitiveKind::Integer { .. }, PrimitiveKind::Integer { .. }) => {
                    Some(ValueConversion {
                        rank: ConversionRank::Compatible,
                        operation: ValueConversionOperation::IntegerNarrow,
                        range_check: RangeCheck::TargetPolicy,
                    })
                }
                (
                    PrimitiveKind::Real {
                        bits: destination_bits,
                    },
                    PrimitiveKind::Real { bits: source_bits },
                ) if destination_bits >= source_bits => Some(ValueConversion {
                    rank: ConversionRank::Widening,
                    operation: ValueConversionOperation::RealWiden,
                    range_check: RangeCheck::None,
                }),
                (PrimitiveKind::Real { .. }, PrimitiveKind::Integer { .. }) => {
                    Some(ValueConversion {
                        rank: ConversionRank::Widening,
                        operation: ValueConversionOperation::IntegerToReal,
                        range_check: RangeCheck::None,
                    })
                }
                _ => None,
            };
        }
        let subrange_base = source_implementation
            .as_any()
            .downcast_ref::<SubrangeType>()
            .map(|subrange| query.types.canonical_type(subrange.base));
        match (self.kind, subrange_base) {
            (PrimitiveKind::Integer { .. }, Some(base))
                if query
                    .types
                    .implementation(base)
                    .is_some_and(|implementation| {
                        matches!(
                            implementation.as_any().downcast_ref::<PrimitiveType>(),
                            Some(PrimitiveType {
                                kind: PrimitiveKind::Integer { .. },
                                ..
                            })
                        )
                    }) =>
            {
                let destination_domain = self.ordinal_domain(query)?;
                let source_domain = query.types.ordinal_domain(source)?;
                let widening = destination_domain.lower <= source_domain.lower
                    && destination_domain.upper >= source_domain.upper;
                Some(ValueConversion {
                    rank: if widening {
                        ConversionRank::Widening
                    } else {
                        ConversionRank::Compatible
                    },
                    operation: if widening {
                        ValueConversionOperation::IntegerWiden
                    } else {
                        ValueConversionOperation::IntegerNarrow
                    },
                    range_check: if widening {
                        RangeCheck::None
                    } else {
                        RangeCheck::TargetPolicy
                    },
                })
            }
            (PrimitiveKind::Real { .. }, Some(base))
                if query
                    .types
                    .implementation(base)
                    .is_some_and(|implementation| {
                        matches!(
                            implementation.as_any().downcast_ref::<PrimitiveType>(),
                            Some(PrimitiveType {
                                kind: PrimitiveKind::Integer { .. },
                                ..
                            })
                        )
                    }) =>
            {
                Some(ValueConversion {
                    rank: ConversionRank::Widening,
                    operation: ValueConversionOperation::IntegerToReal,
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
        if let Some(implicit) = query.types.value_conversion(query.this, source) {
            return Some(ExplicitConversion::Value(implicit));
        }
        let source_type = query.types.canonical_type(source);
        let source_implementation = query.types.implementation(source_type)?;
        let source_primitive = source_implementation
            .as_any()
            .downcast_ref::<PrimitiveType>()
            .map(|primitive| primitive.kind);
        match (self.kind, source_primitive) {
            (PrimitiveKind::Integer { .. }, Some(PrimitiveKind::Integer { .. })) => {
                Some(ExplicitConversion::IntegerTruncate {
                    destination: query.this,
                    source,
                })
            }
            (
                PrimitiveKind::Integer { .. },
                Some(
                    PrimitiveKind::Real { .. }
                    | PrimitiveKind::Boolean
                    | PrimitiveKind::Character
                    | PrimitiveKind::WideCharacter { .. },
                ),
            )
            | (
                PrimitiveKind::Boolean,
                Some(PrimitiveKind::Integer { .. } | PrimitiveKind::Boolean),
            )
            | (
                PrimitiveKind::Character,
                Some(PrimitiveKind::Integer { .. } | PrimitiveKind::Character),
            )
            | (
                PrimitiveKind::WideCharacter { .. },
                Some(PrimitiveKind::Integer { .. } | PrimitiveKind::WideCharacter { .. }),
            ) => Some(ExplicitConversion::OrdinalCast {
                destination: query.this,
                source,
            }),
            (
                PrimitiveKind::Real {
                    bits: destination_bits,
                },
                Some(PrimitiveKind::Real { bits: source_bits }),
            ) if destination_bits < source_bits => Some(ExplicitConversion::RealNarrow {
                destination: query.this,
                source,
            }),
            (PrimitiveKind::Real { .. }, Some(PrimitiveKind::Real { .. })) => {
                Some(ExplicitConversion::Value(ValueConversion {
                    rank: ConversionRank::Compatible,
                    operation: ValueConversionOperation::RealWiden,
                    range_check: RangeCheck::None,
                }))
            }
            (PrimitiveKind::Integer { .. }, None)
                if source_implementation.as_any().is::<EnumType>()
                    || source_implementation.as_any().is::<SubrangeType>() =>
            {
                Some(ExplicitConversion::OrdinalCast {
                    destination: query.this,
                    source,
                })
            }
            (PrimitiveKind::Character, None) | (PrimitiveKind::WideCharacter { .. }, None)
                if source_implementation.as_any().is::<SubrangeType>() =>
            {
                Some(ExplicitConversion::OrdinalCast {
                    destination: query.this,
                    source,
                })
            }
            (PrimitiveKind::Integer { .. }, None)
                if source_implementation.as_any().is::<PointerType>()
                    || source_implementation.as_any().is::<UntypedPointerType>()
                    || source_implementation.as_any().is::<ClassType>()
                    || source_implementation.as_any().is::<InterfaceType>()
                    || source_implementation.as_any().is::<MetaClassType>() =>
            {
                Some(ExplicitConversion::PointerCrossing {
                    destination: query.this,
                    source,
                })
            }
            _ => None,
        }
    }

    fn ordinal_domain(&self, _query: TypeQuery<'_>) -> Option<OrdinalDomain> {
        match self.kind {
            PrimitiveKind::Integer { bits, signed } => {
                if signed {
                    let magnitude = 1_i128.checked_shl(u32::from(bits.saturating_sub(1)))?;
                    Some(OrdinalDomain {
                        lower: -magnitude,
                        upper: magnitude - 1,
                    })
                } else {
                    let upper = 1_i128.checked_shl(u32::from(bits))?.checked_sub(1)?;
                    Some(OrdinalDomain { lower: 0, upper })
                }
            }
            PrimitiveKind::Boolean => Some(OrdinalDomain { lower: 0, upper: 1 }),
            PrimitiveKind::Character => Some(OrdinalDomain {
                lower: 0,
                upper: 255,
            }),
            PrimitiveKind::WideCharacter { bits } => {
                let upper = 1_i128.checked_shl(u32::from(bits))?.checked_sub(1)?;
                Some(OrdinalDomain { lower: 0, upper })
            }
            PrimitiveKind::Real { .. } => None,
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

    fn default_property(&self, query: TypeQuery<'_>) -> Option<SymbolId> {
        (!self.nominal)
            .then(|| query.types.default_property(self.target))
            .flatten()
    }

    fn ordinal_domain(&self, query: TypeQuery<'_>) -> Option<OrdinalDomain> {
        (!self.nominal)
            .then(|| query.types.ordinal_domain(self.target))
            .flatten()
    }

    fn ordinal_base_type(&self, query: TypeQuery<'_>) -> Option<TypeRef> {
        if self.nominal {
            self.ordinal_domain(query).map(|_| query.this)
        } else {
            query.types.ordinal_base_type(self.target)
        }
    }
}

#[derive(Clone, Debug)]
pub struct EnumMember {
    pub name: NameId,
    pub value: i128,
}

#[derive(Clone, Debug)]
pub struct EnumType {
    pub members: Vec<EnumMember>,
    pub domain: OrdinalDomain,
    pub layout: StorageLayout,
}

impl PascalType for EnumType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn storage_layout(&self, _query: TypeQuery<'_>) -> Option<StorageLayout> {
        Some(self.layout)
    }

    fn ordinal_domain(&self, _query: TypeQuery<'_>) -> Option<OrdinalDomain> {
        Some(self.domain)
    }

    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        if query.types.value_conversion(query.this, source).is_some() {
            return Some(ExplicitConversion::Value(ValueConversion::identity()));
        }
        let source = query.types.canonical_type(source);
        let implementation = query.types.implementation(source)?;
        let ordinal_source = matches!(
            implementation.as_any().downcast_ref::<PrimitiveType>(),
            Some(PrimitiveType {
                kind: PrimitiveKind::Integer { .. },
                ..
            })
        ) || implementation.as_any().is::<SubrangeType>();
        ordinal_source.then_some(ExplicitConversion::OrdinalCast {
            destination: query.this,
            source,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SubrangeType {
    pub base: TypeRef,
    pub domain: OrdinalDomain,
    pub layout: StorageLayout,
}

impl PascalType for SubrangeType {
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
        (query.types.ordinal_base_type(source)? == query.types.ordinal_base_type(self.base)?)
            .then_some(ValueConversion {
                rank: ConversionRank::Compatible,
                operation: ValueConversionOperation::IntegerWiden,
                range_check: RangeCheck::TargetPolicy,
            })
    }

    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        if let Some(conversion) = query.types.value_conversion(query.this, source) {
            return Some(ExplicitConversion::Value(conversion));
        }
        let source_implementation = query
            .types
            .implementation(query.types.canonical_type(source))?;
        matches!(
            source_implementation
                .as_any()
                .downcast_ref::<PrimitiveType>(),
            Some(PrimitiveType {
                kind: PrimitiveKind::Real { .. },
                ..
            })
        )
        .then_some(ExplicitConversion::OrdinalCast {
            destination: query.this,
            source,
        })
    }

    fn ordinal_domain(&self, _query: TypeQuery<'_>) -> Option<OrdinalDomain> {
        Some(self.domain)
    }

    fn ordinal_base_type(&self, query: TypeQuery<'_>) -> Option<TypeRef> {
        query.types.ordinal_base_type(self.base)
    }
}

#[derive(Clone, Debug)]
pub struct SetType {
    pub element: TypeRef,
    pub domain: OrdinalDomain,
    pub layout: StorageLayout,
}

impl PascalType for SetType {
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
        let source = query
            .types
            .implementation(query.types.canonical_type(source))?
            .as_any()
            .downcast_ref::<SetType>()?;
        let destination_root = query.types.ordinal_base_type(self.element)?;
        let source_root = query.types.ordinal_base_type(source.element)?;
        (destination_root == source_root
            && self.domain.lower <= source.domain.lower
            && self.domain.upper >= source.domain.upper)
            .then_some(ValueConversion {
                rank: ConversionRank::Compatible,
                operation: ValueConversionOperation::SetConvert,
                range_check: RangeCheck::TargetPolicy,
            })
    }

    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        if let Some(conversion) = query.types.value_conversion(query.this, source) {
            return Some(ExplicitConversion::Value(conversion));
        }
        let source = query
            .types
            .implementation(query.types.canonical_type(source))?
            .as_any()
            .downcast_ref::<SetType>()?;
        (query.types.ordinal_base_type(self.element)?
            == query.types.ordinal_base_type(source.element)?)
        .then_some(ExplicitConversion::Value(ValueConversion {
            rank: ConversionRank::Compatible,
            operation: ValueConversionOperation::SetConvert,
            range_check: RangeCheck::TargetPolicy,
        }))
    }
}

#[derive(Clone, Debug)]
pub struct UnitType;

impl PascalType for UnitType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn storage_layout(&self, _query: TypeQuery<'_>) -> Option<StorageLayout> {
        Some(StorageLayout {
            size: 0,
            alignment: 1,
        })
    }
}

#[derive(Clone, Debug)]
pub struct NilType;

impl PascalType for NilType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Clone, Debug)]
pub struct UntypedPointerType {
    pub layout: StorageLayout,
}

impl PascalType for UntypedPointerType {
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
        let source = query
            .types
            .implementation(query.types.canonical_type(source))?;
        if source.as_any().is::<NilType>() {
            return Some(null_conversion());
        }
        (source.as_any().is::<PointerType>()
            || source.as_any().is::<ClassType>()
            || source.as_any().is::<InterfaceType>()
            || source.as_any().is::<MetaClassType>())
        .then_some(ValueConversion {
            rank: ConversionRank::Compatible,
            operation: ValueConversionOperation::PointerErase,
            range_check: RangeCheck::None,
        })
    }

    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        if let Some(conversion) = query.types.value_conversion(query.this, source) {
            return Some(ExplicitConversion::Value(conversion));
        }
        let source_kind = query
            .types
            .implementation(query.types.canonical_type(source))?;
        if let Some(callable) = source_kind.as_any().downcast_ref::<CallableType>() {
            return (callable.flavor == CallableFlavor::Routine
                && query
                    .types
                    .procedure_pointer_abi()
                    .function_pointer_object_pointer_round_trip)
                .then_some(ExplicitConversion::ProcedurePointerCrossing {
                    destination: query.this,
                    source,
                });
        }
        matches!(
            source_kind.as_any().downcast_ref::<PrimitiveType>(),
            Some(PrimitiveType {
                kind: PrimitiveKind::Integer { .. },
                ..
            })
        )
        .then_some(ExplicitConversion::PointerCrossing {
            destination: query.this,
            source,
        })
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

    fn value_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ValueConversion> {
        let source_implementation = query
            .types
            .implementation(query.types.canonical_type(source))?;
        if source_implementation.as_any().is::<NilType>() {
            return Some(null_conversion());
        }
        if let Some(literal) = source_implementation
            .as_any()
            .downcast_ref::<StringLiteralType>()
        {
            return (query.types.canonical_type(literal.element)
                == query.types.canonical_type(self.target))
            .then_some(ValueConversion {
                rank: ConversionRank::Compatible,
                operation: ValueConversionOperation::StringBorrow,
                range_check: RangeCheck::None,
            });
        }
        if let Some(source_pointer) = source_implementation.as_any().downcast_ref::<PointerType>() {
            if query.types.canonical_type(source_pointer.target)
                == query.types.canonical_type(self.target)
            {
                return Some(ValueConversion::identity());
            }
            return (query.types.is_subtype(source_pointer.target, self.target)
                && query
                    .types
                    .implementation(query.types.canonical_type(self.target))
                    .is_some_and(|implementation| implementation.as_any().is::<ObjectType>()))
            .then_some(ValueConversion {
                rank: ConversionRank::Subtype,
                operation: ValueConversionOperation::ObjectPointerUpcast,
                range_check: RangeCheck::None,
            });
        }
        let source_array = source_implementation.as_any().downcast_ref::<ArrayType>()?;
        (is_character_type(query.types, self.target)
            && is_character_type(query.types, source_array.element)
            && source_array.layout.is_some()
            && query
                .types
                .ordinal_domain(source_array.index)
                .is_some_and(|domain| domain.lower == 0))
        .then_some(ValueConversion {
            rank: ConversionRank::Compatible,
            operation: ValueConversionOperation::StringBorrow,
            range_check: RangeCheck::None,
        })
    }

    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        if let Some(conversion) = query.types.value_conversion(query.this, source) {
            return Some(ExplicitConversion::Value(conversion));
        }
        let source_implementation = query
            .types
            .implementation(query.types.canonical_type(source))?;
        let crossing = source_implementation.as_any().is::<PointerType>()
            || source_implementation.as_any().is::<UntypedPointerType>()
            || matches!(
                source_implementation
                    .as_any()
                    .downcast_ref::<PrimitiveType>(),
                Some(PrimitiveType {
                    kind: PrimitiveKind::Integer { .. },
                    ..
                })
            );
        if crossing {
            return Some(ExplicitConversion::PointerCrossing {
                destination: query.this,
                source,
            });
        }
        let target_is_character = matches!(
            query
                .types
                .implementation(query.types.canonical_type(self.target))?
                .as_any()
                .downcast_ref::<PrimitiveType>(),
            Some(PrimitiveType {
                kind: PrimitiveKind::Character | PrimitiveKind::WideCharacter { .. },
                ..
            })
        );
        let source_is_long_string = source_implementation
            .as_any()
            .downcast_ref::<StringType>()
            .is_some_and(|string| {
                string.kind != StringKind::Short
                    && query.types.canonical_type(string.element)
                        == query.types.canonical_type(self.target)
            });
        (target_is_character && source_is_long_string).then_some(ExplicitConversion::Value(
            ValueConversion {
                rank: ConversionRank::Compatible,
                operation: ValueConversionOperation::StringBorrow,
                range_check: RangeCheck::None,
            },
        ))
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

    fn default_property(&self, query: TypeQuery<'_>) -> Option<SymbolId> {
        self.aggregate
            .as_ref()
            .and_then(|aggregate| aggregate.default_property)
            .or_else(|| {
                self.base
                    .and_then(|base| query.types.default_property(base))
            })
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
        if query
            .types
            .implementation(query.types.canonical_type(source))
            .is_some_and(|implementation| implementation.as_any().is::<NilType>())
        {
            Some(null_conversion())
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
        if query
            .types
            .implementation(query.types.canonical_type(source))
            .is_some_and(|implementation| implementation.as_any().is::<UntypedPointerType>())
        {
            return Some(ExplicitConversion::PointerCrossing {
                destination: query.this,
                source,
            });
        }
        if query
            .types
            .implementation(query.types.canonical_type(source))
            .is_some_and(|implementation| implementation.as_any().is::<InterfaceType>())
        {
            return Some(ExplicitConversion::RelatedDowncast {
                destination: query.this,
                source,
            });
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

#[derive(Clone, Debug)]
pub struct InterfaceType {
    pub aggregate: Option<AggregateShape>,
    pub bases: Vec<TypeRef>,
    pub pointer_layout: StorageLayout,
}

impl PascalType for InterfaceType {
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

    fn has_managed_lifetime(&self, _query: TypeQuery<'_>) -> bool {
        true
    }

    fn member_environment(&self, _query: TypeQuery<'_>) -> Option<EnvironmentId> {
        self.aggregate
            .as_ref()
            .map(|aggregate| aggregate.member_environment)
    }

    fn default_property(&self, query: TypeQuery<'_>) -> Option<SymbolId> {
        self.aggregate
            .as_ref()
            .and_then(|aggregate| aggregate.default_property)
            .or_else(|| {
                self.bases
                    .iter()
                    .find_map(|base| query.types.default_property(*base))
            })
    }

    fn is_subtype_of(&self, query: TypeQuery<'_>, target: TypeRef) -> bool {
        query.this == target
            || self
                .bases
                .iter()
                .any(|base| query.types.is_subtype(*base, target))
    }

    fn value_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ValueConversion> {
        let source_implementation = query
            .types
            .implementation(query.types.canonical_type(source))?;
        if source_implementation.as_any().is::<NilType>() {
            return Some(null_conversion());
        }
        if query.types.is_subtype(source, query.this) {
            return Some(ValueConversion {
                rank: ConversionRank::Subtype,
                operation: ValueConversionOperation::InterfaceUpcast,
                range_check: RangeCheck::None,
            });
        }
        let source_class = source_implementation.as_any().downcast_ref::<ClassType>()?;
        source_class
            .interfaces
            .iter()
            .any(|interface| query.types.is_subtype(*interface, query.this))
            .then_some(ValueConversion {
                rank: ConversionRank::Subtype,
                operation: ValueConversionOperation::InterfaceUpcast,
                range_check: RangeCheck::None,
            })
    }

    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        if let Some(conversion) = query.types.value_conversion(query.this, source) {
            return Some(ExplicitConversion::Value(conversion));
        }
        let source_implementation = query
            .types
            .implementation(query.types.canonical_type(source))?;
        (source_implementation.as_any().is::<InterfaceType>()
            || source_implementation.as_any().is::<ClassType>()
            || source_implementation.as_any().is::<UntypedPointerType>())
        .then_some(ExplicitConversion::RelatedDowncast {
            destination: query.this,
            source,
        })
    }
}

#[derive(Clone, Debug)]
pub struct MetaClassType {
    pub instance: TypeRef,
    pub pointer_layout: StorageLayout,
}

impl PascalType for MetaClassType {
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

    fn value_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ValueConversion> {
        let source_implementation = query
            .types
            .implementation(query.types.canonical_type(source))?;
        if source_implementation.as_any().is::<NilType>() {
            return Some(null_conversion());
        }
        let source = source_implementation
            .as_any()
            .downcast_ref::<MetaClassType>()?;
        query
            .types
            .is_subtype(source.instance, self.instance)
            .then_some(ValueConversion {
                rank: ConversionRank::Subtype,
                operation: ValueConversionOperation::ClassUpcast,
                range_check: RangeCheck::None,
            })
    }

    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        if let Some(conversion) = query.types.value_conversion(query.this, source) {
            return Some(ExplicitConversion::Value(conversion));
        }
        let source_implementation = query
            .types
            .implementation(query.types.canonical_type(source))?;
        (source_implementation.as_any().is::<MetaClassType>()
            || source_implementation.as_any().is::<UntypedPointerType>())
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
    pub default: Option<ConstantValue>,
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
    Module,
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

    fn explicit_adapter_compatible(&self, query: TypeQuery<'_>, source: &Self) -> bool {
        if self.flavor != source.flavor
            || self.flavor == CallableFlavor::Nested
            || self.signature.calling_convention != source.signature.calling_convention
            || self.signature.parameters.len() != source.signature.parameters.len()
        {
            return false;
        }
        let formals_match = self
            .signature
            .parameters
            .iter()
            .zip(&source.signature.parameters)
            .all(|(target, source)| match (target.mode, source.mode) {
                (
                    ParameterMode::Var | ParameterMode::Out | ParameterMode::ConstRef,
                    source_mode,
                ) => {
                    target.mode == source_mode
                        && query.types.same_formal_contract(target.ty, source.ty)
                }
                (
                    ParameterMode::Value | ParameterMode::Const,
                    ParameterMode::Value | ParameterMode::Const,
                ) => query
                    .types
                    .predefined_explicit_conversion(source.ty, target.ty)
                    .is_some(),
                _ => false,
            });
        if !formals_match {
            return false;
        }
        match (self.signature.result, source.signature.result) {
            (None, None) => true,
            (Some(target), Some(source)) => query
                .types
                .predefined_explicit_conversion(target, source)
                .is_some(),
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
        if self.flavor == CallableFlavor::Nested {
            return None;
        }
        if query
            .types
            .implementation(query.types.canonical_type(source))
            .is_some_and(|implementation| implementation.as_any().is::<NilType>())
        {
            return Some(null_conversion());
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

    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        if let Some(conversion) = query.types.value_conversion(query.this, source) {
            return Some(ExplicitConversion::Value(conversion));
        }
        if self.flavor == CallableFlavor::Method
            && query
                .types
                .implementation(query.types.canonical_type(source))
                .is_some_and(|implementation| implementation.as_any().is::<RawMethodType>())
        {
            return Some(ExplicitConversion::ProcedureAdapter {
                destination: query.this,
                source,
            });
        }
        if self.flavor == CallableFlavor::Routine
            && query
                .types
                .implementation(query.types.canonical_type(source))
                .is_some_and(|implementation| implementation.as_any().is::<UntypedPointerType>())
            && query
                .types
                .procedure_pointer_abi()
                .function_pointer_object_pointer_round_trip
        {
            return Some(ExplicitConversion::ProcedurePointerCrossing {
                destination: query.this,
                source,
            });
        }
        let source_ref = source;
        let source = query.types.callable(source_ref)?;
        self.explicit_adapter_compatible(query, source).then_some(
            ExplicitConversion::ProcedureAdapter {
                destination: query.this,
                source: source_ref,
            },
        )
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

#[derive(Clone, Debug)]
pub struct RawMethodType {
    pub layout: StorageLayout,
}

impl PascalType for RawMethodType {
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
        let callable = query.types.callable(source)?;
        (callable.flavor == CallableFlavor::Method).then_some(
            ExplicitConversion::ProcedureAdapter {
                destination: query.this,
                source,
            },
        )
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
    pub default_property: Option<SymbolId>,
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

    fn default_property(&self, _query: TypeQuery<'_>) -> Option<SymbolId> {
        self.aggregate.default_property
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

    fn default_property(&self, _query: TypeQuery<'_>) -> Option<SymbolId> {
        self.aggregate.default_property
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

    fn default_property(&self, query: TypeQuery<'_>) -> Option<SymbolId> {
        self.aggregate.default_property.or_else(|| {
            self.base
                .and_then(|base| query.types.default_property(base))
        })
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
    Utf8,
    Wide,
    Unicode,
}

#[derive(Clone, Debug)]
pub struct StringLiteralType {
    pub element: TypeRef,
    pub index: TypeRef,
    pub length: TypeRef,
    pub character_count: u32,
}

impl PascalType for StringLiteralType {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
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
}

#[derive(Clone, Debug)]
pub struct StringType {
    pub kind: StringKind,
    pub capacity: Option<u32>,
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

    fn value_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ValueConversion> {
        let source_implementation = query
            .types
            .implementation(query.types.canonical_type(source))?;
        if let Some(source) = source_implementation.as_any().downcast_ref::<StringType>() {
            let rank = if source.kind == self.kind
                && source.capacity == self.capacity
                && query.types.canonical_type(source.element)
                    == query.types.canonical_type(self.element)
            {
                ConversionRank::Exact
            } else {
                ConversionRank::Compatible
            };
            return Some(ValueConversion {
                rank,
                operation: if rank == ConversionRank::Exact {
                    ValueConversionOperation::Identity
                } else {
                    ValueConversionOperation::StringConvert
                },
                range_check: if self.kind == StringKind::Short {
                    RangeCheck::TargetPolicy
                } else {
                    RangeCheck::None
                },
            });
        }
        if let Some(source) = source_implementation
            .as_any()
            .downcast_ref::<StringLiteralType>()
        {
            return (query.types.canonical_type(source.element)
                == query.types.canonical_type(self.element))
            .then_some(ValueConversion {
                rank: ConversionRank::Compatible,
                operation: ValueConversionOperation::StringConvert,
                range_check: if self.kind == StringKind::Short {
                    RangeCheck::TargetPolicy
                } else {
                    RangeCheck::None
                },
            });
        }
        if matches!(
            source_implementation
                .as_any()
                .downcast_ref::<PrimitiveType>(),
            Some(PrimitiveType {
                kind: PrimitiveKind::Character | PrimitiveKind::WideCharacter { .. },
                ..
            })
        ) && query.types.canonical_type(source) == query.types.canonical_type(self.element)
        {
            return Some(ValueConversion {
                rank: ConversionRank::Compatible,
                operation: ValueConversionOperation::StringFromCharacter,
                range_check: RangeCheck::None,
            });
        }
        if let Some(pointer) = source_implementation.as_any().downcast_ref::<PointerType>()
            && query.types.canonical_type(pointer.target)
                == query.types.canonical_type(self.element)
        {
            return Some(ValueConversion {
                rank: ConversionRank::Compatible,
                operation: ValueConversionOperation::StringFromPointer,
                range_check: RangeCheck::None,
            });
        }
        if let Some(array) = source_implementation.as_any().downcast_ref::<ArrayType>()
            && query.types.canonical_type(array.element) == query.types.canonical_type(self.element)
        {
            return Some(ValueConversion {
                rank: ConversionRank::Compatible,
                operation: ValueConversionOperation::StringConvert,
                range_check: RangeCheck::None,
            });
        }
        None
    }

    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        query
            .types
            .value_conversion(query.this, source)
            .map(ExplicitConversion::Value)
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

    fn value_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ValueConversion> {
        let source_implementation = query
            .types
            .implementation(query.types.canonical_type(source))?;
        if self.resizable && source_implementation.as_any().is::<NilType>() {
            return Some(null_conversion());
        }
        let source = source_implementation.as_any().downcast_ref::<ArrayType>()?;
        if self.resizable
            && !source.open
            && source.layout.is_some()
            && query
                .types
                .value_conversion(self.element, source.element)
                .is_some()
        {
            return Some(ValueConversion {
                rank: ConversionRank::Compatible,
                operation: ValueConversionOperation::ArrayConvert,
                range_check: RangeCheck::None,
            });
        }
        None
    }

    fn predefined_explicit_conversion_from(
        &self,
        query: TypeQuery<'_>,
        source: TypeRef,
    ) -> Option<ExplicitConversion> {
        query
            .types
            .value_conversion(query.this, source)
            .map(ExplicitConversion::Value)
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
                owner: RoutineOwner::Module,
                flavor,
                signature: RoutineSignature {
                    parameters: vec![FormalParameter {
                        mode: ParameterMode::Value,
                        ty: parameter,
                        default: None,
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
                    default_property: None,
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
                    default_property: None,
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
                capacity: None,
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

    fn primitive(types: &mut TypeRegistry, kind: PrimitiveKind, size: u64) -> TypeRef {
        types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            PrimitiveType {
                kind,
                layout: StorageLayout {
                    size,
                    alignment: u32::try_from(size).unwrap_or(1).max(1),
                },
            },
        )
    }

    fn nil(types: &mut TypeRegistry) -> TypeRef {
        types.allocate_complete(TypeOwner::Builtin, None, env(), NilType)
    }

    #[test]
    fn ordinal_real_boolean_and_character_matrix_is_closed() {
        let mut types = TypeRegistry::new();
        let i32_ty = primitive(
            &mut types,
            PrimitiveKind::Integer {
                bits: 32,
                signed: true,
            },
            4,
        );
        let u64_ty = primitive(
            &mut types,
            PrimitiveKind::Integer {
                bits: 64,
                signed: false,
            },
            8,
        );
        let real32 = primitive(&mut types, PrimitiveKind::Real { bits: 32 }, 4);
        let real64 = primitive(&mut types, PrimitiveKind::Real { bits: 64 }, 8);
        let boolean = primitive(&mut types, PrimitiveKind::Boolean, 1);
        let character = primitive(&mut types, PrimitiveKind::Character, 1);
        let enumeration = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            EnumType {
                members: Vec::new(),
                domain: OrdinalDomain { lower: 0, upper: 2 },
                layout: StorageLayout {
                    size: 1,
                    alignment: 1,
                },
            },
        );

        assert!(matches!(
            types.value_conversion(i32_ty, u64_ty),
            Some(ValueConversion {
                operation: ValueConversionOperation::IntegerNarrow,
                range_check: RangeCheck::TargetPolicy,
                ..
            })
        ));
        assert!(types.value_conversion(real64, i32_ty).is_some());
        assert!(types.value_conversion(real64, real32).is_some());
        assert!(types.value_conversion(real32, real64).is_none());
        assert!(matches!(
            types.predefined_explicit_conversion(real32, real64),
            Some(ExplicitConversion::RealNarrow { .. })
        ));

        for ordinal in [boolean, character, enumeration] {
            assert!(types.value_conversion(i32_ty, ordinal).is_none());
            assert!(matches!(
                types.predefined_explicit_conversion(i32_ty, ordinal),
                Some(ExplicitConversion::OrdinalCast { .. })
            ));
        }
        assert!(types.value_conversion(boolean, i32_ty).is_none());
        assert!(types.value_conversion(character, i32_ty).is_none());
        assert!(matches!(
            types.predefined_explicit_conversion(boolean, i32_ty),
            Some(ExplicitConversion::OrdinalCast { .. })
        ));
        assert!(matches!(
            types.predefined_explicit_conversion(character, i32_ty),
            Some(ExplicitConversion::OrdinalCast { .. })
        ));
    }

    #[test]
    fn string_character_pointer_and_array_matrix_is_closed() {
        let mut types = TypeRegistry::new();
        let integer = i32_type(&mut types);
        let character = primitive(&mut types, PrimitiveKind::Character, 1);
        let short = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            StringType {
                kind: StringKind::Short,
                capacity: Some(31),
                element: character,
                index: integer,
                length: integer,
                layout: StorageLayout {
                    size: 32,
                    alignment: 1,
                },
            },
        );
        let ansi = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            StringType {
                kind: StringKind::Ansi,
                capacity: None,
                element: character,
                index: integer,
                length: integer,
                layout: StorageLayout {
                    size: 8,
                    alignment: 8,
                },
            },
        );
        let pchar = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            PointerType {
                target: character,
                layout: StorageLayout {
                    size: 8,
                    alignment: 8,
                },
            },
        );
        let zero_based = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            SubrangeType {
                base: integer,
                domain: OrdinalDomain { lower: 0, upper: 7 },
                layout: StorageLayout {
                    size: 4,
                    alignment: 4,
                },
            },
        );
        let char_array = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            ArrayType {
                element: character,
                index: zero_based,
                length: integer,
                layout: Some(StorageLayout {
                    size: 8,
                    alignment: 1,
                }),
                resizable: false,
                open: false,
            },
        );
        let literal = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            StringLiteralType {
                element: character,
                index: integer,
                length: integer,
                character_count: 3,
            },
        );

        assert!(types.value_conversion(short, character).is_some());
        assert!(types.value_conversion(short, literal).is_some());
        assert!(types.value_conversion(ansi, short).is_some());
        assert!(types.value_conversion(short, pchar).is_some());
        assert!(types.value_conversion(short, char_array).is_some());
        assert!(types.value_conversion(pchar, char_array).is_some());
        assert!(types.value_conversion(pchar, literal).is_some());
        assert!(types.value_conversion(pchar, short).is_none());
        assert!(types.value_conversion(pchar, character).is_none());
        assert!(types.predefined_explicit_conversion(pchar, short).is_none());
        assert!(matches!(
            types.predefined_explicit_conversion(pchar, ansi),
            Some(ExplicitConversion::Value(ValueConversion {
                operation: ValueConversionOperation::StringBorrow,
                ..
            }))
        ));
        assert!(types.has_managed_lifetime(ansi));
        assert!(!types.has_managed_lifetime(short));
    }

    #[test]
    fn set_pointer_reference_array_and_nil_matrix_is_closed() {
        let mut types = TypeRegistry::new();
        let integer = i32_type(&mut types);
        let nil = nil(&mut types);
        let narrow = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            SubrangeType {
                base: integer,
                domain: OrdinalDomain { lower: 0, upper: 7 },
                layout: StorageLayout {
                    size: 4,
                    alignment: 4,
                },
            },
        );
        let wide = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            SubrangeType {
                base: integer,
                domain: OrdinalDomain {
                    lower: 0,
                    upper: 31,
                },
                layout: StorageLayout {
                    size: 4,
                    alignment: 4,
                },
            },
        );
        let small_set = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            SetType {
                element: narrow,
                domain: OrdinalDomain { lower: 0, upper: 7 },
                layout: StorageLayout {
                    size: 1,
                    alignment: 1,
                },
            },
        );
        let large_set = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            SetType {
                element: wide,
                domain: OrdinalDomain {
                    lower: 0,
                    upper: 31,
                },
                layout: StorageLayout {
                    size: 4,
                    alignment: 1,
                },
            },
        );
        assert!(types.value_conversion(large_set, small_set).is_some());
        assert!(types.value_conversion(small_set, large_set).is_none());
        assert!(
            types
                .predefined_explicit_conversion(small_set, large_set)
                .is_some()
        );

        let fixed = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            ArrayType {
                element: integer,
                index: narrow,
                length: integer,
                layout: Some(StorageLayout {
                    size: 32,
                    alignment: 4,
                }),
                resizable: false,
                open: false,
            },
        );
        let dynamic = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            ArrayType {
                element: integer,
                index: integer,
                length: integer,
                layout: None,
                resizable: true,
                open: false,
            },
        );
        assert!(types.value_conversion(dynamic, fixed).is_some());
        assert!(types.value_conversion(dynamic, nil).is_some());
        assert!(types.value_conversion(fixed, nil).is_none());
        assert!(types.has_managed_lifetime(dynamic));

        let interface = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            InterfaceType {
                aggregate: None,
                bases: Vec::new(),
                pointer_layout: StorageLayout {
                    size: 8,
                    alignment: 8,
                },
            },
        );
        let base = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            ClassType {
                aggregate: None,
                base: None,
                interfaces: vec![interface],
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
        let untyped = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            UntypedPointerType {
                layout: StorageLayout {
                    size: 8,
                    alignment: 8,
                },
            },
        );
        let typed = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            PointerType {
                target: integer,
                layout: StorageLayout {
                    size: 8,
                    alignment: 8,
                },
            },
        );
        assert!(types.value_conversion(base, child).is_some());
        assert!(types.value_conversion(interface, child).is_some());
        assert!(types.value_conversion(interface, nil).is_some());
        assert!(types.has_managed_lifetime(interface));
        assert!(types.value_conversion(typed, nil).is_some());
        assert!(types.value_conversion(untyped, typed).is_some());
        assert!(types.value_conversion(typed, untyped).is_none());
        assert!(matches!(
            types.predefined_explicit_conversion(typed, untyped),
            Some(ExplicitConversion::PointerCrossing { .. })
        ));
        assert!(matches!(
            types.predefined_explicit_conversion(child, interface),
            Some(ExplicitConversion::RelatedDowncast { .. })
        ));
    }

    #[test]
    fn procedural_matrix_separates_plain_method_nested_and_raw_method() {
        let mut types = TypeRegistry::new();
        let integer = i32_type(&mut types);
        let nil = nil(&mut types);
        let plain = callable(&mut types, CallableFlavor::Routine, integer);
        let plain_same = callable(&mut types, CallableFlavor::Routine, integer);
        let method = callable(&mut types, CallableFlavor::Method, integer);
        let nested = callable(&mut types, CallableFlavor::Nested, integer);
        let raw_method = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            RawMethodType {
                layout: StorageLayout {
                    size: 16,
                    alignment: 8,
                },
            },
        );
        let untyped_pointer = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            env(),
            UntypedPointerType {
                layout: StorageLayout {
                    size: 8,
                    alignment: 8,
                },
            },
        );

        assert!(types.value_conversion(plain, plain_same).is_some());
        assert!(types.value_conversion(plain, nil).is_some());
        assert!(types.value_conversion(method, nil).is_some());
        assert!(types.value_conversion(nested, nil).is_none());
        assert!(types.value_conversion(plain, method).is_none());
        assert!(types.value_conversion(plain, nested).is_none());
        assert!(matches!(
            types.predefined_explicit_conversion(raw_method, method),
            Some(ExplicitConversion::ProcedureAdapter { .. })
        ));
        assert!(matches!(
            types.predefined_explicit_conversion(method, raw_method),
            Some(ExplicitConversion::ProcedureAdapter { .. })
        ));
        assert!(matches!(
            types.predefined_explicit_conversion(untyped_pointer, plain),
            Some(ExplicitConversion::ProcedurePointerCrossing { .. })
        ));
        assert!(matches!(
            types.predefined_explicit_conversion(plain, untyped_pointer),
            Some(ExplicitConversion::ProcedurePointerCrossing { .. })
        ));
        assert!(
            types
                .predefined_explicit_conversion(untyped_pointer, method)
                .is_none()
        );
        types.set_procedure_pointer_abi(ProcedurePointerTargetAbi {
            function_pointer_object_pointer_round_trip: false,
        });
        assert!(
            types
                .predefined_explicit_conversion(untyped_pointer, plain)
                .is_none()
        );
        assert!(
            types
                .predefined_explicit_conversion(plain, untyped_pointer)
                .is_none()
        );
    }
}
