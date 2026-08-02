use std::collections::{BTreeMap, BTreeSet};

use crate::{Literal, ModeSnapshot, Operator};

use super::{
    ApplicationCandidate, ApplicationSelection, ArgumentConversion, BoundApplicationTarget,
    BoundExpression, BoundExpressionKind, BoundSetElement, BuiltinInstantiation, BuiltinOperation,
    ExplicitConversion, NumericOperation, OrdinalOperation, ResolvedConversion, SymbolId, TypeRef,
    TypeRegistry, ValueConversionOperation,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstantValue {
    Integer(i128),
    Boolean(bool),
    Character(char),
    String(String),
    Real(String),
    Nil,
    Set(BTreeSet<i128>),
}

impl ConstantValue {
    pub const fn ordinal(&self) -> Option<i128> {
        match self {
            Self::Integer(value) => Some(*value),
            Self::Boolean(value) => Some(*value as i128),
            Self::Character(value) => Some(*value as i128),
            Self::String(_) | Self::Real(_) | Self::Nil | Self::Set(_) => None,
        }
    }

    fn ordinal_variant(&self, value: i128) -> Option<Self> {
        match self {
            Self::Integer(_) => Some(Self::Integer(value)),
            Self::Boolean(_) => match value {
                0 => Some(Self::Boolean(false)),
                1 => Some(Self::Boolean(true)),
                _ => None,
            },
            Self::Character(_) => u32::try_from(value)
                .ok()
                .and_then(char::from_u32)
                .map(Self::Character),
            Self::String(_) | Self::Real(_) | Self::Nil | Self::Set(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstantEntry {
    pub ty: TypeRef,
    pub value: ConstantValue,
}

#[derive(Clone, Debug, Default)]
pub struct ConstantRegistry {
    entries: BTreeMap<SymbolId, ConstantEntry>,
}

impl ConstantRegistry {
    pub fn insert(&mut self, symbol: SymbolId, entry: ConstantEntry) {
        self.entries.insert(symbol, entry);
    }

    pub fn get(&self, symbol: SymbolId) -> Option<&ConstantEntry> {
        self.entries.get(&symbol)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstantEvaluationError {
    NotConstant,
    MissingType,
    UnresolvedApplication,
    InvalidOperand,
    DivisionByZero,
    Overflow,
    ReversedRange,
    OutsideOrdinalDomain { value: i128, target: TypeRef },
}

pub struct ConstantEvaluator<'a> {
    constants: &'a ConstantRegistry,
    types: &'a TypeRegistry,
}

impl<'a> ConstantEvaluator<'a> {
    pub const fn new(constants: &'a ConstantRegistry, types: &'a TypeRegistry) -> Self {
        Self { constants, types }
    }

    pub fn evaluate(
        &self,
        expression: &BoundExpression,
        expected: Option<TypeRef>,
    ) -> Result<ConstantEntry, ConstantEvaluationError> {
        self.evaluate_with_modes(expression, expected, ModeSnapshot::default())
    }

    pub fn evaluate_with_modes(
        &self,
        expression: &BoundExpression,
        expected: Option<TypeRef>,
        declaration_modes: ModeSnapshot,
    ) -> Result<ConstantEntry, ConstantEvaluationError> {
        if expression
            .conversion
            .as_ref()
            .is_some_and(conversion_uses_custom_operator)
        {
            return Err(ConstantEvaluationError::NotConstant);
        }
        match &expression.kind {
            BoundExpressionKind::Literal(literal) => {
                let ty = expected
                    .or(expression.ty)
                    .ok_or(ConstantEvaluationError::MissingType)?;
                let value = match literal {
                    Literal::Integer(value) => ConstantValue::Integer(*value),
                    Literal::Real(value) => ConstantValue::Real(value.clone()),
                    Literal::String(value)
                        if expression.ty == Some(ty) && value.chars().count() == 1 =>
                    {
                        ConstantValue::Character(value.chars().next().unwrap())
                    }
                    Literal::String(value) => ConstantValue::String(value.clone()),
                    Literal::Boolean(value) => ConstantValue::Boolean(*value),
                    Literal::Nil => ConstantValue::Nil,
                };
                self.convert(ConstantEntry { ty, value }, ty, declaration_modes)
            }
            BoundExpressionKind::Symbol { symbol, .. } => {
                let entry = self
                    .constants
                    .get(*symbol)
                    .cloned()
                    .ok_or(ConstantEvaluationError::NotConstant)?;
                if let Some(expected) = expected {
                    self.convert(entry, expected, declaration_modes)
                } else {
                    Ok(entry)
                }
            }
            BoundExpressionKind::Application {
                target,
                operands,
                modes,
                ..
            } => {
                if !application_selected(target) {
                    return Err(ConstantEvaluationError::UnresolvedApplication);
                }
                if application_uses_custom_conversion(target) {
                    return Err(ConstantEvaluationError::NotConstant);
                }
                match target {
                    BoundApplicationTarget::Operator { operator, .. } => {
                        if selected_builtin_operation(target).is_none() {
                            return Err(ConstantEvaluationError::NotConstant);
                        }
                        self.evaluate_operator(*operator, operands, expression.ty, expected, *modes)
                    }
                    BoundApplicationTarget::Builtin { .. } => {
                        let operation = selected_builtin_operation(target)
                            .ok_or(ConstantEvaluationError::UnresolvedApplication)?;
                        self.evaluate_builtin(operation, operands, expected)
                    }
                    BoundApplicationTarget::Conversion { destination, .. } => {
                        let source = operands
                            .first()
                            .ok_or(ConstantEvaluationError::InvalidOperand)?;
                        let value = self.evaluate(source, source.ty)?;
                        self.convert(value, *destination, *modes)
                    }
                    BoundApplicationTarget::Routine { .. }
                    | BoundApplicationTarget::CallableValue { .. }
                    | BoundApplicationTarget::Invalid => Err(ConstantEvaluationError::NotConstant),
                }
            }
            BoundExpressionKind::Set(elements) => {
                let target = expected
                    .or(expression.ty)
                    .ok_or(ConstantEvaluationError::MissingType)?;
                let element_type = self
                    .types
                    .set_element_type(target)
                    .ok_or(ConstantEvaluationError::InvalidOperand)?;
                let domain = self
                    .types
                    .ordinal_domain(element_type)
                    .ok_or(ConstantEvaluationError::InvalidOperand)?;
                let mut values = BTreeSet::new();
                for element in elements {
                    match element {
                        BoundSetElement::Value(value) => {
                            let value = self
                                .evaluate(value, Some(element_type))?
                                .value
                                .ordinal()
                                .ok_or(ConstantEvaluationError::InvalidOperand)?;
                            if !domain.contains(value) {
                                return Err(ConstantEvaluationError::OutsideOrdinalDomain {
                                    value,
                                    target: element_type,
                                });
                            }
                            values.insert(value);
                        }
                        BoundSetElement::Range { low, high } => {
                            let low = self
                                .evaluate(low, Some(element_type))?
                                .value
                                .ordinal()
                                .ok_or(ConstantEvaluationError::InvalidOperand)?;
                            let high = self
                                .evaluate(high, Some(element_type))?
                                .value
                                .ordinal()
                                .ok_or(ConstantEvaluationError::InvalidOperand)?;
                            if low > high {
                                return Err(ConstantEvaluationError::ReversedRange);
                            }
                            if !domain.contains(low) || !domain.contains(high) {
                                return Err(ConstantEvaluationError::OutsideOrdinalDomain {
                                    value: if !domain.contains(low) { low } else { high },
                                    target: element_type,
                                });
                            }
                            values.extend(low..=high);
                        }
                    }
                }
                Ok(ConstantEntry {
                    ty: target,
                    value: ConstantValue::Set(values),
                })
            }
            BoundExpressionKind::Member { .. }
            | BoundExpressionKind::TypeIdentifier { .. }
            | BoundExpressionKind::TypeOperand { .. }
            | BoundExpressionKind::Inherited { .. }
            | BoundExpressionKind::Property { .. }
            | BoundExpressionKind::RoutineDesignator { .. }
            | BoundExpressionKind::ProcedureCode(_)
            | BoundExpressionKind::Address(_)
            | BoundExpressionKind::Index { .. }
            | BoundExpressionKind::Dereference(_)
            | BoundExpressionKind::Error => Err(ConstantEvaluationError::NotConstant),
        }
    }

    fn evaluate_builtin(
        &self,
        operation: &BuiltinOperation,
        operands: &[BoundExpression],
        expected: Option<TypeRef>,
    ) -> Result<ConstantEntry, ConstantEvaluationError> {
        match operation {
            BuiltinOperation::Metadata {
                result_type,
                constant: Some(value),
                ..
            } => {
                let result_type = expected.unwrap_or(*result_type);
                self.convert(
                    ConstantEntry {
                        ty: result_type,
                        value: value.clone(),
                    },
                    result_type,
                    ModeSnapshot::default(),
                )
            }
            BuiltinOperation::Metadata { .. }
            | BuiltinOperation::StepMutation { .. }
            | BuiltinOperation::SetMutation { .. } => Err(ConstantEvaluationError::NotConstant),
            BuiltinOperation::Ordinal {
                operation,
                result_type,
                modes,
                ..
            } => {
                let source = operands
                    .first()
                    .ok_or(ConstantEvaluationError::InvalidOperand)?;
                let source = self.evaluate(source, source.ty)?;
                let ordinal = source
                    .value
                    .ordinal()
                    .ok_or(ConstantEvaluationError::InvalidOperand)?;
                let value = match operation {
                    OrdinalOperation::Odd => ConstantValue::Boolean(ordinal & 1 != 0),
                    OrdinalOperation::Ord => ConstantValue::Integer(ordinal),
                    OrdinalOperation::Chr => {
                        let code = u32::try_from(ordinal)
                            .ok()
                            .and_then(char::from_u32)
                            .ok_or(ConstantEvaluationError::InvalidOperand)?;
                        ConstantValue::Character(code)
                    }
                    OrdinalOperation::Pred => source
                        .value
                        .ordinal_variant(
                            ordinal
                                .checked_sub(1)
                                .ok_or(ConstantEvaluationError::Overflow)?,
                        )
                        .ok_or(ConstantEvaluationError::InvalidOperand)?,
                    OrdinalOperation::Succ => source
                        .value
                        .ordinal_variant(
                            ordinal
                                .checked_add(1)
                                .ok_or(ConstantEvaluationError::Overflow)?,
                        )
                        .ok_or(ConstantEvaluationError::InvalidOperand)?,
                };
                let result_type = expected.unwrap_or(*result_type);
                self.convert(
                    ConstantEntry {
                        ty: result_type,
                        value,
                    },
                    result_type,
                    *modes,
                )
            }
            BuiltinOperation::Numeric {
                operation,
                result_type,
                modes,
                ..
            } => {
                let source = operands
                    .first()
                    .ok_or(ConstantEvaluationError::InvalidOperand)?;
                let source = self.evaluate(source, source.ty)?;
                let ordinal = source
                    .value
                    .ordinal()
                    .ok_or(ConstantEvaluationError::InvalidOperand)?;
                let value = match operation {
                    NumericOperation::Abs => ordinal
                        .checked_abs()
                        .map(ConstantValue::Integer)
                        .ok_or(ConstantEvaluationError::Overflow)?,
                    NumericOperation::Sqr => ordinal
                        .checked_mul(ordinal)
                        .map(ConstantValue::Integer)
                        .ok_or(ConstantEvaluationError::Overflow)?,
                };
                let result_type = expected.unwrap_or(*result_type);
                self.convert(
                    ConstantEntry {
                        ty: result_type,
                        value,
                    },
                    result_type,
                    *modes,
                )
            }
            BuiltinOperation::Operator { .. } => Err(ConstantEvaluationError::InvalidOperand),
        }
    }

    fn evaluate_operator(
        &self,
        operator: Operator,
        operands: &[BoundExpression],
        expression_type: Option<TypeRef>,
        expected: Option<TypeRef>,
        modes: ModeSnapshot,
    ) -> Result<ConstantEntry, ConstantEvaluationError> {
        let values = operands
            .iter()
            .map(|operand| self.evaluate(operand, operand.ty))
            .collect::<Result<Vec<_>, _>>()?;
        let result_type = expected
            .or(expression_type)
            .ok_or(ConstantEvaluationError::MissingType)?;
        let value = match (operator, values.as_slice()) {
            (Operator::Positive, [value]) => value.value.clone(),
            (Operator::Negative, [value]) => ConstantValue::Integer(
                value
                    .value
                    .ordinal()
                    .ok_or(ConstantEvaluationError::InvalidOperand)?
                    .checked_neg()
                    .ok_or(ConstantEvaluationError::Overflow)?,
            ),
            (Operator::Not, [value]) => match value.value {
                ConstantValue::Boolean(value) => ConstantValue::Boolean(!value),
                _ => ConstantValue::Integer(
                    !value
                        .value
                        .ordinal()
                        .ok_or(ConstantEvaluationError::InvalidOperand)?,
                ),
            },
            (operator, [left, right]) => {
                self.evaluate_binary(operator, &left.value, &right.value)?
            }
            _ => return Err(ConstantEvaluationError::InvalidOperand),
        };
        self.convert(
            ConstantEntry {
                ty: result_type,
                value,
            },
            result_type,
            modes,
        )
    }

    fn evaluate_binary(
        &self,
        operator: Operator,
        left: &ConstantValue,
        right: &ConstantValue,
    ) -> Result<ConstantValue, ConstantEvaluationError> {
        if let (ConstantValue::Boolean(left), ConstantValue::Boolean(right)) = (left, right) {
            return match operator {
                Operator::And => Ok(ConstantValue::Boolean(*left && *right)),
                Operator::Or => Ok(ConstantValue::Boolean(*left || *right)),
                Operator::Xor => Ok(ConstantValue::Boolean(*left ^ *right)),
                Operator::Equal => Ok(ConstantValue::Boolean(left == right)),
                Operator::NotEqual => Ok(ConstantValue::Boolean(left != right)),
                _ => Err(ConstantEvaluationError::InvalidOperand),
            };
        }
        let left = left
            .ordinal()
            .ok_or(ConstantEvaluationError::InvalidOperand)?;
        let right = right
            .ordinal()
            .ok_or(ConstantEvaluationError::InvalidOperand)?;
        match operator {
            Operator::Add => left
                .checked_add(right)
                .map(ConstantValue::Integer)
                .ok_or(ConstantEvaluationError::Overflow),
            Operator::Subtract => left
                .checked_sub(right)
                .map(ConstantValue::Integer)
                .ok_or(ConstantEvaluationError::Overflow),
            Operator::Multiply => left
                .checked_mul(right)
                .map(ConstantValue::Integer)
                .ok_or(ConstantEvaluationError::Overflow),
            Operator::IntegerDivide => {
                if right == 0 {
                    Err(ConstantEvaluationError::DivisionByZero)
                } else {
                    left.checked_div(right)
                        .map(ConstantValue::Integer)
                        .ok_or(ConstantEvaluationError::Overflow)
                }
            }
            Operator::Modulo => {
                if right == 0 {
                    Err(ConstantEvaluationError::DivisionByZero)
                } else {
                    left.checked_rem(right)
                        .map(ConstantValue::Integer)
                        .ok_or(ConstantEvaluationError::Overflow)
                }
            }
            Operator::And => Ok(ConstantValue::Integer(left & right)),
            Operator::Or => Ok(ConstantValue::Integer(left | right)),
            Operator::Xor => Ok(ConstantValue::Integer(left ^ right)),
            Operator::ShiftLeft => u32::try_from(right)
                .ok()
                .and_then(|shift| left.checked_shl(shift))
                .map(ConstantValue::Integer)
                .ok_or(ConstantEvaluationError::Overflow),
            Operator::ShiftRight => u32::try_from(right)
                .ok()
                .and_then(|shift| left.checked_shr(shift))
                .map(ConstantValue::Integer)
                .ok_or(ConstantEvaluationError::Overflow),
            Operator::Equal => Ok(ConstantValue::Boolean(left == right)),
            Operator::NotEqual => Ok(ConstantValue::Boolean(left != right)),
            Operator::Less => Ok(ConstantValue::Boolean(left < right)),
            Operator::Greater => Ok(ConstantValue::Boolean(left > right)),
            Operator::LessEqual => Ok(ConstantValue::Boolean(left <= right)),
            Operator::GreaterEqual => Ok(ConstantValue::Boolean(left >= right)),
            Operator::Assign
            | Operator::RealDivide
            | Operator::Positive
            | Operator::Negative
            | Operator::Not
            | Operator::Address
            | Operator::ProcedureSlotAddress
            | Operator::In
            | Operator::Is
            | Operator::As => Err(ConstantEvaluationError::InvalidOperand),
        }
    }

    fn convert(
        &self,
        mut entry: ConstantEntry,
        destination: TypeRef,
        modes: ModeSnapshot,
    ) -> Result<ConstantEntry, ConstantEvaluationError> {
        if let Some(value) = entry.value.ordinal()
            && let Some(domain) = self.types.ordinal_domain(destination)
            && !domain.contains(value)
            && (modes.range_checks || modes.overflow_checks)
        {
            return Err(ConstantEvaluationError::OutsideOrdinalDomain {
                value,
                target: destination,
            });
        }
        entry.ty = destination;
        Ok(entry)
    }
}

fn application_selected(target: &BoundApplicationTarget) -> bool {
    let selection = match target {
        BoundApplicationTarget::Routine { resolution }
        | BoundApplicationTarget::CallableValue { resolution }
        | BoundApplicationTarget::Builtin { resolution }
        | BoundApplicationTarget::Conversion { resolution, .. }
        | BoundApplicationTarget::Operator { resolution, .. } => &resolution.selection,
        BoundApplicationTarget::Invalid => return false,
    };
    matches!(selection, ApplicationSelection::Selected { .. })
}

fn application_uses_custom_conversion(target: &BoundApplicationTarget) -> bool {
    let resolution = match target {
        BoundApplicationTarget::Routine { resolution }
        | BoundApplicationTarget::CallableValue { resolution }
        | BoundApplicationTarget::Builtin { resolution }
        | BoundApplicationTarget::Conversion { resolution, .. }
        | BoundApplicationTarget::Operator { resolution, .. } => resolution,
        BoundApplicationTarget::Invalid => return false,
    };
    resolution.selected_attempt().is_some_and(|attempt| {
        attempt.arguments.iter().any(|argument| {
            argument
                .conversion
                .as_ref()
                .is_some_and(argument_conversion_uses_custom_operator)
        })
    })
}

fn selected_builtin_operation(target: &BoundApplicationTarget) -> Option<&BuiltinOperation> {
    let resolution = match target {
        BoundApplicationTarget::Routine { resolution }
        | BoundApplicationTarget::CallableValue { resolution }
        | BoundApplicationTarget::Builtin { resolution }
        | BoundApplicationTarget::Conversion { resolution, .. }
        | BoundApplicationTarget::Operator { resolution, .. } => resolution,
        BoundApplicationTarget::Invalid => return None,
    };
    let ApplicationCandidate::Builtin {
        instantiation: BuiltinInstantiation::Complete(instance),
        ..
    } = &resolution.selected_attempt()?.candidate
    else {
        return None;
    };
    Some(&instance.operation)
}

fn conversion_uses_custom_operator(conversion: &super::ConversionResolution) -> bool {
    match conversion.selected() {
        Some(ResolvedConversion::Implicit(conversion)) => matches!(
            conversion.operation,
            ValueConversionOperation::CustomOperator { .. }
        ),
        Some(ResolvedConversion::Explicit(ExplicitConversion::CustomOperator { .. })) => true,
        Some(ResolvedConversion::Explicit(ExplicitConversion::Value(conversion))) => matches!(
            conversion.operation,
            ValueConversionOperation::CustomOperator { .. }
        ),
        Some(ResolvedConversion::Explicit(
            ExplicitConversion::IntegerTruncate { .. }
            | ExplicitConversion::RealNarrow { .. }
            | ExplicitConversion::OrdinalCast { .. }
            | ExplicitConversion::PointerCrossing { .. }
            | ExplicitConversion::RelatedDowncast { .. }
            | ExplicitConversion::ProcedureAdapter { .. }
            | ExplicitConversion::ProcedurePointerCrossing { .. }
            | ExplicitConversion::RepresentationOverlay { .. },
        ))
        | None => false,
    }
}

fn argument_conversion_uses_custom_operator(conversion: &ArgumentConversion) -> bool {
    match conversion {
        ArgumentConversion::Implicit(conversion) | ArgumentConversion::Explicit(conversion) => {
            conversion_uses_custom_operator(conversion)
        }
        ArgumentConversion::Storage(_) => false,
    }
}
