use std::collections::BTreeMap;

use crate::{ModeSnapshot, Operator};

use super::{
    BuiltinFamilyId, ConstantValue, FormalParameter, FormalTypeKind, ParameterMode, PrimitiveKind,
    RoutineSignature, SymbolId, TypeRef, TypeRegistry,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinOperandForm {
    Value,
    Type,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuiltinActual {
    pub form: BuiltinOperandForm,
    pub ty: Option<TypeRef>,
    pub addressable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetadataQuery {
    Low,
    High,
    SizeOf,
    Length,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrdinalOperation {
    Odd,
    Ord,
    Chr,
    Pred,
    Succ,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericOperation {
    Abs,
    Sqr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepOperation {
    Increment,
    Decrement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetMutationOperation {
    Include,
    Exclude,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinContract {
    Operator(Operator),
    Metadata(MetadataQuery),
    Ordinal(OrdinalOperation),
    Numeric(NumericOperation),
    StepMutation(StepOperation),
    SetMutation(SetMutationOperation),
}

/// Resolve the hardcoded semantic handler selected by an RTL `external name`.
/// Pascal declaration spelling is intentionally absent from this API: source
/// names exist for ordinary lookup and may be shadowed.
pub fn builtin_contract_for_external_selector(
    selector: &str,
    has_omitted_formals: bool,
) -> Option<BuiltinContract> {
    let contract = match selector {
        "::u_system::o_positive" => BuiltinContract::Operator(Operator::Positive),
        "::u_system::o_negative" | "::u_system::o_unchecked_negative" => {
            BuiltinContract::Operator(Operator::Negative)
        }
        "::u_system::o_add" | "::u_system::o_unchecked_add" => {
            BuiltinContract::Operator(Operator::Add)
        }
        "::u_system::o_subtract" | "::u_system::o_unchecked_subtract" => {
            BuiltinContract::Operator(Operator::Subtract)
        }
        "::u_system::o_multiply" | "::u_system::o_unchecked_multiply" => {
            BuiltinContract::Operator(Operator::Multiply)
        }
        "::u_system::o_divide" => BuiltinContract::Operator(Operator::RealDivide),
        "::u_system::o_intdivide" | "::u_system::o_unchecked_intdivide" => {
            BuiltinContract::Operator(Operator::IntegerDivide)
        }
        "::u_system::o_modulus" => BuiltinContract::Operator(Operator::Modulo),
        "::u_system::o_leftshift" => BuiltinContract::Operator(Operator::ShiftLeft),
        "::u_system::o_rightshift" => BuiltinContract::Operator(Operator::ShiftRight),
        "::u_system::o_equal" => BuiltinContract::Operator(Operator::Equal),
        "::u_system::o_lessthan" => BuiltinContract::Operator(Operator::Less),
        "::u_system::o_lessthanorequal" => BuiltinContract::Operator(Operator::LessEqual),
        "::u_system::o_greaterthan" => BuiltinContract::Operator(Operator::Greater),
        "::u_system::o_greaterthanorequal" => BuiltinContract::Operator(Operator::GreaterEqual),
        "::u_system::o_bitwiseand" | "::u_system::o_logicaland" => {
            BuiltinContract::Operator(Operator::And)
        }
        "::u_system::o_bitwiseor" | "::u_system::o_logicalor" => {
            BuiltinContract::Operator(Operator::Or)
        }
        "::u_system::o_bitwisexor" | "::u_system::o_logicalxor" => {
            BuiltinContract::Operator(Operator::Xor)
        }
        "::u_system::o_logicalnot" => BuiltinContract::Operator(Operator::Not),
        "::u_system::o_in" => BuiltinContract::Operator(Operator::In),
        "::u_system::p_low" => BuiltinContract::Metadata(MetadataQuery::Low),
        "::u_system::p_high" => BuiltinContract::Metadata(MetadataQuery::High),
        "::u_system::p_sizeof" => BuiltinContract::Metadata(MetadataQuery::SizeOf),
        "::u_system::p_length" if has_omitted_formals => {
            BuiltinContract::Metadata(MetadataQuery::Length)
        }
        "::u_system::p_odd" => BuiltinContract::Ordinal(OrdinalOperation::Odd),
        "::u_system::p_ord" => BuiltinContract::Ordinal(OrdinalOperation::Ord),
        "::u_system::p_chr" => BuiltinContract::Ordinal(OrdinalOperation::Chr),
        "::u_system::p_pred" => BuiltinContract::Ordinal(OrdinalOperation::Pred),
        "::u_system::p_succ" => BuiltinContract::Ordinal(OrdinalOperation::Succ),
        "::u_system::p_abs" => BuiltinContract::Numeric(NumericOperation::Abs),
        "::u_system::p_sqr" => BuiltinContract::Numeric(NumericOperation::Sqr),
        "::u_system::p_inc" => BuiltinContract::StepMutation(StepOperation::Increment),
        "::u_system::p_dec" => BuiltinContract::StepMutation(StepOperation::Decrement),
        "::u_system::p_include" => BuiltinContract::SetMutation(SetMutationOperation::Include),
        "::u_system::p_exclude" => BuiltinContract::SetMutation(SetMutationOperation::Exclude),
        _ => return None,
    };
    Some(contract)
}

impl BuiltinContract {
    pub const fn permits_type_operand(self, index: usize) -> bool {
        index == 0
            && matches!(
                self,
                Self::Metadata(MetadataQuery::Low | MetadataQuery::High | MetadataQuery::SizeOf)
            )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltinFamilyDecl {
    pub external_selector: String,
    pub contract: BuiltinContract,
    /// One entry per expanded Pascal formal name. `true` means the source
    /// deliberately omitted its type and the hardcoded handler owns the
    /// missing type relationship.
    pub omitted_formals: Vec<bool>,
    /// Fixed RTL overloads retain their declared Pascal signature. Generic
    /// RTL declarations leave this empty and instantiate a signature from the
    /// actual Pascal operands.
    pub declared_signature: Option<RoutineSignature>,
}

#[derive(Clone, Debug, Default)]
pub struct BuiltinRegistry {
    families: Vec<BuiltinFamilyDecl>,
    by_symbol: BTreeMap<SymbolId, BuiltinFamilyId>,
}

impl BuiltinRegistry {
    pub fn attach(&mut self, symbol: SymbolId, declaration: BuiltinFamilyDecl) -> BuiltinFamilyId {
        let id = BuiltinFamilyId::from_index(self.families.len());
        self.families.push(declaration);
        let previous = self.by_symbol.insert(symbol, id);
        debug_assert!(previous.is_none(), "builtin metadata attached twice");
        id
    }

    pub fn family_for_symbol(&self, symbol: SymbolId) -> Option<BuiltinFamilyId> {
        self.by_symbol.get(&symbol).copied()
    }

    pub fn get(&self, family: BuiltinFamilyId) -> &BuiltinFamilyDecl {
        &self.families[family.index()]
    }

    pub fn len(&self) -> usize {
        self.families.len()
    }

    pub fn is_empty(&self) -> bool {
        self.families.is_empty()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BuiltinTypeContext {
    pub integer: TypeRef,
    pub long_integer: TypeRef,
    pub real: TypeRef,
    pub boolean: TypeRef,
    pub character: TypeRef,
    pub byte: TypeRef,
    pub word: TypeRef,
    pub size_unsigned: TypeRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuiltinOperation {
    Operator {
        operator: Operator,
        operand_types: Vec<TypeRef>,
        result_type: TypeRef,
        modes: ModeSnapshot,
    },
    Metadata {
        query: MetadataQuery,
        operand_form: BuiltinOperandForm,
        operand_type: TypeRef,
        result_type: TypeRef,
        constant: Option<ConstantValue>,
    },
    Ordinal {
        operation: OrdinalOperation,
        operand_type: TypeRef,
        result_type: TypeRef,
        modes: ModeSnapshot,
    },
    Numeric {
        operation: NumericOperation,
        operand_type: TypeRef,
        result_type: TypeRef,
        modes: ModeSnapshot,
    },
    StepMutation {
        operation: StepOperation,
        operand_type: TypeRef,
        delta_type: Option<TypeRef>,
        modes: ModeSnapshot,
    },
    SetMutation {
        operation: SetMutationOperation,
        set_type: TypeRef,
        element_type: TypeRef,
        item_type: TypeRef,
        modes: ModeSnapshot,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltinInstance {
    pub signature: RoutineSignature,
    pub operand_forms: Vec<BuiltinOperandForm>,
    pub operation: BuiltinOperation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuiltinRejection {
    Arity {
        provided: usize,
        minimum: usize,
        maximum: usize,
    },
    MissingType {
        actual_index: usize,
    },
    ExpectedTypeOperand {
        actual_index: usize,
    },
    ExpectedValueOperand {
        actual_index: usize,
    },
    UnsupportedOperand {
        actual_index: usize,
        ty: TypeRef,
    },
    IncompatibleOperands {
        left: TypeRef,
        right: TypeRef,
    },
    MutableOperandRequired {
        actual_index: usize,
    },
    MissingStaticMetadata {
        ty: TypeRef,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuiltinInstantiation {
    Complete(BuiltinInstance),
    Rejected(BuiltinRejection),
}

impl BuiltinFamilyDecl {
    pub fn instantiate(
        &self,
        actuals: &[BuiltinActual],
        types: &TypeRegistry,
        builtins: BuiltinTypeContext,
        modes: ModeSnapshot,
    ) -> BuiltinInstantiation {
        if let Some(signature) = &self.declared_signature {
            return instantiate_declared(self.contract, signature, actuals, modes);
        }
        let result = match self.contract {
            BuiltinContract::Operator(operator) => {
                instantiate_operator(operator, actuals, types, builtins, modes)
            }
            BuiltinContract::Metadata(query) => {
                instantiate_metadata(query, actuals, types, builtins)
            }
            BuiltinContract::Ordinal(operation) => {
                instantiate_ordinal(operation, actuals, types, builtins, modes)
            }
            BuiltinContract::Numeric(operation) => {
                instantiate_numeric(operation, actuals, types, modes)
            }
            BuiltinContract::StepMutation(operation) => {
                instantiate_step(operation, actuals, types, modes)
            }
            BuiltinContract::SetMutation(operation) => {
                instantiate_set_mutation(operation, actuals, types, modes)
            }
        };
        match result {
            Ok(instance) => BuiltinInstantiation::Complete(instance),
            Err(rejection) => BuiltinInstantiation::Rejected(rejection),
        }
    }
}

fn instantiate_declared(
    contract: BuiltinContract,
    signature: &RoutineSignature,
    actuals: &[BuiltinActual],
    modes: ModeSnapshot,
) -> BuiltinInstantiation {
    let mut operand_types = Vec::with_capacity(actuals.len());
    for (index, actual) in actuals.iter().enumerate() {
        if actual.form != BuiltinOperandForm::Value {
            return BuiltinInstantiation::Rejected(BuiltinRejection::ExpectedValueOperand {
                actual_index: index,
            });
        }
        let Some(ty) = actual.ty else {
            return BuiltinInstantiation::Rejected(BuiltinRejection::MissingType {
                actual_index: index,
            });
        };
        operand_types.push(ty);
    }
    let operand_type = operand_types
        .first()
        .copied()
        .or_else(|| signature.parameters.first().map(|formal| formal.ty))
        .expect("intrinsic declaration has an operand");
    let operation = match contract {
        BuiltinContract::Operator(operator) => BuiltinOperation::Operator {
            operator,
            operand_types,
            result_type: signature.result.expect("operator declaration has a result"),
            modes,
        },
        BuiltinContract::Ordinal(operation) => BuiltinOperation::Ordinal {
            operation,
            operand_type,
            result_type: signature
                .result
                .expect("ordinal function declaration has a result"),
            modes,
        },
        BuiltinContract::Numeric(operation) => BuiltinOperation::Numeric {
            operation,
            operand_type,
            result_type: signature
                .result
                .expect("numeric function declaration has a result"),
            modes,
        },
        BuiltinContract::StepMutation(operation) => BuiltinOperation::StepMutation {
            operation,
            operand_type,
            delta_type: operand_types.get(1).copied(),
            modes,
        },
        BuiltinContract::SetMutation(_) => {
            unreachable!("set mutation declarations have omitted formals")
        }
        BuiltinContract::Metadata(_) => {
            unreachable!("metadata contracts need the type registry and stay parameterized")
        }
    };
    BuiltinInstantiation::Complete(BuiltinInstance {
        signature: signature.clone(),
        operand_forms: vec![BuiltinOperandForm::Value; signature.parameters.len()],
        operation,
    })
}

fn value_parameter(ty: TypeRef) -> FormalParameter {
    FormalParameter {
        mode: ParameterMode::Value,
        ty,
        type_kind: FormalTypeKind::Declared,
        default: None,
    }
}

fn signature(parameters: Vec<FormalParameter>, result: Option<TypeRef>) -> RoutineSignature {
    RoutineSignature {
        parameters,
        result,
        calling_convention: super::CallingConvention::Pascal,
    }
}

fn actual_type(
    actuals: &[BuiltinActual],
    index: usize,
    expected_form: BuiltinOperandForm,
) -> Result<TypeRef, BuiltinRejection> {
    let Some(actual) = actuals.get(index) else {
        return Err(BuiltinRejection::Arity {
            provided: actuals.len(),
            minimum: index + 1,
            maximum: index + 1,
        });
    };
    if actual.form != expected_form {
        return Err(match expected_form {
            BuiltinOperandForm::Type => BuiltinRejection::ExpectedTypeOperand {
                actual_index: index,
            },
            BuiltinOperandForm::Value => BuiltinRejection::ExpectedValueOperand {
                actual_index: index,
            },
        });
    }
    actual.ty.ok_or(BuiltinRejection::MissingType {
        actual_index: index,
    })
}

fn expect_arity(
    actuals: &[BuiltinActual],
    minimum: usize,
    maximum: usize,
) -> Result<(), BuiltinRejection> {
    if actuals.len() < minimum || actuals.len() > maximum {
        Err(BuiltinRejection::Arity {
            provided: actuals.len(),
            minimum,
            maximum,
        })
    } else {
        Ok(())
    }
}

fn instantiate_metadata(
    query: MetadataQuery,
    actuals: &[BuiltinActual],
    types: &TypeRegistry,
    builtins: BuiltinTypeContext,
) -> Result<BuiltinInstance, BuiltinRejection> {
    expect_arity(actuals, 1, 1)?;
    let form = actuals[0].form;
    if query == MetadataQuery::Length && form != BuiltinOperandForm::Value {
        return Err(BuiltinRejection::ExpectedValueOperand { actual_index: 0 });
    }
    let operand_type = actual_type(actuals, 0, form)?;
    let (result_type, constant) = match query {
        MetadataQuery::Low | MetadataQuery::High => {
            if let Some(domain) = types.ordinal_domain(operand_type) {
                let value = if query == MetadataQuery::Low {
                    domain.lower
                } else {
                    domain.upper
                };
                (operand_type, Some(ConstantValue::Integer(value)))
            } else if let Some(index) = types.sequence_index_type(operand_type) {
                let constant = types.ordinal_domain(index).map(|domain| {
                    ConstantValue::Integer(if query == MetadataQuery::Low {
                        domain.lower
                    } else {
                        domain.upper
                    })
                });
                (index, constant)
            } else {
                return Err(BuiltinRejection::UnsupportedOperand {
                    actual_index: 0,
                    ty: operand_type,
                });
            }
        }
        MetadataQuery::SizeOf => {
            let layout = types
                .storage_layout(operand_type)
                .ok_or(BuiltinRejection::MissingStaticMetadata { ty: operand_type })?;
            (
                builtins.size_unsigned,
                Some(ConstantValue::Integer(i128::from(layout.size))),
            )
        }
        MetadataQuery::Length => {
            let result_type = types.sequence_length_type(operand_type).ok_or(
                BuiltinRejection::UnsupportedOperand {
                    actual_index: 0,
                    ty: operand_type,
                },
            )?;
            let constant = types
                .static_sequence_length(operand_type)
                .map(|length| ConstantValue::Integer(i128::from(length)));
            (result_type, constant)
        }
    };
    Ok(BuiltinInstance {
        signature: signature(vec![value_parameter(operand_type)], Some(result_type)),
        operand_forms: vec![form],
        operation: BuiltinOperation::Metadata {
            query,
            operand_form: form,
            operand_type,
            result_type,
            constant,
        },
    })
}

fn instantiate_ordinal(
    operation: OrdinalOperation,
    actuals: &[BuiltinActual],
    types: &TypeRegistry,
    builtins: BuiltinTypeContext,
    modes: ModeSnapshot,
) -> Result<BuiltinInstance, BuiltinRejection> {
    expect_arity(actuals, 1, 1)?;
    let operand_type = actual_type(actuals, 0, BuiltinOperandForm::Value)?;
    let primitive = types.primitive_kind(operand_type);
    let ordinal = types.ordinal_domain(operand_type).is_some();
    let result_type = match operation {
        OrdinalOperation::Odd if integer_operand(types, operand_type) => builtins.boolean,
        OrdinalOperation::Ord if ordinal => match primitive {
            Some(PrimitiveKind::Boolean | PrimitiveKind::Character) => builtins.byte,
            Some(PrimitiveKind::WideCharacter { .. }) => builtins.word,
            Some(PrimitiveKind::Integer { .. }) => operand_type,
            Some(PrimitiveKind::Real { .. }) | None => builtins.long_integer,
        },
        OrdinalOperation::Chr if integer_operand(types, operand_type) => builtins.character,
        OrdinalOperation::Pred | OrdinalOperation::Succ if ordinal => operand_type,
        _ => {
            return Err(BuiltinRejection::UnsupportedOperand {
                actual_index: 0,
                ty: operand_type,
            });
        }
    };
    Ok(BuiltinInstance {
        signature: signature(vec![value_parameter(operand_type)], Some(result_type)),
        operand_forms: vec![BuiltinOperandForm::Value],
        operation: BuiltinOperation::Ordinal {
            operation,
            operand_type,
            result_type,
            modes,
        },
    })
}

fn instantiate_numeric(
    operation: NumericOperation,
    actuals: &[BuiltinActual],
    types: &TypeRegistry,
    modes: ModeSnapshot,
) -> Result<BuiltinInstance, BuiltinRejection> {
    expect_arity(actuals, 1, 1)?;
    let operand_type = actual_type(actuals, 0, BuiltinOperandForm::Value)?;
    if !numeric_operand(types, operand_type) {
        return Err(BuiltinRejection::UnsupportedOperand {
            actual_index: 0,
            ty: operand_type,
        });
    }
    Ok(BuiltinInstance {
        signature: signature(vec![value_parameter(operand_type)], Some(operand_type)),
        operand_forms: vec![BuiltinOperandForm::Value],
        operation: BuiltinOperation::Numeric {
            operation,
            operand_type,
            result_type: operand_type,
            modes,
        },
    })
}

fn instantiate_step(
    operation: StepOperation,
    actuals: &[BuiltinActual],
    types: &TypeRegistry,
    modes: ModeSnapshot,
) -> Result<BuiltinInstance, BuiltinRejection> {
    expect_arity(actuals, 1, 2)?;
    let operand_type = actual_type(actuals, 0, BuiltinOperandForm::Value)?;
    if !actuals[0].addressable {
        return Err(BuiltinRejection::MutableOperandRequired { actual_index: 0 });
    }
    if types.ordinal_domain(operand_type).is_none() {
        return Err(BuiltinRejection::UnsupportedOperand {
            actual_index: 0,
            ty: operand_type,
        });
    }
    let delta_type = if actuals.len() == 2 {
        let ty = actual_type(actuals, 1, BuiltinOperandForm::Value)?;
        if !integer_operand(types, ty) {
            return Err(BuiltinRejection::UnsupportedOperand {
                actual_index: 1,
                ty,
            });
        }
        Some(ty)
    } else {
        None
    };
    let mut parameters = vec![FormalParameter {
        mode: ParameterMode::Var,
        ty: operand_type,
        type_kind: FormalTypeKind::Declared,
        default: None,
    }];
    if let Some(delta) = delta_type {
        parameters.push(value_parameter(delta));
    }
    Ok(BuiltinInstance {
        signature: signature(parameters, None),
        operand_forms: vec![BuiltinOperandForm::Value; actuals.len()],
        operation: BuiltinOperation::StepMutation {
            operation,
            operand_type,
            delta_type,
            modes,
        },
    })
}

fn instantiate_set_mutation(
    operation: SetMutationOperation,
    actuals: &[BuiltinActual],
    types: &TypeRegistry,
    modes: ModeSnapshot,
) -> Result<BuiltinInstance, BuiltinRejection> {
    expect_arity(actuals, 2, 2)?;
    let set_type = actual_type(actuals, 0, BuiltinOperandForm::Value)?;
    if !actuals[0].addressable {
        return Err(BuiltinRejection::MutableOperandRequired { actual_index: 0 });
    }
    let element_type =
        types
            .set_element_type(set_type)
            .ok_or(BuiltinRejection::UnsupportedOperand {
                actual_index: 0,
                ty: set_type,
            })?;
    let item_type = actual_type(actuals, 1, BuiltinOperandForm::Value)?;
    Ok(BuiltinInstance {
        signature: signature(
            vec![
                FormalParameter {
                    mode: ParameterMode::Var,
                    ty: set_type,
                    type_kind: FormalTypeKind::Declared,
                    default: None,
                },
                value_parameter(element_type),
            ],
            None,
        ),
        operand_forms: vec![BuiltinOperandForm::Value; 2],
        operation: BuiltinOperation::SetMutation {
            operation,
            set_type,
            element_type,
            item_type,
            modes,
        },
    })
}

fn instantiate_operator(
    operator: Operator,
    actuals: &[BuiltinActual],
    types: &TypeRegistry,
    builtins: BuiltinTypeContext,
    modes: ModeSnapshot,
) -> Result<BuiltinInstance, BuiltinRejection> {
    let arity = if matches!(
        operator,
        Operator::Positive | Operator::Negative | Operator::Not
    ) {
        1
    } else {
        2
    };
    expect_arity(actuals, arity, arity)?;
    let operand_types = (0..arity)
        .map(|index| actual_type(actuals, index, BuiltinOperandForm::Value))
        .collect::<Result<Vec<_>, _>>()?;
    if operator == Operator::In {
        let item_type = operand_types[0];
        let set_type = operand_types[1];
        let element_type =
            types
                .set_element_type(set_type)
                .ok_or(BuiltinRejection::UnsupportedOperand {
                    actual_index: 1,
                    ty: set_type,
                })?;
        return Ok(BuiltinInstance {
            signature: signature(
                vec![value_parameter(element_type), value_parameter(set_type)],
                Some(builtins.boolean),
            ),
            operand_forms: vec![BuiltinOperandForm::Value; 2],
            operation: BuiltinOperation::Operator {
                operator,
                operand_types: vec![item_type, set_type],
                result_type: builtins.boolean,
                modes,
            },
        });
    }
    let mut comparison_formal_type = None;
    let result_type = if arity == 1 {
        let operand = operand_types[0];
        match operator {
            Operator::Positive | Operator::Negative if numeric_operand(types, operand) => operand,
            Operator::Not if boolean_operand(types, operand) => builtins.boolean,
            Operator::Not if integer_operand(types, operand) => operand,
            _ => {
                return Err(BuiltinRejection::UnsupportedOperand {
                    actual_index: 0,
                    ty: operand,
                });
            }
        }
    } else {
        let left = operand_types[0];
        let right = operand_types[1];
        match operator {
            Operator::Equal
            | Operator::NotEqual
            | Operator::Less
            | Operator::Greater
            | Operator::LessEqual
            | Operator::GreaterEqual => {
                comparison_formal_type = Some(
                    common_scalar_type(types, left, right, builtins)
                        .ok_or(BuiltinRejection::IncompatibleOperands { left, right })?,
                );
                builtins.boolean
            }
            Operator::And | Operator::Or | Operator::Xor
                if boolean_operand(types, left) && boolean_operand(types, right) =>
            {
                builtins.boolean
            }
            Operator::Add
            | Operator::Subtract
            | Operator::Multiply
            | Operator::IntegerDivide
            | Operator::Modulo
            | Operator::And
            | Operator::Or
            | Operator::Xor => common_numeric_type(types, left, right, builtins)
                .ok_or(BuiltinRejection::IncompatibleOperands { left, right })?,
            Operator::RealDivide => {
                if numeric_operand(types, left) && numeric_operand(types, right) {
                    builtins.real
                } else {
                    return Err(BuiltinRejection::IncompatibleOperands { left, right });
                }
            }
            Operator::ShiftLeft | Operator::ShiftRight
                if integer_operand(types, left) && integer_operand(types, right) =>
            {
                left
            }
            _ => {
                return Err(BuiltinRejection::IncompatibleOperands { left, right });
            }
        }
    };
    let formal_types = if let Some(comparison) = comparison_formal_type {
        vec![comparison; 2]
    } else if arity == 2
        && matches!(
            operator,
            Operator::Add
                | Operator::Subtract
                | Operator::Multiply
                | Operator::IntegerDivide
                | Operator::Modulo
                | Operator::And
                | Operator::Or
                | Operator::Xor
                | Operator::Equal
                | Operator::NotEqual
                | Operator::Less
                | Operator::Greater
                | Operator::LessEqual
                | Operator::GreaterEqual
        )
        && !(boolean_operand(types, operand_types[0]) && boolean_operand(types, operand_types[1]))
    {
        vec![result_type; 2]
    } else {
        operand_types.clone()
    };
    Ok(BuiltinInstance {
        signature: signature(
            formal_types.into_iter().map(value_parameter).collect(),
            Some(result_type),
        ),
        operand_forms: vec![BuiltinOperandForm::Value; arity],
        operation: BuiltinOperation::Operator {
            operator,
            operand_types,
            result_type,
            modes,
        },
    })
}

fn primitive_root(types: &TypeRegistry, ty: TypeRef) -> Option<PrimitiveKind> {
    let root = types.ordinal_base_type(ty).unwrap_or(ty);
    types.primitive_kind(root)
}

fn integer_operand(types: &TypeRegistry, ty: TypeRef) -> bool {
    matches!(
        primitive_root(types, ty),
        Some(PrimitiveKind::Integer { .. })
    )
}

fn boolean_operand(types: &TypeRegistry, ty: TypeRef) -> bool {
    matches!(primitive_root(types, ty), Some(PrimitiveKind::Boolean))
}

fn numeric_operand(types: &TypeRegistry, ty: TypeRef) -> bool {
    integer_operand(types, ty)
        || matches!(types.primitive_kind(ty), Some(PrimitiveKind::Real { .. }))
}

fn common_scalar_type(
    types: &TypeRegistry,
    left: TypeRef,
    right: TypeRef,
    builtins: BuiltinTypeContext,
) -> Option<TypeRef> {
    let scalar = numeric_operand(types, left)
        || boolean_operand(types, left)
        || types.ordinal_domain(left).is_some();
    if scalar && types.canonical_type(left) == types.canonical_type(right) {
        return Some(left);
    }
    if numeric_operand(types, left) && numeric_operand(types, right) {
        return common_numeric_type(types, left, right, builtins);
    }
    (types.ordinal_base_type(left) == types.ordinal_base_type(right)).then_some(left)
}

fn common_numeric_type(
    types: &TypeRegistry,
    left: TypeRef,
    right: TypeRef,
    builtins: BuiltinTypeContext,
) -> Option<TypeRef> {
    if !numeric_operand(types, left) || !numeric_operand(types, right) {
        return None;
    }
    if types.canonical_type(left) == types.canonical_type(right) {
        return Some(left);
    }
    if matches!(types.primitive_kind(left), Some(PrimitiveKind::Real { .. }))
        || matches!(
            types.primitive_kind(right),
            Some(PrimitiveKind::Real { .. })
        )
    {
        return (numeric_operand(types, left) && numeric_operand(types, right))
            .then_some(builtins.real);
    }
    let left_domain = types.ordinal_domain(left)?;
    let right_domain = types.ordinal_domain(right)?;
    if left_domain.lower <= right_domain.lower && left_domain.upper >= right_domain.upper {
        Some(left)
    } else if right_domain.lower <= left_domain.lower && right_domain.upper >= left_domain.upper {
        Some(right)
    } else {
        None
    }
}
