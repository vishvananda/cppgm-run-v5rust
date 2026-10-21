//! Per-function typed scalar call boundaries and cached ABI layouts. See NOTICE.
use super::{abi::Layout, memory, value_facts::Facts, value_selection::unsupported};
use crate::lowir::{
    Error,
    model::{Arity, Body, Call, InstructionKind as I, Program, Returns, Unwind, Value},
    vocabulary::{Pass, Type},
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CallKind {
    Ordinary,
    Trap,
    Memcpy,
}
pub(super) struct CallPlan {
    pub(super) layout: Layout,
    pub(super) types: Vec<Type>,
    pub(super) passes: Vec<Option<Pass>>,
    pub(super) variadic: bool,
    pub(super) no_unwind: bool,
    pub(super) no_return: bool,
    pub(super) kind: CallKind,
    pub(super) early: u32,
}
#[derive(Clone, Copy)]
struct CallId(u32);
pub(super) struct Calls {
    ids: Vec<Option<CallId>>,
    entries: Vec<CallPlan>,
}
impl Calls {
    pub(super) fn at(&self, pos: usize) -> Option<&CallPlan> {
        self.ids
            .get(pos)
            .copied()
            .flatten()
            .map(|id| &self.entries[id.0 as usize])
    }
    pub(super) fn analyze(program: &Program, body: &Body, facts: &Facts) -> Result<Self, Error> {
        let instructions = &facts.flow.instructions;
        let mut out = Self {
            ids: Vec::new(),
            entries: Vec::new(),
        };
        for (pos, &id) in instructions.iter().enumerate() {
            if !facts.active[pos] {
                continue;
            }
            let I::Call(call) = &body.instruction(id).kind else {
                continue;
            };
            if out.ids.is_empty() {
                out.ids.resize(instructions.len(), None);
            }
            out.ids[pos] = Some(CallId(
                u32::try_from(out.entries.len()).map_err(|_| Error::Limit)?,
            ));
            out.entries
                .push(CallPlan::analyze(program, body, call, facts)?);
        }
        Ok(out)
    }
}
impl CallPlan {
    fn analyze(program: &Program, body: &Body, call: &Call, facts: &Facts) -> Result<Self, Error> {
        let sig = program.call_boundary(call).ok_or_else(unsupported)?;
        if !super::aggregate::memory_value(call.result) && call.result.float_rank() == 0 {
            memory::scalar(call.result)?;
        }
        let args = body.arguments(call.arguments);
        let mut types = Vec::with_capacity(args.len());
        let mut passes = Vec::with_capacity(args.len());
        for (index, &value) in args.iter().enumerate() {
            let ty = if let Some(p) = sig.parameters.get(index) {
                p.ty
            } else {
                match value {
                    Value::Temp(t) => body.temp(t).ty.ok_or(Error::Limit)?,
                    Value::Slot(s) => body.slot(s).ty,
                    Value::Integer(_) => Type::I64,
                    Value::Float(_, k) => k.ty(),
                    Value::Symbol(_) => Type::Ptr,
                }
            };
            if !super::aggregate::memory_value(ty) && ty.float_rank() == 0 {
                memory::scalar(ty)?;
            }
            types.push(ty);
            passes.push(sig.parameters.get(index).and_then(|p| p.pass));
        }
        let layout = Layout::classify_call(sig, &types)?;
        let builtin =
            if let super::value_facts::Alias::Symbol(symbol) = facts.resolve(body, call.callee) {
                program
                    .callee_metadata(Value::Symbol(symbol))
                    .and_then(|metadata| metadata.object)
                    .map(|n| program.names.bytes(n))
            } else {
                None
            };
        let memcpy = named(builtin, b"cppgm_builtin_memcpy")
            && args.len() == 3
            && sig.parameters.len() == 3
            && types == [Type::Ptr, Type::Ptr, Type::I64]
            && passes.iter().all(Option::is_none)
            && sig.result == Type::Ptr
            && sig.boundary.arity != Arity::Variadic;
        Ok(Self {
            layout,
            types,
            passes,
            variadic: sig.boundary.arity == Arity::Variadic,
            no_unwind: sig.boundary.unwind == Unwind::No,
            no_return: sig.boundary.returns == Returns::NoReturn,
            kind: if matches!(call.callee, Value::Symbol(_))
                && named(builtin, b"cppgm_builtin_trap")
            {
                CallKind::Trap
            } else if memcpy {
                CallKind::Memcpy
            } else {
                CallKind::Ordinary
            },
            early: if args.len() > sig.parameters.len()
                || matches!(call.result, Type::Object { .. })
                || sig.parameters.iter().any(|p| super::aggregate::memory_value(p.ty) || p.ty == Type::F80)
                || args.iter().zip(&sig.parameters).any(|(value, parameter)| parameter.pass == Some(Pass::ByAddress) && matches!(*value, Value::Temp(temp) if matches!(body.temp(temp).ty, Some(Type::Object { .. })))) {
                (1 << 7) | (1 << 6) | (1 << 1)
            } else {
                0
            },
        })
    }
}

// C++ strcmp against fixed builtin vocabulary stops at its first mismatch.
// Preserve its first-NUL boundary without scanning an arbitrary object name.
fn named(object: Option<&[u8]>, expected: &[u8]) -> bool {
    object.is_some_and(|name| {
        name.starts_with(expected) && name.get(expected.len()).is_none_or(|&byte| byte == 0)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builtin_identity_preserves_the_c_string_boundary() {
        let name = b"cppgm_builtin_trap";
        assert!(named(Some(name), name));
        assert!(named(Some(b"cppgm_builtin_trap\0ignored"), name));
        assert!(!named(Some(b"cppgm_builtin_trap_suffix"), name));
        assert!(!named(Some(b"cppgm_builtin_tra\0p"), name));
        assert!(!named(None, name));
    }
}
