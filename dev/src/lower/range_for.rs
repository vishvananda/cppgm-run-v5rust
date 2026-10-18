// Translated from lower/statement_lowering.cpp::range_for_statement. See NOTICE.
//! The analyzed equivalent supplies all expressions and declarations; this owner
//! only orders their lowering and manages loop and per-iteration lifetimes.
use super::{Error, function::Function, statements::JumpTargets};
use crate::syntax::ast::{self, Condition};

impl Function<'_, '_, '_, '_> {
    pub(super) fn local_object_type(
        &self,
        entity: crate::sema::lowering::EntityId,
    ) -> Result<crate::sema::lowering::TypeId, Error> {
        let ty = self.graph.lower_entity_type(entity)?;
        if self.graph.lower_range_object(entity) {
            if let crate::sema::lowering::TypeView::Reference { element, .. } =
                self.graph.lower_type(ty)?
            {
                return Ok(element);
            }
            return Err(
                crate::sema::Error::Invariant("range object without reference type").into(),
            );
        }
        Ok(ty)
    }
    pub(super) fn range_for_statement(
        &mut self,
        id: ast::StmtId,
        body: ast::StmtId,
    ) -> Result<(), Error> {
        let facts = self.graph.lower_range_for(id)?;
        let depth = self.lifetime.cleanups.len();
        if let Some(range) = facts.range {
            self.local_variable(range.declarator, range.initializer, None)?;
        }
        self.local_variable(facts.begin.declarator, facts.begin.initializer, None)?;
        if let Some(end) = facts.end {
            self.local_variable(end.declarator, end.initializer, None)?;
        }
        let test = self.new_block("for_cond")?;
        let run = self.new_block("for_body")?;
        let step = self.new_block("for_iter")?;
        let end = self.new_block("for_end")?;
        self.jump(test)?;
        self.place(test)?;
        self.condition_branch(Condition::Expression(facts.condition), run, end, false)?;
        self.place(run)?;
        let scope = self.lifetime.cleanups.len();
        self.jumps.push(JumpTargets {
            end,
            continuation: Some(step),
            cleanup_depth: scope,
        });
        self.local_variable(
            facts.declaration.declarator,
            facts.declaration.initializer,
            None,
        )?;
        self.scoped_statement(body)?;
        self.leave_scope(scope)?;
        self.jumps.pop();
        if !self.terminated {
            self.jump(step)?;
        }
        self.place(step)?;
        self.discard_full_expression(facts.increment)?;
        self.jump(test)?;
        self.place(end)?;
        self.leave_scope(depth)
    }
}
