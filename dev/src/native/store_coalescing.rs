//! Aligned constant-store runs from `select/mir_cleanup.cpp`. See NOTICE.
use super::{
    memory::{Base, Memory},
    mir::{Function, Instruction, Op},
    scalar_mir::{Operand, ScalarOp, bytes},
};
use crate::lowir::vocabulary::Type;
fn store(inst: &Instruction) -> Option<(Memory, Type, u128)> {
    if inst.volatile {
        return None;
    }
    let Op::Scalar(ScalarOp::Store(to, ty, Operand::Immediate(value))) = inst.op else {
        return None;
    };
    let memory = to.memory()?;
    (memory.index.is_none() && matches!(memory.base, Base::Frame | Base::Register(_)))
        .then_some((memory, ty, value))
}
fn merge(run: &mut [Instruction]) {
    let mut lo = i32::MAX;
    let mut hi = i32::MIN;
    for inst in run.iter() {
        let Some((to, ty, _)) = store(inst) else {
            return;
        };
        lo = lo.min(to.displacement);
        hi = hi.max(to.displacement.wrapping_add(i32::from(bytes(ty))));
    }
    let Ok(span) = usize::try_from(i64::from(hi) - i64::from(lo)) else {
        return;
    };
    if span > 64 {
        return;
    }
    let mut values = [0_u8; 64];
    let mut written = [false; 64];
    for inst in run.iter() {
        let Some((to, ty, value)) = store(inst) else {
            return;
        };
        let Ok(at) = usize::try_from(i64::from(to.displacement) - i64::from(lo)) else {
            return;
        };
        let width = usize::from(bytes(ty));
        if at.checked_add(width).is_none_or(|end| end > span) {
            return;
        }
        for (b, &value) in value.to_le_bytes().iter().take(width).enumerate() {
            if written[at + b] {
                return;
            }
            written[at + b] = true;
            values[at + b] = value;
        }
    }
    // At most 64 one-byte chunks. Stack storage stays bounded independently
    // of the input run; no speculative MIR edits occur before admission.
    let mut chunks = [(0_i32, Type::I8, 0_u128); 64];
    let mut count = 0;
    let mut p = 0;
    while p < span {
        if !written[p] {
            p += 1;
            continue;
        }
        let Ok(offset) = i32::try_from(p) else {
            return;
        };
        let disp = lo.wrapping_add(offset);
        let mut width = 8;
        let mut bits = 0_u64;
        while width > 1 {
            let aligned = disp.cast_unsigned() & u32::try_from(width - 1).unwrap_or(u32::MAX) == 0;
            if aligned && p + width <= span && written[p..p + width].iter().all(|&w| w) {
                bits = values[p..p + width]
                    .iter()
                    .enumerate()
                    .fold(0, |v, (b, &byte)| v | u64::from(byte) << (8 * b));
                if width < 8 || i32::try_from(bits.cast_signed()).is_ok() {
                    break;
                }
            }
            width /= 2;
        }
        if width == 1 {
            bits = u64::from(values[p]);
        }
        let ty = match width {
            8 => Type::I64,
            4 => Type::I32,
            2 => Type::I16,
            _ => Type::I8,
        };
        chunks[count] = (disp, ty, i128::from(bits.cast_signed()).cast_unsigned());
        count += 1;
        p += width;
    }
    if count >= run.len() {
        return;
    }
    for (inst, &(disp, ty, value)) in run.iter_mut().zip(&chunks[..count]) {
        let Some((mut to, _, _)) = store(inst) else {
            return;
        };
        to.displacement = disp;
        inst.op = Op::Scalar(ScalarOp::Store(
            Operand::Memory(to),
            ty,
            Operand::Immediate(value),
        ));
    }
    for inst in &mut run[count..] {
        inst.op = Op::Nop;
    }
}
pub(super) fn merge_runs(f: &mut Function) {
    for block in &f.blocks {
        let mut i = block.range.start;
        while i < block.range.end {
            let Some((to, _, _)) = store(&f.instructions[i]) else {
                i += 1;
                continue;
            };
            let mut end = i + 1;
            while end < block.range.end
                && store(&f.instructions[end]).is_some_and(|(other, _, _)| other.base == to.base)
            {
                end += 1;
            }
            if end - i >= 2 {
                merge(&mut f.instructions[i..end]);
            }
            i = end;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inst(at: i32, ty: Type, value: u128) -> Instruction {
        Instruction {
            volatile: false,
            location: None,
            op: Op::Scalar(ScalarOp::Store(
                Operand::Frame(at),
                ty,
                Operand::Immediate(value),
            )),
        }
    }
    #[test]
    fn gaps_overlap_alignment_and_imm32_limits_preserve_store_runs() {
        let mut run = [
            inst(-8, Type::U8, 1),
            inst(-7, Type::U8, 2),
            inst(-4, Type::I32, 0),
        ];
        merge(&mut run);
        assert!(matches!(
            run[0].op,
            Op::Scalar(ScalarOp::Store(_, Type::I16, Operand::Immediate(513)))
        ));
        assert!(matches!(
            run[1].op,
            Op::Scalar(ScalarOp::Store(_, Type::I32, Operand::Immediate(0)))
        ));
        assert!(matches!(run[2].op, Op::Nop));
        for mut run in [
            [inst(-8, Type::I32, 1), inst(-4, Type::I32, 1)],
            [inst(-8, Type::I32, 0), inst(-6, Type::I16, 0)],
            [inst(-8, Type::I8, 0), inst(64, Type::I8, 0)],
        ] {
            merge(&mut run);
            assert!(run.iter().all(|i| !matches!(i.op, Op::Nop)));
        }
    }
}
