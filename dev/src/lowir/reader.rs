//! `LowIR` unit/signature/body grammar. Retained spellings are interned once. See NOTICE.
use super::{
    Error,
    lexer::{Kind, Lexer},
    model::{
        Arity, Body, Function, Linkage, Metadata, NameId, Parameter, Program, Returns, Unwind,
    },
    vocabulary::{Binding, Effects, Pass, Role, Type},
};
use std::collections::HashSet;
pub(super) struct Parser<'a> {
    pub(super) program: &'a mut Program,
    pub(super) lexer: Lexer<'a>,
    pub(super) source: &'a str,
}
impl<'a> Parser<'a> {
    pub(super) fn new(program: &'a mut Program, source: &'a [u8], path: &'a str) -> Self {
        Self {
            program,
            lexer: Lexer::new(source),
            source: path,
        }
    }
    pub(super) fn fail(&self, message: impl Into<String>) -> Error {
        let message = match self.lexer.token.kind {
            Kind::Invalid(problem) => problem.into(),
            _ => message.into(),
        };
        self.fail_at(self.lexer.token.line, message)
    }
    pub(super) fn fail_at(&self, line: u32, message: impl Into<String>) -> Error {
        Error::Diagnostic(format!("{} at {}:{line}", message.into(), self.source))
    }
    pub(super) fn at(&self, kind: Kind) -> bool {
        self.lexer.token.kind == kind
    }
    pub(super) fn at_word(&self, text: &str) -> bool {
        self.at(Kind::Word) && self.lexer.token.word() == text
    }
    pub(super) fn accept(&mut self, kind: Kind) -> bool {
        if !self.at(kind) {
            return false;
        }
        self.lexer.advance();
        true
    }
    pub(super) fn accept_word(&mut self, text: &str) -> bool {
        if !self.at_word(text) {
            return false;
        }
        self.lexer.advance();
        true
    }
    pub(super) fn expect(&mut self, kind: Kind, text: &str) -> Result<(), Error> {
        if self.accept(kind) {
            Ok(())
        } else {
            Err(self.fail(format!(
                "expected '{text}', got '{}'",
                self.lexer.token.spelling()
            )))
        }
    }
    pub(super) fn name(&mut self, kind: Kind, what: &str) -> Result<NameId, Error> {
        if !self.at(kind) {
            return Err(self.fail(format!(
                "expected {what}, got '{}'",
                self.lexer.token.spelling()
            )));
        }
        let name = self.program.names.intern(self.lexer.token.word())?;
        self.lexer.advance();
        Ok(name)
    }
    pub(super) fn parse_type(&mut self) -> Result<Type, Error> {
        let token = self.lexer.token;
        if token.kind == Kind::Word {
            if let Some(ty) = Type::scalar(token.word()) {
                self.lexer.advance();
                return Ok(ty);
            }
            if let Some(object) = token
                .word()
                .strip_prefix("obj<")
                .and_then(|t| t.strip_suffix('>'))
                && let Some((bytes, align)) = object.split_once('x')
            {
                let bytes = self.positive(bytes)?;
                let align = self.positive(align)?;
                if !align.is_power_of_two() {
                    return Err(self.fail("object alignment is not a power of two"));
                }
                if align > bytes {
                    return Err(self.fail("object alignment exceeds size"));
                }
                if bytes >= (1_u64 << 48) {
                    return Err(self.fail("object size is too large"));
                }
                let align = u8::try_from(align.trailing_zeros()).map_err(|_| Error::Limit)?;
                self.lexer.advance();
                return Ok(Type::Object { bytes, align });
            }
        }
        Err(self.fail(format!("unknown LowIR type: {}", token.spelling())))
    }
    pub(super) fn positive(&self, text: &str) -> Result<u64, Error> {
        let mut value = 0_u64;
        let mut valid = !text.is_empty() && !text.starts_with('0');
        for digit in text.bytes() {
            if !digit.is_ascii_digit() || value > (u64::MAX - 9) / 10 {
                valid = false;
                break;
            }
            value = value * 10 + u64::from(digit - b'0');
        }
        if valid {
            Ok(value)
        } else {
            Err(self.fail(format!("invalid positive integer: {text}")))
        }
    }
    pub(super) fn parse(&mut self) -> Result<(), Error> {
        while !self.at(Kind::End) {
            let declaration = self.accept_word("declare");
            if self.accept_word("function") {
                self.function(!declaration)?;
            } else if self.at_word("global") || (!declaration && self.at_word("alias")) {
                return Err(Error::NotImplemented("LowIR globals and object aliases"));
            } else {
                return Err(self.fail(if declaration {
                    "expected declaration kind"
                } else {
                    "expected top-level LowIR item"
                }));
            }
        }
        Ok(())
    }
    fn function(&mut self, definition: bool) -> Result<(), Error> {
        let line = self.lexer.token.line;
        let name = self.name(Kind::Global, "function name")?;
        if self.program.contains(name) {
            return Err(self.fail_at(
                line,
                format!(
                    "duplicate top-level symbol: {}",
                    self.program.names.get(name)
                ),
            ));
        }
        let mut function = Function {
            name,
            parameters: Vec::new(),
            result: Type::Void,
            metadata: Metadata::default(),
            body: None,
        };
        let signature_line = self.lexer.token.line;
        self.expect(Kind::LParen, "(")?;
        let mut names = HashSet::new();
        if !self.at(Kind::RParen) {
            loop {
                let name = self.name(Kind::Temp, "parameter name")?;
                if !names.insert(name) {
                    return Err(self.fail("duplicate parameter"));
                }
                self.expect(Kind::Colon, ":")?;
                let mut parameter = Parameter {
                    name,
                    ty: self.parse_type()?,
                    pass: None,
                    no_alias: false,
                    object_bytes: 0,
                };
                if self.at(Kind::LBracket) {
                    self.parameter_metadata(&mut parameter)?;
                }
                function.parameters.push(parameter);
                if !self.accept(Kind::Comma) {
                    break;
                }
            }
        }
        self.expect(Kind::RParen, ")")?;
        self.expect(Kind::Arrow, "->")?;
        function.result = self.parse_type()?;
        self.metadata(&mut function.metadata)?;
        for (i, p) in function.parameters.iter().enumerate() {
            if p.pass == Some(Pass::IndirectResult) && (i != 0 || function.result != Type::Void) {
                return Err(self.fail_at(signature_line, "invalid indirect result parameter"));
            }
        }
        if definition {
            if self.at(Kind::Debug) {
                return Err(Error::NotImplemented("LowIR debug locations"));
            }
            self.expect(Kind::LBrace, "{")?;
            function.body = Some(self.body(&function)?);
        }
        self.program.add(function)
    }
    fn parameter_metadata(&mut self, parameter: &mut Parameter) -> Result<(), Error> {
        self.lexer.advance();
        let mut seen = HashSet::new();
        loop {
            if !self.at(Kind::Word) {
                return Err(self.fail("invalid parameter metadata"));
            }
            let key = self.lexer.token.word();
            self.lexer.advance();
            self.expect(Kind::Equals, "=")?;
            let value = self.lexer.token.word();
            match key {
                "pass" => {
                    parameter.pass = Pass::parse(value);
                    if parameter.pass.is_none() {
                        return Err(self.fail("invalid parameter pass metadata"));
                    }
                    if parameter.ty != Type::Ptr {
                        return Err(self.fail("non-direct passing requires ptr"));
                    }
                }
                "alias" => {
                    if !self.at_word("noalias") {
                        return Err(self.fail("invalid parameter metadata"));
                    }
                    if parameter.ty != Type::Ptr {
                        return Err(self.fail("alias metadata requires ptr"));
                    }
                    parameter.no_alias = true;
                }
                "object_bytes" => {
                    if !self.at(Kind::Number) {
                        return Err(self.fail("expected positive integer"));
                    }
                    parameter.object_bytes = self.positive(value)?;
                    if parameter.ty != Type::Ptr {
                        return Err(self.fail("object_bytes metadata requires ptr"));
                    }
                }
                "classes" => return Err(Error::NotImplemented("LowIR object ABI classes")),
                _ => return Err(self.fail("invalid parameter metadata")),
            }
            if !seen.insert(key) {
                return Err(self.fail(format!("duplicate metadata key: {key}")));
            }
            self.lexer.advance();
            if !self.accept(Kind::Comma) {
                break;
            }
        }
        self.expect(Kind::RBracket, "]")
    }
    fn metadata(&mut self, metadata: &mut Metadata) -> Result<(), Error> {
        let mut seen = HashSet::new();
        while self.accept(Kind::LBracket) {
            loop {
                if !self.at(Kind::Word) {
                    return Err(self.fail(format!(
                        "expected metadata key, got '{}'",
                        self.lexer.token.spelling()
                    )));
                }
                let key = self.lexer.token.word();
                if !matches!(
                    key,
                    "arity"
                        | "effects"
                        | "unwind"
                        | "return"
                        | "role"
                        | "linkage"
                        | "binding"
                        | "object"
                        | "replaceable"
                        | "keep_alias"
                        | "prefer_local"
                        | "object_root"
                        | "force_inline"
                        | "inline_hint"
                        | "no_inline"
                        | "storage"
                        | "section"
                        | "tls_for"
                        | "result_classes"
                ) {
                    return Err(self.fail(format!("unknown symbol metadata: {key}")));
                }
                self.lexer.advance();
                self.expect(Kind::Equals, "=")?;
                if !seen.insert(key) {
                    return Err(self.fail(format!("duplicate metadata key: {key}")));
                }
                let value = self.lexer.token.word();
                let symbol_key = !matches!(
                    key,
                    "arity"
                        | "effects"
                        | "unwind"
                        | "return"
                        | "result_classes"
                        | "storage"
                        | "section"
                        | "tls_for"
                );
                if symbol_key && !self.at(Kind::Word) && !(key == "object" && self.at(Kind::Global))
                {
                    return Err(self.fail(format!("missing value after {key}")));
                }
                match key {
                    "arity" => {
                        if !self.at_word("variadic") {
                            return Err(self.fail("invalid arity metadata"));
                        }
                        metadata.arity = Arity::Variadic;
                    }
                    "effects" => {
                        metadata.effects = Effects::parse(value);
                        if metadata.effects.is_none() {
                            return Err(self.fail("invalid effects metadata"));
                        }
                    }
                    "unwind" => {
                        if !self.at_word("no") {
                            return Err(self.fail("invalid unwind metadata"));
                        }
                        metadata.unwind = Unwind::No;
                    }
                    "return" => {
                        if !self.at_word("noreturn") {
                            return Err(self.fail("invalid return metadata"));
                        }
                        metadata.returns = Returns::NoReturn;
                    }
                    "role" => {
                        metadata.role = Role::parse(value);
                        if metadata.role.is_none_or(Role::global) {
                            return Err(self.fail("invalid symbol role"));
                        }
                    }
                    "linkage" => {
                        if !self.at_word("c") {
                            return Err(self.fail("invalid linkage metadata"));
                        }
                        metadata.linkage = Linkage::C;
                    }
                    "binding" => {
                        metadata.binding = Binding::parse(value);
                        if metadata.binding.is_none() {
                            return Err(self.fail("invalid binding metadata"));
                        }
                    }
                    "object" => {
                        let value = if self.at(Kind::Global) {
                            format!("@{value}")
                        } else {
                            value.into()
                        };
                        metadata.object = Some(self.program.names.intern(&value)?);
                    }
                    "storage" => return Err(self.fail("storage metadata on non-global")),
                    "section" => return Err(self.fail("section metadata requires a global")),
                    "tls_for" | "result_classes" => {
                        return Err(Error::NotImplemented(
                            "LowIR symbol and object boundary relations",
                        ));
                    }
                    _ => {
                        if !self.at_word("yes") {
                            return Err(self.fail("metadata flag must be yes"));
                        }
                        metadata.flags |= match key {
                            "keep_alias" => 1,
                            "prefer_local" => 2,
                            "object_root" => 4,
                            "force_inline" => 8,
                            "inline_hint" => 16,
                            "no_inline" => 32,
                            _ => 64,
                        };
                    }
                }
                self.lexer.advance();
                if !self.accept(Kind::Comma) {
                    break;
                }
            }
            self.expect(Kind::RBracket, "]")?;
        }
        if metadata.flags & 64 != 0 && metadata.binding != Some(Binding::Weak) {
            return Err(self.fail("replaceable metadata requires binding=weak"));
        }
        Ok(())
    }
    fn body(&mut self, function: &Function) -> Result<Body, Error> {
        let mut body = Body::new(self.source, function.parameters.len());
        for p in &function.parameters {
            let id = body.use_temp(p.name, 0)?;
            body.temp_mut(id).ty = Some(p.ty);
        }
        while !self.accept(Kind::RBrace) {
            if self.at(Kind::End) {
                return Err(self.fail("unexpected end of LowIR"));
            }
            if self.accept_word("slot") {
                if !body.layout.is_empty() {
                    return Err(self.fail("slot declaration after first block"));
                }
                let name = self.name(Kind::Slot, "slot name")?;
                self.expect(Kind::Colon, ":")?;
                let ty = self.parse_type()?;
                if body.find_slot(name).is_some() {
                    return Err(self.fail("duplicate slot"));
                }
                body.add_slot(name, ty)?;
            } else if self.accept_word("block") {
                self.check_end(&body)?;
                let name = self.name(Kind::Block, "block name")?;
                let id = body.use_block(name, self.lexer.token.line)?;
                if body.block(id).placed {
                    return Err(self.fail("duplicate block"));
                }
                body.block_mut(id).placed = true;
                self.expect(Kind::Colon, ":")?;
                body.layout.push(id);
            } else {
                let Some(&block) = body.layout.last() else {
                    return Err(self.fail("instruction outside block"));
                };
                if body
                    .block(block)
                    .instructions
                    .last()
                    .is_some_and(|&id| body.instruction(id).kind.terminator())
                {
                    return Err(self.fail("instruction after terminator"));
                }
                self.instruction(&mut body, block)?;
            }
        }
        if body.layout.is_empty() {
            return Err(self.fail("function has no blocks"));
        }
        self.check_end(&body)?;
        for block in body.blocks() {
            if !block.placed {
                return Err(self.fail_at(block.first_use, "undefined or invalid block target"));
            }
        }
        for temp in &body.temps()[body.parameter_count..] {
            if temp.definition.is_none() {
                return Err(self.fail_at(
                    temp.first_use,
                    format!("undefined temporary: {}", self.program.names.get(temp.name)),
                ));
            }
        }
        Ok(body)
    }
    fn check_end(&self, body: &Body) -> Result<(), Error> {
        if let Some(&id) = body.layout.last() {
            let block = body.block(id);
            if !block
                .instructions
                .last()
                .is_some_and(|&id| body.instruction(id).kind.terminator())
            {
                return Err(self.fail(format!(
                    "block has no terminator: {}",
                    self.program.names.get(block.name)
                )));
            }
        }
        Ok(())
    }
}
