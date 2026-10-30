//! Construction invariants from lowir/validator.cpp, at private store boundaries. See NOTICE.
use super::{
    Body, GlobalInit, Instruction, InstructionKind as I, NameId, Program, RuntimeInstruction as R,
    SymbolDetail, SymbolId, Value,
};
use crate::lowir::{
    Error,
    construction_names::SlotName,
    vocabulary::{Binding, Type},
};
use crate::support::hash::HashSet;

fn problem(
    program: &Program,
    function: Option<SymbolId>,
    message: &str,
    detail: Option<NameId>,
) -> Error {
    let mut bytes = message.as_bytes().to_vec();
    let mut name = |prefix: &[u8], id: NameId| {
        if id != NameId(0) {
            bytes.extend_from_slice(prefix);
            let spelling = program.names.bytes(id);
            let end = spelling
                .iter()
                .position(|&byte| byte == 0)
                .unwrap_or(spelling.len());
            bytes.extend_from_slice(&spelling[..end]);
        }
    };
    if let Some(detail) = detail {
        name(b": ", detail);
    }
    if let Some(function) = function {
        name(b" in function @", program.symbol(function).name);
    }
    match String::from_utf8(bytes) {
        Ok(message) => Error::Diagnostic(message),
        Err(error) => Error::ByteDiagnostic(error.into_bytes()),
    }
}

fn names(
    program: &Program,
    function: Option<SymbolId>,
    values: impl IntoIterator<Item = NameId>,
    duplicate: &str,
) -> Result<(), Error> {
    let mut seen = HashSet::default();
    for name in values {
        if name.0 as usize >= program.names.text.len() {
            return Err(problem(program, function, "invalid name", None));
        }
        if name != NameId(0) && !seen.insert(name) {
            return Err(problem(program, function, duplicate, Some(name)));
        }
    }
    Ok(())
}

fn links(program: &Program) -> Result<(), Error> {
    let valid_symbol = |symbol: SymbolId| symbol.index() < program.symbols.len();
    for symbol in &program.symbols {
        let fail = |message| problem(program, None, message, Some(symbol.name));
        if symbol.name == NameId(0) || symbol.name.0 as usize >= program.names.text.len() {
            return Err(problem(program, None, "invalid symbol name", None));
        }
        let (metadata, function, global) = match symbol.detail {
            SymbolDetail::Function(id) => {
                let function = program
                    .functions
                    .get(id.0 as usize)
                    .ok_or_else(|| fail("invalid function signature"))?;
                (Some(&function.metadata), Some(function), None)
            }
            SymbolDetail::Global(id) => {
                let global = program
                    .globals
                    .get(id.0 as usize)
                    .ok_or_else(|| fail("invalid global initializer"))?;
                (Some(&global.metadata), None, Some(global))
            }
            SymbolDetail::Unresolved => (None, None, None),
        };
        if let Some(metadata) = metadata {
            if metadata.flags & 128 != 0 {
                return Err(fail("invalid symbol metadata"));
            }
            if metadata.tls_for.is_some_and(|target| !valid_symbol(target)) {
                return Err(fail("invalid tls_for target"));
            }
            if metadata.flags & 64 != 0 && metadata.binding != Some(Binding::Weak) {
                return Err(fail("replaceable metadata requires binding=weak"));
            }
        }
        if function
            .is_some_and(|function| function.signature.0 as usize >= program.signatures.len())
        {
            return Err(fail("invalid function signature"));
        }
        if let Some(global) = global {
            match &global.init {
                GlobalInit::Address(target, _) if !valid_symbol(*target) => {
                    return Err(fail("invalid global initializer"));
                }
                GlobalInit::Data(items) => {
                    if global
                        .ty
                        .is_some_and(|ty| !matches!(ty, Type::Object { .. }))
                    {
                        return Err(fail(
                            "a structured global's storage type must be an object type",
                        ));
                    }
                    if items.iter().any(|item| matches!(item, super::DataItem::Address(_, target, _) if !valid_symbol(*target))) {
                        return Err(fail("invalid global initializer"));
                    }
                }
                _ => {}
            }
        }
    }
    for alias in &program.aliases {
        if alias.object == NameId(0)
            || alias.object.0 as usize >= program.names.text.len()
            || !valid_symbol(alias.target)
        {
            return Err(problem(program, None, "invalid object alias", None));
        }
    }
    Ok(())
}

pub fn validate(program: &Program, spellings: bool) -> Result<(), Error> {
    // Enums, owned parameter/data vectors, typed pairs and private store IDs
    // replace the C++ discriminant and span representation checks.
    links(program)?;
    for signature in &program.signatures {
        names(
            program,
            None,
            signature.parameters.iter().map(|parameter| parameter.name),
            "duplicate parameter",
        )?;
    }
    for (index, symbol) in program.symbols.iter().enumerate() {
        let SymbolDetail::Function(id) = symbol.detail else {
            continue;
        };
        let function = &program.functions[id.0 as usize];
        let Some(body) = &function.body else { continue };
        let symbol = SymbolId(u32::try_from(index).map_err(|_| Error::Limit)?);
        if spellings {
            names(
                program,
                Some(symbol),
                body.temps
                    .iter()
                    .filter(|temp| !temp.generated)
                    .map(|temp| temp.name),
                "duplicate temporary name",
            )?;
            names(
                program,
                Some(symbol),
                (0..body.slots.len()).filter_map(|index| match body.slot_name(index) {
                    SlotName::Source(name) => Some(name),
                    _ => None,
                }),
                "duplicate slot",
            )?;
            names(
                program,
                Some(symbol),
                body.blocks.iter().filter_map(super::Block::source_name),
                "duplicate block",
            )?;
        }
        for &block in &body.layout {
            let block = body.blocks.get(block.index()).ok_or_else(|| {
                problem(
                    program,
                    Some(symbol),
                    "undefined or invalid block target",
                    None,
                )
            })?;
            for &id in &block.instructions {
                let inst = body
                    .instructions
                    .get(id.index())
                    .ok_or_else(|| Error::Construction("invalid instruction link".into()))?;
                if let Some(message) = operands(program, body, inst) {
                    return Err(problem(program, Some(symbol), message, None));
                }
                let defines = defines_value(&inst.kind);
                if !defines {
                    if inst.result.is_some() {
                        return Err(problem(
                            program,
                            Some(symbol),
                            "a void instruction has no result",
                            None,
                        ));
                    }
                    continue;
                }
                let temp = inst.result.and_then(|temp| body.temps.get(temp.index()));
                let Some(temp) = temp.filter(|temp| temp.definition == Some(id)) else {
                    return Err(problem(
                        program,
                        Some(symbol),
                        "value instruction without its result temporary",
                        None,
                    ));
                };
                let ty = inst.kind.result_type();
                if ty == Type::Void {
                    return Err(problem(
                        program,
                        Some(symbol),
                        "a void instruction has no result",
                        None,
                    ));
                }
                if temp.ty != Some(ty) {
                    return Err(problem(
                        program,
                        Some(symbol),
                        "result temporary type mismatch",
                        Some(temp.name),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn defines_value(kind: &I) -> bool {
    match kind {
        I::Const(..)
        | I::Copy(..)
        | I::Unary(..)
        | I::Binary(..)
        | I::Compare(..)
        | I::Convert(..)
        | I::Select(..)
        | I::Phi(..)
        | I::Addr(..)
        | I::Load(..)
        | I::Index(..)
        | I::Runtime(
            R::AtomicLoad(..)
            | R::AtomicUpdate(..)
            | R::AtomicCompareExchange { .. }
            | R::VaArg(..)
            | R::StackAlloc(..)
            | R::Exception(..)
            | R::ExceptionSelector(..),
        ) => true,
        I::Call(call) => call.result != Type::Void,
        _ => false,
    }
}

fn operands(program: &Program, body: &Body, inst: &Instruction) -> Option<&'static str> {
    let valid = |value: Value| match value {
        Value::Temp(id) => id.index() < body.temps.len(),
        Value::Slot(id) => id.index() < body.slots.len(),
        Value::Integer(id) | Value::Float(id, _) => (id.0 as usize) < body.literals.len(),
        Value::Symbol(id) => id.index() < program.symbols.len(),
    };
    let arguments_valid = |arguments: super::Arguments| {
        usize::try_from(u64::from(arguments.first) + u64::from(arguments.count))
            .is_ok_and(|end| end <= body.operands.len())
    };
    let kinds = match &inst.kind {
        I::Const(_, value) => matches!(value, Value::Integer(_) | Value::Float(..)),
        I::Addr(value) => matches!(value, Value::Slot(_) | Value::Symbol(_)),
        I::Load(_, _, address) | I::Store(_, _, _, address) => !matches!(address, Value::Float(..)),
        I::Call(call) => {
            if !arguments_valid(call.arguments) {
                return Some("invalid operand for the instruction");
            }
            matches!(call.callee, Value::Temp(_) | Value::Symbol(_))
        }
        I::Runtime(R::EhFilter(arguments, _)) => {
            if !arguments_valid(*arguments) {
                return Some("invalid operand for the instruction");
            }
            body.arguments(*arguments)
                .iter()
                .all(|value| matches!(value, Value::Symbol(_)))
        }
        I::Phi(_, pairs) => pairs
            .iter()
            .all(|(block, _)| block.index() < body.blocks.len()),
        _ => true,
    };
    let mut targets_valid = true;
    inst.kind
        .visit_targets(|target| targets_valid &= target.index() < body.blocks.len());
    if let I::Runtime(runtime) = &inst.kind
        && let Some(target) = runtime.handler_target()
    {
        targets_valid &= target.index() < body.blocks.len();
    }
    let mut values_valid = true;
    body.visit_values(&inst.kind, |value| values_valid &= valid(value));
    if !kinds || !values_valid || !targets_valid {
        return Some("invalid operand for the instruction");
    }
    if inst
        .location
        .is_some_and(|location| location.0.get() as usize > body.locations.len())
    {
        return Some("invalid instruction field");
    }
    if matches!(inst.kind, I::CopyObject(bytes, align, ..) | I::ZeroInit(bytes, align, _) if bytes == 0 || align >= 64)
    {
        return Some("invalid instruction field");
    }
    if matches!(&inst.kind, I::Call(call) if call.signature.is_some_and(|signature| signature.0 as usize >= program.signatures.len()))
    {
        return Some("invalid call signature");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostic(program: &Program, expected: &str) {
        assert!(
            matches!(validate(program, true), Err(Error::Diagnostic(actual)) if actual == expected)
        );
    }
    #[test]
    fn metadata_failures_precede_function_and_global_link_failures() {
        let mut program = crate::lowir::test_program(
            b"global @g:obj<8x8>={zero 8}\nfunction @f()->void {block ^entry: return void}\n",
        )
        .unwrap();
        program.functions[0].signature = super::super::SignatureId(u32::MAX);
        program.functions[0].metadata.flags = 128;
        diagnostic(&program, "invalid symbol metadata: f");
        program.functions[0].metadata.flags = 0;
        diagnostic(&program, "invalid function signature: f");
        program.globals[0].metadata.flags = 128;
        program.globals[0].ty = Some(Type::I64);
        diagnostic(&program, "invalid symbol metadata: g");
        program.globals[0].metadata.flags = 0;
        diagnostic(
            &program,
            "a structured global's storage type must be an object type: g",
        );
    }
}
