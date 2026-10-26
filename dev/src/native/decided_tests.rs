//! Bounded MIR test threading from exact block-exit values. See NOTICE.
use super::{
    condition::Condition as C,
    condition_mir::FlagOp,
    memory::Base,
    mir::{Function, Op},
    scalar_mir::{Operand, ScalarOp},
    x86::Register as R,
};
use crate::lowir::model::Program;
fn condition(cond: C, test: bool, mut a: u128, mut b: u128, bytes: u8) -> Option<bool> {
    let bits = u32::from(bytes) * 8;
    let mask = (1_u128 << bits) - 1;
    let sign = 1_u128 << (bits - 1);
    a &= mask;
    b &= mask;
    let result = if test {
        a & b
    } else {
        a.wrapping_sub(b) & mask
    };
    let zero = result == 0;
    let negative = result & sign != 0;
    let carry = !test && a < b;
    let overflow = !test && ((a ^ b) & (a ^ result) & sign) != 0;
    Some(match cond {
        C::Equal => zero,
        C::NotEqual => !zero,
        C::Below => carry,
        C::AboveEqual => !carry,
        C::BelowEqual => carry || zero,
        C::Above => !carry && !zero,
        C::Less => negative != overflow,
        C::GreaterEqual => negative == overflow,
        C::LessEqual => zero || negative != overflow,
        C::Greater => !zero && negative == overflow,
        C::Parity | C::NotParity => return None,
    })
}
fn exit_value(f: &Function, program: &Program, b: usize, r: R) -> Option<(u128, bool)> {
    let range = f.blocks[b].range.clone();
    let defined = |symbol| {
        program.function_defined(symbol)
            || program
                .global(program.symbol(symbol))
                .is_some_and(|g| !matches!(g.init, crate::lowir::model::GlobalInit::Declaration))
    };
    for i in (range.start..range.end - 1).rev() {
        let op = f.instructions[i].op;
        match op {
            Op::Move(to, bits) if to == r => return Some((bits, false)),
            Op::SymbolMove(to, s) if to == r => return defined(s).then_some((0, true)),
            Op::Scalar(ScalarOp::Move(to, Operand::Immediate(bits))) if to == r => {
                return Some((bits, false));
            }
            Op::Scalar(ScalarOp::Move(to, Operand::Symbol(s) | Operand::SymbolOffset(s, _)))
                if to == r =>
            {
                return defined(s).then_some((0, true));
            }
            Op::Scalar(ScalarOp::Lea(to, source)) if to == r => {
                return source
                    .memory()
                    .filter(|m| {
                        m.index.is_none()
                            && match m.base {
                                Base::Frame | Base::Register(R::RBP | R::RSP) => true,
                                Base::Symbol(s) => defined(s),
                                _ => false,
                            }
                    })
                    .map(|_| (0, true));
            }
            _ => {}
        }
        if let Some(write) = super::mir_cleanup::simple_write(op) {
            if matches!(write,super::mir_cleanup::SimpleWrite::Register(to) if to==r) {
                return None;
            }
        } else if super::mir_cleanup::touches(op, r) {
            return None;
        }
    }
    None
}

pub(super) fn thread(f: &mut Function, program: &Program) {
    for b in 0..f.blocks.len() {
        let range = f.blocks[b].range.clone();
        if range.is_empty() {
            continue;
        }
        let last = range.end - 1;
        for _ in 0..4 {
            let Op::Jump(target) = f.instructions[last].op else {
                break;
            };
            let t = target.index();
            if t == b || t >= f.blocks.len() || f.blocks[t].range.len() != 3 || f.blocks[t].landing
            {
                break;
            }
            let start = f.blocks[t].range.start;
            let (Op::Branch(cc, yes), Op::Jump(no)) =
                (f.instructions[start + 1].op, f.instructions[start + 2].op)
            else {
                break;
            };
            let (ty, r, other, test) = match f.instructions[start].op {
                Op::Scalar(ScalarOp::Condition(flag)) if flag.self_test().is_some() => {
                    let Some((ty, r)) = flag.self_test() else {
                        break;
                    };
                    (ty, r, None, true)
                }
                Op::Scalar(ScalarOp::Condition(FlagOp::Compare(
                    ty,
                    Operand::Register(r),
                    Operand::Immediate(bits),
                ))) => (ty, r, Some(bits), false),
                Op::Scalar(ScalarOp::Condition(FlagOp::MaskTest(
                    ty,
                    Operand::Register(r),
                    Operand::Immediate(bits),
                ))) => (ty, r, Some(bits), true),
                _ => break,
            };
            if r.code() >= 16 {
                break;
            }
            let Some((value, nonzero)) = exit_value(f, program, b, r) else {
                break;
            };
            let bytes = super::scalar_mir::bytes(ty);
            let holds = if !nonzero {
                condition(cc, test, value, other.unwrap_or(value), bytes)
            } else if bytes != 8 || other.is_some_and(|v| v != 0) {
                None
            } else {
                match cc {
                    C::NotEqual | C::Above | C::AboveEqual => Some(true),
                    C::Equal | C::BelowEqual | C::Below => Some(false),
                    _ => None,
                }
            };
            let Some(holds) = holds else {
                break;
            };
            f.instructions[last].op = Op::Jump(if holds { yes } else { no });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lowir::Error;

    #[test]
    fn defined_symbol_offsets_thread_the_same_zero_test_as_symbol_addresses() -> Result<(), Error> {
        let program = crate::lowir::test_program(
            b"global @data : obj<16x8> = zero
              function @f(%condition : i64) -> i64 {
                block ^entry: branch %condition, ^left, ^right
                block ^left:
                  %offset = index i8 @data, 8
                  jump ^merge
                block ^right: jump ^merge
                block ^merge:
                  %address = phi ptr [^left: %offset, ^right: 0]
                  branch %address, ^yes, ^no
                block ^yes: return i64 1
                block ^no: return i64 0
              }",
        )?;
        let function = &program.functions()[0];
        let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
        let selected = super::super::value_selection::select(&program, symbol, function, 1)?;
        let mut output = Vec::new();
        selected
            .write(&program, &mut output)
            .map_err(|_| Error::Limit)?;
        let output = String::from_utf8(output).map_err(|_| Error::Limit)?;
        assert!(output.contains("block ^left\n    mov r8, @data+8\n\n  block ^yes"));
        Ok(())
    }
}
