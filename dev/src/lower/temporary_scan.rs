// Translated from lower/cleanup_lowering.cpp::creates_temporaries. See NOTICE.
//! Full-expression queries use the original operands and selected default arguments.
use super::{Error, function::Function};
use crate::{
    sema::lowering::{CallTarget, ConstructorId, TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};

#[derive(Clone, Copy, Default)]
pub(super) enum CallTemporaries {
    #[default]
    Absent,
    Nothrow,
    ThrowingDestructor,
}
impl CallTemporaries {
    fn include(&mut self, throws: bool) {
        if throws {
            *self = Self::ThrowingDestructor;
        } else if matches!(self, Self::Absent) {
            *self = Self::Nothrow;
        }
    }
    fn present(self) -> bool {
        !matches!(self, Self::Absent)
    }
    fn destructor_throws(self) -> bool {
        matches!(self, Self::ThrowingDestructor)
    }
}
#[derive(Clone, Copy, Default)]
pub(super) struct TemporarySummary {
    pub made: bool,
    pub call_result: CallTemporaries,
    pub caller_destroyed: bool,
}
#[derive(Clone, Copy)]
pub(super) enum ExpressionUse {
    Ordinary,
    Initialization,
    InitializerArgument,
    Value,
    ClassResult,
}
impl Function<'_, '_, '_, '_> {
    // cleanup_lowering.cpp::note_call_arguments: an in-place constructor
    // contributes its operands/defaults, rather than its destination object.
    pub(super) fn note_constructor_temporaries(
        &mut self,
        constructor: ConstructorId,
        written: &[ast::ExprId],
    ) -> Result<(), Error> {
        if !self.lifetime.full_expression
            || !self.lifetime.temporary_segment.allows_eager_arguments()
            || !self.lifetime.region_start.is_pending()
            || self.lifetime.region
        {
            return Ok(());
        }
        let mut made = false;
        for &argument in written {
            let outer_copy = matches!(self.graph.lower_class_copy(argument)?,
                Some(crate::sema::lowering::CopyAction::Constructor { constructor: selected, .. })
                    if selected == constructor);
            made |= if outer_copy {
                // This selected transfer constructs our destination; only
                // evaluating its source can make argument temporaries.
                self.scan_temporaries(argument, true, false, None)?.made
            } else {
                self.expression_temporaries(argument)?.made
            };
            if made {
                break;
            }
        }
        let function = self.graph.lower_constructor_function(constructor)?;
        if !made
            && let Some(entity) = self.graph.lower_constructor_entity(constructor)
            && let TypeView::Function { parameters, .. } = self.graph.lower_type(function.ty)?
        {
            for index in written.len()..parameters.len() {
                if let Some(argument) = self.graph.lower_default(entity, index) {
                    made |= self.expression_temporaries(argument)?.made;
                }
                if made {
                    break;
                }
            }
        }
        if made {
            self.lifetime.temporary_segment = super::lifetime::TemporarySegment::Eager;
            if !function.nothrow {
                self.open_temporary_region_at(self.lifetime.expression_start)?;
            }
        }
        Ok(())
    }
    pub(super) fn begin_constructor_initializer(
        &mut self,
        initializer: ast::Initializer,
        constructor: ConstructorId,
    ) -> Result<bool, Error> {
        self.begin_subobject_initializer(initializer, Some(constructor))
    }
    pub(super) fn begin_subobject_initializer(
        &mut self,
        initializer: ast::Initializer,
        constructor: Option<ConstructorId>,
    ) -> Result<bool, Error> {
        self.begin_initializer(initializer, constructor, false)
    }
    pub(super) fn begin_base_initializer(
        &mut self,
        initializer: ast::Initializer,
        constructor: Option<ConstructorId>,
    ) -> Result<bool, Error> {
        self.begin_initializer(initializer, constructor, true)
    }
    fn begin_initializer(
        &mut self,
        initializer: ast::Initializer,
        constructor: Option<ConstructorId>,
        base: bool,
    ) -> Result<bool, Error> {
        if self.lifetime.full_expression {
            return Ok(false);
        }
        let mut caller = false;
        if let Some(constructor) = constructor {
            let function = self.graph.lower_constructor_function(constructor)?;
            caller = self.caller_destroys_parameters(function.ty)?;
        }
        let (arguments, root) = match initializer.kind {
            ast::InitializerKind::Assign(root) => {
                (crate::syntax::storage::List::default(), Some(root))
            }
            ast::InitializerKind::Parenthesized(arguments) => (arguments, None),
            ast::InitializerKind::Braced(id) => {
                let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind else {
                    return Err(
                        crate::sema::Error::Invariant("delegating initializer list shape").into(),
                    );
                };
                (elements, Some(id))
            }
            _ => return Err(crate::sema::Error::Invariant("delegating initializer shape").into()),
        };
        let mut made = caller;
        let mut calls = false;
        let mut throwing_destructor = false;
        if let Some(root) = root {
            // A class prvalue with no outer transfer constructs the member
            // itself. Only its operands can create expression temporaries.
            let facts = self.graph.lower_expression(root)?;
            let in_place = !facts.is_place
                && matches!(self.graph.lower_type(facts.ty)?, TypeView::Class(_))
                && self.graph.lower_class_copy(root)?.is_none();
            let summary = if in_place {
                self.operand_temporaries(root)?
            } else {
                self.expression_temporaries(root)?
            };
            made |= summary.made;
            calls |= summary.call_result.present() || summary.caller_destroyed;
            throwing_destructor |= summary.call_result.destructor_throws();
        }
        for &argument in self.tree.expr_lists.get(arguments) {
            let summary = if constructor.is_some_and(|constructor| {
                self.graph.lower_elides_conversion(argument, constructor)
            }) {
                // The selected copy takes the conversion function's result
                // in place; only the receiver's operands make temporaries.
                self.operand_temporaries(argument)?
            } else if base {
                // The base's selected transfer initializes the destination;
                // it is not a conversion made while evaluating its argument.
                self.scan_temporaries(argument, true, false, None)?
            } else {
                self.expression_temporaries(argument)?
            };
            made |= summary.made;
            calls |= summary.call_result.present() || summary.caller_destroyed;
            throwing_destructor |= summary.call_result.destructor_throws();
        }
        if !made && self.lifetime.cleanups.is_empty() {
            return Ok(false);
        }
        // The braced list's root describes the destination, so it is not
        // another temporary. Its selected constructor does contribute to
        // the C++ owner's full-expression throw query for that list.
        let mut throwing = root
            .map(|id| self.expression_throws(id))
            .transpose()?
            .unwrap_or(false);
        for &argument in self.tree.expr_lists.get(arguments) {
            throwing |= self.expression_throws(argument)?;
        }
        if !caller && !throwing && (!calls || throwing_destructor) {
            return Ok(false);
        }
        self.lifetime.full_expression = true;
        self.lifetime.region_start = super::lifetime::RegionStart::Pending;
        self.lifetime.temporary_segment = if caller || made && throwing {
            super::lifetime::TemporarySegment::Eager
        } else if made {
            super::lifetime::TemporarySegment::Nothrow {
                floor: self.lifetime.cleanups.len(),
            }
        } else {
            super::lifetime::TemporarySegment::Empty
        };
        self.ensure_insertion()?;
        self.lifetime.expression_start = Some(self.builder.position()?);
        if let Some(root) = root {
            if caller || throwing {
                self.begin_tracking(root, ExpressionUse::Initialization)?;
            }
        } else if !caller && throwing {
            for &argument in self.tree.expr_lists.get(arguments) {
                self.begin_tracking(argument, ExpressionUse::InitializerArgument)?;
            }
        }
        Ok(true)
    }
    pub(super) fn begin_full_expression(
        &mut self,
        root: Option<ast::ExprId>,
        usage: ExpressionUse,
    ) -> Result<bool, Error> {
        if self.lifetime.full_expression || self.static_initializer {
            return Ok(false);
        }
        let Some(root) = root else { return Ok(false) };
        // The return destination excludes the root from materialization, not
        // from the full-expression's segment query. A class call's scalar
        // operands (including an indirect callee) still use that segment.
        let summary = self.expression_temporaries(root)?;
        if !summary.made && self.lifetime.cleanups.is_empty() {
            return Ok(false);
        }
        // class_return supplies a hidden context: once its initialization
        // conversion makes temporaries, call_may_throw brackets even a
        // nonthrowing transfer (cleanup_lowering.cpp).
        let hidden_copy = matches!(usage, ExpressionUse::ClassResult)
            && summary.made
            && matches!(
                self.graph.lower_class_copy(root)?,
                Some(crate::sema::lowering::CopyAction::Constructor { .. })
            );
        let hidden = summary.made
            && (matches!(usage, ExpressionUse::Value | ExpressionUse::ClassResult)
                || self.branch_temporaries(root)?);
        let throwing = hidden_copy || self.expression_throws_in_context(root, hidden)?;
        if (summary.made || !self.lifetime.cleanups.is_empty())
            && (throwing || summary.call_result.present() || summary.caller_destroyed)
        {
            self.lifetime.full_expression = true;
            self.lifetime.region_start = super::lifetime::RegionStart::Pending;
            self.lifetime.temporary_segment = if throwing {
                super::lifetime::TemporarySegment::new(summary.made, hidden)
            } else {
                super::lifetime::TemporarySegment::Nothrow {
                    floor: self.lifetime.cleanups.len(),
                }
            };
            self.ensure_insertion()?;
            self.lifetime.expression_start = Some(self.builder.position()?);
            self.begin_tracking(root, usage)?;
            return Ok(true);
        }
        Ok(false)
    }
    pub(super) fn expression_temporaries(
        &mut self,
        root: ast::ExprId,
    ) -> Result<TemporarySummary, Error> {
        let root = self.without_parentheses(root);
        if let Some(&summary) = self.lifetime.temporary_queries.get(&root) {
            return Ok(summary);
        }
        let summary = self.scan_temporaries(root, true, true, None)?;
        self.lifetime.temporary_queries.insert(root, summary);
        Ok(summary)
    }
    pub(super) fn operand_temporaries(
        &mut self,
        root: ast::ExprId,
    ) -> Result<TemporarySummary, Error> {
        self.scan_temporaries(root, false, true, None)
    }
    pub(super) fn conditional_operand_temporaries(
        &mut self,
        root: ast::ExprId,
        object: TypeId,
    ) -> Result<TemporarySummary, Error> {
        self.scan_temporaries(root, false, true, Some(object))
    }
    fn scan_temporaries(
        &mut self,
        root: ast::ExprId,
        include_root: bool,
        root_transfer: bool,
        object: Option<TypeId>,
    ) -> Result<TemporarySummary, Error> {
        let mut work = std::mem::take(&mut self.lifetime.temporary_work);
        let result =
            self.scan_temporary_operands(root, include_root, root_transfer, object, &mut work);
        work.clear();
        self.lifetime.temporary_work = work;
        result
    }
    fn scan_temporary_operands(
        &self,
        root: ast::ExprId,
        include_root: bool,
        root_transfer: bool,
        object: Option<TypeId>,
        work: &mut Vec<ast::ExprId>,
    ) -> Result<TemporarySummary, Error> {
        let root = self.without_parentheses(root);
        work.push(root);
        let mut summary = TemporarySummary::default();
        while let Some(id) = work.pop() {
            let id = self.without_parentheses(id);
            let facts = self.graph.lower_expression(id)?;
            let mut step = 0;
            while let Some(call) = self.graph.lower_arrow_call(id, step)? {
                if !call.place && self.temporary_type_destroyed(call.result)? {
                    summary.made = true;
                    summary
                        .call_result
                        .include(self.temporary_destructor_throws(call.result)?);
                }
                step += 1;
            }
            if facts.folded.is_some() {
                continue;
            }
            if let Some(crate::sema::lowering::CopyAction::Constructor { constructor, .. }) =
                self.graph.lower_class_copy(id)?
            {
                let function = self.graph.lower_constructor_function(constructor)?;
                // A recorded transfer is an initialization conversion in
                // C++ creates_temporaries. An operand-only query excludes
                // its destination, including a return's caller-owned result.
                let class = self.graph.lower_constructor(constructor).class;
                if root_transfer && include_root || id != root {
                    summary.made |=
                        self.temporary_type_destroyed(self.graph.lower_entity_type(class)?)?;
                }
                let caller = self.caller_destroys_parameters(function.ty)?;
                summary.made |= caller;
                summary.caller_destroyed |= caller;
            }
            if (include_root || id != root)
                && let Some((call, target)) = self.graph.lower_function_conversion(id)?
            {
                summary.made |= !self.graph.lower_builtin_operand(id)
                    && self.temporary_type_destroyed(target)?;
                if self.graph.lower_conversion_copy(id).is_some() && !call.place {
                    summary.made |= self.temporary_type_destroyed(call.result)?;
                }
            }
            let kind = self.tree.expressions.get(id).kind;
            let call =
                matches!(kind, ExprKind::Call { .. }) && !self.graph.lower_functional_call(id)?;
            let operator = self.graph.lower_operator_call(id)?;
            // A converted conditional branch constructs the common object,
            // while its original prvalue receiver may be a different temporary.
            // The C++ query excludes only the class of that destination.
            let distinct_source = if id == root
                && let Some(object) = object
            {
                !matches!((self.graph.lower_type(object)?,self.graph.lower_type(facts.ty)?),
                    (TypeView::Class(a),TypeView::Class(b)) if a == b)
            } else {
                false
            };
            if (include_root || id != root || distinct_source)
                && !facts.is_place
                && self.temporary_type_destroyed(facts.ty)?
            {
                summary.made = true;
                if call || operator || self.graph.lower_literal_call(id)?.is_some() {
                    summary
                        .call_result
                        .include(self.temporary_destructor_throws(facts.ty)?);
                }
            }
            if (include_root || id != root)
                && let Some((constructor, _)) = self.graph.lower_class_conversion(id)
            {
                let ty = self
                    .graph
                    .lower_entity_type(self.graph.lower_constructor(constructor).class)?;
                summary.made |= self.temporary_type_destroyed(ty)?;
            }
            // Constructor call-like nodes can own by-value arguments even
            // when their destination has no destructor. An operand-only
            // query excludes that destination, never these parameters.
            let conversion = self.graph.lower_class_conversion(id).map(|(id, _)| id);
            for constructor in [self.graph.lower_list_constructor(id)?, conversion]
                .into_iter()
                .flatten()
            {
                let ty = self.graph.lower_constructor_function(constructor)?.ty;
                let caller = self.caller_destroys_parameters(ty)?;
                summary.made |= caller;
                summary.caller_destroyed |= caller;
            }
            if operator {
                self.call_temporary_operands(id, usize::MAX, work, &mut summary)?;
            }
            match kind {
                ExprKind::Call { callee, arguments } => {
                    work.extend(self.tree.expr_lists.get(arguments));
                    if call {
                        self.call_temporary_operands(
                            id,
                            self.tree.expr_lists.get(arguments).len(),
                            work,
                            &mut summary,
                        )?;
                        if self.graph.lower_floating_intrinsic(id)?.is_none() {
                            work.push(callee);
                        }
                    }
                }
                ExprKind::Binary { left, right, .. }
                | ExprKind::Subscript {
                    base: left,
                    index: right,
                } => {
                    work.extend([left, right]);
                }
                ExprKind::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                } => {
                    work.extend([condition, then_expr, else_expr]);
                }
                ExprKind::Unary { operand, .. }
                | ExprKind::Postfix { operand, .. }
                | ExprKind::Cast { operand, .. }
                | ExprKind::Delete { operand, .. }
                | ExprKind::Member { base: operand, .. } => work.push(operand),
                ExprKind::FunctionalCast {
                    arguments, braced, ..
                } => {
                    work.extend(self.tree.expr_lists.get(arguments));
                    work.extend(braced);
                }
                ExprKind::Braced { elements, .. } => {
                    work.extend(self.tree.expr_lists.get(elements));
                }
                ExprKind::New {
                    placement,
                    initializer,
                    ..
                } => {
                    let allocation = self.graph.lower_allocation(id)?;
                    if let Some(array) = allocation.array {
                        work.push(array.count);
                    }
                    if let Some((constructor, _)) = allocation.constructor {
                        let ty = self.graph.lower_constructor_function(constructor)?.ty;
                        let caller = self.caller_destroys_parameters(ty)?;
                        summary.made |= caller;
                        summary.caller_destroyed |= caller;
                    }
                    work.extend(self.tree.expr_lists.get(placement.unwrap_or_default()));
                    if let Some(initializer) = initializer {
                        match initializer.kind {
                            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                                work.push(id);
                            }
                            ast::InitializerKind::Parenthesized(arguments) => {
                                work.extend(self.tree.expr_lists.get(arguments));
                            }
                            _ => (),
                        }
                    }
                }
                _ => (),
            }
        }
        Ok(summary)
    }
    fn call_temporary_operands(
        &self,
        id: ast::ExprId,
        given: usize,
        work: &mut Vec<ast::ExprId>,
        summary: &mut TemporarySummary,
    ) -> Result<(), Error> {
        if self.graph.lower_floating_intrinsic(id)?.is_some()
            || self.graph.lower_destructor_evaluation_only(id)?
        {
            return Ok(());
        }
        let (ty, entity) = match self.graph.lower_call(id)? {
            CallTarget::Direct(entity) => (self.graph.lower_function(entity)?.ty, Some(entity)),
            CallTarget::Indirect(ty, _) => (ty, None),
        };
        let TypeView::Function { parameters, .. } = self.graph.lower_type(ty)? else {
            return Err(crate::sema::Error::Invariant("temporary call without function").into());
        };
        let caller = self.caller_destroys_parameters(ty)?;
        summary.made |= caller;
        summary.caller_destroyed |= caller;
        for index in given..parameters.len() {
            if let Some(entity) = entity {
                work.extend(self.graph.lower_default(entity, index));
            }
        }
        Ok(())
    }
    fn caller_destroys_parameters(&self, ty: TypeId) -> Result<bool, Error> {
        let TypeView::Function { parameters, .. } = self.graph.lower_type(ty)? else {
            return Err(
                crate::sema::Error::Invariant("temporary signature without function").into(),
            );
        };
        for &parameter in parameters {
            // A nontrivial destructor implies the indirect class boundary.
            // References and ordinary direct-register classes contribute none.
            if matches!(self.graph.lower_type(parameter)?, TypeView::Class(_))
                && self.temporary_type_destroyed(parameter)?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn temporary_type_destroyed(&self, mut ty: TypeId) -> Result<bool, Error> {
        while let TypeView::Array { element, .. } = self.graph.lower_type(ty)? {
            ty = element;
        }
        Ok(matches!(self.graph.lower_type(ty)?, TypeView::Class(class)
            if self.graph.lower_complete_object(ty)? && self.graph.lower_nontrivial_destructor(class)?))
    }
    fn temporary_destructor_throws(&self, mut ty: TypeId) -> Result<bool, Error> {
        while let TypeView::Array { element, .. } = self.graph.lower_type(ty)? {
            ty = element;
        }
        let TypeView::Class(class) = self.graph.lower_type(ty)? else {
            return Ok(false);
        };
        self.graph
            .lower_destructor_entity(class)
            .map_or(Ok(false), |entity| {
                Ok(!self.graph.lower_function(entity)?.nothrow)
            })
    }
    pub(super) fn constructor_expression(
        &self,
        entity: crate::sema::lowering::EntityId,
        initializer: Option<ast::Initializer>,
        made: bool,
    ) -> Result<bool, Error> {
        let Some(constructor) = self
            .graph
            .lower_construction(entity)
            .filter(|_| initializer.is_some())
        else {
            return Ok(false);
        };
        let function = self.graph.lower_constructor_function(constructor)?;
        let caller = self.caller_destroys_parameters(function.ty)?;
        let single = match initializer.map(|initializer| initializer.kind) {
            Some(ast::InitializerKind::Parenthesized(arguments)) => {
                self.tree.expr_lists.get(arguments).len() == 1
            }
            Some(ast::InitializerKind::Assign(_) | ast::InitializerKind::Braced(_)) => true,
            _ => false,
        };
        // local_object first stages a single temporary-making expression;
        // caller-destroyed parameter forcing applies to the other initializer path.
        Ok(caller && !(single && made) || made && !function.nothrow)
    }
    pub(super) fn initializer_temporaries(
        &mut self,
        initializer: Option<ast::Initializer>,
    ) -> Result<bool, Error> {
        self.scan_initializer_temporaries(initializer, true)
    }
    pub(super) fn local_initializer_temporaries(
        &mut self,
        initializer: Option<ast::Initializer>,
        destination: TypeId,
    ) -> Result<bool, Error> {
        let root = match initializer.map(|value| value.kind) {
            Some(ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id)) => Some(id),
            Some(ast::InitializerKind::Parenthesized(args)) => {
                let args = self.tree.expr_lists.get(args);
                if args.len() == 1 { Some(args[0]) } else { None }
            }
            _ => None,
        };
        let copy_class = match root
            .map(|id| self.graph.lower_class_copy(id))
            .transpose()?
            .flatten()
        {
            Some(crate::sema::lowering::CopyAction::Constructor { constructor, .. }) => {
                Some(self.graph.lower_constructor(constructor).class)
            }
            _ => None,
        };
        let destination = self.graph.lower_type(destination)?;
        let destination_transfer = matches!((copy_class, destination),
            (Some(class), TypeView::Class(target)) if class == target);
        self.scan_initializer_temporaries(initializer, !destination_transfer)
    }
    fn scan_initializer_temporaries(
        &mut self,
        initializer: Option<ast::Initializer>,
        transfer: bool,
    ) -> Result<bool, Error> {
        let Some(initializer) = initializer else {
            return Ok(false);
        };
        match initializer.kind {
            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                self.initializer_expression_temporaries(id, transfer)
            }
            ast::InitializerKind::Parenthesized(arguments) => {
                for &argument in self.tree.expr_lists.get(arguments) {
                    if self.initializer_expression_temporaries(argument, transfer)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }
    fn initializer_expression_temporaries(
        &mut self,
        id: ast::ExprId,
        transfer: bool,
    ) -> Result<bool, Error> {
        let summary = if transfer {
            self.expression_temporaries(id)?
        } else {
            self.scan_temporaries(id, true, false, None)?
        };
        Ok(summary.made)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        lowir::{builder::FunctionBuilder, model::Program, vocabulary::Type},
        support::stack::StackBudget,
    };

    pub(crate) fn assert_constructor_parameter_temporaries(
        graph: &crate::sema::Graph,
        tree: &ast::Tree,
        cursor: &crate::syntax::cursor::Cursor<'_>,
        declarations: crate::syntax::storage::List<ast::DeclId>,
    ) {
        let count = tree.expressions.mark();
        let ast::DeclKind::Function {
            declarator, body, ..
        } = tree
            .declarations
            .get(*tree.decl_lists.get(declarations).last().unwrap())
            .kind
        else {
            panic!("function");
        };
        let entity = graph.lower_declared(declarator).unwrap();
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("parsed body");
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("compound");
        };
        let statement = tree.stmt_lists.get(statements)[0];
        let statement = if matches!(
            tree.statements.get(statement).kind,
            ast::StmtKind::Ambiguous { .. }
        ) {
            graph.lower_statement(statement).unwrap()
        } else {
            statement
        };
        let ast::StmtKind::Expression(Some(root)) = tree.statements.get(statement).kind else {
            panic!("constructor expression");
        };
        let mut program = Program::default();
        let mut table = super::super::symbols::SymbolTable::default();
        let mut symbols = super::super::symbols::Symbols::new(
            graph,
            tree,
            cursor,
            StackBudget::default(),
            &mut table,
        );
        let mut builder = FunctionBuilder::new(&mut program, "f", &[], Type::Void).unwrap();
        builder.block("entry").unwrap();
        let mut function = Function::new(
            graph,
            tree,
            builder,
            &mut symbols,
            Some(entity),
            None,
            StackBudget::default(),
        );
        assert!(
            !function
                .temporary_type_destroyed(graph.lower_expression(root).unwrap().ty)
                .unwrap()
        );
        for summary in [
            function.operand_temporaries(root).unwrap(),
            function.expression_temporaries(root).unwrap(),
        ] {
            assert!(summary.made);
            assert!(summary.caller_destroyed);
            assert!(!summary.call_result.present());
        }
        assert!(function.lifetime.temporary_work.is_empty());
        assert!(function.lifetime.cleanups.is_empty());
        assert!(!function.lifetime.full_expression);
        assert_eq!(tree.expressions.mark(), count);
    }
}
