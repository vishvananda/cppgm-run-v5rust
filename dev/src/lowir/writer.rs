//! Bounded streaming serializer over typed `LowIR` facts. See NOTICE.
use super::LocalNames;
use super::{
    literals::{self, Constant, FloatKind},
    model::{
        Arity, Body, Call, DataItem, DebugLocation, Facts, Function, Global, GlobalInit,
        InstructionKind as I, Linkage, Metadata, Program, Returns, Signature, Unwind, Value,
    },
    vocabulary::Type,
};
use crate::types::floating::FloatingScanner;
use std::io::{self, Write};
pub(crate) fn write(program: &Program, mut out: impl Write) -> io::Result<()> {
    let mut floating = FloatingScanner::default();
    let mut written = false;
    for phase in 0..4 {
        let mut phase_written = false;
        if phase == 0 || phase == 2 {
            for global in program
                .globals_in_publication_order()
                .filter(|g| matches!(g.init, GlobalInit::Declaration) == (phase == 0))
            {
                if written && !phase_written {
                    writeln!(out)?;
                }
                phase_written = true;
                write_global(program, global, &mut floating, &mut out)?;
            }
        } else {
            let functions = program
                .function_declarations()
                .filter(|_| phase == 1)
                .chain(program.publications().iter().filter_map(|&id| {
                    program
                        .function(program.symbol(id))
                        .filter(|f| phase == 3 && f.body.is_some())
                }));
            for function in functions {
                if written && !phase_written {
                    writeln!(out)?;
                }
                phase_written = true;
                write_function(program, function, &mut floating, &mut out)?;
            }
        }
        written |= phase_written;
    }
    write_aliases(program, &mut out)?;
    out.flush()
}
pub(crate) fn write_aliases(program: &Program, out: &mut impl Write) -> io::Result<()> {
    for alias in program.aliases() {
        writeln!(
            out,
            "alias object {} = @{}",
            program.names.get(alias.object),
            program.names.get(program.symbol(alias.target).name)
        )?;
    }
    Ok(())
}
pub(crate) fn write_globals(
    program: &Program,
    out: &mut impl Write,
    definitions_follow: bool,
) -> io::Result<()> {
    let mut floating = FloatingScanner::default();
    let mut written = false;
    for phase in 0..3 {
        let mut phase_written = false;
        if phase == 1 {
            for function in program.function_declarations() {
                if written && !phase_written {
                    writeln!(out)?;
                }
                phase_written = true;
                write_function(program, function, &mut floating, out)?;
            }
        } else {
            for global in program
                .globals_in_publication_order()
                .filter(|global| matches!(global.init, GlobalInit::Declaration) == (phase == 0))
            {
                if written && !phase_written {
                    writeln!(out)?;
                }
                phase_written = true;
                write_global(program, global, &mut floating, out)?;
            }
        }
        written |= phase_written;
    }
    if written && definitions_follow {
        writeln!(out)?;
    }
    Ok(())
}
fn write_address(
    program: &Program,
    target: super::model::SymbolId,
    addend: i64,
    out: &mut impl Write,
) -> io::Result<()> {
    write!(
        out,
        "addr @{}",
        program.names.get(program.symbol(target).name)
    )?;
    if addend > 0 {
        write!(out, " + {addend}")?;
    }
    if addend < 0 {
        write!(out, " - {}", addend.unsigned_abs())?;
    }
    Ok(())
}
fn write_global(
    program: &Program,
    global: &Global,
    floating: &mut FloatingScanner,
    out: &mut impl Write,
) -> io::Result<()> {
    let declaration = matches!(global.init, GlobalInit::Declaration);
    write!(
        out,
        "{}global @{}",
        if declaration { "declare " } else { "" },
        program.names.get(global.name)
    )?;
    if let Some(ty) = global.ty {
        write!(out, " : {ty}")?;
    }
    metadata(program, Some(&global.metadata), None, out)?;
    if declaration {
        return writeln!(out);
    }
    write!(out, " = ")?;
    match &global.init {
        GlobalInit::Declaration => return Ok(()),
        GlobalInit::Zero => write!(out, "zero")?,
        GlobalInit::Constant(constant) => write_constant(
            out,
            floating,
            *constant,
            global.ty.unwrap_or(Type::Void),
            program.object_facts,
            false,
        )?,
        GlobalInit::Address(target, addend) => write_address(program, *target, *addend, out)?,
        GlobalInit::Data(items) => {
            writeln!(out, "{{")?;
            for item in items {
                write!(out, "  ")?;
                match item {
                    DataItem::Zero(amount) => write!(out, "zero {amount}")?,
                    DataItem::Constant(ty, constant) => {
                        write!(out, "{ty} ")?;
                        write_constant(out, floating, *constant, *ty, program.object_facts, false)?;
                    }
                    DataItem::Address(ty, target, addend) => {
                        write!(out, "{ty} ")?;
                        write_address(program, *target, *addend, out)?;
                    }
                }
                writeln!(out)?;
            }
            write!(out, "}}")?;
        }
    }
    writeln!(out)
}
pub(crate) fn write_definition(
    program: &Program,
    function: &Function,
    output: &mut impl Write,
) -> io::Result<()> {
    write_function(program, function, &mut FloatingScanner::default(), output)
}
fn write_function(
    program: &Program,
    f: &Function,
    floating: &mut FloatingScanner,
    out: &mut impl Write,
) -> io::Result<()> {
    write_function_header(program, f, f.body.is_some(), out)?;
    let Some(body) = &f.body else { return Ok(()) };
    write_function_body(program, body, floating, out)
}
pub(crate) fn write_definition_header(
    program: &Program,
    f: &Function,
    out: &mut impl Write,
) -> io::Result<()> {
    write_function_header(program, f, true, out)
}
fn write_function_header(
    program: &Program,
    f: &Function,
    defined: bool,
    out: &mut impl Write,
) -> io::Result<()> {
    write!(
        out,
        "{}function @{}",
        if defined { "" } else { "declare " },
        program.names.get(f.name)
    )?;
    let signature = program.signature(f.signature);
    let mut local_names = f
        .body
        .as_ref()
        .map(|body| LocalNames::new(&program.names, body).map_err(local_error))
        .transpose()?;
    parameters(program, signature, local_names.as_mut(), out)?;
    metadata(program, Some(&f.metadata), Some(signature), out)?;
    if !defined {
        return writeln!(out);
    }
    write_location(program, f.location, out)?;
    writeln!(out, " {{")
}
pub(crate) fn write_definition_body(
    program: &Program,
    f: &Function,
    out: &mut impl Write,
) -> io::Result<()> {
    let body = f
        .body
        .as_ref()
        .ok_or_else(|| io::Error::other("missing deferred definition body"))?;
    write_function_body(program, body, &mut FloatingScanner::default(), out)
}
fn write_function_body(
    program: &Program,
    body: &Body,
    floating: &mut FloatingScanner,
    out: &mut impl Write,
) -> io::Result<()> {
    let mut names = LocalNames::new(&program.names, body).map_err(local_error)?;
    for (index, slot) in body.slots().iter().enumerate() {
        writeln!(
            out,
            "  slot ${} : {}",
            names.slot(index).map_err(local_error)?,
            slot.ty
        )?;
    }
    for (i, &block) in body.layout.iter().enumerate() {
        if i != 0 || !body.slots().is_empty() {
            writeln!(out)?;
        }
        writeln!(
            out,
            "  block ^{}:",
            names.block(block).map_err(local_error)?
        )?;
        for &id in &body.block(block).instructions {
            let inst = body.instruction(id);
            write!(out, "    ")?;
            if let Some(t) = inst.result {
                write!(out, "%{} = ", names.temp(t.index()).map_err(local_error)?)?;
            }
            let mut writer = InstructionWriter {
                program,
                body,
                out,
                floating,
                locals: &mut names,
            };
            writer.instruction(&inst.kind)?;
            write_location(program, body.location(inst), out)?;
            writeln!(out)?;
        }
    }
    writeln!(out, "}}")
}
fn write_location(
    program: &Program,
    location: Option<DebugLocation>,
    out: &mut impl Write,
) -> io::Result<()> {
    if let Some(location) = location {
        write!(out, " !dbg(")?;
        // C++'s location writer uses a C string, even though the interner keeps its full bytes.
        let file = program.names.bytes(location.file);
        let end = file
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(file.len());
        out.write_all(&file[..end])?;
        write!(out, ", {}, {})", location.line, location.column)?;
    }
    Ok(())
}
fn local_error(error: super::Error) -> io::Error {
    match error {
        super::Error::Output(error) => error,
        super::Error::Limit => io::Error::other("local naming limit"),
        error => io::Error::other(format!("{error:?}")),
    }
}
fn parameters(
    program: &Program,
    signature: &Signature,
    mut locals: Option<&mut LocalNames<'_>>,
    out: &mut impl Write,
) -> io::Result<()> {
    write!(out, "(")?;
    for (i, p) in signature.parameters.iter().enumerate() {
        if i != 0 {
            write!(out, ", ")?;
        }
        let name = if let Some(names) = &mut locals {
            names.temp(i).map_err(local_error)?
        } else {
            program.names.get(p.name)
        };
        write!(out, "%")?;
        if name.is_empty() {
            let mut name = format!("arg{i}");
            while signature
                .parameters
                .iter()
                .any(|other| program.names.get(other.name) == name)
            {
                name.push('_');
            }
            write!(out, "{name}")?;
        } else {
            write!(out, "{name}")?;
        }
        write!(out, " : {}", p.ty)?;
        let mut fields = Vec::new();
        if let Some(pass) = p.pass {
            fields.push(format!("pass={}", pass.spelling()));
        }
        if p.no_alias {
            fields.push("alias=noalias".into());
        }
        if p.object_bytes != 0 {
            fields.push(format!("object_bytes={}", p.object_bytes));
        }
        if let Some(classes) = p.classes {
            fields.push(format!("classes={classes}"));
        }
        if !fields.is_empty() {
            write!(out, " [{}]", fields.join(", "))?;
        }
    }
    write!(out, ") -> {}", signature.result)?;
    Ok(())
}
fn metadata(
    program: &Program,
    symbol: Option<&Metadata>,
    signature: Option<&Signature>,
    out: &mut impl Write,
) -> io::Result<()> {
    let mut fields = Vec::new();
    if let Some(signature) = signature {
        let m = &signature.boundary;
        if m.arity == Arity::Variadic {
            fields.push("arity=variadic".into());
        }
        if let Some(effects) = m.effects {
            fields.push(format!("effects={}", effects.spelling()));
        }
        if m.unwind == Unwind::No {
            fields.push("unwind=no".into());
        }
        if m.returns == Returns::NoReturn {
            fields.push("return=noreturn".into());
        }
        if let Some(classes) = m.result_classes {
            fields.push(format!("result_classes={classes}"));
        }
    }
    if let Some(m) = symbol {
        if let Some(storage) = m.storage {
            fields.push(format!("storage={}", storage.spelling()));
        }
        if let Some(section) = m.section {
            fields.push(format!("section={}", program.names.get(section)));
        }
        if let Some(role) = m.role {
            fields.push(format!("role={}", role.spelling()));
        }
        if m.linkage == Linkage::C {
            fields.push("linkage=c".into());
        }
        if let Some(binding) = m.binding {
            fields.push(format!("binding={}", binding.spelling()));
        }
        if m.flags & 64 != 0 {
            fields.push("replaceable=yes".into());
        }
        if let Some(object) = m.object {
            fields.push(format!("object={}", program.names.get(object)));
        }
        if let Some(target) = m.tls_for {
            fields.push(format!(
                "tls_for=@{}",
                program.names.get(program.symbol(target).name)
            ));
        }
        for (i, name) in [
            "keep_alias",
            "prefer_local",
            "object_root",
            "force_inline",
            "inline_hint",
            "no_inline",
        ]
        .iter()
        .enumerate()
        {
            if m.flags & (1 << i) != 0 {
                fields.push(format!("{name}=yes"));
            }
        }
    }
    if !fields.is_empty() {
        write!(out, " [{}]", fields.join(", "))?;
    }
    Ok(())
}
struct InstructionWriter<'a, 'b, W> {
    program: &'a Program,
    body: &'a Body,
    out: &'b mut W,
    floating: &'b mut FloatingScanner,
    locals: &'b mut LocalNames<'a>,
}
impl<W: Write> InstructionWriter<'_, '_, W> {
    fn value(&mut self, value: Value, ty: Type) -> io::Result<()> {
        match value {
            Value::Temp(t) => write!(
                self.out,
                "%{}",
                self.locals.temp(t.index()).map_err(local_error)?
            ),
            Value::Slot(s) => write!(
                self.out,
                "${}",
                self.locals.slot(s.index()).map_err(local_error)?
            ),
            Value::Symbol(id) => write!(
                self.out,
                "@{}",
                self.program.names.get(self.program.symbol(id).name)
            ),
            Value::Integer(id) => write_constant(
                self.out,
                self.floating,
                Constant::Integer(self.body.literal(id)),
                ty,
                self.program.object_facts,
                false,
            ),
            Value::Float(id, kind) => write_constant(
                self.out,
                self.floating,
                Constant::Float(kind, self.body.literal(id)),
                ty,
                self.program.object_facts,
                false,
            ),
        }
    }
    fn block(&mut self, block: super::model::BlockId) -> io::Result<()> {
        write!(
            self.out,
            "^{}",
            self.locals.block(block).map_err(local_error)?
        )
    }
    fn comma(&mut self) -> io::Result<()> {
        write!(self.out, ", ")
    }
    fn memory_facts(&mut self, f: Facts) -> io::Result<()> {
        if f.volatile() {
            write!(self.out, "volatile ")?;
        }
        if f.typed() {
            write!(self.out, "typed ")?;
        }
        Ok(())
    }
    fn call(&mut self, call: &Call) -> io::Result<()> {
        write!(self.out, "call {} ", call.result)?;
        self.value(call.callee, Type::Void)?;
        write!(self.out, "(")?;
        for (index, &argument) in self.body.arguments(call.arguments).iter().enumerate() {
            if index != 0 {
                self.comma()?;
            }
            let kind = self
                .program
                .call_boundary(call)
                .and_then(|s| s.parameters.get(index))
                .and_then(|p| FloatKind::from_type(p.ty));
            if let (Value::Integer(id), Some(kind)) = (argument, kind) {
                write_constant(
                    self.out,
                    self.floating,
                    Constant::Integer(self.body.literal(id)),
                    kind.ty(),
                    self.program.object_facts,
                    true,
                )?;
            } else {
                self.value(argument, Type::Void)?;
            }
        }
        write!(self.out, ")")?;
        if call.copy_elision || self.program.object_facts && !call.argument_classes.is_empty() {
            let mut separator = " [";
            if call.copy_elision {
                write!(self.out, "{separator}elision=copy")?;
                separator = ", ";
            }
            if self.program.object_facts {
                for &(argument, classes) in &call.argument_classes {
                    write!(self.out, "{separator}classes={argument}:{classes}")?;
                    separator = ", ";
                }
            }
            write!(self.out, "]")?;
        }
        if let Some(id) = call.signature {
            write!(self.out, " as ")?;
            let signature = self.program.signature(id);
            parameters(self.program, signature, None, self.out)?;
            metadata(self.program, None, Some(signature), self.out)?;
        }
        Ok(())
    }
    fn instruction(&mut self, inst: &I) -> io::Result<()> {
        match inst {
            I::Const(t, v) | I::Copy(t, v) => {
                write!(
                    self.out,
                    "{} {t} ",
                    if matches!(inst, I::Const(..)) {
                        "const"
                    } else {
                        "copy"
                    }
                )?;
                self.value(*v, *t)
            }
            I::Unary(op, t, v) => {
                write!(self.out, "unary {} {t} ", op.spelling())?;
                self.value(*v, *t)
            }
            I::Binary(op, facts, t, a, b) => {
                write!(self.out, "binary {} ", op.spelling())?;
                if facts.nowrap() {
                    write!(self.out, "nowrap ")?;
                }
                if facts.exact() {
                    write!(self.out, "exact ")?;
                }
                write!(self.out, "{t} ")?;
                self.value(*a, *t)?;
                self.comma()?;
                self.value(*b, *t)
            }
            I::Compare(op, t, a, b) => {
                write!(self.out, "cmp {} {t} ", op.spelling())?;
                self.value(*a, *t)?;
                self.comma()?;
                self.value(*b, *t)
            }
            I::Convert(op, to, from, v) => {
                write!(self.out, "convert {} {to} {from} ", op.spelling())?;
                self.value(*v, *from)
            }
            I::Select(t, condition, a, b) => {
                write!(self.out, "select {t} ")?;
                self.value(*condition, Type::I64)?;
                self.comma()?;
                self.value(*a, *t)?;
                self.comma()?;
                self.value(*b, *t)
            }
            I::Phi(t, pairs) => {
                write!(self.out, "phi {t} [")?;
                for (i, &(b, v)) in pairs.iter().enumerate() {
                    if i != 0 {
                        self.comma()?;
                    }
                    self.block(b)?;
                    write!(self.out, ": ")?;
                    self.value(v, *t)?;
                }
                write!(self.out, "]")
            }
            I::Addr(v) => {
                write!(self.out, "addr ")?;
                self.value(*v, Type::Void)
            }
            I::Load(f, t, v) => {
                write!(self.out, "load ")?;
                self.memory_facts(*f)?;
                write!(self.out, "{t} ")?;
                self.value(*v, Type::Void)
            }
            I::Store(f, t, a, b) => {
                write!(self.out, "store ")?;
                self.memory_facts(*f)?;
                write!(self.out, "{t} ")?;
                self.value(*a, *t)?;
                self.comma()?;
                self.value(*b, Type::Void)
            }
            I::Index(t, projection, a, b) => {
                write!(self.out, "index {t}")?;
                if let Some(p) = projection {
                    write!(self.out, " [projection={}]", p.spelling())?;
                }
                write!(self.out, " ")?;
                self.value(*a, Type::Void)?;
                self.comma()?;
                self.value(*b, Type::Void)
            }
            I::CopyObject(bytes, align, source, dest) => {
                write!(self.out, "copyobj {bytes}x{} ", 1_u64 << align)?;
                self.value(*source, Type::Void)?;
                self.comma()?;
                self.value(*dest, Type::Void)
            }
            I::ZeroInit(bytes, align, dest) => {
                write!(self.out, "zeroinit {bytes}x{} ", 1_u64 << align)?;
                self.value(*dest, Type::Void)
            }
            I::Jump(b) => {
                write!(self.out, "jump ")?;
                self.block(*b)
            }
            I::Branch(v, a, b) => {
                write!(self.out, "branch ")?;
                self.value(*v, Type::I64)?;
                self.comma()?;
                self.block(*a)?;
                self.comma()?;
                self.block(*b)
            }
            I::Switch(v, default, pairs) => {
                write!(self.out, "switch ")?;
                self.value(*v, Type::Void)?;
                self.comma()?;
                self.block(*default)?;
                for &(v, b) in pairs {
                    self.comma()?;
                    self.value(v, Type::Void)?;
                    write!(self.out, ":")?;
                    self.block(b)?;
                }
                Ok(())
            }
            I::Return(t, v) => {
                write!(self.out, "return {t}")?;
                if let Some(v) = v {
                    write!(self.out, " ")?;
                    self.value(*v, *t)?;
                }
                Ok(())
            }
            I::Call(call) => self.call(call),
            I::Unreachable => write!(self.out, "unreachable"),
            I::Runtime(inst) => self.runtime_instruction(inst),
        }
    }
    fn runtime_instruction(&mut self, inst: &super::model::RuntimeInstruction) -> io::Result<()> {
        use super::model::RuntimeInstruction as R;
        match inst {
            R::AtomicLoad(ty, address, order, _) => {
                write!(self.out, "atomic_load {ty} ")?;
                self.value(*address, Type::Ptr)?;
                write!(self.out, ", {}", order.number())
            }
            R::AtomicStore(ty, value, address, order) => {
                write!(self.out, "atomic_store {ty} ")?;
                self.value(*value, *ty)?;
                self.comma()?;
                self.value(*address, Type::Ptr)?;
                write!(self.out, ", {}", order.number())
            }
            R::AtomicUpdate(operation, ty, address, value, order) => {
                write!(self.out, "{} {ty} ", operation.spelling())?;
                self.value(*address, Type::Ptr)?;
                self.comma()?;
                self.value(*value, *ty)?;
                write!(self.out, ", {}", order.number())
            }
            R::AtomicCompareExchange {
                ty,
                address,
                expected,
                desired,
                success,
                failure,
            } => {
                write!(self.out, "atomic_compare_exchange {ty} ")?;
                self.value(*address, Type::Ptr)?;
                self.comma()?;
                self.value(*expected, Type::Ptr)?;
                self.comma()?;
                self.value(*desired, *ty)?;
                write!(self.out, ", {}, {}", success.number(), failure.number())
            }
            R::Fence(fence, order) => write!(self.out, "{} {}", fence.spelling(), order.number()),
            R::VaStart(list) => {
                write!(self.out, "va_start ")?;
                self.value(*list, Type::Void)
            }
            R::VaArg(ty, list, classes) => {
                write!(self.out, "va_arg {ty} ")?;
                self.value(*list, Type::Ptr)?;
                if let Some(classes) = classes {
                    write!(self.out, " [classes={classes}]")?;
                }
                Ok(())
            }
            R::StackAlloc(size) => {
                write!(self.out, "stack_alloc ")?;
                self.value(*size, Type::Void)
            }
            R::Exception(ty) => write!(self.out, "exception {ty}"),
            R::ExceptionSelector(ty) => write!(self.out, "exception_selector {ty}"),
            R::EhTry(block) => {
                write!(self.out, "eh_try ")?;
                self.block(*block)
            }
            R::EhCleanup(block) => {
                write!(self.out, "eh_cleanup")?;
                if let Some(block) = block {
                    write!(self.out, " ")?;
                    self.block(*block)?;
                }
                Ok(())
            }
            R::EhCatch(symbol, selector) => {
                write!(self.out, "eh_catch ")?;
                self.value(Value::Symbol(*symbol), Type::Void)?;
                self.selector(*selector, ", ")
            }
            R::EhFilter(arguments, selector) => {
                write!(self.out, "eh_filter")?;
                let symbols = self.body.arguments(*arguments);
                for (index, &symbol) in symbols.iter().enumerate() {
                    write!(self.out, "{}", if index == 0 { " " } else { ", " })?;
                    self.value(symbol, Type::Void)?;
                }
                self.selector(*selector, if symbols.is_empty() { " " } else { ", " })
            }
            R::EhCatchAll(selector) => {
                write!(self.out, "eh_catch_all")?;
                self.selector(*selector, ", ")
            }
            R::EhEnd => write!(self.out, "eh_end"),
            R::Throw(ty, value) => {
                write!(self.out, "throw {ty} ")?;
                self.value(*value, *ty)
            }
            R::Resume => write!(self.out, "resume"),
        }
    }
    fn selector(&mut self, selector: Option<i64>, separator: &str) -> io::Result<()> {
        if let Some(selector) = selector {
            write!(self.out, "{separator}{selector}")?;
        }
        Ok(())
    }
}
fn write_constant(
    out: &mut impl Write,
    scanner: &mut FloatingScanner,
    constant: Constant,
    context: Type,
    exact: bool,
    marked: bool,
) -> io::Result<()> {
    let text = match constant {
        Constant::Integer(bits) => {
            if let Some(kind) = FloatKind::from_type(context) {
                literals::integer_as_float(scanner, bits, kind, false)
                    .and_then(|bits| literals::text(scanner, kind, bits, exact, marked))
            } else {
                Ok(integer_text(bits, context))
            }
        }
        Constant::Float(kind, bits) => {
            literals::text(scanner, kind, bits, exact, context.float_rank() == 0)
        }
    }
    .map_err(|_| io::Error::other("floating literal storage limit"))?;
    out.write_all(text.as_bytes())
}
fn integer_text(mut bits: u128, ty: Type) -> String {
    let width = if ty == Type::Ptr { 64 } else { ty.bits() };
    let signed = width > 1 && !matches!(ty, Type::Ptr | Type::U8 | Type::U16 | Type::U32);
    if width != 0 && width < 128 {
        let mask = (1_u128 << width) - 1;
        bits &= mask;
        if signed && bits & (1_u128 << (width - 1)) != 0 {
            bits |= !mask;
        }
    }
    if (width == 0 || signed) && bits >> 127 != 0 {
        format!("-{}", bits.wrapping_neg())
    } else {
        bits.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::model::{Boundary, Parameter};

    #[test]
    fn unnamed_boundary_parameters_skip_explicit_name_collisions() {
        let mut program = Program::default();
        let parameters = [
            ("", Type::Ptr),
            ("arg0", Type::I32),
            ("arg0_", Type::I64),
            ("", Type::I8),
        ]
        .into_iter()
        .map(|(name, ty)| Parameter {
            name: program.names.intern(name).unwrap(),
            ty,
            pass: None,
            no_alias: false,
            object_bytes: 0,
            classes: None,
        })
        .collect();
        let signature = Signature {
            parameters,
            result: Type::Void,
            boundary: Boundary::default(),
        };
        let mut output = Vec::new();
        super::parameters(&program, &signature, None, &mut output).unwrap();
        assert_eq!(
            output,
            b"(%arg0__ : ptr, %arg0 : i32, %arg0_ : i64, %arg3 : i8) -> void"
        );
    }
}
