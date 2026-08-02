use std::collections::BTreeSet;

use crate::{ModeSnapshot, explicit_operator_identifier, implicit_operator_identifier};

use super::{
    CallableFlavor, EnvironmentId, ExplicitConversion, LookupRequest, ScopeGraph, SymbolId,
    SymbolKind, TypeRef, TypeRegistry, ValueConversion, ValueConversionOperation,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversionMode {
    Implicit,
    Explicit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomConversionKind {
    Implicit,
    Explicit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversionCandidate {
    Predefined,
    Custom {
        symbol: SymbolId,
        callable_type: TypeRef,
        kind: CustomConversionKind,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversionRejection {
    NoPredefinedConversion,
    NotCallable,
    ReceiverRequired,
    Arity {
        actual: usize,
    },
    MissingResult,
    ResultTypeMismatch {
        declared: TypeRef,
        requested: TypeRef,
    },
    NoPredefinedInputConversion {
        formal: TypeRef,
        source: TypeRef,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedConversion {
    Implicit(ValueConversion),
    Explicit(ExplicitConversion),
}

impl ResolvedConversion {
    pub const fn implicit(&self) -> Option<&ValueConversion> {
        match self {
            Self::Implicit(conversion) => Some(conversion),
            Self::Explicit(_) => None,
        }
    }

    pub const fn explicit(&self) -> Option<&ExplicitConversion> {
        match self {
            Self::Implicit(_) => None,
            Self::Explicit(conversion) => Some(conversion),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversionAttempt {
    pub candidate: ConversionCandidate,
    pub result: Option<ResolvedConversion>,
    pub rejections: Vec<ConversionRejection>,
    preference: Option<(u8, u8)>,
}

impl ConversionAttempt {
    pub fn is_viable(&self) -> bool {
        self.rejections.is_empty() && self.result.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversionSelection {
    Selected { attempt: usize },
    Ambiguous { attempts: Vec<usize> },
    NoViable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversionResolution {
    pub mode: ConversionMode,
    pub modes: ModeSnapshot,
    pub source: TypeRef,
    pub destination: TypeRef,
    pub attempts: Vec<ConversionAttempt>,
    pub selection: ConversionSelection,
}

impl ConversionResolution {
    pub fn selected_attempt(&self) -> Option<&ConversionAttempt> {
        let ConversionSelection::Selected { attempt } = self.selection else {
            return None;
        };
        self.attempts.get(attempt)
    }

    pub fn selected(&self) -> Option<&ResolvedConversion> {
        self.selected_attempt()?.result.as_ref()
    }

    pub fn is_ambiguous(&self) -> bool {
        matches!(self.selection, ConversionSelection::Ambiguous { .. })
    }
}

pub struct ConversionResolver<'a> {
    types: &'a TypeRegistry,
    scopes: &'a ScopeGraph,
    environment: EnvironmentId,
    modes: ModeSnapshot,
}

#[derive(Clone, Copy)]
struct CustomProbe {
    kind: CustomConversionKind,
    requested_mode: ConversionMode,
    destination: TypeRef,
    source: TypeRef,
    tier: u8,
}

impl<'a> ConversionResolver<'a> {
    pub const fn new(
        types: &'a TypeRegistry,
        scopes: &'a ScopeGraph,
        environment: EnvironmentId,
        modes: ModeSnapshot,
    ) -> Self {
        Self {
            types,
            scopes,
            environment,
            modes,
        }
    }

    pub fn resolve_implicit(&self, destination: TypeRef, source: TypeRef) -> ConversionResolution {
        let mut attempts = vec![self.predefined_implicit(destination, source)];
        attempts.extend(self.custom_attempts(
            implicit_operator_identifier(self.modes.range_checks),
            CustomProbe {
                kind: CustomConversionKind::Implicit,
                requested_mode: ConversionMode::Implicit,
                destination,
                source,
                tier: 1,
            },
        ));
        finish_resolution(
            ConversionMode::Implicit,
            self.modes,
            source,
            destination,
            attempts,
        )
    }

    pub fn resolve_explicit(&self, destination: TypeRef, source: TypeRef) -> ConversionResolution {
        let mut attempts = self.custom_attempts(
            explicit_operator_identifier(),
            CustomProbe {
                kind: CustomConversionKind::Explicit,
                requested_mode: ConversionMode::Explicit,
                destination,
                source,
                tier: 0,
            },
        );
        attempts.extend(self.custom_attempts(
            implicit_operator_identifier(self.modes.range_checks),
            CustomProbe {
                kind: CustomConversionKind::Implicit,
                requested_mode: ConversionMode::Explicit,
                destination,
                source,
                tier: 1,
            },
        ));
        attempts.push(self.predefined_explicit(destination, source));
        finish_resolution(
            ConversionMode::Explicit,
            self.modes,
            source,
            destination,
            attempts,
        )
    }

    fn predefined_implicit(&self, destination: TypeRef, source: TypeRef) -> ConversionAttempt {
        match self.types.value_conversion(destination, source) {
            Some(conversion) => ConversionAttempt {
                preference: Some((0, conversion_rank_priority(conversion.rank))),
                candidate: ConversionCandidate::Predefined,
                result: Some(ResolvedConversion::Implicit(conversion)),
                rejections: Vec::new(),
            },
            None => ConversionAttempt {
                preference: None,
                candidate: ConversionCandidate::Predefined,
                result: None,
                rejections: vec![ConversionRejection::NoPredefinedConversion],
            },
        }
    }

    fn predefined_explicit(&self, destination: TypeRef, source: TypeRef) -> ConversionAttempt {
        match self
            .types
            .predefined_explicit_conversion(destination, source)
        {
            Some(conversion) => ConversionAttempt {
                preference: Some((2, 0)),
                candidate: ConversionCandidate::Predefined,
                result: Some(ResolvedConversion::Explicit(conversion)),
                rejections: Vec::new(),
            },
            None => ConversionAttempt {
                preference: None,
                candidate: ConversionCandidate::Predefined,
                result: None,
                rejections: vec![ConversionRejection::NoPredefinedConversion],
            },
        }
    }

    fn custom_attempts(&self, identifier: &str, probe: CustomProbe) -> Vec<ConversionAttempt> {
        let Some(name) = self.scopes.names().lookup(identifier) else {
            return Vec::new();
        };
        let Some(lookup) =
            self.scopes
                .lookup_symbol(self.environment, name, LookupRequest::ORDINARY)
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
                let SymbolKind::Routine(callable_type) = self.scopes.symbol(hit.symbol).kind else {
                    return None;
                };
                Some((hit.symbol, callable_type))
            })
            .map(|(symbol, callable_type)| self.custom_attempt(symbol, callable_type, probe))
            .collect()
    }

    fn custom_attempt(
        &self,
        symbol: SymbolId,
        callable_type: TypeRef,
        probe: CustomProbe,
    ) -> ConversionAttempt {
        let CustomProbe {
            kind,
            requested_mode,
            destination,
            source,
            tier,
        } = probe;
        let candidate = ConversionCandidate::Custom {
            symbol,
            callable_type,
            kind,
        };
        let Some(callable) = self.types.callable(callable_type) else {
            return ConversionAttempt {
                candidate,
                result: None,
                rejections: vec![ConversionRejection::NotCallable],
                preference: None,
            };
        };
        if matches!(
            callable.flavor,
            CallableFlavor::Method | CallableFlavor::ClassMethod
        ) {
            return ConversionAttempt {
                candidate,
                result: None,
                rejections: vec![ConversionRejection::ReceiverRequired],
                preference: None,
            };
        }
        if callable.signature.parameters.len() != 1 {
            return ConversionAttempt {
                candidate,
                result: None,
                rejections: vec![ConversionRejection::Arity {
                    actual: callable.signature.parameters.len(),
                }],
                preference: None,
            };
        }
        let Some(result_type) = callable.signature.result else {
            return ConversionAttempt {
                candidate,
                result: None,
                rejections: vec![ConversionRejection::MissingResult],
                preference: None,
            };
        };
        if !self.types.same_formal_contract(result_type, destination) {
            return ConversionAttempt {
                candidate,
                result: None,
                rejections: vec![ConversionRejection::ResultTypeMismatch {
                    declared: result_type,
                    requested: destination,
                }],
                preference: None,
            };
        }
        let formal = callable.signature.parameters[0].ty;
        let Some(input) = self.types.value_conversion(formal, source) else {
            return ConversionAttempt {
                candidate,
                result: None,
                rejections: vec![ConversionRejection::NoPredefinedInputConversion {
                    formal,
                    source,
                }],
                preference: None,
            };
        };
        let input_priority = conversion_rank_priority(input.rank);
        let custom_implicit = ValueConversion {
            rank: super::ConversionRank::Operator,
            operation: ValueConversionOperation::CustomOperator {
                symbol,
                callable_type,
                input: Box::new(input.clone()),
            },
            range_check: super::RangeCheck::None,
        };
        let result = match (requested_mode, kind) {
            (ConversionMode::Implicit, CustomConversionKind::Implicit) => {
                ResolvedConversion::Implicit(custom_implicit)
            }
            (ConversionMode::Explicit, CustomConversionKind::Implicit) => {
                ResolvedConversion::Explicit(ExplicitConversion::Value(custom_implicit))
            }
            (ConversionMode::Explicit, CustomConversionKind::Explicit) => {
                ResolvedConversion::Explicit(ExplicitConversion::CustomOperator {
                    symbol,
                    callable_type,
                    input,
                })
            }
            (ConversionMode::Implicit, CustomConversionKind::Explicit) => unreachable!(),
        };
        ConversionAttempt {
            candidate,
            result: Some(result),
            rejections: Vec::new(),
            preference: Some((tier, input_priority)),
        }
    }
}

fn finish_resolution(
    mode: ConversionMode,
    modes: ModeSnapshot,
    source: TypeRef,
    destination: TypeRef,
    attempts: Vec<ConversionAttempt>,
) -> ConversionResolution {
    let best = attempts
        .iter()
        .enumerate()
        .filter_map(|(index, attempt)| attempt.preference.map(|preference| (index, preference)))
        .min_by_key(|(_, preference)| *preference)
        .map(|(_, preference)| preference);
    let selection = match best {
        None => ConversionSelection::NoViable,
        Some(best) => {
            let winners = attempts
                .iter()
                .enumerate()
                .filter_map(|(index, attempt)| (attempt.preference == Some(best)).then_some(index))
                .collect::<Vec<_>>();
            match winners.as_slice() {
                [attempt] => ConversionSelection::Selected { attempt: *attempt },
                _ => ConversionSelection::Ambiguous { attempts: winners },
            }
        }
    };
    ConversionResolution {
        mode,
        modes,
        source,
        destination,
        attempts,
        selection,
    }
}

pub const fn conversion_rank_priority(rank: super::ConversionRank) -> u8 {
    match rank {
        super::ConversionRank::Exact => 0,
        super::ConversionRank::Subtype => 1,
        super::ConversionRank::Widening => 2,
        super::ConversionRank::Compatible => 3,
        super::ConversionRank::Operator => 4,
    }
}
