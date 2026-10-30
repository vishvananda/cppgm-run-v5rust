//! Repeated-byte loop recognition and emission from `opt/loop_passes.cpp`.
//! See NOTICE.
use super::{
    induction::{Exit, Facts, Induction},
    memory_contents::bytes,
    natural_loops::{Forest, LoopId},
    numeric_fold::normalize,
    pass_context::Context,
};
use crate::lowir::{
    Error,
    model::{
        BlockId, Body, Boundary, Call, Facts as Flags, Function, InstId, Instruction,
        InstructionKind as I, LinkEditor, Metadata, Parameter, Program, Signature, SymbolDetail,
        SymbolId, Unwind, Value,
    },
    vocabulary::{Binary, Binding, Convert, Type},
};

const MEMSET: &str = "__cppgm_builtin_memset";

pub(super) fn available(program: &Program) -> bool {
    program
        .names
        .find(MEMSET)
        .and_then(|n| program.find_symbol(n))
        .is_none_or(|s| {
            matches!(
                program.symbol(s).detail,
                SymbolDetail::Unresolved | SymbolDetail::Function(_)
            )
        })
}
pub(super) fn fill_symbol(program: &mut Program, owner: SymbolId) -> Result<SymbolId, Error> {
    let name = program.names.intern(MEMSET)?;
    let symbol = program.reference_from(name, owner)?;
    if program.symbol(symbol).declared() {
        return Ok(symbol);
    }
    let empty = program.names.intern("")?;
    let signature = program.add_signature(Signature {
        parameters: [Type::Ptr, Type::I32, Type::I64]
            .into_iter()
            .map(|ty| Parameter {
                name: empty,
                ty,
                pass: None,
                no_alias: false,
                object_bytes: 0,
                classes: None,
            })
            .collect(),
        result: Type::Ptr,
        boundary: Boundary {
            unwind: Unwind::No,
            ..Boundary::default()
        },
    })?;
    let object = program.names.intern("cppgm_builtin_memset")?;
    program.add(
        symbol,
        Function {
            name,
            signature,
            metadata: Metadata {
                binding: Some(Binding::Strong),
                object: Some(object),
                ..Metadata::default()
            },
            body: None,
            location: None,
        },
    )?;
    Ok(symbol)
}

fn pattern(facts: &Facts<'_>, loop_: LoopId, iv: Induction) -> Option<(InstId, Option<InstId>)> {
    let mut store = None;
    let mut load = None;
    for &block in &facts.forest.get(loop_).blocks {
        for &id in &facts.body.block(block).instructions {
            match facts.body.instruction(id).kind {
                I::Store(flags, ..) => {
                    if store.is_some() || flags.volatile() {
                        return None;
                    }
                    store = Some(id);
                }
                I::Load(flags, _, address) => {
                    if load.is_some() || flags.volatile() || !facts.invariant(loop_, address) {
                        return None;
                    }
                    load = Some(id);
                }
                I::Jump(_) | I::Branch(..) => (),
                ref kind if super::pure(kind) => (),
                _ => return None,
            }
        }
    }
    let store = store?;
    if !iv.pointer {
        return None;
    }
    let l = facts.forest.get(loop_);
    let at = facts.block_of[store.index()]?;
    if at == l.header || !facts.cfg.dominates(at, l.latches[0]) {
        return None;
    }
    let I::Store(_, ty, value, mut address) = facts.body.instruction(store).kind else {
        return None;
    };
    let size = bytes(ty);
    if ty.bits() == 0 || size == 0 || u64::try_from(iv.step).ok()? != size {
        return None;
    }
    for _ in 0..4 {
        let Value::Temp(t) = address else {
            break;
        };
        if t == iv.value {
            break;
        }
        let def = facts.body.temp(t).definition?;
        let I::Copy(_, v) = facts.body.instruction(def).kind else {
            return None;
        };
        address = v;
    }
    if address != Value::Temp(iv.value) {
        return None;
    }
    let matches = match value {
        Value::Integer(k) => {
            let bits = normalize(ty, facts.body.literal(k));
            // The frozen x86-64 oracle tests Literal::low even for i128;
            // its word shifts repeat after eight bytes. Preserve that
            // observed low-word rule without an out-of-range host shift.
            (1..size).all(|k| (bits >> (8 * (k % 8))) & 255 == bits & 255)
                && load.is_none()
        }
        Value::Temp(t) if size == 1 => load.map_or_else(|| facts.invariant(loop_, value), |id| {
            facts.body.instruction(id).result == Some(t)
                && matches!(facts.body.instruction(id).kind, I::Load(_, loaded, _) if loaded == ty)
        }),
        _ => false,
    };
    matches.then_some((store, load))
}

struct Emitter<'a> {
    body: &'a mut Body,
    links: LinkEditor,
    block: BlockId,
    after: Option<InstId>,
}
impl Emitter<'_> {
    fn emit(&mut self, kind: I, ty: Type, location: Option<InstId>) -> Result<Value, Error> {
        let result = self.body.generated_temp(ty)?;
        let id = self
            .body
            .append(self.block, Instruction::new(Some(result), kind, 0))?;
        if let Some(location) = location {
            self.body.inherit_location(id, location);
        }
        self.links.insert_after(self.block, self.after, id)?;
        self.after = Some(id);
        Ok(Value::Temp(result))
    }
    fn count(&mut self, init: Value, bound: Value) -> Result<Value, Error> {
        if let Value::Temp(t) = bound
            && let Some(def) = self.body.temp(t).definition
            && let I::Index(ty, _, base, count) = self.body.instruction(def).kind
            && base == init
            && bytes(ty) == 1
        {
            return Ok(count);
        }
        let end = self.emit(I::Copy(Type::I64, bound), Type::I64, None)?;
        let begin = self.emit(I::Copy(Type::I64, init), Type::I64, None)?;
        self.emit(
            I::Binary(Binary::Sub, Flags::default(), Type::I64, end, begin),
            Type::I64,
            None,
        )
    }
    fn fill(
        &mut self,
        iv: Induction,
        test: Exit,
        store: InstId,
        load: Option<InstId>,
        symbol: SymbolId,
    ) -> Result<(), Error> {
        let init = iv.init.ok_or(Error::Limit)?;
        let count = self.count(init, test.bound)?;
        let I::Store(_, ty, mut value, _) = self.body.instruction(store).kind else {
            return Err(Error::Limit);
        };
        let byte = if let Value::Integer(k) = value {
            self.body.integer(self.body.literal(k) & 255)?
        } else {
            if let Some(load) = load {
                let I::Load(_, _, address) = self.body.instruction(load).kind else {
                    return Err(Error::Limit);
                };
                value = self.emit(I::Load(Flags::default(), ty, address), ty, None)?;
            }
            self.emit(
                I::Convert(Convert::ZeroExtend, Type::I32, ty, value),
                Type::I32,
                None,
            )?
        };
        let arguments = self.body.add_arguments(&[init, byte, count])?;
        self.emit(
            I::Call(Call {
                result: Type::Ptr,
                callee: Value::Symbol(symbol),
                arguments,
                copy_elision: false,
                argument_classes: (Vec::new()).into(),
                signature: None,
            }),
            Type::Ptr,
            Some(store),
        )?;
        Ok(())
    }
}

pub(super) fn emit(
    context: &mut Context<'_>,
    forest: &Forest,
    loop_: LoopId,
    test: Exit,
    ivs: &[Induction],
) -> Result<bool, Error> {
    let l = forest.get(loop_);
    let Some(pre) = l.preheader else {
        return Ok(false);
    };
    if test.exiting != l.header || !available(context.program) {
        return Ok(false);
    }
    let facts = Facts {
        body: context.body,
        cfg: context.cfg()?,
        forest,
        block_of: context.block_map(),
    };
    let Some((iv, (store, load))) = ivs
        .iter()
        .copied()
        .filter(|iv| test.compared == Value::Temp(iv.value) && iv.step >= 0)
        .find_map(|iv| pattern(&facts, loop_, iv).map(|p| (iv, p)))
    else {
        return Ok(false);
    };
    let symbol = fill_symbol(context.program, context.symbol)?;
    let links = LinkEditor::new(context.body)?;
    let last = *context
        .body
        .block(pre)
        .instructions
        .last()
        .ok_or(Error::Limit)?;
    let after = links.previous(last);
    let mut emitter = Emitter {
        body: context.body,
        links,
        block: pre,
        after,
    };
    emitter.fill(iv, test, store, load, symbol)?;
    emitter.links.commit(emitter.body)?;
    Ok(true)
}
