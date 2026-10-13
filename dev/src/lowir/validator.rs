//! C++ `LowIR` validator's layout-order, scalar/fact and phi checks. See NOTICE.
use super::{
    Error,
    model::{BlockId, Body, Function, Instruction, InstructionKind as I, Program, Value},
    vocabulary::{Binary, Convert, Role, Type, Unary},
};
use std::collections::HashSet;
pub(super) fn validate(program: &Program) -> Result<(), Error> {
    let mut roles = HashSet::new();
    for function in program.functions() {
        if let Some(role) = function.metadata.role {
            if !roles.insert(role.spelling()) {
                return Err(Error::Diagnostic(format!(
                    "duplicate singleton role: {}",
                    program.names.get(function.name)
                )));
            }
            if role == Role::Entry && function.body.is_none() {
                return Err(Error::Diagnostic(format!(
                    "entry role requires a function definition: {}",
                    program.names.get(function.name)
                )));
            }
        }
    }
    for function in program.functions() {
        if let Some(body) = &function.body {
            validate_body(program, function, body)?;
        }
    }
    Ok(())
}
fn fail(
    program: &Program,
    function: &Function,
    body: &Body,
    inst: &Instruction,
    message: impl Into<String>,
) -> Error {
    Error::Diagnostic(format!(
        "{} in function @{} at {}:{}",
        message.into(),
        program.names.get(function.name),
        body.source,
        inst.line
    ))
}
fn validate_body(program: &Program, function: &Function, body: &Body) -> Result<(), Error> {
    let mut positions = vec![0; body.instructions().len()];
    let mut predecessors = vec![HashSet::new(); body.blocks().len()];
    let mut position = 0;
    for &block in &body.layout {
        for &id in &body.block(block).instructions {
            positions[id.index()] = position;
            position += 1;
            body.instruction(id).kind.visit_targets(|target| {
                predecessors[target.index()].insert(block);
            });
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
                inst.kind.visit_values(|v| {
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
                            Value::Integer(_) => None,
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
                    if *ty != function.result || value.is_some() == (*ty == Type::Void) {
                        return Err(error("return type does not match function"));
                    }
                }
                I::Phi(ty, pairs) => {
                    if let Some(message) =
                        phi_error(program, body, *ty, pairs, &predecessors[block.index()])
                    {
                        return Err(fail(program, function, body, inst, message));
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
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
            Value::Slot(_) => return Some("phi incoming operand is not a scalar value".into()),
            _ => {}
        }
    }
    if pairs.len() != predecessors.len() {
        return Some("phi must name every ordinary predecessor exactly once".into());
    }
    None
}
