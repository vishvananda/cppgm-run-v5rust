// Translated from lower/vector_lowering.cpp. See NOTICE.
//! Vector operations use per-function addressed storage and recorded lane conversions.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lex::pp_lexer::Punctuator as P,
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{
            Binary, Compare, Convert, EightbyteClass as E, ObjectClasses, Projection, Type, Unary,
        },
    },
    sema::{
        Graph,
        lowering::{Reduction, Scalar, TypeId, TypeView},
    },
    syntax::{ast, storage::List},
};

pub(super) fn classes(
    graph: &Graph,
    ty: TypeId,
    element: TypeId,
    bound: u64,
) -> Result<ObjectClasses, Error> {
    let bytes = types::layout(graph, ty)?.0;
    let floating = matches!(graph.lower_type(element)?, TypeView::Scalar(Scalar::Basic(f)) if f.floating_rank() != 0);
    Ok(if bound == 1 && floating {
        ObjectClasses::Memory
    } else if bytes <= 4 {
        ObjectClasses::Direct(if floating { E::Sse } else { E::Integer }, None)
    } else if bytes == 8 {
        ObjectClasses::Direct(E::Sse, None)
    } else if bytes == 16 {
        ObjectClasses::Direct(E::Sse, Some(E::SseUp))
    } else {
        ObjectClasses::Memory
    })
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn vector_builtin_lanes(
        &mut self,
        id: ast::ExprId,
        arguments: &[ast::ExprId],
        kind: crate::lex::vector_lanes::Kind,
        selector: crate::sema::lowering::LaneSelector,
    ) -> Result<RValue, Error> {
        use crate::{lex::vector_lanes::Kind, sema::lowering::LaneSelector};
        let result = self.graph.lower_expression(id)?.ty;
        if kind == Kind::Initialize {
            let (lane, _) = self.vector_shape(result)?;
            let (slot, address) = self.zeroed_vector(result)?;
            for (index, &argument) in arguments.iter().enumerate() {
                let value = self.operand(argument, Context::Argument)?;
                let value = self.fit(value, lane)?;
                let index = u64::try_from(index).map_err(|_| crate::lowir::Error::Limit)?;
                self.set_vector_lane(address, result, index, value)?;
            }
            return self.load_vector_storage(slot, result);
        }
        let vector = self.graph.lower_builtin_operand_type(arguments[0])?;
        let (lane, bound) = self.vector_shape(vector)?;
        let value = self.operand(arguments[0], Context::Argument)?;
        let address = self.vector_storage(value)?;
        let value = if kind == Kind::Insert {
            let value = self.operand(arguments[1], Context::Argument)?;
            Some(self.fit(value, lane)?)
        } else {
            None
        };
        match selector {
            LaneSelector::Constant(selected) => {
                self.discard(arguments[arguments.len() - 1])?;
                if let Some(value) = value {
                    self.set_vector_lane(address, vector, u64::from(selected), value)?;
                } else {
                    return self.vector_lane(address, vector, u64::from(selected));
                }
            }
            LaneSelector::Runtime => {
                let selected = self.operand(arguments[arguments.len() - 1], Context::Argument)?;
                let integer = selected.source;
                let selected = self.fit(selected, integer)?;
                let selected = self.emit(
                    I::Convert(Convert::SignExtend, Type::I64, selected.ty, selected.value),
                    integer,
                )?;
                let mask = self.builder.integer(u128::from(bound - 1))?;
                let selected = self.emit(
                    I::Binary(
                        Binary::And,
                        Facts::default(),
                        Type::I64,
                        selected.value,
                        mask,
                    ),
                    integer,
                )?;
                let storage = types::storage(self.graph, lane)?;
                let place = self
                    .emit(
                        I::Index(
                            storage,
                            Some(Projection::ArrayElement),
                            address,
                            selected.value,
                        ),
                        lane,
                    )?
                    .value;
                if let Some(value) = value {
                    self.write_place(place, lane, value.value)?;
                } else {
                    return self.read_place(place, lane);
                }
            }
        }
        self.emit(
            I::Load(
                types::memory(self.graph, vector)?,
                types::storage(self.graph, vector)?,
                address,
            ),
            result,
        )
    }

    pub(super) fn builtin_value(
        &mut self,
        id: ast::ExprId,
        builtin: ast::ValueBuiltin,
        operand: ast::ExprId,
    ) -> Result<RValue, Error> {
        let to = self.graph.lower_expression(id)?.ty;
        if builtin == ast::ValueBuiltin::BitCast
            && matches!(self.graph.lower_type(to)?, TypeView::Class(_))
        {
            return Err(crate::sema::Error::at(
                self.tree.expressions.get(id).location,
                "a __builtin_bit_cast to a class type",
            )
            .into());
        }
        let from = self.graph.lower_builtin_operand_type(operand)?;
        let value = self.operand(operand, Context::Operand)?;
        if builtin == ast::ValueBuiltin::BitCast {
            let value = self.fit(value, from)?;
            return self.reinterpret_bytes(value, to);
        }
        let (from_lane, _) = self.vector_shape(from)?;
        let (to_lane, bound) = self.vector_shape(to)?;
        let source = self.vector_storage(value)?;
        let (slot, out) = self.zeroed_vector(to)?;
        for index in 0..bound {
            let value = self.vector_lane(source, from, index)?;
            let value = self.convert(
                RValue {
                    source: from_lane,
                    ..value
                },
                to_lane,
                Context::Cast,
            )?;
            self.set_vector_lane(out, to, index, value)?;
        }
        self.load_vector_storage(slot, to)
    }

    pub(super) fn reinterpret_bytes(&mut self, value: RValue, to: TypeId) -> Result<RValue, Error> {
        let slot = self.generated_slot("vector", value.ty)?;
        self.effect(I::Store(Facts::default(), value.ty, value.value, slot))?;
        let address = self.emit(I::Addr(slot), value.source)?.value;
        self.emit(
            I::Load(Facts::default(), types::storage(self.graph, to)?, address),
            to,
        )
    }

    pub(super) fn reduce_vector(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        kind: Reduction,
    ) -> Result<RValue, Error> {
        let vector = self.graph.lower_builtin_operand_type(operand)?;
        let (lane, bound) = self.vector_shape(vector)?;
        if bound > 64 {
            return Err(crate::sema::Error::at(
                self.tree.expressions.get(id).location,
                "a vector operation on more than 64 lanes",
            )
            .into());
        }
        let value = self.operand(operand, Context::Argument)?;
        let source = self.vector_storage(value)?;
        let mut result = self.vector_lane(source, vector, 0)?;
        let soft = super::soft_float::kind(self.graph, lane)?;
        let floating = types::storage(self.graph, lane)?.float_rank() != 0 || soft.is_some();
        let signed = types::signed(self.graph, lane)? || floating;
        for index in 1..bound {
            let x = self.vector_lane(source, vector, index)?;
            let op = match kind {
                Reduction::And => Some(Binary::And),
                Reduction::Or => Some(Binary::Or),
                Reduction::Xor => Some(Binary::Xor),
                Reduction::Add => Some(Binary::Add),
                Reduction::Multiply => Some(Binary::Mul),
                Reduction::Max | Reduction::Min => None,
            };
            if let Some(op) = op {
                result = self.emit(
                    I::Binary(op, Facts::default(), result.ty, result.value, x.value),
                    lane,
                )?;
            } else {
                let best = self.generated_slot("vector", result.ty)?;
                let memory = types::memory(self.graph, lane)?;
                self.effect(I::Store(memory, result.ty, result.value, best))?;
                let predicate = match (kind == Reduction::Max, signed) {
                    (true, true) => Compare::Gt,
                    (true, false) => Compare::Ugt,
                    (false, true) => Compare::Lt,
                    (false, false) => Compare::Ult,
                };
                let take = self.new_block("vector_take")?;
                let join = self.new_block("vector_join")?;
                let unordered = if floating {
                    self.new_block("vector_unordered")?
                } else {
                    join
                };
                let better = if let Some(soft) = soft {
                    self.soft_compare(predicate, soft, x, result, lane)?
                } else {
                    self.emit(I::Compare(predicate, x.ty, x.value, result.value), lane)?
                };
                self.effect(I::Branch(better.value, take, unordered))?;
                if floating {
                    self.place(unordered)?;
                    let nan = if let Some(soft) = soft {
                        self.soft_compare(Compare::Ne, soft, result, result, lane)?
                    } else {
                        self.emit(
                            I::Compare(Compare::Ne, result.ty, result.value, result.value),
                            lane,
                        )?
                    };
                    self.effect(I::Branch(nan.value, take, join))?;
                }
                self.place(take)?;
                self.effect(I::Store(memory, result.ty, x.value, best))?;
                self.jump(join)?;
                self.place(join)?;
                result = self.emit(I::Load(memory, result.ty, best), lane)?;
            }
        }
        Ok(result)
    }

    fn zeroed_vector(&mut self, ty: TypeId) -> Result<(Value, Value), Error> {
        let TypeView::Vector { bound, .. } = self.graph.lower_type(ty)? else {
            return Err(crate::sema::Error::Invariant("vector storage type").into());
        };
        if bound > 64 {
            return Err(crate::sema::Error::at(
                crate::support::source::SourceOffset(0),
                "a vector operation on more than 64 lanes",
            )
            .into());
        }
        let storage = types::storage(self.graph, ty)?;
        let (bytes, align) = types::layout(self.graph, ty)?;
        let slot = self.generated_slot("vector", storage)?;
        self.effect(I::ZeroInit(bytes, align, slot))?;
        let address = self.emit(I::Addr(slot), ty)?.value;
        Ok((slot, address))
    }
    pub(in crate::lower) fn vector_list(
        &mut self,
        ty: TypeId,
        elements: List<ast::ExprId>,
    ) -> Result<RValue, Error> {
        let TypeView::Vector { element: lane, .. } = self.graph.lower_type(ty)? else {
            return Err(crate::sema::Error::Invariant("vector list type").into());
        };
        let (slot, address) = self.zeroed_vector(ty)?;
        for (index, &expression) in self.tree.expr_lists.get(elements).iter().enumerate() {
            let index = u64::try_from(index).map_err(|_| crate::lowir::Error::Limit)?;
            let value = self.operand(expression, Context::Initialization)?;
            let value = self.fit(value, lane)?;
            self.set_vector_lane(address, ty, index, value)?;
        }
        self.load_vector_storage(slot, ty)
    }
    fn load_vector_storage(&mut self, slot: Value, vector: TypeId) -> Result<RValue, Error> {
        self.emit(
            I::Load(Facts::default(), types::storage(self.graph, vector)?, slot),
            vector,
        )
    }
    pub(super) fn vector_compound_assignment(
        &mut self,
        op: P,
        operands: [ast::ExprId; 2],
        types: [TypeId; 2],
    ) -> Result<(Value, RValue), Error> {
        let [left, right] = operands;
        let [source, operation] = types;
        let rhs = self.operand(right, Context::CompoundAssignment)?;
        let place = self.scalar_place(left)?;
        let (state, old) =
            if let Some((state, old)) = self.begin_atomic_place_loop(place, source)? {
                (Some(state), old)
            } else {
                (None, self.read_place(place, source)?)
            };
        let lhs = self.convert(old, operation, Context::CompoundAssignment)?;
        let right_vector = matches!(
            self.graph
                .lower_type(self.graph.lower_builtin_operand_type(right)?)?,
            TypeView::Vector { .. }
        );
        let value = self.vector_operation(op, operation, [(lhs, true), (rhs, right_vector)])?;
        let value = self.convert(value, source, Context::CompoundAssignment)?;
        let value = self.fit(value, source)?;
        if let Some(state) = state {
            self.end_atomic_place_loop(place, source, state, value)?;
        } else {
            self.write_place(place, source, value.value)?;
        }
        Ok((place, value))
    }
    fn vector_shape(&self, ty: TypeId) -> Result<(TypeId, u64), Error> {
        match self.graph.lower_type(ty)? {
            TypeView::Vector { element, bound } => Ok((element, bound)),
            _ => Err(crate::sema::Error::Invariant("vector operation type").into()),
        }
    }
    fn vector_storage(&mut self, value: RValue) -> Result<Value, Error> {
        let slot = self.generated_slot("vector", value.ty)?;
        self.effect(I::Store(Facts::default(), value.ty, value.value, slot))?;
        Ok(self.emit(I::Addr(slot), value.source)?.value)
    }
    fn set_vector_lane(
        &mut self,
        address: Value,
        vector: TypeId,
        index: u64,
        value: RValue,
    ) -> Result<(), Error> {
        let (lane, _) = self.vector_shape(vector)?;
        let boolean = types::boolean(self.graph, lane)?;
        let offset = self.builder.integer(u128::from(if boolean {
            index / 8
        } else {
            index * types::layout(self.graph, lane)?.0
        }))?;
        let place = self
            .emit(
                I::Index(Type::I8, Some(Projection::Field), address, offset),
                lane,
            )?
            .value;
        let value = if boolean {
            let bit = if value.ty == Type::U8 {
                value.value
            } else {
                self.emit(
                    I::Convert(Convert::Truncate, Type::U8, value.ty, value.value),
                    lane,
                )?
                .value
            };
            let bit = if index.is_multiple_of(8) {
                bit
            } else {
                let shift = self.builder.integer(u128::from(index % 8))?;
                self.emit(
                    I::Binary(Binary::Shl, Facts::default(), Type::U8, bit, shift),
                    lane,
                )?
                .value
            };
            let old = self.read_place(place, lane)?;
            self.emit(
                I::Binary(Binary::Or, Facts::default(), Type::U8, old.value, bit),
                lane,
            )?
            .value
        } else {
            value.value
        };
        self.write_place(place, lane, value)
    }
    fn vector_lane(&mut self, address: Value, vector: TypeId, index: u64) -> Result<RValue, Error> {
        let (lane, _) = self.vector_shape(vector)?;
        let boolean = types::boolean(self.graph, lane)?;
        let offset = self.builder.integer(u128::from(if boolean {
            index / 8
        } else {
            index * types::layout(self.graph, lane)?.0
        }))?;
        let place = self
            .emit(
                I::Index(Type::I8, Some(Projection::Field), address, offset),
                lane,
            )?
            .value;
        let value = self.read_place(place, lane)?;
        if !boolean {
            return Ok(value);
        }
        let value = if index.is_multiple_of(8) {
            value
        } else {
            let shift = self.builder.integer(u128::from(index % 8))?;
            self.emit(
                I::Binary(Binary::UShr, Facts::default(), Type::U8, value.value, shift),
                lane,
            )?
        };
        let one = self.builder.integer(1)?;
        self.emit(
            I::Binary(Binary::And, Facts::default(), Type::U8, value.value, one),
            lane,
        )
    }
    pub(super) fn vector_operation(
        &mut self,
        op: P,
        vector: TypeId,
        operands: [(RValue, bool); 2],
    ) -> Result<RValue, Error> {
        let op = match op {
            P::AddAssign => P::Add,
            P::SubtractAssign => P::Subtract,
            P::MultiplyAssign => P::Multiply,
            P::DivideAssign => P::Divide,
            P::RemainderAssign => P::Remainder,
            P::AndAssign => P::BitAnd,
            P::OrAssign => P::BitOr,
            P::XorAssign => P::Xor,
            P::ShiftLeftAssign => P::ShiftLeft,
            P::ShiftRightAssign => P::ShiftRight,
            other => other,
        };
        let [(left, left_vector), (right, right_vector)] = operands;
        let (lane, bound) = self.vector_shape(vector)?;
        let a = if left_vector {
            Some(self.vector_storage(left)?)
        } else {
            None
        };
        let b = if right_vector {
            Some(self.vector_storage(right)?)
        } else {
            None
        };
        let (slot, out) = self.zeroed_vector(vector)?;
        for index in 0..bound {
            let x = if let Some(a) = a {
                self.vector_lane(a, vector, index)?
            } else {
                left
            };
            let y = if let Some(b) = b {
                self.vector_lane(b, vector, index)?
            } else {
                right
            };
            let value = self.scalar_arithmetic(op, lane, x, y)?;
            self.set_vector_lane(out, vector, index, value)?;
        }
        self.load_vector_storage(slot, vector)
    }
    pub(super) fn vector_compare(
        &mut self,
        op: P,
        result: TypeId,
        operands: [(RValue, bool); 2],
    ) -> Result<RValue, Error> {
        let [(left, left_vector), (right, right_vector)] = operands;
        let vector = if left_vector {
            left.source
        } else {
            right.source
        };
        let (lane, bound) = self.vector_shape(vector)?;
        let (truth_lane, _) = self.vector_shape(result)?;
        let boolean = types::boolean(self.graph, truth_lane)?;
        let truth_type = types::storage(self.graph, truth_lane)?;
        let soft = super::soft_float::kind(self.graph, lane)?;
        let signed = types::signed(self.graph, lane)?
            || types::storage(self.graph, lane)?.float_rank() != 0
            || soft.is_some();
        let predicate = match (op, signed) {
            (P::Equal, _) => Compare::Eq,
            (P::NotEqual, _) => Compare::Ne,
            (P::Less, true) => Compare::Lt,
            (P::Less, false) => Compare::Ult,
            (P::LessEqual, true) => Compare::Le,
            (P::LessEqual, false) => Compare::Ule,
            (P::Greater, true) => Compare::Gt,
            (P::Greater, false) => Compare::Ugt,
            (_, true) => Compare::Ge,
            (_, false) => Compare::Uge,
        };
        let a = if left_vector {
            Some(self.vector_storage(left)?)
        } else {
            None
        };
        let b = if right_vector {
            Some(self.vector_storage(right)?)
        } else {
            None
        };
        let (slot, out) = self.zeroed_vector(result)?;
        for index in 0..bound {
            let x = if let Some(a) = a {
                self.vector_lane(a, vector, index)?
            } else {
                left
            };
            let y = if let Some(b) = b {
                self.vector_lane(b, vector, index)?
            } else {
                right
            };
            let truth = if let Some(kind) = soft {
                self.soft_compare(predicate, kind, x, y, truth_lane)?
            } else {
                self.emit(I::Compare(predicate, x.ty, x.value, y.value), truth_lane)?
            };
            let value = if boolean {
                truth
            } else {
                let zero = self.builder.integer(0)?;
                let mask = self.emit(
                    I::Binary(Binary::Sub, Facts::default(), Type::I64, zero, truth.value),
                    truth_lane,
                )?;
                if truth_type == Type::I64 {
                    mask
                } else {
                    self.emit(
                        I::Convert(Convert::Truncate, truth_type, Type::I64, mask.value),
                        truth_lane,
                    )?
                }
            };
            self.set_vector_lane(out, result, index, value)?;
        }
        self.load_vector_storage(slot, result)
    }
    pub(super) fn vector_unary(
        &mut self,
        op: P,
        vector: TypeId,
        value: RValue,
    ) -> Result<RValue, Error> {
        let (lane, bound) = self.vector_shape(vector)?;
        let address = self.vector_storage(value)?;
        let (slot, out) = self.zeroed_vector(vector)?;
        for index in 0..bound {
            let x = self.vector_lane(address, vector, index)?;
            let value = if op == P::Complement {
                self.emit(I::Unary(Unary::BitNot, x.ty, x.value), lane)?
            } else if let Some(kind) = super::soft_float::kind(self.graph, lane)? {
                self.soft_negate(kind, x)?
            } else {
                self.emit(I::Unary(Unary::Neg, x.ty, x.value), lane)?
            };
            self.set_vector_lane(out, vector, index, value)?;
        }
        self.load_vector_storage(slot, vector)
    }
    fn vector_index(&mut self, index: ast::ExprId) -> Result<RValue, Error> {
        let value = self.expression(index, Context::Operand)?;
        self.convert_integer(
            RValue {
                literal: false,
                ..value
            },
            value.source,
            Type::I64,
            true,
            Context::Additive,
        )
    }
    fn vector_index_place(
        &mut self,
        address: Value,
        index: ast::ExprId,
        lane: TypeId,
    ) -> Result<Value, Error> {
        let offset = self.vector_index(index)?;
        let size = self
            .builder
            .integer(u128::from(types::layout(self.graph, lane)?.0))?;
        let offset = self.emit(
            I::Binary(Binary::Mul, Facts::default(), Type::I64, offset.value, size),
            offset.source,
        )?;
        Ok(self
            .emit(
                I::Index(
                    Type::I8,
                    Some(Projection::ArrayElement),
                    address,
                    offset.value,
                ),
                lane,
            )?
            .value)
    }
    pub(super) fn vector_lane_place(
        &mut self,
        id: ast::ExprId,
        base: ast::ExprId,
        index: ast::ExprId,
    ) -> Result<Value, Error> {
        let lane = self.graph.lower_expression(id)?.ty;
        let base = self.without_parentheses(base);
        let address = self.address(base)?;
        self.vector_index_place(address, index, lane)
    }
    pub(super) fn vector_lane_value(
        &mut self,
        id: ast::ExprId,
        base: ast::ExprId,
        index: ast::ExprId,
    ) -> Result<RValue, Error> {
        let lane = self.graph.lower_expression(id)?.ty;
        let base = self.without_parentheses(base);
        let facts = self.graph.lower_expression(base)?;
        let boolean = types::boolean(self.graph, lane)?;
        if !boolean && facts.is_place {
            let place = self.vector_lane_place(id, base, index)?;
            return self.read_place(place, lane);
        }
        let address = if facts.is_place {
            self.address(base)?
        } else {
            let value = self.expression(base, Context::Operand)?;
            self.vector_storage(value)?
        };
        if !boolean {
            let place = self.vector_index_place(address, index, lane)?;
            return self.read_place(place, lane);
        }
        let position = self.vector_index(index)?;
        let three = self.builder.integer(3)?;
        let offset = self.emit(
            I::Binary(
                Binary::Shr,
                Facts::default(),
                Type::I64,
                position.value,
                three,
            ),
            lane,
        )?;
        let place = self
            .emit(
                I::Index(
                    Type::I8,
                    Some(Projection::ArrayElement),
                    address,
                    offset.value,
                ),
                lane,
            )?
            .value;
        let seven = self.builder.integer(7)?;
        let shift = self.emit(
            I::Binary(
                Binary::And,
                Facts::default(),
                Type::I64,
                position.value,
                seven,
            ),
            lane,
        )?;
        let shift = self.emit(
            I::Convert(Convert::Truncate, Type::U8, Type::I64, shift.value),
            lane,
        )?;
        let byte = self.read_place(place, lane)?;
        let bit = self.emit(
            I::Binary(
                Binary::UShr,
                Facts::default(),
                Type::U8,
                byte.value,
                shift.value,
            ),
            lane,
        )?;
        let one = self.builder.integer(1)?;
        self.emit(
            I::Binary(Binary::And, Facts::default(), Type::U8, bit.value, one),
            lane,
        )
    }
}
