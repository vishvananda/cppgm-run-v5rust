// Translated from lower/expression_lowering.cpp::bind_extended/extension_path.
// See NOTICE.
//! A reference's original expression selects temporary storage; its declaring object owns cleanup.
use super::{Error, conversions::Context, function::Function};
use crate::{
    lex::pp_lexer::Punctuator as P,
    lowir::model::Value,
    sema::lowering::{EntityId, TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};

#[derive(Default)]
pub(super) struct Extensions {
    pub aggregate: Option<EntityId>,
    pub static_object: bool,
    pub list_owner: Option<EntityId>,
    owner: Option<EntityId>,
    pub root: Option<ast::ExprId>,
    path: Vec<ast::ExprId>,
}
impl Extensions {
    pub(super) fn has_scope(&self) -> bool {
        self.owner.is_some()
    }
}

impl Function<'_, '_, '_, '_> {
    pub(super) fn persistent_alive(&mut self, ty: TypeId, storage: Value) -> Result<(), Error> {
        let Value::Symbol(global) = storage else {
            return Ok(());
        };
        // A static reference registers its successfully initialized temporaries
        // together. A static object's reference-member temporary registers as
        // soon as it completes, before later members can throw.
        if self
            .static_temporaries
            .as_ref()
            .is_some_and(|local| !local.reference)
        {
            return self.register_static_destruction(global, ty);
        }
        if let Some(flag) = self.symbols.persistent_finalizer(
            self.builder.program(),
            global,
            ty,
            self.static_temporaries
                .as_mut()
                .map(|local| &mut local.made),
        )? {
            let one = self.builder.integer(1)?;
            self.effect(crate::lowir::model::InstructionKind::Store(
                crate::lowir::model::Facts::default(),
                crate::lowir::vocabulary::Type::I8,
                one,
                Value::Symbol(flag),
            ))?;
        }
        Ok(())
    }
    pub(super) fn extended_temporary_owner(&self, id: ast::ExprId) -> Option<EntityId> {
        self.reference_lifetimes
            .path
            .contains(&self.without_parentheses(id))
            .then_some(self.reference_lifetimes.owner)
            .flatten()
    }
    pub(super) fn persistent_temporary_path(&self, id: ast::ExprId) -> bool {
        self.persistent_references
            && self.reference_lifetimes.owner.is_none()
            && self
                .reference_lifetimes
                .path
                .contains(&self.without_parentheses(id))
    }
    pub(super) fn comma_class_temporary(
        &mut self,
        root: ast::ExprId,
        right: ast::ExprId,
        use_: super::materialization::TemporaryUse,
        persistent: bool,
    ) -> Result<Value, Error> {
        let mark = self.reference_lifetimes.path.len();
        if self.reference_lifetimes.path.contains(&root) {
            self.reference_lifetimes
                .path
                .push(self.without_parentheses(right));
        }
        let result = self.class_temporary(right, use_, persistent);
        self.reference_lifetimes.path.truncate(mark);
        result
    }
    pub(super) fn bind_static_reference(
        &mut self,
        id: ast::ExprId,
        ty: TypeId,
    ) -> Result<Value, Error> {
        let mark = self.reference_lifetimes.path.len();
        self.reference_extension_path(id)?;
        let result = self.bind_reference(id, ty, Context::Initialization);
        self.reference_lifetimes.path.truncate(mark);
        result
    }
    fn conversion_temporary_destroyed(&mut self, mut id: ast::ExprId) -> Result<bool, Error> {
        loop {
            id = self.without_parentheses(id);
            if let Some((call, _)) = self.graph.lower_function_conversion(id)? {
                return Ok(!call.place && self.symbols.class_destructor(call.result)?.is_some());
            }
            if let Some((constructor, _)) = self.class_conversion(id) {
                let ty = self
                    .graph
                    .lower_entity_type(self.graph.lower_constructor(constructor).class)?;
                return Ok(self.symbols.class_destructor(ty)?.is_some());
            }
            if self.graph.lower_expression(id)?.reference_cast.is_some()
                && let ExprKind::Cast { operand, .. } = self.tree.expressions.get(id).kind
                && (self.graph.lower_expression(operand)?.temporary.is_some()
                    || self.class_conversion(operand).is_some()
                    || self.graph.lower_function_conversion(operand)?.is_some())
            {
                id = operand;
            } else {
                return Ok(false);
            }
        }
    }
    pub(super) fn bind_aggregate_reference(
        &mut self,
        id: ast::ExprId,
        ty: TypeId,
    ) -> Result<Value, Error> {
        if let Some(owner) = self.reference_lifetimes.aggregate {
            self.bind_extended_reference(id, ty, owner)
        } else if self.persistent_references || self.reference_lifetimes.static_object {
            self.bind_static_reference(id, ty)
        } else {
            self.bind_reference(id, ty, Context::Initialization)
        }
    }
    pub(super) fn bind_extended_reference(
        &mut self,
        id: ast::ExprId,
        ty: TypeId,
        owner: EntityId,
    ) -> Result<Value, Error> {
        let mark = self.reference_lifetimes.path.len();
        let own = self.conversion_temporary_destroyed(id)?;
        if !own {
            self.reference_extension_path(id)?;
        }
        let saved = self.reference_lifetimes.owner.replace(owner);
        let saved_root = self
            .reference_lifetimes
            .root
            .replace(self.without_parentheses(id));
        let expression = self.lifetime.full_expression;
        self.lifetime.full_expression |= self.expression_throws(id)?;
        let result = self.bind_reference(id, ty, Context::Initialization);
        if result.is_ok() && own {
            self.extend_last_temporary(owner)?;
        }
        self.reference_lifetimes.owner = saved;
        self.reference_lifetimes.root = saved_root;
        self.reference_lifetimes.path.truncate(mark);
        // The declaration closes the region after it stores the bound address.
        // Keep a newly opened full-expression alive through that store.
        self.lifetime.full_expression = expression || self.lifetime.region;
        result
    }
    fn reference_extension_path(&mut self, mut id: ast::ExprId) -> Result<(), Error> {
        self.check_stack()?;
        // A user conversion binds its own result, never the receiver's
        // original prvalue (expression_lowering.cpp::extension_path).
        if self.graph.lower_expression(id)?.temporary.is_some()
            || self.graph.lower_function_conversion(id)?.is_some()
        {
            return Ok(());
        }
        loop {
            id = self.without_parentheses(id);
            let facts = self.graph.lower_expression(id)?;
            if !facts.is_place
                && matches!(
                    self.graph.lower_type(facts.ty)?,
                    TypeView::Class(_) | TypeView::Array { .. }
                )
            {
                self.reference_lifetimes.path.push(id);
                return Ok(());
            }
            let overloaded = self.graph.lower_operator_call(id)?;
            id = match self.tree.expressions.get(id).kind {
                ExprKind::Member { base, op, .. }
                    if op.token.as_punctuator() == Some(P::Dot) && !overloaded =>
                {
                    base
                }
                ExprKind::Subscript { base, index } if !overloaded => {
                    if matches!(
                        self.graph
                            .lower_type(self.graph.lower_expression(base)?.ty)?,
                        TypeView::Array { .. }
                    ) {
                        base
                    } else if matches!(
                        self.graph
                            .lower_type(self.graph.lower_expression(index)?.ty)?,
                        TypeView::Array { .. }
                    ) {
                        index
                    } else {
                        return Ok(());
                    }
                }
                ExprKind::Cast { operand, .. }
                    if facts.reference_cast.is_some()
                        && self.graph.lower_expression(operand)?.temporary.is_none()
                        && self.class_conversion(operand).is_none()
                        && self.graph.lower_function_conversion(operand)?.is_none() =>
                {
                    operand
                }
                ExprKind::Conditional {
                    then_expr,
                    else_expr,
                    ..
                } if facts.is_place => {
                    self.reference_extension_path(then_expr)?;
                    else_expr
                }
                ExprKind::Binary { op, right, .. }
                    if op.token.as_punctuator() == Some(P::Comma) && !overloaded =>
                {
                    right
                }
                _ => return Ok(()),
            };
        }
    }
    pub(super) fn initializer_extends_temporary(
        &mut self,
        initializer: Option<ast::Initializer>,
    ) -> Result<bool, Error> {
        let Some(initializer) = initializer else {
            return Ok(false);
        };
        let mut work = match initializer.kind {
            ast::InitializerKind::Assign(expr) | ast::InitializerKind::Braced(expr) => vec![expr],
            ast::InitializerKind::Parenthesized(args) => self.tree.expr_lists.get(args).to_vec(),
            _ => return Ok(false),
        };
        while let Some(expr) = work.pop() {
            let expr = self.without_parentheses(expr);
            if self.list_object(expr)?.is_some() && self.destroyed_list_array(expr)? {
                return Ok(true);
            }
            if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(expr).kind {
                work.extend(self.tree.expr_lists.get(elements));
            } else if self.graph.lower_reference_binding(expr)? {
                if self.conversion_temporary_destroyed(expr)? {
                    return Ok(true);
                }
                let mark = self.reference_lifetimes.path.len();
                self.reference_extension_path(expr)?;
                let mut destroyed = false;
                for index in mark..self.reference_lifetimes.path.len() {
                    let temporary = self.reference_lifetimes.path[index];
                    destroyed |= self.destroyed_list_array(temporary)?;
                    destroyed |= self
                        .symbols
                        .class_destructor(self.graph.lower_expression(temporary)?.ty)?
                        .is_some();
                }
                self.reference_lifetimes.path.truncate(mark);
                if destroyed {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        lowir::{builder::FunctionBuilder, model::Program, vocabulary::Type},
        support::stack::StackBudget,
        syntax::ast::{Body, DeclKind, StmtKind},
    };

    pub(crate) fn assert_extended_ownership(
        graph: &crate::sema::Graph,
        tree: &ast::Tree,
        cursor: &crate::syntax::cursor::Cursor<'_>,
        declarations: crate::syntax::storage::List<ast::DeclId>,
        per_owner: usize,
    ) {
        let count = tree.expressions.mark();
        let DeclKind::Function {
            declarator, body, ..
        } = tree
            .declarations
            .get(*tree.decl_lists.get(declarations).last().unwrap())
            .kind
        else {
            panic!("function");
        };
        let entity = graph.lower_declared(declarator).unwrap();
        let Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("parsed body");
        };
        let StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("compound");
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
        let mut owners = Vec::new();
        for &statement in tree.stmt_lists.get(statements) {
            let StmtKind::Declaration(declaration) = tree.statements.get(statement).kind else {
                panic!("declaration");
            };
            let DeclKind::Simple { declarators, .. } = tree.declarations.get(declaration).kind
            else {
                panic!("simple declaration");
            };
            let declarator = tree.init_declarators.get(declarators)[0];
            owners.push(graph.lower_declared(declarator.declarator).unwrap());
            function.declaration(declaration).unwrap();
            assert_eq!(function.lifetime.cleanups.len(), owners.len() * per_owner);
            let cleanup = &function.lifetime.cleanups[function.lifetime.cleanups.len() - per_owner];
            assert_eq!(cleanup.variable, owners.last().copied());
            assert!(matches!(
                cleanup.place,
                super::super::lifetime::CleanupPlace::Extended(_)
            ));
            assert!(!cleanup.member());
            assert!(function.reference_lifetimes.path.is_empty());
            assert!(function.reference_lifetimes.owner.is_none());
            assert!(!function.lifetime.full_expression);
        }
        assert_ne!(owners[0], owners[1]);
        function.leave_scope(per_owner).unwrap();
        assert_eq!(function.lifetime.cleanups.len(), per_owner);
        assert_eq!(function.lifetime.cleanups[0].variable, Some(owners[0]));
        function.leave_scope(0).unwrap();
        assert!(function.lifetime.cleanups.is_empty());
        assert_eq!(tree.expressions.mark(), count);
    }
}
