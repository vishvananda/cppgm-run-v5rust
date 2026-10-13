//! Bounded streaming serializer over typed `LowIR` facts. See NOTICE.
use super::{
    model::{
        Arity, Body, Facts, Function, InstructionKind as I, Linkage, Program, Returns, Unwind,
        Value,
    },
    vocabulary::Type,
};
use std::io::{self, Write};
pub(super) fn write(program: &Program, mut out: impl Write) -> io::Result<()> {
    let mut written = false;
    for definition in [false, true] {
        let mut phase_written = false;
        for function in program
            .functions()
            .iter()
            .filter(|f| f.body.is_some() == definition)
        {
            if written && !phase_written {
                writeln!(out)?;
            }
            phase_written = true;
            write_function(program, function, &mut out)?;
        }
        written |= phase_written;
    }
    out.flush()
}
fn write_function(program: &Program, f: &Function, out: &mut impl Write) -> io::Result<()> {
    write!(
        out,
        "{}function @{}(",
        if f.body.is_some() { "" } else { "declare " },
        program.names.get(f.name)
    )?;
    for (i, p) in f.parameters.iter().enumerate() {
        if i != 0 {
            write!(out, ", ")?;
        }
        write!(out, "%{} : {}", program.names.get(p.name), p.ty)?;
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
        if !fields.is_empty() {
            write!(out, " [{}]", fields.join(", "))?;
        }
    }
    write!(out, ") -> {}", f.result)?;
    let m = &f.metadata;
    let mut fields = Vec::new();
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
    if !fields.is_empty() {
        write!(out, " [{}]", fields.join(", "))?;
    }
    let Some(body) = &f.body else {
        return writeln!(out);
    };
    writeln!(out, " {{")?;
    for slot in body.slots() {
        writeln!(
            out,
            "  slot ${} : {}",
            program.names.get(slot.name),
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
            program.names.get(body.block(block).name)
        )?;
        for &id in &body.block(block).instructions {
            let inst = body.instruction(id);
            write!(out, "    ")?;
            if let Some(t) = inst.result {
                write!(out, "%{} = ", program.names.get(body.temp(t).name))?;
            }
            let mut writer = InstructionWriter { program, body, out };
            writer.instruction(&inst.kind)?;
            writeln!(out)?;
        }
    }
    writeln!(out, "}}")
}
struct InstructionWriter<'a, W> {
    program: &'a Program,
    body: &'a Body,
    out: &'a mut W,
}
impl<W: Write> InstructionWriter<'_, W> {
    fn value(&mut self, value: Value, ty: Type) -> io::Result<()> {
        match value {
            Value::Temp(t) => write!(
                self.out,
                "%{}",
                self.program.names.get(self.body.temp(t).name)
            ),
            Value::Slot(s) => write!(
                self.out,
                "${}",
                self.program.names.get(self.body.slot(s).name)
            ),
            Value::Integer(id) => self
                .out
                .write_all(integer_text(self.body.literal(id), ty).as_bytes()),
        }
    }
    fn block(&mut self, block: super::model::BlockId) -> io::Result<()> {
        write!(
            self.out,
            "^{}",
            self.program.names.get(self.body.block(block).name)
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
            I::Unreachable => write!(self.out, "unreachable"),
        }
    }
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
