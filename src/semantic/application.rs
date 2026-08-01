use super::{
    CallableFlavor, ConstantValue, ExplicitConversion, ParameterMode, ReceiverId, SymbolId,
    TypeRef, TypeRegistry, ValueConversion,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationReceiver {
    None,
    Lookup(ReceiverId),
    Explicit,
    ImplicitSelf,
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
    Implicit(ValueConversion),
    Explicit(ExplicitConversion),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArgumentBinding {
    pub actual_index: usize,
    pub formal_index: usize,
    pub formal_type: TypeRef,
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
                ArgumentConversion::Implicit(conversion) => {
                    Some(conversion_rank_priority(conversion))
                }
                ArgumentConversion::Explicit(_) => Some(0),
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
}

impl<'a> ApplicationResolver<'a> {
    pub const fn new(types: &'a TypeRegistry, untyped_parameter: TypeRef) -> Self {
        Self {
            types,
            untyped_parameter,
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
                    Some(ValueConversion {
                        rank: super::ConversionRank::Compatible,
                        operation: super::ValueConversionOperation::UntypedStorage,
                        range_check: super::RangeCheck::None,
                    })
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
                            Some(ValueConversion::identity())
                        }
                    }
                    ParameterMode::Value | ParameterMode::Const | ParameterMode::ConstRef => {
                        let conversion = self.types.value_conversion(formal.ty, actual_type);
                        if conversion.is_none() {
                            rejections.push(CandidateRejection::NoImplicitConversion {
                                actual_index,
                                destination: formal.ty,
                                source: actual_type,
                            });
                        }
                        conversion
                    }
                }
            };
            arguments.push(ArgumentBinding {
                actual_index,
                formal_index: actual_index,
                formal_type: formal.ty,
                conversion: conversion.map(ArgumentConversion::Implicit),
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
                let conversion = self
                    .types
                    .predefined_explicit_conversion(destination, source);
                if conversion.is_none() {
                    rejections.push(CandidateRejection::NoExplicitConversion {
                        destination,
                        source,
                    });
                }
                arguments.push(ArgumentBinding {
                    actual_index: 0,
                    formal_index: 0,
                    formal_type: destination,
                    conversion: conversion.map(ArgumentConversion::Explicit),
                });
            } else {
                rejections.push(CandidateRejection::MissingActualType { actual_index: 0 });
                arguments.push(ArgumentBinding {
                    actual_index: 0,
                    formal_index: 0,
                    formal_type: destination,
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

fn dominates(left: &CandidateAttempt, right: &CandidateAttempt) -> bool {
    let left = left.explicit_ranks();
    let right = right.explicit_ranks();
    left.len() == right.len()
        && left.iter().zip(&right).all(|(left, right)| left <= right)
        && left.iter().zip(&right).any(|(left, right)| left < right)
}

fn conversion_rank_priority(conversion: &ValueConversion) -> u8 {
    match conversion.rank {
        super::ConversionRank::Exact => 0,
        super::ConversionRank::Subtype => 1,
        super::ConversionRank::Widening => 2,
        super::ConversionRank::Compatible => 3,
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
            },
        )
    }

    #[test]
    fn equal_viable_candidates_are_ambiguous_even_with_different_default_counts() {
        let mut types = TypeRegistry::new();
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
        let resolution = ApplicationResolver::new(&types, TypeRef(999)).resolve(
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
