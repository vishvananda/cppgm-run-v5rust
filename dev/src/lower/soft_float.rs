// Translated from lower/soft_float_lowering.cpp. See NOTICE.
//! Hosted floating values retain their bytes; runtime helpers own arithmetic.
use crate::lower::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    symbols::Symbols,
    types,
};
use crate::{
    lex::pp_lexer::Punctuator as P,
    lowir::{
        model::{
            Boundary, Call, Facts, Function as IrFunction, InstructionKind as I, Linkage, Metadata,
            Parameter, Program, Signature, SymbolId, Unwind, Value,
        },
        vocabulary::{
            Binary, Binding, Compare, Convert, EightbyteClass, ObjectClasses, Projection, Type,
            Unary,
        },
    },
    sema::{
        Graph,
        lowering::{Scalar, TypeId, TypeView},
    },
    syntax::ast::{self, ExprKind},
    types::FundamentalType as F,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::lower) enum Kind {
    Half,
    BFloat,
    Quad,
}
pub(in crate::lower) fn kind(graph: &Graph, ty: TypeId) -> Result<Option<Kind>, Error> {
    Ok(match graph.lower_type(ty)? {
        TypeView::Scalar(Scalar::Basic(F::Float16)) => Some(Kind::Half),
        TypeView::Scalar(Scalar::Basic(F::BFloat16)) => Some(Kind::BFloat),
        TypeView::Scalar(Scalar::Basic(F::Float128 | F::GnuFloat128)) => Some(Kind::Quad),
        _ => None,
    })
}
pub(in crate::lower) fn storage(kind: Kind) -> Type {
    if kind == Kind::Quad {
        Type::Object {
            bytes: 16,
            align: 4,
        }
    } else {
        Type::Object { bytes: 2, align: 1 }
    }
}
pub(in crate::lower) fn classes(kind: Kind) -> ObjectClasses {
    ObjectClasses::Direct(
        EightbyteClass::Sse,
        (kind == Kind::Quad).then_some(EightbyteClass::SseUp),
    )
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::lower) enum Helper {
    AddTF,
    SubTF,
    MulTF,
    DivTF,
    NegTF,
    EqTF,
    NeTF,
    LtTF,
    LeTF,
    GtTF,
    GeTF,
    ExtendSFTF,
    ExtendDFTF,
    ExtendXFTF,
    TruncTFSF,
    TruncTFDF,
    TruncTFXF,
    FloatSITF,
    FloatUnsSITF,
    FloatDITF,
    FloatUnsDITF,
    FloatTITF,
    FloatUnsTITF,
    FixTFSI,
    FixUnsTFSI,
    FixTFDI,
    FixUnsTFDI,
    FixTFTI,
    FixUnsTFTI,
    ExtendHFSF,
    ExtendHFDF,
    ExtendHFXF,
    ExtendHFTF,
    TruncSFHF,
    TruncDFHF,
    TruncXFHF,
    TruncTFHF,
    FloatTIHF,
    FloatUnsTIHF,
    FixHFTI,
    FixUnsHFTI,
    TruncSFBF,
    TruncDFBF,
    TruncXFBF,
    TruncTFBF,
}
const SPECS: [(&str, &[u8], u8); 45] = [
    ("__addtf3", b"qq", b'q'),
    ("__subtf3", b"qq", b'q'),
    ("__multf3", b"qq", b'q'),
    ("__divtf3", b"qq", b'q'),
    ("__negtf2", b"q", b'q'),
    ("__eqtf2", b"qq", b'l'),
    ("__netf2", b"qq", b'l'),
    ("__lttf2", b"qq", b'l'),
    ("__letf2", b"qq", b'l'),
    ("__gttf2", b"qq", b'l'),
    ("__getf2", b"qq", b'l'),
    ("__extendsftf2", b"f", b'q'),
    ("__extenddftf2", b"d", b'q'),
    ("__extendxftf2", b"x", b'q'),
    ("__trunctfsf2", b"q", b'f'),
    ("__trunctfdf2", b"q", b'd'),
    ("__trunctfxf2", b"q", b'x'),
    ("__floatsitf", b"i", b'q'),
    ("__floatunsitf", b"u", b'q'),
    ("__floatditf", b"l", b'q'),
    ("__floatunditf", b"l", b'q'),
    ("__floattitf", b"t", b'q'),
    ("__floatuntitf", b"t", b'q'),
    ("__fixtfsi", b"q", b'i'),
    ("__fixunstfsi", b"q", b'u'),
    ("__fixtfdi", b"q", b'l'),
    ("__fixunstfdi", b"q", b'l'),
    ("__fixtfti", b"q", b't'),
    ("__fixunstfti", b"q", b't'),
    ("__extendhfsf2", b"h", b'f'),
    ("__extendhfdf2", b"h", b'd'),
    ("__extendhfxf2", b"h", b'x'),
    ("__extendhftf2", b"h", b'q'),
    ("__truncsfhf2", b"f", b'h'),
    ("__truncdfhf2", b"d", b'h'),
    ("__truncxfhf2", b"x", b'h'),
    ("__trunctfhf2", b"q", b'h'),
    ("__floattihf", b"t", b'h'),
    ("__floatuntihf", b"t", b'h'),
    ("__fixhfti", b"h", b't'),
    ("__fixunshfti", b"h", b't'),
    ("__truncsfbf2", b"f", b'b'),
    ("__truncdfbf2", b"d", b'b'),
    ("__truncxfbf2", b"x", b'b'),
    ("__trunctfbf2", b"q", b'b'),
];
fn spec_type(code: u8) -> Type {
    match code {
        b'h' | b'b' => storage(Kind::Half),
        b'q' => storage(Kind::Quad),
        b'f' => Type::F32,
        b'd' => Type::F64,
        b'x' => Type::F80,
        b'i' => Type::I32,
        b'u' => Type::U32,
        b't' => Type::I128,
        _ => Type::I64,
    }
}
fn spec_classes(code: u8, object: bool) -> Option<ObjectClasses> {
    if !object {
        return None;
    }
    match code {
        b'h' | b'b' => Some(classes(Kind::Half)),
        b'q' => Some(classes(Kind::Quad)),
        _ => None,
    }
}
impl Symbols<'_, '_> {
    fn soft_helper(&mut self, program: &mut Program, helper: Helper) -> Result<SymbolId, Error> {
        if let Some(&symbol) = self.soft_float_helpers.get(&helper) {
            return Ok(symbol);
        }
        let (name, parameters, result) = SPECS[helper as usize];
        let symbol = self.table.allocate(
            program,
            &format!("__cppgm_libgcc_{}", &name[2..]),
            None,
            true,
        )?;
        let mut params = Vec::with_capacity(parameters.len());
        for (index, &code) in parameters.iter().enumerate() {
            params.push(Parameter {
                name: program.names.intern(&format!("arg{index}"))?,
                ty: spec_type(code),
                pass: None,
                object_bytes: 0,
                no_alias: false,
                classes: spec_classes(code, self.object_mode()),
            });
        }
        let signature = program.add_signature(Signature {
            parameters: params,
            result: spec_type(result),
            boundary: Boundary {
                result_classes: spec_classes(result, self.object_mode()),
                unwind: Unwind::No,
                ..Boundary::default()
            },
        })?;
        let object = program.names.intern(name)?;
        program.add(
            symbol,
            IrFunction {
                name: program.symbol(symbol).name,
                signature,
                metadata: Metadata {
                    linkage: Linkage::C,
                    binding: Some(Binding::Strong),
                    object: Some(object),
                    ..Metadata::default()
                },
                body: None,
                location: None,
            },
        )?;
        program.publish_runtime_declaration(symbol);
        self.soft_float_helpers.insert(helper, symbol);
        Ok(symbol)
    }
}
impl Function<'_, '_, '_, '_> {
    fn soft_call(
        &mut self,
        helper: Helper,
        first: RValue,
        second: Option<RValue>,
        source: TypeId,
    ) -> Result<RValue, Error> {
        let callee = self.symbols.soft_helper(self.builder.program(), helper)?;
        let arguments = if let Some(second) = second {
            self.builder.arguments(&[first.value, second.value])?
        } else {
            self.builder.arguments(&[first.value])?
        };
        self.emit(
            I::Call(Call {
                result: spec_type(SPECS[helper as usize].2),
                callee: Value::Symbol(callee),
                arguments,
                signature: None,
                copy_elision: false,
                argument_classes: Vec::new(),
            }),
            source,
        )
    }
    pub(in crate::lower) fn soft_constant(
        &mut self,
        kind: Kind,
        bits: u128,
        source: TypeId,
    ) -> Result<RValue, Error> {
        // Frozen soft_constant uses the quad-sized holder for bfloat constants,
        // while their declared storage and helper signatures stay two bytes.
        let ty = storage(if kind == Kind::Half {
            Kind::Half
        } else {
            Kind::Quad
        });
        let slot = self.generated_slot("soft_float", ty)?;
        let address = self.emit(I::Addr(slot), source)?.value;
        let low = self.builder.integer(if kind == Kind::Quad {
            i128::from(
                (u64::try_from(bits & u128::from(u64::MAX))
                    .map_err(|_| crate::lowir::Error::Limit)?)
                .cast_signed(),
            )
            .cast_unsigned()
        } else {
            bits
        })?;
        let word = if kind == Kind::Quad {
            Type::I64
        } else {
            Type::U16
        };
        self.effect(I::Store(Facts::memory(false, true), word, low, address))?;
        if kind == Kind::Quad {
            let offset = self.builder.integer(8)?;
            let address = self
                .emit(
                    I::Index(Type::I8, Some(Projection::Field), address, offset),
                    source,
                )?
                .value;
            let high = self.builder.integer(
                i128::from(
                    (u64::try_from(bits >> 64).map_err(|_| crate::lowir::Error::Limit)?)
                        .cast_signed(),
                )
                .cast_unsigned(),
            )?;
            self.effect(I::Store(Facts::memory(false, true), word, high, address))?;
        }
        self.emit(I::Load(Facts::memory(false, false), ty, slot), source)
    }
    fn soft_convert_op(
        &mut self,
        op: Convert,
        to: Type,
        value: RValue,
        source: TypeId,
    ) -> Result<RValue, Error> {
        self.emit(I::Convert(op, to, value.ty, value.value), source)
    }
    fn soft_binary(&mut self, op: Binary, left: RValue, right: RValue) -> Result<RValue, Error> {
        self.emit(
            I::Binary(op, Facts::default(), left.ty, left.value, right.value),
            left.source,
        )
    }
    pub(in crate::lower) fn bfloat_widened(&mut self, value: RValue) -> Result<RValue, Error> {
        let slot = self.generated_slot("soft_float", value.ty)?;
        self.effect(I::Store(
            Facts::memory(false, false),
            value.ty,
            value.value,
            slot,
        ))?;
        let address = self.emit(I::Addr(slot), value.source)?.value;
        let bits = self.emit(
            I::Load(Facts::memory(false, true), Type::U16, address),
            value.source,
        )?;
        let upper = self.soft_convert_op(Convert::ZeroExtend, Type::I32, bits, value.source)?;
        let shift = self.builder.integer(16)?;
        let upper = self.emit(
            I::Binary(Binary::Shl, Facts::default(), Type::I32, upper.value, shift),
            value.source,
        )?;
        let word = self.generated_slot("soft_float", Type::I32)?;
        self.effect(I::Store(
            Facts::memory(false, true),
            Type::I32,
            upper.value,
            word,
        ))?;
        self.emit(
            I::Load(Facts::memory(false, true), Type::F32, word),
            value.source,
        )
    }
    pub(in crate::lower) fn soft_convert(
        &mut self,
        value: RValue,
        target: TypeId,
    ) -> Result<RValue, Error> {
        use Helper as H;
        let from = kind(self.graph, value.source)?;
        let to = kind(self.graph, target)?;
        if from == Some(Kind::BFloat) || to == Some(Kind::BFloat) {
            return self.bfloat_convert(value, target);
        }
        if let (Some(from), Some(to)) = (from, to) {
            return if from == to {
                Ok(RValue {
                    source: target,
                    ..value
                })
            } else {
                self.soft_call(
                    if to == Kind::Quad {
                        H::ExtendHFTF
                    } else {
                        H::TruncTFHF
                    },
                    value,
                    None,
                    target,
                )
            };
        }
        if let Some(to) = to {
            if value.ty.float_rank() != 0 {
                return self.soft_call(floating_helper(value.ty, to, true), value, None, target);
            }
            let signed = types::signed(self.graph, value.source)?;
            let bits = value.ty.bits();
            if bits == 128 {
                return self.soft_call(
                    match (to, signed) {
                        (Kind::Quad, true) => H::FloatTITF,
                        (Kind::Quad, false) => H::FloatUnsTITF,
                        (_, true) => H::FloatTIHF,
                        _ => H::FloatUnsTIHF,
                    },
                    value,
                    None,
                    target,
                );
            }
            if to == Kind::Half {
                let single = self.soft_convert_op(
                    if signed {
                        Convert::SignedToFloat
                    } else {
                        Convert::UnsignedToFloat
                    },
                    Type::F32,
                    value,
                    value.source,
                )?;
                return self.soft_call(H::TruncSFHF, single, None, target);
            }
            if bits == 64 {
                return self.soft_call(
                    if signed {
                        H::FloatDITF
                    } else {
                        H::FloatUnsDITF
                    },
                    value,
                    None,
                    target,
                );
            }
            if bits == 32 && !signed {
                return self.soft_call(H::FloatUnsSITF, value, None, target);
            }
            let integer =
                self.convert_integer(value, value.source, Type::I32, true, Context::Operand)?;
            return self.soft_call(H::FloatSITF, integer, None, target);
        }
        let from = from.ok_or(crate::sema::Error::Invariant(
            "soft conversion without kind",
        ))?;
        let storage = types::storage(self.graph, target)?;
        if storage.float_rank() != 0 {
            return self.soft_call(floating_helper(storage, from, false), value, None, target);
        }
        let signed = types::signed(self.graph, target)?;
        let bits = storage.bits();
        if bits == 128 {
            return self.soft_call(
                match (from, signed) {
                    (Kind::Quad, true) => H::FixTFTI,
                    (Kind::Quad, false) => H::FixUnsTFTI,
                    (_, true) => H::FixHFTI,
                    _ => H::FixUnsHFTI,
                },
                value,
                None,
                target,
            );
        }
        if from == Kind::Half {
            let single = self.soft_call(H::ExtendHFSF, value, None, value.source)?;
            return self.soft_convert_op(
                if signed {
                    Convert::FloatToSigned
                } else {
                    Convert::FloatToUnsigned
                },
                storage,
                single,
                target,
            );
        }
        if bits == 64 {
            return self.soft_call(
                if signed { H::FixTFDI } else { H::FixUnsTFDI },
                value,
                None,
                target,
            );
        }
        if bits == 32 && !signed {
            return self.soft_call(H::FixUnsTFSI, value, None, target);
        }
        let whole = self.soft_call(H::FixTFSI, value, None, target)?;
        if bits == 32 {
            Ok(whole)
        } else {
            self.soft_convert_op(Convert::Truncate, storage, whole, target)
        }
    }
    fn bfloat_convert(&mut self, value: RValue, target: TypeId) -> Result<RValue, Error> {
        use Helper as H;
        let from = kind(self.graph, value.source)?;
        let to = kind(self.graph, target)?;
        if from == to {
            return Ok(RValue {
                source: target,
                ..value
            });
        }
        if to == Some(Kind::BFloat) {
            let helper = match from {
                Some(Kind::Quad) => return self.soft_call(H::TruncTFBF, value, None, target),
                Some(Kind::Half) => {
                    let single = self.soft_call(H::ExtendHFSF, value, None, value.source)?;
                    return self.soft_call(H::TruncSFBF, single, None, target);
                }
                _ => match value.ty {
                    Type::F32 => H::TruncSFBF,
                    Type::F64 => H::TruncDFBF,
                    Type::F80 => H::TruncXFBF,
                    _ => {
                        let signed = types::signed(self.graph, value.source)?;
                        let single = self.soft_convert_op(
                            if signed {
                                Convert::SignedToFloat
                            } else {
                                Convert::UnsignedToFloat
                            },
                            Type::F32,
                            value,
                            value.source,
                        )?;
                        return self.soft_call(H::TruncSFBF, single, None, target);
                    }
                },
            };
            return self.soft_call(helper, value, None, target);
        }
        let wide = self.bfloat_widened(value)?;
        if to == Some(Kind::Half) {
            return self.soft_call(H::TruncSFHF, wide, None, target);
        }
        if to == Some(Kind::Quad) {
            return self.soft_call(H::ExtendSFTF, wide, None, target);
        }
        let ty = types::storage(self.graph, target)?;
        if ty == Type::F32 {
            return Ok(RValue {
                source: target,
                ..wide
            });
        }
        self.soft_convert_op(
            if ty.float_rank() != 0 {
                Convert::FloatExtend
            } else if types::signed(self.graph, target)? {
                Convert::FloatToSigned
            } else {
                Convert::FloatToUnsigned
            },
            ty,
            wide,
            target,
        )
    }
    pub(in crate::lower) fn soft_arithmetic(
        &mut self,
        op: Binary,
        kind: Kind,
        a: RValue,
        b: RValue,
        source: TypeId,
    ) -> Result<RValue, Error> {
        use Helper as H;
        if kind == Kind::Quad {
            let helper = match op {
                Binary::Add => H::AddTF,
                Binary::Sub => H::SubTF,
                Binary::Mul => H::MulTF,
                Binary::Div => H::DivTF,
                _ => return Err(crate::sema::Error::Invariant("soft arithmetic operator").into()),
            };
            return self.soft_call(helper, a, Some(b), source);
        }
        let (a, b, helper) = if kind == Kind::BFloat {
            let b = self.bfloat_widened(b)?;
            (self.bfloat_widened(a)?, b, H::TruncSFBF)
        } else {
            let a = self.soft_call(H::ExtendHFSF, a, None, a.source)?;
            let b = self.soft_call(H::ExtendHFSF, b, None, b.source)?;
            (a, b, H::TruncSFHF)
        };
        let result = self.soft_binary(op, a, b)?;
        self.soft_call(helper, result, None, source)
    }
    pub(in crate::lower) fn soft_negate(
        &mut self,
        kind: Kind,
        value: RValue,
    ) -> Result<RValue, Error> {
        use Helper as H;
        if kind == Kind::Quad {
            return self.soft_call(H::NegTF, value, None, value.source);
        }
        let wide = if kind == Kind::BFloat {
            self.bfloat_widened(value)?
        } else {
            self.soft_call(H::ExtendHFSF, value, None, value.source)?
        };
        let negative = self.emit(I::Unary(Unary::Neg, wide.ty, wide.value), value.source)?;
        self.soft_call(
            if kind == Kind::BFloat {
                H::TruncSFBF
            } else {
                H::TruncSFHF
            },
            negative,
            None,
            value.source,
        )
    }
    pub(in crate::lower) fn soft_compare(
        &mut self,
        predicate: Compare,
        kind: Kind,
        a: RValue,
        b: RValue,
        source: TypeId,
    ) -> Result<RValue, Error> {
        use Helper as H;
        if kind != Kind::Quad {
            let b = if kind == Kind::Half {
                self.soft_call(H::ExtendHFSF, b, None, b.source)?
            } else {
                self.bfloat_widened(b)?
            };
            let a = if kind == Kind::Half {
                self.soft_call(H::ExtendHFSF, a, None, a.source)?
            } else {
                self.bfloat_widened(a)?
            };
            return self.emit(I::Compare(predicate, Type::F32, a.value, b.value), source);
        }
        let helper = match predicate {
            Compare::Eq => H::EqTF,
            Compare::Ne => H::NeTF,
            Compare::Lt => H::LtTF,
            Compare::Le => H::LeTF,
            Compare::Gt => H::GtTF,
            Compare::Ge => H::GeTF,
            _ => return Err(crate::sema::Error::Invariant("unsigned soft comparison").into()),
        };
        let result = self.soft_call(helper, a, Some(b), source)?;
        let zero = self.builder.integer(0)?;
        self.emit(I::Compare(predicate, Type::I64, result.value, zero), source)
    }
    pub(in crate::lower) fn bfloat_operation(&self, id: ast::ExprId) -> Result<bool, Error> {
        let id = self.without_parentheses(id);
        if self.graph.lower_operator_call(id)?
            || kind(self.graph, self.graph.lower_expression(id)?.ty)? != Some(Kind::BFloat)
        {
            return Ok(false);
        }
        Ok(match self.tree.expressions.get(id).kind {
            ExprKind::Binary {
                op,
                assignment: false,
                ..
            } => matches!(
                op.token.as_punctuator(),
                Some(P::Add | P::Subtract | P::Multiply | P::Divide)
            ),
            ExprKind::Unary { op, .. } => {
                matches!(op.token.as_punctuator(), Some(P::Add | P::Subtract))
            }
            _ => false,
        })
    }
    pub(in crate::lower) fn bfloat_promoted(&mut self, id: ast::ExprId) -> Result<RValue, Error> {
        self.check_stack()?;
        if !self.bfloat_operation(id)? {
            let value = self.operand(id, Context::Operand)?;
            return self.bfloat_widened(value);
        }
        let id = self.without_parentheses(id);
        if let ExprKind::Unary { op, operand } = self.tree.expressions.get(id).kind {
            let value = self.bfloat_promoted(operand)?;
            return if op.token.as_punctuator() == Some(P::Subtract) {
                self.emit(I::Unary(Unary::Neg, Type::F32, value.value), value.source)
            } else {
                Ok(value)
            };
        }
        // Iterative left spine keeps long chains within one function's scratch.
        let mut chain = Vec::new();
        let mut next = id;
        while let ExprKind::Binary {
            left, right, op, ..
        } = self.tree.expressions.get(next).kind
        {
            if !self.bfloat_operation(next)? {
                break;
            }
            chain.push((left, right, op));
            next = self.without_parentheses(left);
        }
        let mut value = self.bfloat_promoted(next)?;
        for &(_, right, op) in chain.iter().rev() {
            let right = self.bfloat_promoted(right)?;
            value = self.soft_binary(arithmetic(op.token.as_punctuator())?, value, right)?;
        }
        Ok(value)
    }
    pub(in crate::lower) fn bfloat_arithmetic(&mut self, id: ast::ExprId) -> Result<RValue, Error> {
        let value = self.bfloat_promoted(id)?;
        self.soft_call(
            Helper::TruncSFBF,
            value,
            None,
            self.graph.lower_expression(id)?.ty,
        )
    }
    pub(in crate::lower) fn bfloat_compound(
        &mut self,
        op: Binary,
        left: RValue,
        right: RValue,
        operation: TypeId,
        target: TypeId,
    ) -> Result<RValue, Error> {
        let left = self.convert(left, operation, Context::CompoundAssignment)?;
        let left = self.bfloat_widened(left)?;
        let result = self.soft_binary(op, left, right)?;
        // The operation is a float value although the semantic source carries
        // the bfloat operation type. Select the exact destination route here.
        if let Some(kind) = kind(self.graph, target)? {
            return self.soft_call(
                if kind == Kind::BFloat {
                    Helper::TruncSFBF
                } else {
                    floating_helper(Type::F32, kind, true)
                },
                result,
                None,
                target,
            );
        }
        let ty = types::storage(self.graph, target)?;
        if ty == Type::F32 {
            return Ok(RValue {
                source: target,
                ..result
            });
        }
        if types::boolean(self.graph, target)? {
            let zero = self
                .builder
                .constant(crate::lowir::literals::Constant::Float(
                    crate::lowir::literals::FloatKind::F32,
                    0,
                ))?;
            let truth = self.emit(
                I::Compare(Compare::Ne, Type::F32, result.value, zero),
                target,
            )?;
            return self.emit(
                I::Convert(Convert::Truncate, Type::U8, truth.ty, truth.value),
                target,
            );
        }
        let op = if ty.float_rank() != 0 {
            Convert::FloatExtend
        } else if types::signed(self.graph, target)? {
            Convert::FloatToSigned
        } else {
            Convert::FloatToUnsigned
        };
        self.soft_convert_op(op, ty, result, target)
    }
}
fn arithmetic(op: Option<P>) -> Result<Binary, Error> {
    match op {
        Some(P::Add) => Ok(Binary::Add),
        Some(P::Subtract) => Ok(Binary::Sub),
        Some(P::Multiply) => Ok(Binary::Mul),
        Some(P::Divide) => Ok(Binary::Div),
        _ => Err(crate::sema::Error::Invariant("bfloat arithmetic token").into()),
    }
}
fn floating_helper(ty: Type, kind: Kind, widen: bool) -> Helper {
    use Helper as H;
    match (kind, widen, ty) {
        (Kind::Quad, true, Type::F32) => H::ExtendSFTF,
        (Kind::Quad, true, Type::F64) => H::ExtendDFTF,
        (Kind::Quad, true, _) => H::ExtendXFTF,
        (Kind::Quad, false, Type::F32) => H::TruncTFSF,
        (Kind::Quad, false, Type::F64) => H::TruncTFDF,
        (Kind::Quad, false, _) => H::TruncTFXF,
        (_, true, Type::F32) => H::TruncSFHF,
        (_, true, Type::F64) => H::TruncDFHF,
        (_, true, _) => H::TruncXFHF,
        (_, false, Type::F32) => H::ExtendHFSF,
        (_, false, Type::F64) => H::ExtendHFDF,
        (_, false, _) => H::ExtendHFXF,
    }
}
