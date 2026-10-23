//! `LowIR` instruction grammar and explicit target literal arithmetic. See NOTICE.
use super::{
    Error,
    lexer::Kind,
    literals::{self, Constant, FloatKind},
    model::{BlockId, Body, Call, Facts, Instruction, InstructionKind as I, Value},
    reader::Parser,
    vocabulary::{Binary, Compare, Convert, Projection, Type, Unary, named_literal},
};
impl Parser<'_> {
    /// Shared signed target word scanner for addends, selectors and memory orders.
    pub(super) fn small_integer(&mut self, what: &str) -> Result<i64, Error> {
        let negative = self.accept(Kind::Minus)?;
        if !self.at(Kind::Number) {
            return Err(self.fail(format!("invalid {what}")));
        }
        let bits = self.integer(Type::I64).map_err(|error| match error {
            Error::Diagnostic(_) | Error::ByteDiagnostic(_) => self.fail(format!("invalid {what}")),
            error => error,
        })?;
        let low = u64::try_from(bits & u128::from(u64::MAX)).map_err(|_| Error::Limit)?;
        let value = i64::from_le_bytes(low.to_le_bytes());
        Ok(if negative {
            value.wrapping_neg()
        } else {
            value
        })
    }
    pub(super) fn value(&mut self, body: &mut Body, context: Type) -> Result<Value, Error> {
        let token = self.lexer.token;
        match token.kind {
            Kind::Temp => {
                let name = self.program.names.intern(token.word())?;
                let id = body.use_temp(name, token.line)?;
                self.advance()?;
                Ok(Value::Temp(id))
            }
            Kind::Slot => {
                let name = self.program.names.intern(token.word())?;
                let Some(id) = body.find_slot(name) else {
                    return Err(self.fail(format!("undefined slot: {}", token.word())));
                };
                self.advance()?;
                Ok(Value::Slot(id))
            }
            Kind::Global => {
                let name = self.program.names.intern(token.word())?;
                let symbol = self.program.reference(name, self.source_id, token.line)?;
                self.advance()?;
                Ok(Value::Symbol(symbol))
            }
            Kind::Block => Err(self.fail("block label used as value")),
            _ => {
                if !self.value_start() {
                    return Err(self.fail_token("expected value, got '", "'"));
                }
                let constant = self.constant(context)?;
                body.constant(constant)
            }
        }
    }
    fn value_start(&self) -> bool {
        matches!(
            self.lexer.token.kind,
            Kind::Temp | Kind::Slot | Kind::Global | Kind::Number | Kind::Minus
        ) || (self.at(Kind::Word) && named_literal(self.lexer.token.word()))
    }
    pub(super) fn integer(&mut self, context: Type) -> Result<u128, Error> {
        match self.constant(context)? {
            Constant::Integer(bits) => Ok(bits),
            Constant::Float(..) => Err(self.fail("invalid floating literal")),
        }
    }
    pub(super) fn constant(&mut self, context: Type) -> Result<Constant, Error> {
        let negative = self.accept(Kind::Minus)?;
        let token = self.lexer.token;
        if self.accept_word("nullptr")? {
            if negative {
                return Err(self.fail("expected literal"));
            }
            return Ok(Constant::Integer(0));
        }
        let text = token.word();
        if token.kind != Kind::Number {
            if token.kind == Kind::Word && named_literal(text) {
                self.advance()?;
                if context.bits() != 0 || context == Type::Ptr {
                    return Err(self.fail("invalid floating literal"));
                }
                return Ok(literals::special(text, context, negative));
            }
            return Err(self.fail("expected literal"));
        }
        if literals::floating_spelling(text) {
            if context.bits() != 0 || context == Type::Ptr {
                return Err(self.fail("invalid floating literal"));
            }
            let result = literals::scan(&mut self.floating, text, context, negative)?
                .ok_or_else(|| self.fail("invalid floating literal"))?;
            self.advance()?;
            return Ok(result);
        }
        let hex = text.starts_with("0x") || text.starts_with("0X");
        let (base, digits) = if hex {
            (16_u128, &text[2..])
        } else if text.len() > 1 && text.starts_with('0') {
            (8, &text[1..])
        } else {
            (10, text)
        };
        let mut value = 0_u128;
        let mut valid = !digits.is_empty();
        for c in digits.bytes() {
            let digit = match c {
                b'0'..=b'9' => u128::from(c - b'0'),
                b'a'..=b'f' => u128::from(c - b'a' + 10),
                b'A'..=b'F' => u128::from(c - b'A' + 10),
                _ => 99,
            };
            valid &= digit < base;
            value = value.wrapping_mul(base).wrapping_add(digit);
        }
        if !valid {
            return Err(self.fail_token("invalid integer literal: ", ""));
        }
        if negative {
            value = value.wrapping_neg();
        }
        let result = if let Some(kind) = FloatKind::from_type(context) {
            let bits = literals::integer_as_float(&mut self.floating, value, kind, negative)?;
            Constant::Float(kind, bits)
        } else {
            if !matches!(context, Type::I128 | Type::Void) {
                value &= u128::from(u64::MAX);
                if value & (1_u128 << 63) != 0 {
                    value |= u128::MAX << 64;
                }
            }
            Constant::Integer(value)
        };
        self.advance()?;
        Ok(result)
    }
    pub(super) fn target(&mut self, body: &mut Body) -> Result<BlockId, Error> {
        if !self.at(Kind::Block) {
            return Err(self.fail("undefined or invalid block target"));
        }
        let token = self.lexer.token;
        let name = self.program.names.intern(token.word())?;
        let id = body.use_block(name, token.line)?;
        self.advance()?;
        Ok(id)
    }
    fn operation<T>(&mut self, parse: fn(&str) -> Option<T>) -> Result<T, Error> {
        let operation = if self.at(Kind::Word) {
            parse(self.lexer.token.word())
        } else {
            None
        };
        let Some(operation) = operation else {
            return Err(self.fail_token("unknown LowIR operation: ", ""));
        };
        self.advance()?;
        Ok(operation)
    }
    pub(super) fn instruction(&mut self, body: &mut Body, block: BlockId) -> Result<(), Error> {
        let line = self.lexer.token.line;
        let result = if self.at(Kind::Temp) {
            let name = self.name(Kind::Temp, "temporary name")?;
            self.expect(Kind::Equals, "=")?;
            let id = body.use_temp(name, self.lexer.token.line)?;
            if body.temp_is_parameter(id) || body.temp(id).definition.is_some() {
                return Err(self.fail("duplicate temporary definition"));
            }
            Some(id)
        } else {
            None
        };
        let head = self.lexer.token;
        if head.kind != Kind::Word {
            return Err(self.fail_token(
                if result.is_some() {
                    "unknown rvalue instruction: "
                } else {
                    "unknown instruction: "
                },
                "",
            ));
        }
        self.advance()?;
        let kind = if result.is_some() {
            self.rvalue(body, head.word(), head.line)?
        } else {
            self.effect(body, head.word(), head.line)?
        };
        if let Some(result) = result {
            let ty = kind.result_type();
            if ty == Type::Void {
                return Err(self.fail_at(head.line, "a void instruction has no result"));
            }
            body.temp_mut(result).ty = Some(ty);
        }
        let id = body.append(block, Instruction::new(result, kind, line))?;
        if self.at(Kind::Debug)
            || matches!(self.lexer.token.kind, Kind::Invalid(_))
                && self.lexer.token.text.starts_with(b"!")
        {
            body.set_location(id, self.debug_location()?)?;
        }
        Ok(())
    }
    fn rvalue(&mut self, body: &mut Body, head: &str, line: u32) -> Result<I, Error> {
        Ok(match head {
            "const" => {
                let ty = self.parse_type()?;
                if !self.value_start()
                    || matches!(
                        self.lexer.token.kind,
                        Kind::Temp | Kind::Slot | Kind::Global
                    )
                {
                    return Err(self.fail("expected literal"));
                }
                let constant = self.constant(ty)?;
                I::Const(ty, body.constant(constant)?)
            }
            "copy" => {
                let ty = self.parse_type()?;
                I::Copy(ty, self.value(body, ty)?)
            }
            "unary" => {
                let op = self.operation(Unary::parse)?;
                let ty = self.parse_type()?;
                I::Unary(op, ty, self.value(body, ty)?)
            }
            "binary" => {
                let op = self.operation(Binary::parse)?;
                let facts =
                    Facts::arithmetic(self.accept_word("nowrap")?, self.accept_word("exact")?);
                let ty = self.parse_type()?;
                let a = self.value(body, ty)?;
                self.expect(Kind::Comma, ",")?;
                I::Binary(op, facts, ty, a, self.value(body, ty)?)
            }
            "cmp" => {
                let op = self.operation(Compare::parse)?;
                let ty = self.parse_type()?;
                let a = self.value(body, ty)?;
                self.expect(Kind::Comma, ",")?;
                I::Compare(op, ty, a, self.value(body, ty)?)
            }
            "convert" => {
                let op = self.operation(Convert::parse)?;
                let to = self.parse_type()?;
                let from = self.parse_type()?;
                I::Convert(op, to, from, self.value(body, from)?)
            }
            "select" => {
                let ty = self.parse_type()?;
                let condition = self.value(body, Type::I64)?;
                self.expect(Kind::Comma, ",")?;
                let a = self.value(body, ty)?;
                self.expect(Kind::Comma, ",")?;
                I::Select(ty, condition, a, self.value(body, ty)?)
            }
            "phi" => {
                let ty = self.parse_type()?;
                self.expect(Kind::LBracket, "[")?;
                if !self.at(Kind::Block) {
                    return Err(self.fail("phi requires predecessor/value pairs"));
                }
                let mut pairs = Vec::new();
                loop {
                    let target = self.target(body)?;
                    self.expect(Kind::Colon, ":")?;
                    pairs.push((target, self.value(body, ty)?));
                    if !self.accept(Kind::Comma)? {
                        break;
                    }
                }
                self.expect(Kind::RBracket, "]")?;
                I::Phi(ty, pairs)
            }
            "addr" => {
                if !matches!(self.lexer.token.kind, Kind::Slot | Kind::Global) {
                    return Err(self.fail(if self.at(Kind::Block) {
                        "block label used as value"
                    } else {
                        "expected a slot, global or function to address"
                    }));
                }
                I::Addr(self.value(body, Type::Void)?)
            }
            "load" => {
                let facts = self.memory_facts()?;
                let ty = self.parse_type()?;
                I::Load(facts, ty, self.value(body, Type::Ptr)?)
            }
            "index" => {
                let ty = self.parse_type()?;
                let mut projection = None;
                if self.accept(Kind::LBracket)? {
                    if !self.accept_word("projection")? {
                        return Err(self.fail("invalid index metadata"));
                    }
                    self.expect(Kind::Equals, "=")?;
                    projection = Projection::parse(self.lexer.token.word());
                    if projection.is_none() {
                        return Err(self.fail("invalid index projection"));
                    }
                    self.advance()?;
                    if !self.accept(Kind::RBracket)? {
                        return Err(self.fail("invalid index metadata"));
                    }
                }
                let a = self.value(body, Type::Ptr)?;
                self.expect(Kind::Comma, ",")?;
                I::Index(ty, projection, a, self.value(body, Type::Void)?)
            }
            "call" => I::Call(self.call(body)?),
            _ => I::Runtime(self.runtime_rvalue(body, head)?.ok_or_else(|| {
                self.fail_at(line, format!("unknown rvalue instruction: {head}"))
            })?),
        })
    }
    fn memory_facts(&mut self) -> Result<Facts, Error> {
        Ok(Facts::memory(
            self.accept_word("volatile")?,
            self.accept_word("typed")?,
        ))
    }
    fn call(&mut self, body: &mut Body) -> Result<Call, Error> {
        let result = self.parse_type()?;
        if !matches!(self.lexer.token.kind, Kind::Global | Kind::Temp) {
            return Err(self.fail_token(
                "expected a function, global or temporary callee, got '",
                "'",
            ));
        }
        let callee = self.value(body, Type::Void)?;
        self.expect(Kind::LParen, "(")?;
        self.arguments.clear();
        if !self.at(Kind::RParen) {
            loop {
                let argument = self.value(body, Type::Void)?;
                self.arguments.push(argument);
                if !self.accept(Kind::Comma)? {
                    break;
                }
            }
        }
        self.expect(Kind::RParen, ")")?;
        let arguments = body.add_arguments(&self.arguments)?;
        let mut copy_elision = false;
        let mut argument_classes = Vec::new();
        if self.accept(Kind::LBracket)? {
            loop {
                if !copy_elision && self.accept_word("elision")? {
                    self.expect(Kind::Equals, "=")?;
                    if !self.accept_word("copy")? {
                        return Err(self.fail("invalid call metadata"));
                    }
                    copy_elision = true;
                } else if self.object_facts && self.accept_word("classes")? {
                    self.expect(Kind::Equals, "=")?;
                    if !self.at(Kind::Number) {
                        return Err(self.fail("expected argument index"));
                    }
                    let text = self.lexer.token.word();
                    let argument = if text == "0" { 0 } else { self.positive(text)? };
                    self.advance()?;
                    self.expect(Kind::Colon, ":")?;
                    if argument >= self.arguments.len() as u64 {
                        return Err(self.fail("classes metadata names no argument"));
                    }
                    let classes = self.object_classes(Type::Object {
                        bytes: 16,
                        align: 3,
                    })?;
                    argument_classes
                        .push((u32::try_from(argument).map_err(|_| Error::Limit)?, classes));
                } else {
                    return Err(self.fail("invalid call metadata"));
                }
                if !self.accept(Kind::Comma)? {
                    break;
                }
            }
            self.expect(Kind::RBracket, "]")?;
        }
        let signature = if self.accept_word("as")? {
            Some(self.signature(None)?)
        } else {
            None
        };
        Ok(Call {
            result,
            callee,
            arguments,
            copy_elision,
            argument_classes,
            signature,
        })
    }
    fn span(&mut self) -> Result<(u64, u8), Error> {
        if !self.at(Kind::Number) {
            return Err(self.fail("invalid bulk-memory span"));
        }
        let text = self.lexer.token.word();
        let (bytes, align) = if let Some((bytes, align)) = text.split_once('x') {
            (self.positive(bytes)?, self.positive(align)?)
        } else {
            (self.positive(text)?, 1)
        };
        if !align.is_power_of_two() {
            return Err(self.fail("object alignment is not a power of two"));
        }
        let align = u8::try_from(align.trailing_zeros()).map_err(|_| Error::Limit)?;
        self.advance()?;
        Ok((bytes, align))
    }
    fn effect(&mut self, body: &mut Body, head: &str, line: u32) -> Result<I, Error> {
        Ok(match head {
            "store" => {
                let facts = self.memory_facts()?;
                let ty = self.parse_type()?;
                let a = self.value(body, ty)?;
                self.expect(Kind::Comma, ",")?;
                I::Store(facts, ty, a, self.value(body, Type::Ptr)?)
            }
            "copyobj" | "zeroinit" => {
                let (bytes, align) = self.span()?;
                let source = self.value(body, Type::Ptr)?;
                if head == "copyobj" {
                    self.expect(Kind::Comma, ",")?;
                    I::CopyObject(bytes, align, source, self.value(body, Type::Ptr)?)
                } else {
                    I::ZeroInit(bytes, align, source)
                }
            }
            "jump" => I::Jump(self.target(body)?),
            "branch" => {
                let condition = self.value(body, Type::I64)?;
                self.expect(Kind::Comma, ",")?;
                let a = self.target(body)?;
                self.expect(Kind::Comma, ",")?;
                I::Branch(condition, a, self.target(body)?)
            }
            "switch" => {
                let condition = self.value(body, Type::Void)?;
                self.expect(Kind::Comma, ",")?;
                let default = self.target(body)?;
                let mut pairs = Vec::new();
                while self.accept(Kind::Comma)? {
                    let value = self.value(body, Type::Void)?;
                    self.expect(Kind::Colon, ":")?;
                    pairs.push((value, self.target(body)?));
                }
                I::Switch(condition, default, pairs)
            }
            "return" => {
                let ty = self.parse_type()?;
                I::Return(
                    ty,
                    if ty == Type::Void {
                        None
                    } else {
                        Some(self.value(body, ty)?)
                    },
                )
            }
            "unreachable" => I::Unreachable,
            "call" => {
                let call = self.call(body)?;
                if call.result != Type::Void {
                    return Err(self.fail_at(line, "a call without a result must be void"));
                }
                I::Call(call)
            }
            _ => I::Runtime(
                self.runtime_effect(body, head)?
                    .ok_or_else(|| self.fail_at(line, format!("unknown instruction: {head}")))?,
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::lowir::{test_program_dialect, writer};

    #[test]
    fn host_call_classes_roundtrip_without_changing_the_declared_signature() {
        let source = b"declare function @sink(%n : i32) -> void [arity=variadic]\nfunction @f() -> void {\n slot $a : obj<4x4>\n block ^entry:\n call void @sink(0, $a, $a) [classes=2:sse_none, classes=1:sse_none]\n return void\n}\n";
        let program = test_program_dialect(source, true).unwrap();
        let mut first = Vec::new();
        writer::write(&program, &mut first).unwrap();
        assert!(
            String::from_utf8_lossy(&first).contains("[classes=2:sse_none, classes=1:sse_none]")
        );
        let copy = test_program_dialect(&first, true).unwrap();
        let mut second = Vec::new();
        writer::write(&copy, &mut second).unwrap();
        assert_eq!(first, second);
        assert!(test_program_dialect(source, false).is_err());
        let invalid = String::from_utf8_lossy(source).replace("classes=2:", "classes=3:");
        let error = test_program_dialect(invalid.as_bytes(), true)
            .err()
            .unwrap();
        assert!(matches!(error, crate::lowir::Error::ByteDiagnostic(message)
            if message.starts_with(b"classes metadata names no argument at ")));
    }
}
