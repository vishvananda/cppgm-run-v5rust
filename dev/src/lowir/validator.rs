//! C++ `LowIR` validator's layout-order, scalar/fact and phi checks. See NOTICE.
use super::{
    Error,
    model::{
        Arity, BlockId, Body, Call, Function, Instruction, InstructionKind as I, Program,
        RuntimeInstruction as R, SymbolDetail, Value,
    },
    vocabulary::{Binary, Convert, Role, Storage, Type, Unary},
};
use std::collections::HashSet;
pub(super) fn validate(program: &Program) -> Result<(), Error> {
    let mut roles = HashSet::new();
    let mut wrapped = vec![false; program.symbols().len()];
    for symbol in program.symbols() {
        let Some(metadata) = program.symbol_metadata(symbol) else {
            continue;
        };
        let error = |message: &str, name| {
            Error::Diagnostic(format!("{message}: {}", program.names.get(name)))
        };
        let function = matches!(symbol.detail, SymbolDetail::Function(_));
        if let Some(role) = metadata.role {
            if role.global() == function {
                return Err(error(
                    if function {
                        "invalid symbol role"
                    } else {
                        "function role on global"
                    },
                    symbol.name,
                ));
            }
            if role != Role::RttiData && !roles.insert(role) {
                return Err(error("duplicate singleton role", symbol.name));
            }
            if role == Role::Entry && program.function(symbol).is_none_or(|f| f.body.is_none()) {
                return Err(error(
                    "entry role requires a function definition",
                    symbol.name,
                ));
            }
        }
        if function && (metadata.storage.is_some() || metadata.section.is_some()) {
            return Err(error("storage metadata on non-global", symbol.name));
        }
        if let Some(target) = metadata.tls_for {
            if !function {
                return Err(error("tls_for metadata requires a function", symbol.name));
            }
            let target_symbol = program.symbol(target);
            if program
                .global(target_symbol)
                .is_none_or(|g| g.metadata.storage != Some(Storage::ThreadLocal))
            {
                return Err(error(
                    "tls_for target is not thread-local global",
                    target_symbol.name,
                ));
            }
            if wrapped[target.index()] {
                return Err(error("duplicate tls wrapper", target_symbol.name));
            }
            wrapped[target.index()] = true;
        }
    }
    for function in program.functions_in_symbol_order() {
        if let Some(body) = &function.body {
            validate_body(program, function, body)?;
        }
    }
    Ok(())
}
/// Audit the stored links/results that textual construction establishes while parsing.
pub(super) fn validate_constructed(program: &Program) -> Result<(), Error> {
    program.finish_symbols()?;
    for function in program.functions_in_symbol_order() {
        if let Some(body) = &function.body {
            for (index, inst) in body.instructions().iter().enumerate() {
                let ty = inst.kind.result_type();
                let message = if inst.result.is_some() != (ty != Type::Void) {
                    Some(if ty == Type::Void {
                        "a void instruction has no result"
                    } else {
                        "value instruction without its result temporary"
                    })
                } else if let Some(result) = inst.result {
                    let temp = body.temp(result);
                    if temp.definition.is_none_or(|id| id.index() != index) {
                        Some("value instruction without its result temporary")
                    } else if temp.ty != Some(ty) {
                        Some("result temporary type mismatch")
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(message) = message {
                    return Err(fail(program, function, body, inst, message));
                }
            }
        }
    }
    validate(program)
}
fn fail(
    program: &Program,
    function: &Function,
    body: &Body,
    inst: &Instruction,
    message: impl Into<String>,
) -> Error {
    let message = message.into();
    let name = program.names.get(function.name);
    let message = format!("{message} in function @{name}");
    if let Some(source) = body.source {
        Error::at_path(message, program.source(source), inst.line)
    } else {
        Error::Diagnostic(message)
    }
}
fn validate_body(program: &Program, function: &Function, body: &Body) -> Result<(), Error> {
    if body.layout.is_empty() {
        return Err(Error::Diagnostic(format!(
            "function has no blocks in function @{}",
            program.names.get(function.name)
        )));
    }
    let mut positions = vec![0; body.instructions().len()];
    let mut predecessors = vec![HashSet::new(); body.blocks().len()];
    let mut handlers = vec![false; body.blocks().len()];
    let mut position = 0;
    for &block in &body.layout {
        let last = body.block(block).instructions.last().copied();
        if last.is_none_or(|id| !program.may_end_block(&body.instruction(id).kind)) {
            return Err(Error::Diagnostic(format!(
                "block has no terminator: {} in function @{}",
                program.names.get(body.block(block).name),
                program.names.get(function.name)
            )));
        }
        for &id in &body.block(block).instructions {
            positions[id.index()] = position;
            position += 1;
            body.instruction(id).kind.visit_targets(|target| {
                predecessors[target.index()].insert(block);
            });
            if let I::Runtime(inst) = &body.instruction(id).kind
                && let Some(target) = inst.handler_target()
            {
                handlers[target.index()] = true;
            }
        }
    }
    for &block in &body.layout {
        let mut ordinary = false;
        for &id in &body.block(block).instructions {
            let inst = body.instruction(id);
            let error = |message| fail(program, function, body, inst, message);
            if matches!(inst.kind, I::Phi(..)) && ordinary {
                return Err(error("phi instructions must precede ordinary instructions"));
            }
            ordinary |= !matches!(inst.kind, I::Phi(..));
            let mut early_use = None;
            if !matches!(inst.kind, I::Phi(..)) {
                body.visit_values(&inst.kind, |v| {
                    if let Value::Temp(t) = v
                        && !body.temp_is_parameter(t)
                        && body
                            .temp(t)
                            .definition
                            .is_some_and(|def| positions[def.index()] >= positions[id.index()])
                        && early_use.is_none()
                    {
                        early_use = Some(body.temp(t).name);
                    }
                });
            }
            if let Some(name) = early_use {
                return Err(fail(
                    program,
                    function,
                    body,
                    inst,
                    format!("undefined temporary: {}", program.names.get(name)),
                ));
            }
            match &inst.kind {
                I::Binary(op, facts, ty, ..) => {
                    let integer = ty.bits() > 1;
                    if facts.nowrap()
                        && !(integer
                            && matches!(op, Binary::Add | Binary::Sub | Binary::Mul | Binary::Shl))
                    {
                        return Err(error("nowrap requires an integer add, sub, mul or shl"));
                    }
                    if facts.exact()
                        && !(integer
                            && matches!(
                                op,
                                Binary::Div | Binary::UDiv | Binary::Shr | Binary::UShr
                            ))
                    {
                        return Err(error("exact requires an integer div, udiv, shr or ushr"));
                    }
                }
                I::Load(facts, ty, _) | I::Store(facts, ty, ..) => {
                    if facts.typed() && !(*ty == Type::Ptr || ty.bits() > 8 || ty.float_rank() != 0)
                    {
                        return Err(error(
                            "typed requires a pointer, floating or wider-than-byte integer access",
                        ));
                    }
                }
                I::Unary(Unary::ByteSwap, ty, _) => {
                    if !matches!(
                        ty,
                        Type::I16 | Type::U16 | Type::I32 | Type::U32 | Type::I64
                    ) {
                        return Err(error("invalid bswap type"));
                    }
                }
                I::Unary(Unary::Ctz | Unary::Clz, ty, _) => {
                    if !matches!(ty, Type::I32 | Type::U32 | Type::I64) {
                        return Err(error("invalid ctz/clz type"));
                    }
                }
                I::Compare(_, ty, a, b) => {
                    for v in [a, b] {
                        let actual = match *v {
                            Value::Temp(t) => body.temp(t).ty,
                            Value::Slot(s) => Some(body.slot(s).ty),
                            Value::Integer(_) | Value::Float(..) | Value::Symbol(_) => None,
                        };
                        if actual == Some(Type::I128) && *ty != Type::I128 {
                            return Err(error("comparison operand type mismatch"));
                        }
                    }
                }
                I::Select(ty, ..) => {
                    if !(ty.bits() != 0 || *ty == Type::Ptr) || *ty == Type::I128 {
                        return Err(error("invalid select type"));
                    }
                }
                I::Convert(op, to, from, _) => {
                    if !valid_conversion(*op, *to, *from) {
                        return Err(error("invalid conversion widths or categories"));
                    }
                }
                I::Return(ty, value) => {
                    if *ty != program.signature(function.signature).result
                        || value.is_some() == (*ty == Type::Void)
                    {
                        return Err(error("return type does not match function"));
                    }
                }
                I::Phi(ty, pairs) => {
                    if handlers[block.index()] {
                        return Err(error("phi is not permitted in an exception handler block"));
                    }
                    if let Some(message) =
                        phi_error(program, body, *ty, pairs, &predecessors[block.index()])
                    {
                        return Err(fail(program, function, body, inst, message));
                    }
                }
                I::Call(call) => {
                    if let Some(message) = call_error(program, body, call) {
                        return Err(fail(program, function, body, inst, message));
                    }
                }
                I::Runtime(inst) => {
                    if let Some(message) = runtime_error(program, function, body, inst) {
                        return Err(error(message));
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}
fn value_type(body: &Body, value: Value) -> Option<Type> {
    match value {
        Value::Temp(id) => body.temp(id).ty,
        Value::Slot(id) => Some(body.slot(id).ty),
        Value::Symbol(_) => Some(Type::Ptr),
        Value::Integer(_) | Value::Float(..) => None,
    }
}
fn runtime_error(
    program: &Program,
    function: &Function,
    body: &Body,
    inst: &R,
) -> Option<&'static str> {
    match *inst {
        R::VaStart(list) => {
            if program.signature(function.signature).boundary.arity != Arity::Variadic {
                return Some("va_start requires a variadic function");
            }
            if value_type(body, list) != Some(Type::Ptr) {
                return Some("va_start requires a pointer value");
            }
        }
        R::VaArg(ty, list) => {
            if value_type(body, list) != Some(Type::Ptr) {
                return Some("va_arg requires a pointer value");
            }
            if !matches!(
                ty,
                Type::I8
                    | Type::U8
                    | Type::I16
                    | Type::U16
                    | Type::I32
                    | Type::U32
                    | Type::I64
                    | Type::F64
                    | Type::Ptr
            ) {
                return Some("va_arg scalar type is not supported");
            }
        }
        R::StackAlloc(size)
            if !matches!(size, Value::Integer(_))
                && value_type(body, size).is_none_or(|ty| ty.bits() == 0) =>
        {
            return Some("stack_alloc requires an integer size");
        }
        _ => {}
    }
    None
}
fn call_error(program: &Program, body: &Body, call: &Call) -> Option<String> {
    let direct = program.callee_function(call.callee).is_some();
    let signature = program.call_boundary(call);
    if call.signature.is_none() && !direct {
        return Some("indirect call requires signature".into());
    }
    let signature = signature?;
    if call.result != signature.result {
        return Some(
            if call.signature.is_some() {
                "call signature return mismatch"
            } else {
                "direct call return mismatch"
            }
            .into(),
        );
    }
    let arguments = body.arguments(call.arguments);
    let count = arguments.len();
    let parameters = signature.parameters.len();
    if (signature.boundary.arity == Arity::Fixed && count != parameters) || count < parameters {
        return Some("call arity mismatch".into());
    }
    for (parameter, argument) in signature.parameters.iter().zip(arguments) {
        if let Value::Slot(id) = *argument
            && parameter.pass.is_none()
            && body.slot(id).ty != parameter.ty
        {
            return Some(format!(
                "call argument slot type does not match the parameter type: {}",
                program.names.get(body.slot(id).name)
            ));
        }
    }
    if call.copy_elision {
        if !direct {
            return Some("copy elision requires a direct call".into());
        }
        if call.result != Type::Void || count < 2 {
            return Some(
                "copy elision requires a direct void call with destination and source".into(),
            );
        }
    }
    None
}
fn valid_conversion(op: Convert, to: Type, from: Type) -> bool {
    if to == from {
        return true;
    }
    match op {
        Convert::SignExtend | Convert::ZeroExtend => {
            to.bits() != 0 && from.bits() != 0 && to.bits() > from.bits()
        }
        Convert::Truncate => to.bits() != 0 && from.bits() != 0 && to.bits() < from.bits(),
        Convert::SignedToFloat | Convert::UnsignedToFloat => {
            to.float_rank() != 0 && from.bits() != 0
        }
        Convert::FloatToSigned | Convert::FloatToUnsigned => {
            to.bits() != 0 && from.float_rank() != 0
        }
        Convert::FloatExtend => {
            to.float_rank() != 0 && from.float_rank() != 0 && to.float_rank() > from.float_rank()
        }
        Convert::FloatTruncate => {
            to.float_rank() != 0 && from.float_rank() != 0 && to.float_rank() < from.float_rank()
        }
    }
}
fn phi_error(
    program: &Program,
    body: &Body,
    ty: Type,
    pairs: &[(BlockId, Value)],
    predecessors: &HashSet<BlockId>,
) -> Option<String> {
    if matches!(
        ty,
        Type::Void | Type::I128 | Type::F80 | Type::Object { .. }
    ) {
        return Some("phi requires a directly representable scalar type".into());
    }
    let mut seen = HashSet::new();
    for &(from, value) in pairs {
        if !seen.insert(from) {
            return Some(format!(
                "duplicate phi predecessor: {}",
                program.names.get(body.block(from).name)
            ));
        }
        if !predecessors.contains(&from) {
            return Some(format!(
                "phi label is not a predecessor of its block: {}",
                program.names.get(body.block(from).name)
            ));
        }
        match value {
            Value::Temp(t) if body.temp(t).ty != Some(ty) => {
                return Some(format!(
                    "phi incoming temporary type mismatch: {}",
                    program.names.get(body.temp(t).name)
                ));
            }
            Value::Integer(_) if ty.bits() == 0 && ty != Type::Ptr => {
                return Some("phi integer literal type mismatch".into());
            }
            Value::Float(_, kind) if kind.ty() != ty => {
                return Some("phi floating literal type mismatch".into());
            }
            Value::Symbol(_) if ty != Type::Ptr => {
                return Some("phi global operand requires ptr type".into());
            }
            Value::Slot(_) => return Some("phi incoming operand is not a scalar value".into()),
            _ => {}
        }
    }
    if pairs.len() != predecessors.len() {
        return Some("phi must name every ordinary predecessor exactly once".into());
    }
    None
}
