//! C++ `LowIR` validator's layout-order, scalar/fact and phi checks. See NOTICE.
use super::{
    Error,
    model::{
        Arity, BlockId, Body, Call, Function, Instruction, InstructionKind as I, Program,
        RuntimeInstruction as R, SymbolDetail, Value,
    },
    vocabulary::{Binary, Convert, Role, Storage, Type, Unary},
};
use crate::support::hash::HashSet;
pub(super) fn validate(program: &Program) -> Result<(), Error> {
    validate_options(program, true)
}
fn validate_options(program: &Program, source_locations: bool) -> Result<(), Error> {
    let mut roles = HashSet::default();
    let mut wrapped = vec![false; program.symbols().len()];
    for symbol in program.symbols() {
        if !symbol.declared() {
            return Err(Error::Diagnostic(format!(
                "undefined top-level symbol: {}",
                program.names.get(symbol.name)
            )));
        }
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
            validate_body(program, function, body, source_locations)?;
        }
    }
    Ok(())
}
/// Audit the stored links/results that textual construction establishes while parsing.
#[inline]
pub fn validate_constructed(program: &Program) -> Result<(), Error> {
    validate_constructed_with_spellings(program, true)
}
#[inline]
pub fn validate_constructed_with_spellings(
    program: &Program,
    spellings: bool,
) -> Result<(), Error> {
    super::model::store_validation::validate(program, spellings)?;
    validate_options(program, false)
}

fn fail(
    program: &Program,
    function: &Function,
    body: &Body,
    inst: &Instruction,
    message: impl Into<String>,
    source_locations: bool,
) -> Error {
    let message = message.into();
    let name = program.names.get(function.name);
    let message = format!("{message} in function @{name}");
    if let Some(source) = body.source.filter(|_| source_locations) {
        Error::at_path(message, program.source(source), inst.line)
    } else {
        Error::Diagnostic(message)
    }
}
fn validate_body(
    program: &Program,
    function: &Function,
    body: &Body,
    source_locations: bool,
) -> Result<(), Error> {
    if body.layout.is_empty() {
        return Err(Error::Diagnostic(format!(
            "function has no blocks in function @{}",
            program.names.get(function.name)
        )));
    }
    let mut positions = vec![usize::MAX; body.instructions().len()];
    let mut placed = vec![false; body.blocks().len()];
    let mut predecessors = vec![HashSet::default(); body.blocks().len()];
    let mut handlers = vec![false; body.blocks().len()];
    let mut position = 0;
    for &block in &body.layout {
        if placed[block.index()] {
            return Err(Error::Diagnostic(format!(
                "duplicate block: {} in function @{}",
                program.names.get(body.block(block).name),
                program.names.get(function.name)
            )));
        }
        placed[block.index()] = true;
        let last = body.block(block).instructions.last().copied();
        if last.is_none_or(|id| !program.may_end_block(&body.instruction(id).kind)) {
            return Err(Error::Diagnostic(format!(
                "block has no terminator: {} in function @{}",
                program.names.get(body.block(block).name),
                program.names.get(function.name)
            )));
        }
        let instructions = &body.block(block).instructions;
        for (index, &id) in instructions.iter().enumerate() {
            positions[id.index()] = position;
            position += 1;
            if body.instruction(id).kind.terminator() && index + 1 < instructions.len() {
                return Err(fail(
                    program,
                    function,
                    body,
                    body.instruction(instructions[index + 1]),
                    "instruction after terminator",
                    source_locations,
                ));
            }
        }
    }
    for &block in &body.layout {
        for &id in &body.block(block).instructions {
            let inst = body.instruction(id);
            let mut invalid = false;
            inst.kind.visit_targets(|target| {
                if placed.get(target.index()).copied().unwrap_or(false) {
                    predecessors[target.index()].insert(block);
                } else {
                    invalid = true;
                }
            });
            if let I::Runtime(runtime) = &inst.kind
                && let Some(target) = runtime.handler_target()
            {
                if placed.get(target.index()).copied().unwrap_or(false) {
                    handlers[target.index()] = true;
                } else {
                    invalid = true;
                }
            }
            if invalid {
                return Err(fail(
                    program,
                    function,
                    body,
                    inst,
                    "undefined or invalid block target",
                    source_locations,
                ));
            }
        }
    }
    for temp in body.temps().iter().skip(body.parameter_count) {
        if temp.definition.is_none() {
            let detail = if temp.name == super::model::NameId::empty() {
                String::new()
            } else {
                format!(": {}", program.names.get(temp.name))
            };
            return Err(Error::Diagnostic(format!(
                "undefined temporary{detail} in function @{}",
                program.names.get(function.name)
            )));
        }
    }
    for &block in &body.layout {
        let mut ordinary = false;
        for &id in &body.block(block).instructions {
            let inst = body.instruction(id);
            let error = |message| fail(program, function, body, inst, message, source_locations);
            if matches!(inst.kind, I::Phi(..)) && ordinary {
                return Err(error("phi instructions must precede ordinary instructions"));
            }
            ordinary |= !matches!(inst.kind, I::Phi(..));
            let mut early_use = None;
            if !matches!(inst.kind, I::Phi(..)) {
                body.visit_values(&inst.kind, |v| {
                    if let Value::Temp(t) = v
                        && !body.temp_is_parameter(t)
                        && body.temp(t).definition.is_some_and(|def| {
                            positions
                                .get(def.index())
                                .is_none_or(|&position| position >= positions[id.index()])
                        })
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
                    source_locations,
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
                        return Err(fail(
                            program,
                            function,
                            body,
                            inst,
                            message,
                            source_locations,
                        ));
                    }
                }
                I::Call(call) => {
                    if let Some(message) = call_error(program, body, call) {
                        return Err(fail(
                            program,
                            function,
                            body,
                            inst,
                            message,
                            source_locations,
                        ));
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
        R::VaArg(ty, list, _) => {
            if value_type(body, list) != Some(Type::Ptr) {
                return Some("va_arg requires a pointer value");
            }
            let scalar = matches!(
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
            );
            let object =
                program.object_facts && matches!(ty, Type::Object { .. } | Type::F80 | Type::I128);
            if !scalar && !object {
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
    if pairs.is_empty() {
        return Some("phi requires predecessor/value pairs".into());
    }
    let mut seen = HashSet::default();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        literals::{Constant, FloatKind},
        model::{Boundary, Facts, NameId, Parameter, Signature},
    };

    fn diagnostic(program: &Program, message: &str) {
        assert!(
            matches!(validate_constructed(program), Err(Error::Diagnostic(actual)) if actual == message)
        );
    }

    #[test]
    fn unlinked_definitions_fail_only_when_an_ordinary_use_reads_them() {
        for used in [false, true] {
            let source = format!(
                "function @f()->i64 {{block ^entry: %removed = const i64 1 %live = const i64 2 return i64 %{}}}\n",
                if used { "removed" } else { "live" }
            );
            let mut program = crate::lowir::test_program(source.as_bytes()).unwrap();
            let symbol = program
                .find_symbol(program.names.find("f").unwrap())
                .unwrap();
            let mut body = program.take_body(symbol).unwrap().unwrap();
            let block = body.layout[0];
            let retained = body.block(block).instructions[1..].to_vec();
            body.replace_links(block, retained).unwrap();
            program.restore_body(symbol, body).unwrap();
            if used {
                diagnostic(&program, "undefined temporary: removed in function @f");
            } else {
                validate_constructed(&program).unwrap();
            }
        }
    }

    #[test]
    fn optimizer_intermediate_checks_defer_only_body_spellings() {
        let mut program = crate::lowir::test_program(
            b"function @f(%p:i64)->i64 {block ^entry: %value = copy i64 %p return i64 %value}\n",
        )
        .unwrap();
        let symbol = program
            .find_symbol(program.names.find("f").unwrap())
            .unwrap();
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let value = body
            .instruction(body.block(body.layout[0]).instructions[0])
            .result
            .unwrap();
        body.temp_mut(value).name = body.temps()[0].name;
        program.restore_body(symbol, body).unwrap();
        diagnostic(&program, "duplicate temporary name: p in function @f");
        validate_constructed_with_spellings(&program, false).unwrap();
    }

    #[test]
    fn constructed_spans_reject_only_zero_and_out_of_range_alignment_fields() {
        for (bytes, align, valid) in [
            (0, 3, false),
            (8, 64, false),
            (8, 3, true),
            (u64::MAX, 63, true),
        ] {
            let mut program = Program::default();
            let mut builder =
                FunctionBuilder::new(&mut program, "f", &[("p", Type::Ptr)], Type::Void).unwrap();
            builder.block("entry").unwrap();
            let pointer = builder.parameter(0).unwrap();
            builder.effect(I::ZeroInit(bytes, align, pointer)).unwrap();
            builder.effect(I::Return(Type::Void, None)).unwrap();
            builder.finish().unwrap();
            if valid {
                validate_constructed(&program).unwrap();
            } else {
                diagnostic(&program, "invalid instruction field in function @f");
            }
        }
    }

    #[test]
    fn constructed_target_bounds_and_placement_have_distinct_diagnostics() {
        for foreign in [false, true] {
            let mut program =
                crate::lowir::test_program(b"function @f()->void {block ^entry: return void}\n")
                    .unwrap();
            let symbol = program
                .find_symbol(program.names.find("f").unwrap())
                .unwrap();
            let mut body = program.take_body(symbol).unwrap().unwrap();
            let block = body.layout[0];
            let target = if foreign {
                let mut other = Body::new(None, 0);
                other.generated_block().unwrap();
                other.generated_block().unwrap();
                other.generated_block().unwrap()
            } else {
                body.generated_block().unwrap()
            };
            let instruction = body.block(block).instructions[0];
            body.replace_kind(instruction, I::Jump(target));
            program.restore_body(symbol, body).unwrap();
            diagnostic(
                &program,
                if foreign {
                    "invalid operand for the instruction in function @f"
                } else {
                    "undefined or invalid block target in function @f"
                },
            );
        }
    }

    #[test]
    fn constructed_operand_kinds_are_checked_before_semantic_rules() {
        for operation in ["constant", "address", "load", "store", "callee", "filter"] {
            let mut program = Program::default();
            let mut builder =
                FunctionBuilder::new(&mut program, "f", &[("p", Type::Ptr)], Type::Void).unwrap();
            builder.block("entry").unwrap();
            let parameter = builder.parameter(0).unwrap();
            let integer = builder.integer(1).unwrap();
            let float = builder
                .constant(Constant::Float(FloatKind::F64, u128::from(1_f64.to_bits())))
                .unwrap();
            match operation {
                "constant" => {
                    builder.generated(I::Const(Type::I64, parameter)).unwrap();
                }
                "address" => {
                    builder.generated(I::Addr(parameter)).unwrap();
                }
                "load" => {
                    builder
                        .generated(I::Load(Facts::default(), Type::I64, float))
                        .unwrap();
                }
                "store" => builder
                    .effect(I::Store(Facts::default(), Type::I64, integer, float))
                    .unwrap(),
                "callee" => {
                    let callee = builder.generated_slot("callee", Type::Ptr).unwrap();
                    let arguments = builder.arguments(&[]).unwrap();
                    builder
                        .effect(I::Call(Call {
                            result: Type::Void,
                            callee,
                            arguments,
                            copy_elision: false,
                            argument_classes: (Vec::new()).into(),
                            signature: None,
                        }))
                        .unwrap();
                }
                "filter" => {
                    let arguments = builder.arguments(&[integer]).unwrap();
                    builder
                        .effect(I::Runtime(R::EhFilter(arguments, None)))
                        .unwrap();
                }
                _ => unreachable!(),
            }
            builder.effect(I::Return(Type::Void, None)).unwrap();
            builder.finish().unwrap();
            diagnostic(
                &program,
                "invalid operand for the instruction in function @f",
            );
        }
    }

    #[test]
    fn void_value_opcodes_still_require_their_result_temporary() {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "f", &[], Type::Void).unwrap();
        builder.block("entry").unwrap();
        let literal = builder.integer(0).unwrap();
        builder.effect(I::Const(Type::Void, literal)).unwrap();
        builder.effect(I::Return(Type::Void, None)).unwrap();
        builder.finish().unwrap();
        diagnostic(
            &program,
            "value instruction without its result temporary in function @f",
        );
    }

    #[test]
    fn predecessor_free_blocks_cannot_contain_empty_phis() {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "f", &[], Type::Void).unwrap();
        builder.block("entry").unwrap();
        builder.generated(I::Phi(Type::I64, Vec::new())).unwrap();
        builder.effect(I::Return(Type::Void, None)).unwrap();
        builder.finish().unwrap();
        diagnostic(
            &program,
            "phi requires predecessor/value pairs in function @f",
        );
    }

    #[test]
    fn constructed_layout_and_undefined_temps_are_rejected() {
        for problem in ["duplicate", "terminator", "temporary"] {
            let mut program =
                crate::lowir::test_program(b"function @f()->void {block ^entry: return void}\n")
                    .unwrap();
            let symbol = program
                .find_symbol(program.names.find("f").unwrap())
                .unwrap();
            let mut body = program.take_body(symbol).unwrap().unwrap();
            let block = body.layout[0];
            let expected = match problem {
                "duplicate" => {
                    body.layout.push(block);
                    "duplicate block: entry in function @f"
                }
                "terminator" => {
                    body.append(
                        block,
                        Instruction::new(None, I::Return(Type::Void, None), 0),
                    )
                    .unwrap();
                    "instruction after terminator in function @f"
                }
                "temporary" => {
                    let temp = body.generated_temp(Type::I64).unwrap();
                    body.temp_mut(temp).name = program.names.intern("missing").unwrap();
                    "undefined temporary: missing in function @f"
                }
                _ => unreachable!(),
            };
            program.restore_body(symbol, body).unwrap();
            diagnostic(&program, expected);
        }
    }

    #[test]
    fn signature_checks_distinguish_unnamed_parameters_from_duplicate_names() {
        for named in [false, true] {
            let mut program = Program::default();
            let name = if named {
                program.names.intern("p").unwrap()
            } else {
                NameId::empty()
            };
            let parameter = || Parameter {
                name,
                ty: Type::I32,
                pass: None,
                no_alias: false,
                object_bytes: 0,
                classes: None,
            };
            program
                .add_signature(Signature {
                    parameters: vec![parameter(), parameter()],
                    result: Type::Void,
                    boundary: Boundary::default(),
                })
                .unwrap();
            if named {
                diagnostic(&program, "duplicate parameter: p");
            } else {
                validate_constructed(&program).unwrap();
            }
        }
    }
}
