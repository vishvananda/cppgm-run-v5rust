// Translated from lower/function_lowering.cpp and exception_lowering.cpp. See NOTICE.
//! Function-try regions enclose subobject work and retain their original body identity.
use super::{Error, construction::Entry, function::Function};
use crate::{
    lowir::model::{InstructionKind as I, RuntimeInstruction as R},
    sema::lowering::FunctionName,
    syntax::ast,
};

#[derive(Clone, Copy)]
pub(super) struct FunctionTry {
    pub body: ast::StmtId,
    pub rethrow: bool,
}

impl Function<'_, '_, '_, '_> {
    pub(super) fn source_body(
        &mut self,
        entry: Entry,
        body: Option<&ast::FunctionBody>,
        calls_complete: bool,
    ) -> Result<(), Error> {
        let destructor = matches!(entry.facts(self.graph)?.name, FunctionName::Destructor(_));
        let deleting = matches!(entry, Entry::DestructorDeleting(_));
        let function_try = body.filter(|_| !deleting).and_then(|body| {
            body.handlers.map(|clauses| {
                self.handlers.function_try = Some(FunctionTry {
                    body: body.compound,
                    rethrow: destructor || matches!(entry, Entry::Constructor(..)),
                });
                (body.compound, clauses)
            })
        });
        let context = function_try
            .map(|(body, clauses)| self.begin_try(body, clauses))
            .transpose()?;
        if let Entry::Constructor(id, variant) = entry {
            let location = body.map_or(crate::support::source::SourceOffset(0), |body| {
                self.tree.statements.get(body.compound).location
            });
            self.initialize_constructor(id, variant, location)?;
        }
        if destructor {
            self.destructor_prologue(
                entry
                    .entity(self.graph)
                    .ok_or(crate::sema::Error::Invariant("destructor without entity"))?,
                deleting,
                calls_complete,
            )?;
        }
        if let Some(body) = body.filter(|_| !calls_complete) {
            self.statement(body.compound)?;
        }
        self.finish_destructor()?;
        // Handlers run after the body and its subobjects, with no detached
        // body target or constructor record still considered live.
        self.construction.detached = false;
        if let Some(context) = context {
            if self.construction.active.is_some()
                && !self.terminated
                && self.lifetime.cleanups.is_empty()
            {
                self.effect(I::Runtime(R::EhEnd))?;
                self.construction.active = None;
            }
            // A handler's no-return fallback does not leave the body's
            // constructor record. Keep that record only for finish_body's
            // dead end after the complete try statement, as the oracle does.
            let active = self.construction.active.take();
            let result = self.end_try(context);
            self.construction.active = active;
            result?;
        }
        Ok(())
    }
    pub(super) fn function_try_subobjects(&self, context: Option<usize>) -> bool {
        context == Some(0)
            && self.handlers.function_try.is_some()
            && (self.construction.active.is_some() || self.destruction.cleanup.is_some())
    }
}
