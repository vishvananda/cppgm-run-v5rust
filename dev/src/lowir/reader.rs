//! `LowIR` unit/signature/body grammar. Retained spellings are interned once. See NOTICE.
use super::{
    Error,
    lexer::{Kind, Lexer},
    model::{
        Arity, Body, Boundary, DebugLocation, Function, Linkage, Metadata, NameId, Parameter,
        Program, Returns, Signature, SignatureId, SourceId, Unwind, Value,
    },
    vocabulary::{Binding, Effects, ObjectClasses, Pass, Role, Storage, Type},
};
use std::{collections::HashSet, path::Path};
pub(super) struct Parser<'a> {
    pub(super) program: &'a mut Program,
    pub(super) lexer: Lexer<'a>,
    pub(super) source: &'a Path,
    pub(super) source_id: SourceId,
    pub(super) arguments: Vec<Value>,
    pub(super) floating: crate::types::floating::FloatingScanner,
    pub(super) object_facts: bool,
}
impl<'a> Parser<'a> {
    pub(super) fn new(
        program: &'a mut Program,
        source: &'a [u8],
        path: &'a Path,
    ) -> Result<Self, Error> {
        program.prepare_reader()?;
        let source_id = program.add_source(path)?;
        let lexer = Lexer::new(source, &mut program.names)?;
        Ok(Self {
            program,
            lexer,
            source: path,
            source_id,
            arguments: Vec::new(),
            floating: crate::types::floating::FloatingScanner::default(),
            object_facts: false,
        })
    }
    pub(super) fn fail(&self, message: impl Into<String>) -> Error {
        let message = match self.lexer.token.kind {
            Kind::Invalid(problem) => problem.into(),
            _ => message.into(),
        };
        self.fail_at(self.lexer.token.line, message)
    }
    pub(super) fn fail_at(&self, line: u32, message: impl Into<String>) -> Error {
        Error::at_path(message.into(), self.source, line)
    }
    /// Render a token only at a diagnostic boundary, retaining arbitrary debug-file bytes.
    pub(super) fn fail_token(&self, prefix: &str, suffix: &str) -> Error {
        if let Kind::Invalid(problem) = self.lexer.token.kind {
            return self.fail_at(self.lexer.token.line, problem);
        }
        let mut message = prefix.as_bytes().to_vec();
        self.lexer.token.append_spelling(&mut message);
        message.extend_from_slice(suffix.as_bytes());
        Error::at_path(message, self.source, self.lexer.token.line)
    }
    pub(super) fn at(&self, kind: Kind) -> bool {
        self.lexer.token.kind == kind
    }
    pub(super) fn at_word(&self, text: &str) -> bool {
        self.at(Kind::Word) && self.lexer.token.word() == text
    }
    pub(super) fn advance(&mut self) -> Result<(), Error> {
        self.lexer.advance(&mut self.program.names)
    }
    pub(super) fn accept(&mut self, kind: Kind) -> Result<bool, Error> {
        if !self.at(kind) {
            return Ok(false);
        }
        self.advance()?;
        Ok(true)
    }
    pub(super) fn accept_word(&mut self, text: &str) -> Result<bool, Error> {
        if !self.at_word(text) {
            return Ok(false);
        }
        self.advance()?;
        Ok(true)
    }
    pub(super) fn expect(&mut self, kind: Kind, text: &str) -> Result<(), Error> {
        if self.accept(kind)? {
            Ok(())
        } else {
            Err(self.fail_token(&format!("expected '{text}', got '"), "'"))
        }
    }
    pub(super) fn name(&mut self, kind: Kind, what: &str) -> Result<NameId, Error> {
        if !self.at(kind) {
            return Err(self.fail_token(&format!("expected {what}, got '"), "'"));
        }
        let name = self.program.names.intern(self.lexer.token.word())?;
        self.advance()?;
        Ok(name)
    }
    pub(super) fn parse_type(&mut self) -> Result<Type, Error> {
        let token = self.lexer.token;
        if token.kind == Kind::Word {
            if let Some(ty) = Type::scalar(token.word()) {
                self.advance()?;
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
                self.advance()?;
                return Ok(Type::Object { bytes, align });
            }
        }
        Err(self.fail_token("unknown LowIR type: ", ""))
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
            let declaration = self.accept_word("declare")?;
            if self.accept_word("function")? {
                self.function(!declaration)?;
            } else if self.accept_word("global")? {
                self.global_item(!declaration)?;
            } else if !declaration && self.accept_word("alias")? {
                self.object_alias()?;
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
        let symbol = self.program.reference(name, self.source_id, line)?;
        if self.program.symbol(symbol).declared() {
            return Err(self.fail_at(
                line,
                format!(
                    "duplicate top-level symbol: {}",
                    self.program.names.get(name)
                ),
            ));
        }
        let mut metadata = Metadata::default();
        let signature = self.signature(Some(&mut metadata))?;
        let mut function = Function {
            name,
            signature,
            metadata,
            body: None,
            location: None,
        };
        if definition {
            if self.at(Kind::Debug) || matches!(self.lexer.token.kind, Kind::Invalid(_)) {
                function.location = Some(self.debug_location()?);
            }
            self.expect(Kind::LBrace, "{")?;
            function.body = Some(self.body(&function)?);
        }
        self.program.add(symbol, function)
    }
    pub(super) fn debug_location(&mut self) -> Result<DebugLocation, Error> {
        let token = self.lexer.token;
        let debug = token
            .debug
            .ok_or_else(|| self.fail("malformed !dbg(file, line, column) location"))?;
        for (field, value) in [("line", debug.line), ("column", debug.column)] {
            if value == 0 || value > u64::from(u32::MAX) {
                return Err(self.fail_token(&format!("invalid positive integer: {field} of "), ""));
            }
        }
        let location = DebugLocation {
            file: self.program.names.intern_bytes(debug.file)?,
            line: u32::try_from(debug.line).map_err(|_| Error::Limit)?,
            column: u32::try_from(debug.column).map_err(|_| Error::Limit)?,
        };
        self.advance()?;
        Ok(location)
    }
    pub(super) fn signature(
        &mut self,
        symbol: Option<&mut Metadata>,
    ) -> Result<SignatureId, Error> {
        let mut signature = Signature {
            parameters: Vec::new(),
            result: Type::Void,
            boundary: Boundary::default(),
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
                    classes: None,
                };
                if self.at(Kind::LBracket) {
                    self.parameter_metadata(&mut parameter)?;
                }
                signature.parameters.push(parameter);
                if !self.accept(Kind::Comma)? {
                    break;
                }
            }
        }
        self.expect(Kind::RParen, ")")?;
        self.expect(Kind::Arrow, "->")?;
        signature.result = self.parse_type()?;
        self.metadata(symbol, &mut signature.boundary, signature.result, false)?;
        for (i, p) in signature.parameters.iter().enumerate() {
            if p.pass == Some(Pass::IndirectResult) && (i != 0 || signature.result != Type::Void) {
                return Err(self.fail_at(signature_line, "invalid indirect result parameter"));
            }
        }
        self.program.add_signature(signature)
    }
    fn parameter_metadata(&mut self, parameter: &mut Parameter) -> Result<(), Error> {
        self.advance()?;
        let mut seen = HashSet::new();
        loop {
            if !self.at(Kind::Word) {
                return Err(self.fail("invalid parameter metadata"));
            }
            let key = self.lexer.token.word();
            self.advance()?;
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
                "classes" => {
                    if seen.contains(key) {
                        return Err(self.fail("duplicate metadata key: classes"));
                    }
                    parameter.classes = Some(self.object_classes(parameter.ty)?);
                    seen.insert(key);
                    if !self.accept(Kind::Comma)? {
                        break;
                    }
                    continue;
                }
                _ => return Err(self.fail("invalid parameter metadata")),
            }
            if !seen.insert(key) {
                return Err(self.fail(format!("duplicate metadata key: {key}")));
            }
            self.advance()?;
            if !self.accept(Kind::Comma)? {
                break;
            }
        }
        self.expect(Kind::RBracket, "]")
    }
    pub(super) fn object_classes(&mut self, ty: Type) -> Result<ObjectClasses, Error> {
        let bytes = match ty {
            Type::Object { bytes, .. } => bytes,
            Type::I128 | Type::F80 => 16,
            other => u64::from(other.bits().div_ceil(8)),
        };
        if !self.object_facts && (!matches!(ty, Type::Object { .. }) || bytes > 16) {
            return Err(self.fail("classes metadata requires a direct object of matching size"));
        }
        let classes = if self.at(Kind::Word) {
            if self.object_facts && self.at_word("x87") {
                Some(ObjectClasses::X87)
            } else {
                ObjectClasses::parse(self.lexer.token.word(), bytes)
            }
        } else {
            None
        };
        let Some(classes) = classes else {
            return Err(self.fail("invalid classes metadata"));
        };
        self.advance()?;
        Ok(classes)
    }
    pub(super) fn metadata(
        &mut self,
        mut symbol: Option<&mut Metadata>,
        boundary: &mut Boundary,
        result: Type,
        on_global: bool,
    ) -> Result<(), Error> {
        let mut seen = HashSet::new();
        while self.accept(Kind::LBracket)? {
            loop {
                if !self.at(Kind::Word) {
                    return Err(self.fail_token("expected metadata key, got '", "'"));
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
                self.advance()?;
                self.expect(Kind::Equals, "=")?;
                if !seen.insert(key) {
                    return Err(self.fail(format!("duplicate metadata key: {key}")));
                }
                if symbol.is_none()
                    && !matches!(
                        key,
                        "arity" | "effects" | "unwind" | "return" | "result_classes"
                    )
                {
                    return Err(self.fail("symbol metadata on call signature"));
                }
                let boundary_key = matches!(
                    key,
                    "arity" | "effects" | "unwind" | "return" | "result_classes"
                );
                if on_global && boundary_key {
                    return Err(self.fail("function metadata on non-function"));
                }
                if on_global
                    && matches!(
                        key,
                        "tls_for" | "force_inline" | "inline_hint" | "no_inline"
                    )
                {
                    return Err(self.fail(format!("{key} metadata requires a function")));
                }
                if key == "tls_for" {
                    let target = self.symbol_reference("tls_for global")?;
                    if let Some(metadata) = symbol.as_deref_mut() {
                        metadata.tls_for = Some(target);
                    }
                    if !self.accept(Kind::Comma)? {
                        break;
                    }
                    continue;
                }
                if !on_global && matches!(key, "storage" | "section") {
                    return Err(self.fail(if key == "storage" {
                        "storage metadata on non-global"
                    } else {
                        "section metadata requires a global"
                    }));
                }
                let value = self.lexer.token.word();
                if !(boundary_key
                    || self.at(Kind::Word)
                    || (key == "object" && self.at(Kind::Global)))
                {
                    return Err(self.fail(format!("missing value after {key}")));
                }
                match key {
                    "arity" => {
                        if !self.at_word("variadic") {
                            return Err(self.fail("invalid arity metadata"));
                        }
                        boundary.arity = Arity::Variadic;
                    }
                    "effects" => {
                        boundary.effects = Effects::parse(value);
                        if boundary.effects.is_none() {
                            return Err(self.fail("invalid effects metadata"));
                        }
                    }
                    "unwind" => {
                        if !self.at_word("no") {
                            return Err(self.fail("invalid unwind metadata"));
                        }
                        boundary.unwind = Unwind::No;
                    }
                    "return" => {
                        if !self.at_word("noreturn") {
                            return Err(self.fail("invalid return metadata"));
                        }
                        boundary.returns = Returns::NoReturn;
                    }
                    "result_classes" => {
                        let classes = self.object_classes(result)?;
                        if !self.object_facts && matches!(classes, ObjectClasses::Memory) {
                            return Err(self.fail("a memory-class result is returned through an indirect_result parameter"));
                        }
                        boundary.result_classes = Some(classes);
                        if !self.accept(Kind::Comma)? {
                            break;
                        }
                        continue;
                    }
                    _ => {
                        let Some(metadata) = symbol.as_deref_mut() else {
                            return Err(self.fail("symbol metadata on call signature"));
                        };
                        self.symbol_metadata(metadata, key, on_global)?;
                    }
                }
                self.advance()?;
                if !self.accept(Kind::Comma)? {
                    break;
                }
            }
            self.expect(Kind::RBracket, "]")?;
        }
        if symbol.is_some_and(|m| m.flags & 64 != 0 && m.binding != Some(Binding::Weak)) {
            return Err(self.fail("replaceable metadata requires binding=weak"));
        }
        Ok(())
    }
    fn symbol_metadata(
        &mut self,
        metadata: &mut Metadata,
        key: &str,
        on_global: bool,
    ) -> Result<(), Error> {
        let value = self.lexer.token.word();
        match key {
            "role" => {
                metadata.role = Role::parse(value);
                let Some(role) = metadata.role else {
                    return Err(self.fail("invalid symbol role"));
                };
                if role.global() != on_global {
                    return Err(self.fail(if on_global {
                        "function role on global"
                    } else {
                        "invalid symbol role"
                    }));
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
            "storage" => {
                metadata.storage = Storage::parse(value);
                if metadata.storage.is_none() {
                    return Err(self.fail("invalid global storage metadata"));
                }
            }
            "section" => {
                if !value
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.'))
                {
                    return Err(self.fail("invalid global section name"));
                }
                metadata.section = Some(self.program.names.intern(value)?);
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
        Ok(())
    }
    fn body(&mut self, function: &Function) -> Result<Body, Error> {
        let signature = self.program.signature(function.signature);
        let mut body = Body::new(Some(self.source_id), signature.parameters.len());
        for p in &signature.parameters {
            let id = body.use_temp(p.name, 0)?;
            body.temp_mut(id).ty = Some(p.ty);
        }
        while !self.accept(Kind::RBrace)? {
            if self.at(Kind::End) {
                return Err(self.fail("unexpected end of LowIR"));
            }
            if self.accept_word("slot")? {
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
            } else if self.accept_word("block")? {
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
            if !block.instructions.last().is_some_and(|&id| {
                body.instruction(id).kind.terminator()
                    || matches!(
                        body.instruction(id).kind,
                        super::model::InstructionKind::Call(_)
                    )
            }) {
                return Err(self.fail(format!(
                    "block has no terminator: {}",
                    self.program.names.get(block.name)
                )));
            }
        }
        Ok(())
    }
}
