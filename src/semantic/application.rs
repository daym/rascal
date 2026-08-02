use super::{
    CallableFlavor, ConstantValue, ConversionResolution, ConversionResolver, ConversionSelection,
    ParameterMode, ReceiverId, ResolvedConversion, SemanticUse, SymbolId, TypeRef, TypeRegistry,
    ValueConversion,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationReceiver {
    None,
    Lookup(ReceiverId),
    Explicit,
    ImplicitSelf,
    ClassIdentifier(TypeRef),
    Inherited,
    StaticLink,
    CallableValue { lookup_receiver: Option<ReceiverId> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplicationCandidate {
    Routine {
        symbol: SymbolId,
        callable_type: TypeRef,
        receiver: ApplicationReceiver,
    },
    CallableValue {
        symbol: Option<SymbolId>,
        callable_type: TypeRef,
        receiver: ApplicationReceiver,
    },
    Conversion {
        destination: TypeRef,
    },
}

impl ApplicationCandidate {
    pub const fn callable_type(&self) -> Option<TypeRef> {
        match self {
            Self::Routine { callable_type, .. } | Self::CallableValue { callable_type, .. } => {
                Some(*callable_type)
            }
            Self::Conversion { .. } => None,
        }
    }

    pub const fn symbol(&self) -> Option<SymbolId> {
        match self {
            Self::Routine { symbol, .. } => Some(*symbol),
            Self::CallableValue { symbol, .. } => *symbol,
            Self::Conversion { .. } => None,
        }
    }

    pub const fn receiver(&self) -> ApplicationReceiver {
        match self {
            Self::Routine { receiver, .. } | Self::CallableValue { receiver, .. } => *receiver,
            Self::Conversion { .. } => ApplicationReceiver::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualArgument {
    pub ty: Option<TypeRef>,
    pub addressable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArgumentConversion {
    Implicit(ConversionResolution),
    Explicit(ConversionResolution),
    Storage(ValueConversion),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArgumentBinding {
    pub actual_index: usize,
    pub formal_index: usize,
    pub formal_type: TypeRef,
    pub required_use: SemanticUse,
    pub conversion: Option<ArgumentConversion>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefaultArgumentBinding {
    pub formal_index: usize,
    pub formal_type: TypeRef,
    pub value: ConstantValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CandidateRejection {
    NotCallable {
        callable_type: TypeRef,
    },
    MissingReceiver {
        callable_type: TypeRef,
    },
    UnexpectedReceiver {
        callable_type: TypeRef,
    },
    Arity {
        provided: usize,
        minimum: usize,
        maximum: usize,
    },
    MissingActualType {
        actual_index: usize,
    },
    ArgumentNotAddressable {
        actual_index: usize,
        mode: ParameterMode,
    },
    FormalContractMismatch {
        actual_index: usize,
        formal: TypeRef,
        actual: TypeRef,
    },
    NoImplicitConversion {
        actual_index: usize,
        destination: TypeRef,
        source: TypeRef,
    },
    NoExplicitConversion {
        destination: TypeRef,
        source: TypeRef,
    },
    AmbiguousConversion {
        destination: TypeRef,
        source: TypeRef,
        attempts: Vec<usize>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateAttempt {
    pub candidate: ApplicationCandidate,
    pub arguments: Vec<ArgumentBinding>,
    pub defaults: Vec<DefaultArgumentBinding>,
    pub result_type: Option<TypeRef>,
    pub rejections: Vec<CandidateRejection>,
}

impl CandidateAttempt {
    pub fn is_viable(&self) -> bool {
        self.rejections.is_empty()
    }

    fn explicit_ranks(&self) -> Vec<u8> {
        self.arguments
            .iter()
            .filter_map(|argument| match argument.conversion.as_ref()? {
                ArgumentConversion::Implicit(conversion) => conversion_rank_priority(conversion),
                ArgumentConversion::Explicit(conversion) => conversion_rank_priority(conversion),
                ArgumentConversion::Storage(conversion) => {
                    Some(super::conversion_rank_priority(conversion.rank))
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplicationSelection {
    Selected { attempt: usize },
    Ambiguous { attempts: Vec<usize> },
    NoViable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplicationResolution {
    pub attempts: Vec<CandidateAttempt>,
    pub selection: ApplicationSelection,
}

impl ApplicationResolution {
    pub fn selected_attempt(&self) -> Option<&CandidateAttempt> {
        let ApplicationSelection::Selected { attempt } = self.selection else {
            return None;
        };
        self.attempts.get(attempt)
    }

    pub fn selected_symbol(&self) -> Option<SymbolId> {
        self.selected_attempt()?.candidate.symbol()
    }

    pub fn result_type(&self) -> Option<TypeRef> {
        self.selected_attempt()?.result_type
    }

    pub fn is_ambiguous(&self) -> bool {
        matches!(self.selection, ApplicationSelection::Ambiguous { .. })
    }
}

pub struct ApplicationResolver<'a> {
    types: &'a TypeRegistry,
    untyped_parameter: TypeRef,
    conversions: &'a ConversionResolver<'a>,
}

impl<'a> ApplicationResolver<'a> {
    pub const fn new(
        types: &'a TypeRegistry,
        untyped_parameter: TypeRef,
        conversions: &'a ConversionResolver<'a>,
    ) -> Self {
        Self {
            types,
            untyped_parameter,
            conversions,
        }
    }

    pub fn resolve(
        &self,
        candidates: Vec<ApplicationCandidate>,
        actuals: &[ActualArgument],
    ) -> ApplicationResolution {
        let attempts = candidates
            .into_iter()
            .map(|candidate| self.attempt(candidate, actuals))
            .collect::<Vec<_>>();
        let viable = attempts
            .iter()
            .enumerate()
            .filter_map(|(index, attempt)| attempt.is_viable().then_some(index))
            .collect::<Vec<_>>();
        let selection = match viable.as_slice() {
            [] => ApplicationSelection::NoViable,
            [attempt] => ApplicationSelection::Selected { attempt: *attempt },
            _ => {
                let winners = viable
                    .iter()
                    .copied()
                    .filter(|candidate| {
                        viable.iter().copied().all(|other| {
                            candidate == &other
                                || dominates(&attempts[*candidate], &attempts[other])
                        })
                    })
                    .collect::<Vec<_>>();
                match winners.as_slice() {
                    [attempt] => ApplicationSelection::Selected { attempt: *attempt },
                    _ => ApplicationSelection::Ambiguous { attempts: viable },
                }
            }
        };
        ApplicationResolution {
            attempts,
            selection,
        }
    }

    fn attempt(
        &self,
        candidate: ApplicationCandidate,
        actuals: &[ActualArgument],
    ) -> CandidateAttempt {
        match candidate {
            ApplicationCandidate::Routine { callable_type, .. }
            | ApplicationCandidate::CallableValue { callable_type, .. } => {
                self.attempt_callable(candidate, callable_type, actuals)
            }
            ApplicationCandidate::Conversion { destination } => {
                self.attempt_conversion(candidate, destination, actuals)
            }
        }
    }

    fn attempt_callable(
        &self,
        candidate: ApplicationCandidate,
        callable_type: TypeRef,
        actuals: &[ActualArgument],
    ) -> CandidateAttempt {
        let mut rejections = Vec::new();
        let Some(callable) = self.types.callable(callable_type) else {
            return CandidateAttempt {
                candidate,
                arguments: Vec::new(),
                defaults: Vec::new(),
                result_type: None,
                rejections: vec![CandidateRejection::NotCallable { callable_type }],
            };
        };
        validate_receiver(&candidate, callable.flavor, callable_type, &mut rejections);
        let minimum = callable
            .signature
            .parameters
            .iter()
            .rposition(|parameter| parameter.default.is_none())
            .map_or(0, |index| index + 1);
        let maximum = callable.signature.parameters.len();
        if actuals.len() < minimum || actuals.len() > maximum {
            rejections.push(CandidateRejection::Arity {
                provided: actuals.len(),
                minimum,
                maximum,
            });
        }

        let mut arguments = Vec::new();
        for (actual_index, (formal, actual)) in callable
            .signature
            .parameters
            .iter()
            .zip(actuals)
            .enumerate()
        {
            let Some(actual_type) = actual.ty else {
                rejections.push(CandidateRejection::MissingActualType { actual_index });
                arguments.push(ArgumentBinding {
                    actual_index,
                    formal_index: actual_index,
                    formal_type: formal.ty,
                    required_use: formal_semantic_use(formal.mode),
                    conversion: None,
                });
                continue;
            };
            let conversion = if formal.ty == self.untyped_parameter {
                if !actual.addressable {
                    rejections.push(CandidateRejection::ArgumentNotAddressable {
                        actual_index,
                        mode: formal.mode,
                    });
                    None
                } else {
                    Some(ArgumentConversion::Storage(ValueConversion {
                        rank: super::ConversionRank::Compatible,
                        operation: super::ValueConversionOperation::UntypedStorage,
                        range_check: super::RangeCheck::None,
                    }))
                }
            } else {
                match formal.mode {
                    ParameterMode::Var | ParameterMode::Out => {
                        if !actual.addressable {
                            rejections.push(CandidateRejection::ArgumentNotAddressable {
                                actual_index,
                                mode: formal.mode,
                            });
                            None
                        } else if !self.types.same_formal_contract(formal.ty, actual_type) {
                            rejections.push(CandidateRejection::FormalContractMismatch {
                                actual_index,
                                formal: formal.ty,
                                actual: actual_type,
                            });
                            None
                        } else {
                            Some(ArgumentConversion::Storage(ValueConversion::identity()))
                        }
                    }
                    ParameterMode::Value | ParameterMode::Const | ParameterMode::ConstRef => {
                        let conversion = self.conversions.resolve_implicit(formal.ty, actual_type);
                        match &conversion.selection {
                            ConversionSelection::Selected { .. } => {
                                Some(ArgumentConversion::Implicit(conversion))
                            }
                            ConversionSelection::Ambiguous { attempts } => {
                                rejections.push(CandidateRejection::AmbiguousConversion {
                                    destination: formal.ty,
                                    source: actual_type,
                                    attempts: attempts.clone(),
                                });
                                None
                            }
                            ConversionSelection::NoViable => {
                                rejections.push(CandidateRejection::NoImplicitConversion {
                                    actual_index,
                                    destination: formal.ty,
                                    source: actual_type,
                                });
                                None
                            }
                        }
                    }
                }
            };
            arguments.push(ArgumentBinding {
                actual_index,
                formal_index: actual_index,
                formal_type: formal.ty,
                required_use: formal_semantic_use(formal.mode),
                conversion,
            });
        }
        let defaults = if actuals.len() <= maximum {
            callable.signature.parameters[actuals.len()..]
                .iter()
                .enumerate()
                .filter_map(|(offset, formal)| {
                    formal.default.clone().map(|value| DefaultArgumentBinding {
                        formal_index: actuals.len() + offset,
                        formal_type: formal.ty,
                        value,
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        CandidateAttempt {
            candidate,
            arguments,
            defaults,
            result_type: callable.signature.result,
            rejections,
        }
    }

    fn attempt_conversion(
        &self,
        candidate: ApplicationCandidate,
        destination: TypeRef,
        actuals: &[ActualArgument],
    ) -> CandidateAttempt {
        let mut rejections = Vec::new();
        let mut arguments = Vec::new();
        if actuals.len() != 1 {
            rejections.push(CandidateRejection::Arity {
                provided: actuals.len(),
                minimum: 1,
                maximum: 1,
            });
        }
        if let Some(actual) = actuals.first() {
            if let Some(source) = actual.ty {
                let conversion = self.conversions.resolve_explicit(destination, source);
                let conversion = match &conversion.selection {
                    ConversionSelection::Selected { .. } => {
                        Some(ArgumentConversion::Explicit(conversion))
                    }
                    ConversionSelection::Ambiguous { attempts } => {
                        rejections.push(CandidateRejection::AmbiguousConversion {
                            destination,
                            source,
                            attempts: attempts.clone(),
                        });
                        None
                    }
                    ConversionSelection::NoViable => {
                        rejections.push(CandidateRejection::NoExplicitConversion {
                            destination,
                            source,
                        });
                        None
                    }
                };
                arguments.push(ArgumentBinding {
                    actual_index: 0,
                    formal_index: 0,
                    formal_type: destination,
                    required_use: SemanticUse::Value,
                    conversion,
                });
            } else {
                rejections.push(CandidateRejection::MissingActualType { actual_index: 0 });
                arguments.push(ArgumentBinding {
                    actual_index: 0,
                    formal_index: 0,
                    formal_type: destination,
                    required_use: SemanticUse::Value,
                    conversion: None,
                });
            }
        }
        CandidateAttempt {
            candidate,
            arguments,
            defaults: Vec::new(),
            result_type: Some(destination),
            rejections,
        }
    }
}

fn validate_receiver(
    candidate: &ApplicationCandidate,
    flavor: CallableFlavor,
    callable_type: TypeRef,
    rejections: &mut Vec<CandidateRejection>,
) {
    let receiver = candidate.receiver();
    match candidate {
        ApplicationCandidate::CallableValue { .. } => {
            if !matches!(receiver, ApplicationReceiver::CallableValue { .. }) {
                rejections.push(CandidateRejection::MissingReceiver { callable_type });
            }
        }
        ApplicationCandidate::Routine { .. } => match flavor {
            CallableFlavor::Routine => {
                if receiver != ApplicationReceiver::None {
                    rejections.push(CandidateRejection::UnexpectedReceiver { callable_type });
                }
            }
            CallableFlavor::Method => {
                if !matches!(
                    receiver,
                    ApplicationReceiver::Lookup(_)
                        | ApplicationReceiver::Explicit
                        | ApplicationReceiver::ImplicitSelf
                        | ApplicationReceiver::Inherited
                ) {
                    rejections.push(CandidateRejection::MissingReceiver { callable_type });
                }
            }
            CallableFlavor::ClassMethod => {
                if !matches!(
                    receiver,
                    ApplicationReceiver::ClassIdentifier(_)
                        | ApplicationReceiver::Explicit
                        | ApplicationReceiver::ImplicitSelf
                        | ApplicationReceiver::Inherited
                ) {
                    rejections.push(CandidateRejection::MissingReceiver { callable_type });
                }
            }
            CallableFlavor::Nested => {
                if receiver != ApplicationReceiver::StaticLink {
                    rejections.push(CandidateRejection::MissingReceiver { callable_type });
                }
            }
        },
        ApplicationCandidate::Conversion { .. } => {}
    }
}

const fn formal_semantic_use(mode: ParameterMode) -> SemanticUse {
    match mode {
        ParameterMode::Var | ParameterMode::Out => SemanticUse::MutablePlace,
        ParameterMode::Value | ParameterMode::Const | ParameterMode::ConstRef => SemanticUse::Value,
    }
}

fn dominates(left: &CandidateAttempt, right: &CandidateAttempt) -> bool {
    let left = left.explicit_ranks();
    let right = right.explicit_ranks();
    left.len() == right.len()
        && left.iter().zip(&right).all(|(left, right)| left <= right)
        && left.iter().zip(&right).any(|(left, right)| left < right)
}

fn conversion_rank_priority(conversion: &ConversionResolution) -> Option<u8> {
    match conversion.selected()? {
        ResolvedConversion::Implicit(conversion) => {
            Some(super::conversion_rank_priority(conversion.rank))
        }
        ResolvedConversion::Explicit(super::ExplicitConversion::Value(conversion)) => {
            Some(super::conversion_rank_priority(conversion.rank))
        }
        ResolvedConversion::Explicit(
            super::ExplicitConversion::IntegerTruncate { .. }
            | super::ExplicitConversion::RealNarrow { .. }
            | super::ExplicitConversion::OrdinalCast { .. }
            | super::ExplicitConversion::PointerCrossing { .. }
            | super::ExplicitConversion::RelatedDowncast { .. }
            | super::ExplicitConversion::ProcedureAdapter { .. }
            | super::ExplicitConversion::ProcedurePointerCrossing { .. }
            | super::ExplicitConversion::RepresentationOverlay { .. }
            | super::ExplicitConversion::CustomOperator { .. },
        ) => Some(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{
        CallingConvention, EnvironmentRequirement, FormalParameter, RoutineOwner, RoutineSignature,
        TypeOwner,
    };

    fn callable(
        types: &mut TypeRegistry,
        parameter_types: &[(TypeRef, bool)],
        result: Option<TypeRef>,
    ) -> TypeRef {
        types.allocate_complete(
            TypeOwner::Builtin,
            None,
            super::super::EnvironmentId(0),
            super::super::CallableType {
                owner: RoutineOwner::Module,
                flavor: CallableFlavor::Routine,
                signature: RoutineSignature {
                    parameters: parameter_types
                        .iter()
                        .map(|(ty, has_default)| FormalParameter {
                            mode: ParameterMode::Value,
                            ty: *ty,
                            default: has_default.then_some(ConstantValue::Integer(0)),
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
                method: None,
                overload: false,
            },
        )
    }

    #[test]
    fn equal_viable_candidates_are_ambiguous_even_with_different_default_counts() {
        let mut types = TypeRegistry::new();
        let scopes = super::super::ScopeGraph::new();
        let scalar = types.allocate_complete(
            TypeOwner::Builtin,
            None,
            super::super::EnvironmentId(0),
            super::super::OpaqueType {
                layout: None,
                reference_type: false,
                managed_lifetime: false,
            },
        );
        let first = callable(&mut types, &[(scalar, false)], None);
        let second = callable(&mut types, &[(scalar, false), (scalar, true)], None);
        let conversions = ConversionResolver::new(
            &types,
            &scopes,
            scopes.current_environment(),
            crate::ModeSnapshot::default(),
        );
        let resolution = ApplicationResolver::new(&types, TypeRef(999), &conversions).resolve(
            vec![
                ApplicationCandidate::Routine {
                    symbol: SymbolId(0),
                    callable_type: first,
                    receiver: ApplicationReceiver::None,
                },
                ApplicationCandidate::Routine {
                    symbol: SymbolId(1),
                    callable_type: second,
                    receiver: ApplicationReceiver::None,
                },
            ],
            &[ActualArgument {
                ty: Some(scalar),
                addressable: false,
            }],
        );
        assert!(matches!(
            resolution.selection,
            ApplicationSelection::Ambiguous { .. }
        ));
        assert_eq!(resolution.attempts[1].defaults.len(), 1);
    }
}
