//! Named indirect return storage from `opt/copy_coalescing.cpp`. See NOTICE.
use super::{memory_contents::bytes, pass_context::Context};
use crate::lowir::{
    Error,
    model::{Body, InstructionKind as I, LinkEditor, SlotId, TempId, Value},
    vocabulary::{Pass, Type},
};

fn direct_slot(body: &Body, value: Value) -> Option<SlotId> {
    match value {
        Value::Slot(slot) => Some(slot),
        Value::Temp(t) if !body.temp_is_parameter(t) => {
            match body.instruction(body.temp(t).definition?).kind {
                I::Addr(Value::Slot(slot)) => Some(slot),
                _ => None,
            }
        }
        _ => None,
    }
}
struct ReturnStorage {
    slot: SlotId,
    result: TempId,
    copies: Vec<bool>,
    of_slot: Vec<bool>,
    of_result: Vec<bool>,
}
impl ReturnStorage {
    fn find(context: &Context<'_>) -> Result<Option<Self>, Error> {
        let body = &context.body;
        let function = context
            .program
            .function(context.program.symbol(context.symbol))
            .ok_or(Error::Limit)?;
        let mut result = None;
        for (index, p) in context
            .program
            .signature(function.signature)
            .parameters
            .iter()
            .enumerate()
        {
            let Ok(Value::Temp(temp)) = body.parameter_value(index) else {
                break;
            };
            if p.pass == Some(Pass::IndirectResult) {
                result = Some(temp);
            }
        }
        let Some(result) = result else {
            return Ok(None);
        };
        let mut slot = None;
        let mut copies = vec![false; body.instructions().len()];
        for &block in &body.layout {
            let instructions = &body.block(block).instructions;
            let Some((&last, rest)) = instructions.split_last() else {
                continue;
            };
            if !matches!(body.instruction(last).kind, I::Return(..)) {
                continue;
            }
            let Some(&previous) = rest.last() else {
                continue;
            };
            let I::CopyObject(size, _, source, destination) = body.instruction(previous).kind
            else {
                continue;
            };
            if destination != Value::Temp(result) {
                continue;
            }
            let Some(candidate) = direct_slot(body, source) else {
                continue;
            };
            if bytes(body.slot(candidate).ty) != size || slot.is_some_and(|s| s != candidate) {
                continue;
            }
            slot = Some(candidate);
            copies[previous.index()] = true;
        }
        let Some(slot) = slot else {
            return Ok(None);
        };
        let mut storage = Self {
            slot,
            result,
            copies,
            of_slot: vec![false; body.temps().len()],
            of_result: vec![false; body.temps().len()],
        };
        storage.of_result[result.index()] = true;
        storage.addresses(body);
        Ok(Some(storage))
    }
    fn addresses(&mut self, body: &Body) {
        for &block in &body.layout {
            for &id in &body.block(block).instructions {
                let inst = body.instruction(id);
                let Some(result) = inst.result else {
                    continue;
                };
                match inst.kind {
                    I::Addr(Value::Slot(slot)) if slot == self.slot => {
                        self.of_slot[result.index()] = true;
                    }
                    I::Index(_, _, Value::Temp(base), _)
                    | I::Copy(Type::Ptr, Value::Temp(base)) => {
                        self.of_slot[result.index()] = self.of_slot[base.index()];
                        self.of_result[result.index()] = self.of_result[base.index()];
                    }
                    _ => {}
                }
            }
        }
    }
    fn address(context: &Context<'_>, kind: &I, at: usize) -> bool {
        match kind {
            I::Load(..) | I::ZeroInit(..) | I::Index(..) => at == 0,
            I::Store(..) => at == 1,
            I::CopyObject(..) => at <= 1,
            I::Copy(Type::Ptr, _) | I::Addr(_) => true,
            // Rust stores only the callee followed by actual arguments. C++'s
            // two unused inline operands precede those same arguments.
            I::Call(call) => at
                .checked_sub(1)
                .and_then(|index| context.program.call_boundary(call)?.parameters.get(index))
                .is_some_and(|p| p.pass == Some(Pass::IndirectResult)),
            _ => false,
        }
    }
    fn permits(&self, context: &Context<'_>) -> bool {
        let body = &context.body;
        for &block in &body.layout {
            let mut first_result = None;
            let mut last_slot = None;
            for (position, &id) in body.block(block).instructions.iter().enumerate() {
                let inst = body.instruction(id);
                let mut at = 0;
                let mut ok = true;
                let mut uses_slot = false;
                let mut uses_result = false;
                body.visit_values(&inst.kind, |v| {
                    let operand = at;
                    at += 1;
                    let slot = matches!(v,Value::Slot(s) if s==self.slot)
                        || matches!(v,Value::Temp(t) if self.of_slot[t.index()]);
                    let result = matches!(v,Value::Temp(t) if self.of_result[t.index()]);
                    if !slot && !result {
                        return;
                    }
                    uses_slot |= slot;
                    uses_result |= result;
                    ok &= Self::address(context, &inst.kind, operand);
                });
                if !ok {
                    return false;
                }
                if !self.copies[id.index()] {
                    if uses_result && first_result.is_none() {
                        first_result = Some(position);
                    }
                    if uses_slot {
                        last_slot = Some(position);
                    }
                }
            }
            if let Some(first) = first_result {
                let returns = body
                    .block(block)
                    .instructions
                    .last()
                    .is_some_and(|&id| matches!(body.instruction(id).kind, I::Return(..)));
                if !returns || last_slot.is_some_and(|last| last > first) {
                    return false;
                }
            }
        }
        true
    }
    fn replace(&self, value: Value) -> Value {
        if value == Value::Slot(self.slot) {
            Value::Temp(self.result)
        } else {
            value
        }
    }
    fn apply(&self, body: &mut Body) -> Result<(), Error> {
        let mut links = LinkEditor::new(body)?;
        for block in body.layout.clone() {
            let mut cursor = links.first(block);
            while let Some(id) = cursor {
                cursor = links.next(id);
                if self.copies[id.index()] {
                    links.unlink(block, id)?;
                    continue;
                }
                if let I::Addr(Value::Slot(slot)) = body.instruction(id).kind {
                    if slot == self.slot {
                        body.replace_kind(id, I::Copy(Type::Ptr, Value::Temp(self.result)));
                    }
                } else {
                    body.rewrite_values(id, |v| Ok(self.replace(v)))?;
                }
            }
        }
        links.commit(body)
    }
}
pub(super) fn name(context: &mut Context<'_>) -> Result<bool, Error> {
    let Some(storage) = ReturnStorage::find(context)? else {
        return Ok(false);
    };
    if !storage.permits(context) {
        return Ok(false);
    }
    storage.apply(context.body)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::super::pass_context::Pass;
    #[test]
    fn indirect_result_arguments_can_name_returned_storage() {
        let source = "declare function @g(%out : ptr [pass=indirect_result],%v : i64)->void\nfunction @f(%ret : ptr [pass=indirect_result])->void {slot $a : obj<16x8> block ^entry:%a = addr $a call void @g(%a,1) copyobj 16x8 $a, %ret return void}";
        let (output, facts) = super::super::pass_view(source, 1, Pass::NameReturnedSlot);
        assert!(facts.0);
        assert!(!output.contains("copyobj"));
        assert!(output.contains("%a = copy ptr %ret"));
        let escapes = source.replace("%out : ptr [pass=indirect_result]", "%out : ptr");
        assert!(
            !super::super::pass_view(&escapes, 1, Pass::NameReturnedSlot)
                .1
                .0
        );
    }
    #[test]
    fn slot_use_after_result_prevents_naming() {
        let source = "function @f(%ret : ptr [pass=indirect_result])->void {slot $a : obj<16x8> block ^entry:store i64 1, %ret store i64 0, $a copyobj 16x8 $a, %ret return void}";
        assert!(
            !super::super::pass_view(source, 1, Pass::NameReturnedSlot)
                .1
                .0
        );
        let earlier = source.replace(
            "store i64 1, %ret store i64 0, $a",
            "store i64 0, $a store i64 1, %ret",
        );
        assert!(
            super::super::pass_view(&earlier, 1, Pass::NameReturnedSlot)
                .1
                .0
        );
    }
}
