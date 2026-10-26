//! Explicit audit-only validation from `opt/optimizer.cpp`. See NOTICE.
use super::pass_context::{Context, Pass};
use crate::lowir::{
    self, Error,
    model::{Body, Value},
};
use std::{io::Write, path::Path};

#[derive(Clone, Copy, Default)]
pub(crate) struct Verification<'a> {
    pub(crate) enabled: bool,
    pub(crate) dump: Option<&'a Path>,
}

impl Context<'_> {
    pub(super) fn verify_after(&mut self, pass: &str) -> Result<(), Error> {
        if !self.verification.enabled {
            return Ok(());
        }
        // Attach the function for this whole-program audit, then return ownership
        // even on failure. No body is cloned or serialized to feed validation.
        let body = std::mem::replace(self.body, Body::new(None, 0));
        self.program.restore_body(self.symbol, body)?;
        let result = lowir::validator::validate_constructed(self.program);
        *self.body = self.program.take_body(self.symbol)?.ok_or(Error::Limit)?;
        let problem = match result {
            Ok(()) if self.verification.dump.is_some() && self.unlinked_definition_used() => Some(
                format!("unlinked definition used after {pass} in @{}", self.name()).into_bytes(),
            ),
            Ok(()) => None,
            Err(Error::Diagnostic(message)) => Some(self.invalid_message(pass, message.as_bytes())),
            Err(Error::ByteDiagnostic(message)) => Some(self.invalid_message(pass, &message)),
            Err(error) => return Err(error),
        };
        let Some(problem) = problem else {
            return Ok(());
        };
        let message = [b"lowiropt verify: ", problem.as_slice(), b"\n"].concat();
        let Some(dump) = self.verification.dump else {
            return Err(Error::PassVerification(message));
        };
        let _ = std::io::stderr().lock().write_all(&message);
        let body = std::mem::replace(self.body, Body::new(None, 0));
        self.program.restore_body(self.symbol, body)?;
        // The frozen audit mode ignores a failed dump write and continues.
        let _ = lowir::write_file(self.program, dump);
        *self.body = self.program.take_body(self.symbol)?.ok_or(Error::Limit)?;
        Ok(())
    }

    fn name(&self) -> &str {
        self.program
            .names
            .get(self.program.symbol(self.symbol).name)
    }

    fn invalid_message(&self, pass: &str, message: &[u8]) -> Vec<u8> {
        [
            format!("invalid after {pass} in @{}: ", self.name()).as_bytes(),
            message,
        ]
        .concat()
    }

    fn unlinked_definition_used(&self) -> bool {
        let mut linked = vec![false; self.body.instructions().len()];
        for &block in &self.body.layout {
            for &id in &self.body.block(block).instructions {
                linked[id.index()] = true;
            }
        }
        let mut found = false;
        for &block in &self.body.layout {
            for &id in &self.body.block(block).instructions {
                self.body
                    .visit_values(&self.body.instruction(id).kind, |v| {
                        found |= self.definition_unlinked(v, &linked);
                    });
            }
        }
        found
    }
    fn definition_unlinked(&self, value: Value, linked: &[bool]) -> bool {
        let Value::Temp(temp) = value else {
            return false;
        };
        !self.body.temp_is_parameter(temp)
            && self
                .body
                .temp(temp)
                .definition
                .is_none_or(|def| !linked.get(def.index()).copied().unwrap_or(false))
    }
}

impl Pass {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::SplitSlots => "split_aggregate_slots",
            Self::PromoteSlots => "promote_slots",
            Self::SimplifyValues => "simplify_values",
            Self::ForwardMemory => "forward_memory",
            Self::DeadCode => "remove_dead_code",
            Self::Constants => "propagate_constants",
            Self::CleanupCfg => "cleanup_cfg",
            Self::ThreadJumps => "thread_jumps",
            Self::StripRegions => "strip_dead_regions",
            Self::CoalesceCopies => "coalesce_copies",
            Self::RemoveLoops => "remove_loops",
            Self::UnrollLoops => "unroll_loops",
            Self::RotateLoops => "rotate_loops",
            Self::FormSwitches => "form_switches",
            Self::FormSelects => "form_selects",
            Self::ExpandWide => "expand_wide_integers",
            Self::HoistLoads => "hoist_loop_loads",
            Self::MarkCounters => "mark_no_wrap_counters",
            Self::VersionFills => "version_fill_loops",
            Self::NameReturnedSlot => "name_returned_slot",
            Self::SplitOrBranches => "split_or_branches",
            Self::SpecializePhiCompares => "specialize_phi_compares",
            Self::SplitBranchCompares => "split_branch_compares",
            Self::DefinitionOrder => "restore_definition_order",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{model::InstructionKind, vocabulary::Type};

    #[test]
    fn construction_checks_only_instructions_still_owned_by_the_layout() {
        let mut program = lowir::test_program(
            b"function @f()->void {block ^entry:%dead=const i64 7 return void}",
        )
        .unwrap();
        let symbol = program.publications()[0];
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let block = body.layout[0];
        let mut links = body.block(block).instructions.clone();
        let removed = links.remove(0);
        let mut inst = body.instruction(removed).clone();
        inst.result = None;
        body.update_instruction(removed, inst);
        body.replace_links(block, links).unwrap();
        program.restore_body(symbol, body).unwrap();
        lowir::validator::validate_constructed(&program).unwrap();
    }

    #[test]
    fn validation_names_the_pass_and_restores_the_owned_body_on_failure() {
        let mut program =
            lowir::test_program(b"function @f()->i64 {block ^entry:return i64 7}").unwrap();
        let symbol = program.publications()[0];
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let id = body.block(body.layout[0]).instructions[0];
        body.replace_kind(id, InstructionKind::Return(Type::Void, None));
        let mut context = Context::new(&mut program, symbol, &mut body, 1, &[], &[]);
        context.verification.enabled = true;
        let result = context.verify_after("forward_memory");
        assert!(
            matches!(result, Err(Error::PassVerification(bytes)) if bytes == b"lowiropt verify: invalid after forward_memory in @f: return type does not match function in function @f\n")
        );
        assert_eq!(context.body.layout.len(), 1);
        assert!(
            context
                .program
                .function(context.program.symbol(symbol))
                .unwrap()
                .body
                .is_none()
        );
    }
}
