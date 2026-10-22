//! Exit values of effect-free header inductions from `opt/loop_passes.cpp`.
//! See NOTICE.
use super::{
    alias_facts::literal_integer,
    induction::{Exit, Facts, Induction},
    natural_loops::LoopId,
    pass_context::Context,
};
use crate::lowir::{
    Error,
    model::{
        BlockId, Body, Facts as Flags, InstId, Instruction, InstructionKind as I, LinkEditor, Value,
    },
    vocabulary::{Binary, Convert, Type},
};

fn fits(ty: Type) -> bool {
    ty == Type::Ptr || ty.bits() != 0 && !matches!(ty, Type::I1 | Type::I128)
}
struct Emitter<'a> {
    body: &'a mut Body,
    links: LinkEditor,
    block: BlockId,
    location: InstId,
    after: Option<InstId>,
}
impl Emitter<'_> {
    fn emit(&mut self, kind: I, ty: Type) -> Result<Value, Error> {
        let temp = self.body.generated_temp(ty)?;
        let id = self
            .body
            .append(self.block, Instruction::new(Some(temp), kind, 0))?;
        self.body.inherit_location(id, self.location);
        self.links.insert_after(self.block, self.after, id)?;
        self.after = Some(id);
        Ok(Value::Temp(temp))
    }
    fn literal(&mut self, value: i64) -> Result<Value, Error> {
        self.body.integer(literal_integer(value))
    }
    fn binary(&mut self, op: Binary, ty: Type, a: Value, b: Value) -> Result<Value, Error> {
        self.emit(I::Binary(op, Flags::default(), ty, a, b), ty)
    }
    fn trips(&mut self, test: Exit, tested: Induction, ty: Type) -> Result<Value, Error> {
        if let Some(trips) = test.trips {
            return self.literal(trips);
        }
        let init = tested.init.ok_or(Error::Limit)?;
        if tested.pointer {
            let bytes = self.binary(Binary::Sub, Type::Ptr, test.bound, init)?;
            let step = self.literal(tested.step)?;
            return self.emit(
                I::Binary(
                    Binary::Div,
                    Flags::arithmetic(false, true),
                    Type::I64,
                    bytes,
                    step,
                ),
                Type::I64,
            );
        }
        let diff = if tested.step == 1 {
            self.binary(Binary::Sub, ty, test.bound, init)?
        } else {
            self.binary(Binary::Sub, ty, init, test.bound)?
        };
        if ty == Type::I64 {
            Ok(diff)
        } else {
            self.emit(
                I::Convert(Convert::ZeroExtend, Type::I64, ty, diff),
                Type::I64,
            )
        }
    }
    fn final_value(&mut self, iv: Induction, ty: Type, trips: Value) -> Result<Value, Error> {
        let init = iv.init.ok_or(Error::Limit)?;
        let step = self.literal(iv.step)?;
        if iv.pointer {
            let bytes = self.binary(Binary::Mul, Type::I64, trips, step)?;
            // Preserve the frozen helper's explicit temporary type: it uses
            // the instruction's stored element type for this index result.
            return self.emit(I::Index(Type::I8, None, init, bytes), Type::I8);
        }
        let count = if ty == Type::I64 {
            trips
        } else {
            self.emit(I::Convert(Convert::Truncate, ty, Type::I64, trips), ty)?
        };
        let product = self.binary(Binary::Mul, ty, count, step)?;
        self.binary(Binary::Add, ty, init, product)
    }
}
fn read_values(facts: &Facts<'_>, loop_: LoopId, ivs: &[Induction]) -> Option<Vec<bool>> {
    let mut owners = vec![None; facts.body.temps().len()];
    for (index, iv) in ivs.iter().enumerate() {
        owners[iv.value.index()] = Some(index);
    }
    let mut read = vec![false; ivs.len()];
    for &block in &facts.body.layout {
        if facts.forest.contains(loop_, block) {
            continue;
        }
        for &id in &facts.body.block(block).instructions {
            let mut ok = true;
            facts
                .body
                .visit_values(&facts.body.instruction(id).kind, |v| {
                    let Value::Temp(temp) = v else {
                        return;
                    };
                    if facts.invariant(loop_, v) {
                        return;
                    }
                    if let Some(index) = owners[temp.index()] {
                        read[index] = true;
                    } else {
                        ok = false;
                    }
                });
            if !ok {
                return None;
            }
        }
    }
    Some(read)
}
pub(super) fn compute(
    context: &mut Context<'_>,
    forest: &super::natural_loops::Forest,
    loop_: LoopId,
    test: Exit,
    ivs: &[Induction],
) -> Result<bool, Error> {
    let l = forest.get(loop_);
    let Some(pre) = l.preheader else {
        return Ok(false);
    };
    if test.exiting != l.header {
        return Ok(false);
    }
    let Value::Temp(compared) = test.compared else {
        return Ok(false);
    };
    let Some(tested) = ivs.iter().rev().find(|iv| iv.value == compared).copied() else {
        return Ok(false);
    };
    if test.trips.is_none() && !tested.pointer && tested.step != 1 && tested.step != -1 {
        return Ok(false);
    }
    let tested_type = context.body.temp(tested.value).ty.ok_or(Error::Limit)?;
    if !fits(tested_type) {
        return Ok(false);
    }
    let facts = Facts {
        body: context.body,
        cfg: context.cfg()?,
        forest,
        block_of: context.block_map(),
    };
    let Some(read) = read_values(&facts, loop_, ivs) else {
        return Ok(false);
    };
    for (iv, &read) in ivs.iter().zip(&read) {
        if read && !fits(context.body.temp(iv.value).ty.ok_or(Error::Limit)?) {
            return Ok(false);
        }
    }
    let location = *context
        .body
        .block(pre)
        .instructions
        .last()
        .ok_or(Error::Limit)?;
    let links = LinkEditor::new(context.body)?;
    let after = links.previous(location);
    let mut replacements = vec![None; context.body.temps().len()];
    let mut emitter = Emitter {
        body: context.body,
        links,
        block: pre,
        location,
        after,
    };
    let trips = emitter.trips(test, tested, tested_type)?;
    for (iv, &read) in ivs.iter().zip(&read) {
        if !read {
            continue;
        }
        let ty = emitter.body.temp(iv.value).ty.ok_or(Error::Limit)?;
        let value = if iv.value == tested.value {
            test.value
        } else {
            emitter.final_value(*iv, ty, trips)?
        };
        replacements[iv.value.index()] = Some(value);
    }
    emitter.links.commit(emitter.body)?;
    for block in context.body.layout.clone() {
        if forest.contains(loop_, block) {
            continue;
        }
        for id in context.body.block(block).instructions.clone() {
            context.body.rewrite_values(id, |v| {
                Ok(match v {
                    Value::Temp(temp) if v != test.compared => replacements
                        .get(temp.index())
                        .copied()
                        .flatten()
                        .unwrap_or(v),
                    _ => v,
                })
            })?;
        }
    }
    context.prepare_block_map(super::pass_context::Pass::RemoveLoops, false);
    context.invalidate_block_map();
    Ok(true)
}
