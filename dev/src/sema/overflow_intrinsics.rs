// Translated from analyzer_builtins.cpp and constant_expressions.cpp. See NOTICE.
//! Checked integral operands and exact signed-magnitude overflow evaluation.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, TemporaryValue as Value, nonconstant},
    constants::Constant,
    expression_facts::{ExprFacts, ValueCategory},
    graph::ScopeId,
    runtime_intrinsics::RuntimeIntrinsic,
    types::{Scalar, TypeKind},
};
use crate::syntax::ast;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OverflowOperation {
    Add,
    Subtract,
    Multiply,
}

impl Analyzer<'_> {
    pub(super) fn overflow_builtin(
        &mut self,
        id: ast::ExprId,
        arguments: &[ast::ExprId],
        operation: OverflowOperation,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        if arguments.len() != 3 {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "wrong number of arguments to an overflow builtin",
            ));
        }
        for (index, &argument) in arguments.iter().enumerate() {
            let operand = self.typed_expression(argument, scope)?;
            self.check_resolved_function(argument)?;
            let location = self.tree.expressions.get(argument).location;
            let target = if index == 2 {
                let pointer = self.decayed_type(operand.ty)?;
                let TypeKind::Pointer(result) = self.graph.types.get(pointer).kind else {
                    return Err(Error::at(
                        location,
                        "an overflow builtin's result is no pointer to a modifiable integer",
                    ));
                };
                if self.type_cv(result).0 & 1 != 0
                    || !matches!(self.graph.types.get(result).kind,
                        TypeKind::Scalar(scalar) if scalar != Scalar::Bool && super::integral::IntegralKind::of(scalar).is_some())
                {
                    return Err(Error::at(
                        location,
                        "an overflow builtin's result is no pointer to a modifiable integer",
                    ));
                }
                pointer
            } else {
                if !self.is_integral_operand(operand.ty) {
                    return Err(Error::at(
                        location,
                        "an overflow builtin's operand is no integer",
                    ));
                }
                self.promoted_type(operand.ty)?
            };
            self.convert_operand_scalar(argument, target, false)?;
        }
        self.graph
            .expression_facts
            .set_runtime_intrinsic(id, RuntimeIntrinsic::Overflow(operation));
        let boolean = self.graph.types.scalar(Scalar::Bool)?;
        self.new_expression_facts(boolean, ValueCategory::Prvalue)
    }

    pub(super) fn constant_overflow_intrinsic(
        &mut self,
        id: ast::ExprId,
        operation: OverflowOperation,
        scope: ScopeId,
    ) -> Result<Constant, Error> {
        let ast::ExprKind::Call { arguments, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("overflow intrinsic without call"));
        };
        let [left, right, pointer] = *self.tree.expr_lists.get(arguments) else {
            return Err(Error::Invariant(
                "overflow intrinsic without three operands",
            ));
        };
        let a = self.overflow_constant_operand(left, scope)?;
        let b = self.overflow_constant_operand(right, scope)?;
        let pointer = self.overflow_constant_operand(pointer, scope)?;
        let location = self.tree.expressions.get(id).location;
        let (Value::Integer(a), Value::Integer(b)) = (a, b) else {
            return Err(Error::Invariant(
                "overflow intrinsic without integer values",
            ));
        };
        if a.indeterminate() || b.indeterminate() {
            return Err(super::constant_addresses::unreported_constant(location));
        }
        let Value::Pointer(pointer) = pointer else {
            return Err(nonconstant(
                location,
                "an overflow builtin's result through a null pointer",
            ));
        };
        if pointer.root == Root::Null {
            return Err(nonconstant(
                location,
                "an overflow builtin's result through a null pointer",
            ));
        }
        let a = self.exact_overflow_operand(a)?;
        let b = self.exact_overflow_operand(b)?;
        let exact = ExactInteger::calculate(operation, a, b);
        let result = self.bare_type(pointer.designated)?;
        let kind = self
            .integral_kind(result)
            .ok_or(Error::Invariant("overflow result kind"))?;
        let bits = kind.bits - u32::from(kind.signed);
        let limit = if bits == 128 {
            None
        } else {
            Some(1u128 << bits)
        };
        let fits = !exact.carry
            && (!exact.negative || kind.signed || exact.magnitude == 0)
            && limit.is_none_or(|limit| {
                exact.magnitude < limit || exact.negative && kind.signed && exact.magnitude == limit
            });
        let wrapped = if exact.negative {
            exact.magnitude.wrapping_neg()
        } else {
            exact.magnitude
        };
        let stored = self.convert_integer_constant(Constant::from_wide(result, wrapped), result)?;
        self.write_constant_value(
            Address {
                designated: result,
                ..pointer
            },
            Value::Integer(stored),
            location,
        )?;
        self.boolean(!fits)
    }

    fn overflow_constant_operand(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Value, Error> {
        let facts = self.graph.expression_facts.get(id)?;
        let target = facts.conversion.map_or(facts.ty, |step| step.target);
        self.constant_operand_value(id, target, scope)
    }

    fn exact_overflow_operand(&self, value: Constant) -> Result<ExactInteger, Error> {
        let kind = self
            .integral_kind(value.ty)
            .ok_or(Error::Invariant("overflow operand kind"))?;
        let bits = self.wide_integer(value)?;
        let negative = kind.signed && bits >> 127 != 0;
        Ok(ExactInteger {
            negative,
            magnitude: if negative { bits.wrapping_neg() } else { bits },
            carry: false,
        })
    }
}

// Only the low 128 bits and whether the high 128 bits are nonzero are
// observable: operands have at most 128 bits and the exact result is never
// used as another operand. Rust's overflowing arithmetic preserves both.
#[derive(Clone, Copy)]
struct ExactInteger {
    negative: bool,
    magnitude: u128,
    carry: bool,
}
impl ExactInteger {
    fn calculate(operation: OverflowOperation, a: Self, mut b: Self) -> Self {
        if operation == OverflowOperation::Subtract {
            b.negative = !b.negative;
        }
        let (negative, magnitude, carry) = if operation == OverflowOperation::Multiply {
            let (value, carry) = a.magnitude.overflowing_mul(b.magnitude);
            (a.negative != b.negative, value, carry)
        } else if a.negative == b.negative {
            let (value, carry) = a.magnitude.overflowing_add(b.magnitude);
            (a.negative, value, carry)
        } else if a.magnitude < b.magnitude {
            (b.negative, b.magnitude - a.magnitude, false)
        } else {
            (a.negative, a.magnitude - b.magnitude, false)
        };
        Self {
            negative,
            magnitude,
            carry,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{constants::ConstantState, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn overflow_effects_stay_in_each_constant_invocation() {
        let source = SourceBuffer::new(br"
            template<class T>constexpr int product(T x,T y) {
                unsigned char r[2]={11,12};
                bool overflow=__builtin_mul_overflow(x,y,r+1);
                return r[0]+r[1]+(overflow?1000:0);
            }
            constexpr int a=product(3,7);
            constexpr int b=product(20,20);
            constexpr int c=product(-1,1);
            constexpr bool wide() {
                unsigned __int128 r=0;
                bool overflow=__builtin_mul_overflow(~(unsigned __int128)0,~(unsigned __int128)0,&r);
                return overflow&&r==1;
            }
            constexpr bool d=wide();
        ".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"overflow-frames").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"a", b"b", b"c", b"d"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        for (name, bits) in names.into_iter().zip([32, 1155, 1266, 1]) {
            let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
            let entity = analyzer.graph.binding(binding).entity;
            assert!(
                matches!(analyzer.graph.entity(entity).object().constant,ConstantState::Value(value) if value.bits==bits)
            );
        }
        assert!(!analyzer.constexpr_frames.active());
        assert!(analyzer.constant_evaluation.is_none());
    }
}

#[cfg(test)]
mod designation_tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn imported_intrinsics_cannot_become_function_addresses() {
        for name in [
            "__builtin_mul_overflow",
            "__builtin_popcount",
            "__atomic_load_n",
        ] {
            let source = SourceBuffer::new(
                format!("namespace N{{using ::{name};}}auto p=&N::{name};").into_bytes(),
            )
            .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"intrinsic-address").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            assert!(matches!(
                analyzer.analyze(declarations),
                Err(Error::UnconditionalDiagnostic {
                    message: "a builtin expanded where it is called named other than in a call",
                    ..
                })
            ));
        }
    }
}
