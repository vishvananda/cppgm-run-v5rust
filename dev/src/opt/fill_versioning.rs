//! Late runtime-byte fill versioning from `opt/loop_passes.cpp::version_fills`.
//! See NOTICE.
use super::{alias_facts::signed_word, loop_fills, loop_memory::Emitter, pass_context::Context};
use crate::lowir::{
    Error,
    model::{
        BlockId, Body, Call, Facts as Flags, InstId, InstructionKind as I, LinkEditor,
        OptimizerBlock, Value,
    },
    vocabulary::{Binary, Compare, Convert, Type},
};
struct Shape {
    wide: BlockId,
    pre: BlockId,
    tail: BlockId,
    phi: InstId,
    init: Value,
    q2: Value,
    v64: Value,
    chunks: Value,
    joined: Vec<(InstId, Value)>,
}
fn definition(body: &Body, v: Value) -> Option<&I> {
    let Value::Temp(t) = v else { return None };
    Some(&body.instruction(body.temp(t).definition?).kind)
}
fn literal(body: &Body, v: Value, want: i64) -> bool {
    matches!(v,Value::Integer(k) if signed_word(body.literal(k))==want)
}
fn shape(body: &Body, wide: BlockId) -> Option<Shape> {
    let ids = &body.block(wide).instructions;
    let [phi, s1, index8, s2, index16, compare, branch] = *ids.as_slice() else {
        return None;
    };
    let I::Phi(_, pairs) = &body.instruction(phi).kind else {
        return None;
    };
    if pairs.len() != 2 {
        return None;
    }
    let q = Value::Temp(body.instruction(phi).result?);
    let I::Store(_, Type::I64, v64 @ Value::Temp(_), address) = body.instruction(s1).kind else {
        return None;
    };
    if address != q {
        return None;
    }
    let I::Index(_, _, base, eight) = body.instruction(index8).kind else {
        return None;
    };
    if base != q || !literal(body, eight, 8) {
        return None;
    }
    let I::Store(_, _, value, address) = body.instruction(s2).kind else {
        return None;
    };
    if value != v64 || Some(address) != body.instruction(index8).result.map(Value::Temp) {
        return None;
    }
    let I::Index(_, _, base, sixteen) = body.instruction(index16).kind else {
        return None;
    };
    if base != q || !literal(body, sixteen, 16) {
        return None;
    }
    let q2 = Value::Temp(body.instruction(index16).result?);
    let I::Compare(Compare::Ne, _, a, b) = body.instruction(compare).kind else {
        return None;
    };
    let end16 = if a == q2 {
        b
    } else if b == q2 {
        a
    } else {
        return None;
    };
    let I::Branch(condition, yes, tail) = body.instruction(branch).kind else {
        return None;
    };
    if Some(condition) != body.instruction(compare).result.map(Value::Temp) || yes != wide {
        return None;
    }
    let from = usize::from(pairs[0].0 == wide);
    if pairs[1 - from] != (wide, q2) {
        return None;
    }
    let (pre, init) = pairs[from];
    let I::Branch(enter, yes, no) = body.instruction(*body.block(pre).instructions.last()?).kind
    else {
        return None;
    };
    if yes != wide || no != tail {
        return None;
    }
    let I::Compare(Compare::Ne, _, a, b) = *definition(body, enter)? else {
        return None;
    };
    if !((a == init && b == end16) || (a == end16 && b == init)) {
        return None;
    }
    let I::Index(_, _, base, chunks) = *definition(body, end16)? else {
        return None;
    };
    if base != init {
        return None;
    }
    let I::Binary(Binary::And, _, _, _, mask) = *definition(body, chunks)? else {
        return None;
    };
    if !literal(body, mask, -16) {
        return None;
    }
    let mut joined = Vec::new();
    for &id in &body.block(tail).instructions {
        let I::Phi(_, pairs) = &body.instruction(id).kind else {
            break;
        };
        let from_pre = pairs.iter().find(|&&(b, _)| b == pre)?.1;
        let from_wide = pairs.iter().find(|&&(b, _)| b == wide)?.1;
        if from_pre != from_wide {
            return None;
        }
        joined.push((id, from_pre));
    }
    Some(Shape {
        wide,
        pre,
        tail,
        phi,
        init,
        q2,
        v64,
        chunks,
        joined,
    })
}
fn version(
    context: &mut Context<'_>,
    s: Shape,
    symbol: crate::lowir::model::SymbolId,
) -> Result<(), Error> {
    let sized = context.body.optimizer_block(OptimizerBlock::FillSized)?;
    let same = context.body.optimizer_block(OptimizerBlock::FillSame)?;
    let fill = context.body.optimizer_block(OptimizerBlock::FillFast)?;
    let links = LinkEditor::new(context.body)?;
    let mut e = Emitter {
        body: context.body,
        links,
        location: Some(s.phi),
    };
    let mut at = None;
    let minimum = e.literal(128)?;
    let longer = e.value(
        sized,
        &mut at,
        I::Compare(Compare::Uge, Type::I64, s.chunks, minimum),
        Type::I64,
    )?;
    e.emit(sized, &mut at, I::Branch(longer, same, s.wide), Type::Void)?;
    at = None;
    let mask = e.literal(255)?;
    let low = e.value(
        same,
        &mut at,
        I::Binary(Binary::And, Flags::default(), Type::I64, s.v64, mask),
        Type::I64,
    )?;
    let replicate = e.literal(0x0101_0101_0101_0101)?;
    let spread = e.value(
        same,
        &mut at,
        I::Binary(Binary::Mul, Flags::default(), Type::I64, low, replicate),
        Type::I64,
    )?;
    let equal = e.value(
        same,
        &mut at,
        I::Compare(Compare::Eq, Type::I64, spread, s.v64),
        Type::I64,
    )?;
    e.emit(same, &mut at, I::Branch(equal, fill, s.wide), Type::Void)?;
    at = None;
    let byte = e.value(
        fill,
        &mut at,
        I::Convert(Convert::Truncate, Type::I32, Type::I64, low),
        Type::I32,
    )?;
    let arguments = e.body.add_arguments(&[s.init, byte, s.chunks])?;
    e.emit(
        fill,
        &mut at,
        I::Call(Call {
            result: Type::Ptr,
            callee: Value::Symbol(symbol),
            arguments,
            copy_elision: false,
            argument_classes: (Vec::new()).into(),
            signature: None,
        }),
        Type::Ptr,
    )?;
    e.emit(fill, &mut at, I::Jump(s.tail), Type::Void)?;
    let last = *e
        .body
        .block(s.pre)
        .instructions
        .last()
        .ok_or(Error::Limit)?;
    let mut entry = e.body.instruction(last).clone();
    let I::Branch(_, yes, _) = &mut entry.kind else {
        return Err(Error::Limit);
    };
    *yes = sized;
    e.body.update_instruction(last, entry);
    let mut phi = e.body.instruction(s.phi).clone();
    let I::Phi(_, pairs) = &mut phi.kind else {
        return Err(Error::Limit);
    };
    *pairs = vec![(sized, s.init), (same, s.init), (s.wide, s.q2)];
    e.body.update_instruction(s.phi, phi);
    for (id, value) in s.joined {
        e.body.append_phi_predecessor(id, fill, value)?;
    }
    let at = e
        .body
        .layout
        .iter()
        .position(|&b| b == s.wide)
        .ok_or(Error::Limit)?;
    e.body.layout.splice(at..at, [sized, same, fill]);
    e.links.commit(e.body)
}
pub(super) fn run(context: &mut Context<'_>) -> Result<bool, Error> {
    let mut changed = false;
    let mut index = 0;
    while index < context.body.layout.len() {
        let wide = context.body.layout[index];
        if let Some(s) = shape(context.body, wide) {
            if !loop_fills::available(context.program) {
                break;
            }
            let symbol = loop_fills::fill_symbol(context.program, context.symbol)?;
            version(context, s, symbol)?;
            changed = true;
        }
        index += 1;
    }
    context.invalidate_block_map();
    if changed {
        context.invalidate_cfg();
    }
    Ok(changed)
}
