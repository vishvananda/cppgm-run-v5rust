// Translated from lower/complex_lowering.cpp. See NOTICE.
//! Complex values cross lowering as objects, with component order and helper demand preserved.
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
        vocabulary::{Binary, Binding, Compare, Projection, Type, Unary},
    },
    sema::{
        Graph,
        lowering::{TypeId, TypeView},
    },
    syntax::ast,
};

pub(in crate::lower) fn part(graph: &Graph, ty: TypeId) -> Result<Option<TypeId>, Error> {
    Ok(if let TypeView::Complex(part) = graph.lower_type(ty)? {
        Some(part)
    } else {
        None
    })
}
impl Symbols<'_, '_> {
    fn complex_helper(
        &mut self,
        program: &mut Program,
        divide: bool,
        ty: TypeId,
    ) -> Result<SymbolId, Error> {
        let component = part(self.graph, ty)?
            .ok_or(crate::sema::Error::Invariant("complex helper without part"))?;
        let storage = types::storage(self.graph, component)?;
        let mode = match (
            crate::lower::soft_float::kind(self.graph, component)?,
            storage,
        ) {
            (Some(crate::lower::soft_float::Kind::Half), _) => 'h',
            (Some(crate::lower::soft_float::Kind::Quad), _) => 't',
            (_, Type::F32) => 's',
            (_, Type::F64) => 'd',
            _ => 'x',
        };
        if let Some(&symbol) = self.state.complex_helpers.get(&(divide, mode)) {
            return Ok(symbol);
        }
        let name = format!("__{}{mode}c3", if divide { "div" } else { "mul" });
        let symbol = self.state.table.allocate(
            program,
            &format!("__cppgm_libgcc_{}", &name[2..]),
            None,
            true,
        )?;
        let mut parameters = Vec::with_capacity(4);
        for index in 0..4 {
            parameters.push(Parameter {
                name: program.names.intern(&format!("arg{index}"))?,
                ty: storage,
                pass: None,
                object_bytes: 0,
                no_alias: false,
                classes: if self.object_mode() {
                    crate::lower::soft_float::kind(self.graph, component)?
                        .map(crate::lower::soft_float::classes)
                } else {
                    None
                },
            });
        }
        let signature = program.add_signature(Signature {
            parameters,
            result: types::storage(self.graph, ty)?,
            boundary: Boundary {
                result_classes: if self.object_mode() {
                    self.object_argument_classes(ty)?
                } else {
                    None
                },
                unwind: Unwind::No,
                ..Boundary::default()
            },
        })?;
        let object = program.names.intern(&name)?;
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
        self.state.complex_helpers.insert((divide, mode), symbol);
        Ok(symbol)
    }
}
impl Function<'_, '_, '_, '_> {
    pub(in crate::lower) fn complex_value(
        &mut self,
        ty: TypeId,
        real: RValue,
        imaginary: RValue,
    ) -> Result<RValue, Error> {
        let component = part(&*self.graph, ty)?
            .ok_or(crate::sema::Error::Invariant("complex value without part"))?;
        let whole = types::storage(&*self.graph, ty)?;
        let slot = self.generated_slot("complex", whole)?;
        let address = self.emit(I::Addr(slot), ty)?.value;
        self.write_place(address, component, real.value)?;
        let second = self.complex_offset(address, component)?;
        self.write_place(second, component, imaginary.value)?;
        self.read_place(slot, ty)
    }
    fn complex_offset(&mut self, address: Value, component: TypeId) -> Result<Value, Error> {
        let bytes = self
            .builder
            .integer(u128::from(types::layout(&*self.graph, component)?.0))?;
        Ok(self
            .emit(
                I::Index(Type::I8, Some(Projection::Field), address, bytes),
                component,
            )?
            .value)
    }
    pub(in crate::lower) fn complex_parts(&mut self, value: RValue) -> Result<[RValue; 2], Error> {
        let component = part(&*self.graph, value.source)?
            .ok_or(crate::sema::Error::Invariant("complex parts without part"))?;
        let slot = self.generated_slot("complex", value.ty)?;
        self.write_place(slot, value.source, value.value)?;
        let address = self.emit(I::Addr(slot), value.source)?.value;
        let real = self.read_place(address, component)?;
        let second = self.complex_offset(address, component)?;
        let imaginary = self.read_place(second, component)?;
        Ok([real, imaginary])
    }
    pub(in crate::lower) fn complex_component_place(
        &mut self,
        operand: ast::ExprId,
        imaginary: bool,
    ) -> Result<Value, Error> {
        let ty = self.graph.lower_expression(operand)?.ty;
        let place = self.scalar_place(operand)?;
        let Some(component) = part(&*self.graph, ty)? else {
            return Ok(place);
        };
        let address = if matches!(place, Value::Slot(_) | Value::Symbol(_)) {
            self.emit(I::Addr(place), ty)?.value
        } else {
            place
        };
        if imaginary {
            self.complex_offset(address, component)
        } else {
            Ok(address)
        }
    }
    pub(in crate::lower) fn complex_component_value(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        imaginary: bool,
    ) -> Result<RValue, Error> {
        let ty = self.graph.lower_expression(id)?.ty;
        let facts = self.graph.lower_expression(operand)?;
        if part(&*self.graph, facts.ty)?.is_none() {
            if !imaginary {
                return self.expression(operand, Context::Operand);
            }
            self.discard(operand)?;
            return self.zero(ty);
        }
        if facts.is_place {
            let place = self.complex_component_place(operand, imaginary)?;
            return self.read_place(place, ty);
        }
        let value = self.expression(operand, Context::Operand)?;
        // Reading one part must not emit a load of the other part.
        let slot = self.generated_slot("complex", value.ty)?;
        self.write_place(slot, facts.ty, value.value)?;
        let address = self.emit(I::Addr(slot), facts.ty)?.value;
        let address = if imaginary {
            self.complex_offset(address, ty)?
        } else {
            address
        };
        self.read_place(address, ty)
    }
    pub(in crate::lower) fn complex_convert(
        &mut self,
        value: RValue,
        target: TypeId,
        context: Context,
    ) -> Result<RValue, Error> {
        let component = part(&*self.graph, target)?
            .ok_or(crate::sema::Error::Invariant("complex conversion target"))?;
        let [real, imaginary] = if part(&*self.graph, value.source)?.is_some() {
            let [real, imaginary] = self.complex_parts(value)?;
            let imaginary = self.convert(imaginary, component, context)?;
            let imaginary = self.fit(imaginary, component)?;
            let real = self.convert(real, component, context)?;
            let real = self.fit(real, component)?;
            [real, imaginary]
        } else {
            let zero = self.zero(component)?;
            let real = self.convert(value, component, context)?;
            [self.fit(real, component)?, zero]
        };
        self.complex_value(target, real, imaginary)
    }
    pub(in crate::lower) fn complex_list(
        &mut self,
        ty: TypeId,
        elements: crate::syntax::storage::List<ast::ExprId>,
        context: Context,
    ) -> Result<RValue, Error> {
        let elements = self.owned_expression_ids(elements);
        let elements = elements.as_slice();
        match elements {
            [] => self.zero(ty),
            [single] => {
                let value = self.operand(*single, context)?;
                self.convert(value, ty, context)
            }
            [_, _] => {
                let component = part(&*self.graph, ty)?
                    .ok_or(crate::sema::Error::Invariant("complex list component"))?;
                let whole = types::storage(&*self.graph, ty)?;
                let (bytes, align) = types::layout(&*self.graph, ty)?;
                let slot = self.generated_slot("vector", whole)?;
                self.effect(I::ZeroInit(bytes, align, slot))?;
                let address = self.emit(I::Addr(slot), ty)?.value;
                for (index, &element) in elements.iter().enumerate() {
                    let value = self.operand(element, Context::Initialization)?;
                    let value = self.fit(value, component)?;
                    let offset = if index == 0 {
                        0
                    } else {
                        types::layout(&*self.graph, component)?.0
                    };
                    let offset = self.builder.integer(u128::from(offset))?;
                    let place = self
                        .emit(
                            I::Index(Type::I8, Some(Projection::Field), address, offset),
                            component,
                        )?
                        .value;
                    self.write_place(place, component, value.value)?;
                }
                self.read_place(slot, ty)
            }
            _ => Err(crate::sema::Error::Invariant("complex list arity").into()),
        }
    }
    pub(in crate::lower) fn complex_increment(
        &mut self,
        old: RValue,
        decrement: bool,
    ) -> Result<RValue, Error> {
        let [real, imaginary] = self.complex_parts(old)?;
        let one = if let Some(kind) = crate::lower::soft_float::kind(&*self.graph, real.source)? {
            let bits = match kind {
                crate::lower::soft_float::Kind::Half => 0x3c00,
                crate::lower::soft_float::Kind::BFloat => 0x3f80,
                crate::lower::soft_float::Kind::Quad => 0x3fff_u128 << 112,
            };
            self.soft_constant(kind, bits, real.source)?
        } else if let Some(kind) = crate::lowir::literals::FloatKind::from_type(real.ty) {
            let bits = match kind {
                crate::lowir::literals::FloatKind::F32 => u128::from(1f32.to_bits()),
                crate::lowir::literals::FloatKind::F64 => u128::from(1f64.to_bits()),
                crate::lowir::literals::FloatKind::F80 => (0x3fff_u128 << 64) | (1_u128 << 63),
            };
            RValue {
                value: self
                    .builder
                    .constant(crate::lowir::literals::Constant::Float(kind, bits))?,
                ty: real.ty,
                source: real.source,
                literal: false,
                bits: None,
            }
        } else {
            RValue {
                value: self.builder.integer(1)?,
                ty: real.ty,
                source: real.source,
                literal: false,
                bits: Some(1),
            }
        };
        let real = self.part_binary(if decrement { P::Subtract } else { P::Add }, real, one)?;
        self.complex_value(old.source, real, imaginary)
    }
    pub(in crate::lower) fn make_complex(
        &mut self,
        id: ast::ExprId,
        arguments: crate::syntax::storage::List<ast::ExprId>,
    ) -> Result<RValue, Error> {
        let args = self.tree.expr_lists.get(arguments);
        let [real, imaginary] = *args else {
            return Err(crate::sema::Error::Invariant("complex construction arity").into());
        };
        let real = self.operand(real, Context::Argument)?;
        let imaginary = self.operand(imaginary, Context::Argument)?;
        self.complex_value(self.graph.lower_expression(id)?.ty, real, imaginary)
    }
    fn part_binary(&mut self, op: P, a: RValue, b: RValue) -> Result<RValue, Error> {
        let signed = types::signed(&*self.graph, a.source)?;
        let binary = match op {
            P::Add => Binary::Add,
            P::Subtract => Binary::Sub,
            P::Multiply => Binary::Mul,
            P::Divide
                if signed
                    || a.ty.float_rank() != 0
                    || crate::lower::soft_float::kind(&*self.graph, a.source)?.is_some() =>
            {
                Binary::Div
            }
            P::Divide => Binary::UDiv,
            _ => return Err(crate::sema::Error::Invariant("complex arithmetic token").into()),
        };
        if let Some(kind) = crate::lower::soft_float::kind(&*self.graph, a.source)? {
            return self.soft_arithmetic(binary, kind, a, b, a.source);
        }
        self.emit(
            I::Binary(
                binary,
                Facts::arithmetic(
                    signed && matches!(a.ty, Type::I32 | Type::I64) && op != P::Divide,
                    false,
                ),
                a.ty,
                a.value,
                b.value,
            ),
            a.source,
        )
    }
    fn part_negate(&mut self, value: RValue) -> Result<RValue, Error> {
        if let Some(kind) = crate::lower::soft_float::kind(&*self.graph, value.source)? {
            self.soft_negate(kind, value)
        } else {
            self.emit(I::Unary(Unary::Neg, value.ty, value.value), value.source)
        }
    }
    pub(in crate::lower) fn complex_unary(
        &mut self,
        op: P,
        value: RValue,
    ) -> Result<RValue, Error> {
        let [real, imaginary] = self.complex_parts(value)?;
        let real = if op == P::Subtract {
            self.part_negate(real)?
        } else {
            real
        };
        let imaginary = self.part_negate(imaginary)?;
        self.complex_value(value.source, real, imaginary)
    }
    pub(in crate::lower) fn complex_arithmetic(
        &mut self,
        op: P,
        ty: TypeId,
        left: RValue,
        right: RValue,
    ) -> Result<RValue, Error> {
        let left_complex = part(&*self.graph, left.source)?.is_some();
        let right_complex = part(&*self.graph, right.source)?.is_some();
        let component = part(&*self.graph, ty)?
            .ok_or(crate::sema::Error::Invariant("complex arithmetic type"))?;
        let [a, b] = if left_complex {
            self.complex_parts(left)?
        } else {
            [left, left]
        };
        let [c, d] = if right_complex {
            self.complex_parts(right)?
        } else {
            [right, right]
        };
        let (real, imaginary) = match op {
            P::Add | P::Subtract => {
                let real = self.part_binary(op, a, c)?;
                let imaginary = if !right_complex {
                    b
                } else if !left_complex {
                    if op == P::Subtract {
                        self.part_negate(d)?
                    } else {
                        d
                    }
                } else {
                    self.part_binary(op, b, d)?
                };
                (real, imaginary)
            }
            P::Multiply if left_complex && right_complex => {
                return self.complex_product(ty, [a, b, c, d]);
            }
            P::Multiply => (
                self.part_binary(op, a, c)?,
                if left_complex {
                    self.part_binary(op, b, c)?
                } else {
                    self.part_binary(op, a, d)?
                },
            ),
            _ if !right_complex => (self.part_binary(op, a, c)?, self.part_binary(op, b, c)?),
            _ => {
                let b = if left_complex {
                    b
                } else {
                    self.zero(component)?
                };
                return self.complex_quotient(ty, [a, b, c, d]);
            }
        };
        self.complex_value(ty, real, imaginary)
    }
    fn complex_helper_call(
        &mut self,
        divide: bool,
        ty: TypeId,
        parts: [RValue; 4],
    ) -> Result<RValue, Error> {
        // The frozen emitter allocates the helper declaration after its arguments.
        let arguments = self.builder.arguments(&parts.map(|part| part.value))?;
        let symbol = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .complex_helper(self.builder.program(), divide, ty)?;
        self.emit(
            I::Call(Call {
                result: types::storage(&*self.graph, ty)?,
                callee: Value::Symbol(symbol),
                arguments,
                signature: None,
                copy_elision: false,
                argument_classes: (Vec::new()).into(),
            }),
            ty,
        )
    }
    fn complex_product(&mut self, ty: TypeId, parts: [RValue; 4]) -> Result<RValue, Error> {
        let [a, b, c, d] = parts;
        if matches!(a.ty, Type::Object { .. }) {
            return self.complex_helper_call(false, ty, parts);
        }
        let bd = self.part_binary(P::Multiply, b, d)?;
        let ac = self.part_binary(P::Multiply, a, c)?;
        let real = self.part_binary(P::Subtract, ac, bd)?;
        let bc = self.part_binary(P::Multiply, b, c)?;
        let ad = self.part_binary(P::Multiply, a, d)?;
        let imaginary = self.part_binary(P::Add, ad, bc)?;
        let product = self.complex_value(ty, real, imaginary)?;
        if a.ty.float_rank() == 0 {
            return Ok(product);
        }
        let slot = self.generated_slot("complex", product.ty)?;
        self.write_place(slot, ty, product.value)?;
        let real_nan = self.new_block("complex_nan")?;
        let recompute = self.new_block("complex_recompute")?;
        let end = self.new_block("complex_end")?;
        let test = self.emit(
            I::Compare(Compare::Ne, real.ty, real.value, real.value),
            real.source,
        )?;
        self.effect(I::Branch(test.value, real_nan, end))?;
        self.terminated = true;
        self.place(real_nan)?;
        let test = self.emit(
            I::Compare(Compare::Ne, imaginary.ty, imaginary.value, imaginary.value),
            imaginary.source,
        )?;
        self.effect(I::Branch(test.value, recompute, end))?;
        self.terminated = true;
        self.place(recompute)?;
        let result = self.complex_helper_call(false, ty, parts)?;
        self.write_place(slot, ty, result.value)?;
        self.jump(end)?;
        self.place(end)?;
        self.read_place(slot, ty)
    }
    fn complex_quotient(&mut self, ty: TypeId, parts: [RValue; 4]) -> Result<RValue, Error> {
        let [a, b, c, d] = parts;
        if a.ty.float_rank() != 0
            || crate::lower::soft_float::kind(&*self.graph, a.source)?.is_some()
        {
            return self.complex_helper_call(true, ty, parts);
        }
        let whole = types::storage(&*self.graph, ty)?;
        let slot = self.generated_slot("complex", whole)?;
        let mc = self.part_magnitude(c)?;
        let md = self.part_magnitude(d)?;
        let by_d = self.new_block("complex_by_d")?;
        let by_c = self.new_block("complex_by_c")?;
        let end = self.new_block("complex_end")?;
        let compare = if types::signed(&*self.graph, a.source)? {
            Compare::Lt
        } else {
            Compare::Ult
        };
        let test = self.emit(I::Compare(compare, c.ty, mc.value, md.value), a.source)?;
        self.effect(I::Branch(test.value, by_d, by_c))?;
        self.terminated = true;
        for (block, other) in [(by_d, true), (by_c, false)] {
            self.place(block)?;
            let ratio = if other {
                self.part_binary(P::Divide, c, d)?
            } else {
                self.part_binary(P::Divide, d, c)?
            };
            let numerator = self.part_binary(P::Multiply, if other { c } else { d }, ratio)?;
            let divisor = self.part_binary(P::Add, numerator, if other { d } else { c })?;
            let re = self.part_binary(P::Multiply, if other { a } else { b }, ratio)?;
            let real = self.part_binary(P::Add, re, if other { b } else { a })?;
            let im = self.part_binary(P::Multiply, if other { b } else { a }, ratio)?;
            let imaginary = if other {
                self.part_binary(P::Subtract, im, a)?
            } else {
                self.part_binary(P::Subtract, b, im)?
            };
            let imaginary = self.part_binary(P::Divide, imaginary, divisor)?;
            let real = self.part_binary(P::Divide, real, divisor)?;
            let value = self.complex_value(ty, real, imaginary)?;
            self.write_place(slot, ty, value.value)?;
            self.jump(end)?;
        }
        self.place(end)?;
        self.read_place(slot, ty)
    }
    fn part_magnitude(&mut self, value: RValue) -> Result<RValue, Error> {
        if !types::signed(&*self.graph, value.source)? {
            return Ok(value);
        }
        let shift = self.builder.integer(u128::from(value.ty.bits() - 1))?;
        let sign = self.emit(
            I::Binary(Binary::Shr, Facts::default(), value.ty, value.value, shift),
            value.source,
        )?;
        let xor = self.emit(
            I::Binary(
                Binary::Xor,
                Facts::default(),
                value.ty,
                value.value,
                sign.value,
            ),
            value.source,
        )?;
        self.emit(
            I::Binary(
                Binary::Sub,
                Facts::default(),
                value.ty,
                xor.value,
                sign.value,
            ),
            value.source,
        )
    }
    fn part_compare(
        &mut self,
        equal: bool,
        a: RValue,
        b: RValue,
        truth: TypeId,
    ) -> Result<RValue, Error> {
        let predicate = if equal { Compare::Eq } else { Compare::Ne };
        if let Some(kind) = crate::lower::soft_float::kind(&*self.graph, a.source)? {
            self.soft_compare(predicate, kind, a, b, truth)
        } else {
            self.emit(I::Compare(predicate, a.ty, a.value, b.value), truth)
        }
    }
    pub(in crate::lower) fn complex_equal(
        &mut self,
        equal: bool,
        left: RValue,
        right: RValue,
        truth: TypeId,
    ) -> Result<RValue, Error> {
        let [a, b] = self.complex_parts(left)?;
        let [c, d] = self.complex_parts(right)?;
        let reals = self.part_compare(equal, a, c, truth)?;
        let imaginaries = self.part_compare(equal, b, d, truth)?;
        self.emit(
            I::Binary(
                if equal { Binary::And } else { Binary::Or },
                Facts::default(),
                reals.ty,
                reals.value,
                imaginaries.value,
            ),
            truth,
        )
    }
    pub(in crate::lower) fn complex_truth(
        &mut self,
        value: RValue,
        truth: TypeId,
    ) -> Result<RValue, Error> {
        let [real, imaginary] = self.complex_parts(value)?;
        let zero = self.zero(real.source)?;
        let a = self.part_compare(false, real, zero, truth)?;
        let zero = self.zero(imaginary.source)?;
        let b = self.part_compare(false, imaginary, zero, truth)?;
        self.emit(
            I::Binary(Binary::Or, Facts::default(), a.ty, a.value, b.value),
            truth,
        )
    }
}
