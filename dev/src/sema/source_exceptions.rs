// Translated from sema/analyzer_handlers.cpp and analyzer_bodies.cpp. See NOTICE.
//! Clause 15 retains exception object types and handler variables on original syntax IDs.
use super::{
    Error,
    analyzer::Analyzer,
    construction::ConstructorId,
    copy_initialization::{CopyOrigin, CopyUse},
    declarators::DeclaratorInfo,
    expression_facts::{ExprFacts, ValueCategory},
    graph::{Binding, BindingId, BindingKind, EntityId, ScopeId},
    types::{RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    support::source::SourceOffset,
    syntax::{
        ast::{self, ExprKind},
        storage::List,
    },
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct SourceExceptions {
    // Written order and repetitions are observable in host exception filters.
    // Keep these independently of the sorted sets used for compatibility.
    specifications: HashMap<EntityId, Vec<TypeId>>,
    thrown: HashMap<ast::ExprId, TypeId>,
    // Handler bodies have stable syntax identities, including unnamed declarations.
    handlers: HashMap<ast::StmtId, (TypeId, Option<BindingId>)>,
    parameters: HashSet<EntityId>,
}

impl SourceExceptions {
    pub(super) fn specification(&self, entity: EntityId) -> Option<&[TypeId]> {
        self.specifications.get(&entity).map(Vec::as_slice)
    }

    pub(super) fn record_specification(&mut self, entity: EntityId, types: Vec<TypeId>) {
        // Redeclarations check compatibility with the first declaration;
        // their spelling does not replace its emitted specification.
        self.specifications.entry(entity).or_insert(types);
    }

    pub(super) fn inherit_specification(&mut self, entity: EntityId, origin: EntityId) {
        if let Some(types) = self.specifications.get(&origin).cloned() {
            self.record_specification(entity, types);
        }
    }
}

impl super::Graph {
    pub(crate) fn lower_throw_specification(&self, entity: EntityId) -> Option<&[TypeId]> {
        self.source_exceptions.specification(entity)
    }
}

impl Analyzer<'_> {
    pub(super) fn throw_operand(
        &mut self,
        operand: ast::ExprId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        self.typed_expression(operand, scope)?;
        self.check_resolved_function(operand)?;
        let facts = self.graph.expression_facts.get(operand)?;
        if facts.functions.is_some() || facts.object.is_some() && facts.function.is_some() {
            return Err(Error::at(location, "a throw operand that is not a value"));
        }
        let mut ty = facts.ty;
        match self.graph.types.get(ty).kind {
            TypeKind::Scalar(Scalar::Void) => {
                return Err(Error::at(location, "a throw operand of type void (15.1p3)"));
            }
            TypeKind::Array { element, .. } => ty = self.graph.types.pointer(element)?,
            TypeKind::Function { .. } => ty = self.graph.types.pointer(ty)?,
            _ => (),
        }
        ty = self.bare_type(ty)?;
        if let TypeKind::Class(class) = self.graph.types.get(ty).kind {
            if !self.complete_class(class)? || self.graph.entity(class).scope.is_none() {
                return Err(Error::at(
                    location,
                    "a throw operand of incomplete class type (15.1p3)",
                ));
            }
            self.check_abstract_object(ty, location)?;
            self.class_parameter_destruction(ty, scope, location)?;
            let implicit_move = self.throw_names_movable_object(operand)?;
            self.record_class_copy(
                operand,
                ty,
                scope,
                CopyUse::Return { implicit_move },
                location,
            )?;
        } else {
            if let TypeKind::Pointer(element) = self.graph.types.get(ty).kind {
                self.complete_throw_pointee(element, location)?;
            }
            self.convert_scalar(operand, ty, false)?;
        }
        self.prepare_type_information(ty)?;
        if self.lowering() && self.unevaluated == 0 {
            self.request_object_layout(ty, location);
        }
        self.graph.source_exceptions.thrown.insert(operand, ty);
        Ok(ty)
    }

    fn complete_throw_pointee(
        &mut self,
        element: TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if matches!(
            self.graph.types.get(element).kind,
            TypeKind::Scalar(Scalar::Void) | TypeKind::Function { .. }
        ) {
            return Ok(());
        }
        self.complete_pointee(element, location)
    }

    fn throw_names_movable_object(&self, operand: ast::ExprId) -> Result<bool, Error> {
        if !self.names_automatic_object(operand)? {
            return Ok(false);
        }
        let root = self.graph.expression_facts.root(operand);
        let ExprKind::Id(name) = self.tree.expressions.get(root).kind else {
            return Ok(false);
        };
        let binding = self
            .graph
            .resolved_name(name)
            .ok_or(Error::Invariant("throw object without binding"))?;
        let binding = self.graph.binding(binding);
        if binding.kind != BindingKind::Variable
            || self
                .graph
                .source_exceptions
                .parameters
                .contains(&binding.entity)
        {
            return Ok(false);
        }
        let home = self
            .graph
            .entity(binding.entity)
            .home
            .ok_or(Error::Invariant("throw variable without home"))?;
        Ok(self
            .body_context
            .as_ref()
            .and_then(|body| body.try_parent)
            .is_none_or(|parent| parent != home && self.graph.encloses(parent, home)))
    }

    pub(super) fn throw_expression(
        &mut self,
        id: ast::ExprId,
        operand: Option<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        if let Some(operand) = operand {
            self.throw_operand(operand, scope, self.tree.expressions.get(id).location)?;
        }
        let ty = self.graph.types.scalar(Scalar::Void)?;
        self.new_expression_facts(ty, ValueCategory::Prvalue)
    }

    pub(super) fn try_statement(
        &mut self,
        body: ast::StmtId,
        handlers: List<ast::Handler>,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let saved = self.body_context.as_ref().and_then(|body| body.try_parent);
        if let Some(context) = self.body_context.as_mut() {
            context.try_parent = Some(scope);
            context.handled = context
                .handled
                .checked_add(1)
                .ok_or(Error::Limit("exception context limit"))?;
        }
        let result = (|| {
            let ast::StmtKind::Compound(list) = self.tree.statements.get(body).kind else {
                return Err(Error::Invariant("try body is not compound"));
            };
            let result = self.compound_region(list, scope, false, true);
            if let Some(context) = self.body_context.as_mut() {
                context.try_parent = saved;
            }
            result?;
            self.handlers(handlers, scope)
        })();
        if let Some(context) = self.body_context.as_mut() {
            context.try_parent = saved;
            context.handled -= 1;
        }
        result
    }

    pub(super) fn exception_declaration(
        &mut self,
        body: ast::StmtId,
        info: &DeclaratorInfo,
        declarator: Option<ast::DeclaratorId>,
        block: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let mut ty = info.ty;
        match self.graph.types.get(ty).kind {
            TypeKind::Reference {
                kind: RefKind::Rvalue,
                ..
            } => {
                return Err(Error::at(
                    location,
                    "a handler of rvalue reference type (15.3p1)",
                ));
            }
            TypeKind::Array { element, .. } => ty = self.graph.types.pointer(element)?,
            TypeKind::Function { .. } => ty = self.graph.types.pointer(ty)?,
            _ => (),
        }
        let reference = matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. });
        let object = if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            element
        } else {
            ty
        };
        if let TypeKind::Pointer(pointee) = self.graph.types.get(object).kind
            && let Some(class) = self.graph.class_of(pointee)
            && !self.complete_class(class)?
        {
            return Err(Error::at(
                location,
                "a handler of pointer to incomplete type (15.3p1)",
            ));
        }
        let class = self.graph.class_of(object);
        if let Some(class) = class {
            if !self.complete_class(class)? {
                return Err(Error::at(
                    location,
                    "a handler of incomplete class type (15.3p1)",
                ));
            }
            if !reference {
                self.check_abstract_object(object, location)?;
            }
        }
        let mut binding = None;
        if info.name.is_some() || class.is_some() && !reference {
            let entity = self.graph.add_entity(Some(ty), info.name, Keyword::Int)?;
            self.graph.entity_mut(entity).defined = true;
            let declared = self.graph.add_binding(
                block,
                Some(block),
                Binding {
                    kind: BindingKind::Variable,
                    name: info.name,
                    entity,
                    ty: Some(ty),
                    key: Keyword::Int,
                },
            )?;
            self.graph.source_exceptions.parameters.insert(entity);
            if let Some(declarator) = declarator {
                super::record(
                    &mut self.graph.declared_bindings,
                    declarator.index(),
                    declared,
                );
            }
            if let Some(class) = class.filter(|_| !reference) {
                let constructor = self.handler_copy(class, object, block, location)?;
                self.graph.entity_mut(entity).constructor = Some(constructor);
                self.class_parameter_destruction(object, block, location)?;
            }
            binding = Some(declared);
        }
        self.prepare_type_information(object)?;
        self.graph
            .source_exceptions
            .handlers
            .insert(body, (ty, binding));
        Ok(())
    }

    fn handler_copy(
        &mut self,
        class: EntityId,
        ty: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<ConstructorId, Error> {
        let source = ExprFacts::new(ty, ValueCategory::Lvalue);
        let selected = self.select_subobject_copy(class, source, true);
        let selected = match selected {
            Ok(selected) if !self.selected_copy_properties(class, selected)?.deleted => selected,
            Ok(_) | Err(Error::Diagnostic { .. }) => {
                return Err(Error::at(
                    location,
                    "a handler's class object without an accessible copy constructor (15.3p16)",
                ));
            }
            Err(error) => return Err(error),
        };
        if let CopyOrigin::Declared(binding) = selected.origin {
            self.check_special_member_access(binding, class, scope, location)?;
        }
        let constructor = self.copy_constructor_plan(class, selected, location)?;
        self.demand_constructor_use(constructor);
        Ok(constructor)
    }
}

impl super::Graph {
    pub(crate) fn lower_handler(
        &self,
        body: ast::StmtId,
    ) -> Result<(TypeId, Option<EntityId>), Error> {
        let &(ty, binding) = self
            .source_exceptions
            .handlers
            .get(&body)
            .ok_or(Error::Invariant("handler without retained declaration"))?;
        Ok((ty, binding.map(|id| self.binding(id).entity)))
    }

    pub(crate) fn lower_thrown_type(&self, operand: ast::ExprId) -> Result<TypeId, Error> {
        self.source_exceptions
            .thrown
            .get(&operand)
            .copied()
            .ok_or(Error::Invariant(
                "throw operand without retained exception type",
            ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn dynamic_specifications_keep_order_and_empty_origin_without_reparsing() {
        let source = SourceBuffer::new(
            b"void first()throw(double,int,double);void first()throw(int,double);\
              void empty()throw();void nonthrowing()noexcept;\
              template<class T>void unused()throw(T){T::missing();}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"dynamic-specifications").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let bodies = tree.bodies.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        analyzer.finish_unit_exceptions().unwrap();
        let mut entity = |name: &[u8]| {
            let name = analyzer.identifiers.find(name).unwrap();
            let binding = analyzer
                .unqualified(
                    analyzer.graph.global,
                    name,
                    super::super::lookup::Filter::Ordinary,
                )
                .unwrap()
                .unwrap();
            analyzer.graph.binding(binding).entity
        };
        let first = entity(b"first");
        let empty = entity(b"empty");
        let nonthrowing = entity(b"nonthrowing");
        let unused = entity(b"unused");
        let types = analyzer.graph.lower_throw_specification(first).unwrap();
        assert_eq!(types.len(), 3);
        assert_eq!(types[0], types[2]);
        assert!(matches!(
            analyzer.graph.types.get(types[0]).kind,
            TypeKind::Scalar(Scalar::Basic(crate::types::FundamentalType::Double))
        ));
        assert!(matches!(
            analyzer.graph.types.get(types[1]).kind,
            TypeKind::Scalar(Scalar::Basic(crate::types::FundamentalType::Int))
        ));
        assert_eq!(
            analyzer.graph.lower_throw_specification(empty),
            Some([].as_slice())
        );
        assert!(
            analyzer
                .graph
                .lower_throw_specification(nonthrowing)
                .is_none()
        );
        assert!(!analyzer.graph.function_templates.body_demanded(unused));
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert_eq!(analyzer.tree.bodies.mark(), bodies);
    }

    #[test]
    fn throw_moves_only_eligible_local_names_and_retains_handler_ownership() {
        let source = SourceBuffer::new(
            b"struct A{A();A(const A&);A(A&&);~A();};\
            void f(A p){A outer;try{A inner;throw inner;}catch(A caught){throw caught;}\
            try{throw outer;}catch(A&ref){throw ref;}throw p;throw outer;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"throw-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let bodies = tree.bodies.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        let mut thrown = analyzer
            .graph
            .source_exceptions
            .thrown
            .keys()
            .copied()
            .collect::<Vec<_>>();
        thrown.sort_by_key(|&id| analyzer.tree.expressions.get(id).location);
        let kinds = thrown
            .iter()
            .map(|&id| {
                let selected = analyzer
                    .graph
                    .expression_facts
                    .get(id)
                    .unwrap()
                    .class_copy
                    .unwrap();
                let CopyOrigin::Declared(binding) = selected.origin else {
                    panic!("declared copy");
                };
                analyzer
                    .graph
                    .entity(analyzer.graph.binding(binding).entity)
                    .copy_kind
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            [
                RefKind::Rvalue,
                RefKind::Lvalue,
                RefKind::Lvalue,
                RefKind::Lvalue,
                RefKind::Lvalue,
                RefKind::Rvalue
            ]
        );
        assert_eq!(analyzer.graph.source_exceptions.handlers.len(), 2);
        for &(ty, binding) in analyzer.graph.source_exceptions.handlers.values() {
            let binding = binding.unwrap();
            let entity = analyzer.graph.binding(binding).entity;
            assert_eq!(analyzer.graph.entity(entity).ty, Some(ty));
            assert!(
                analyzer
                    .graph
                    .source_exceptions
                    .parameters
                    .contains(&entity)
            );
            let home = analyzer.graph.entity(entity).home.unwrap();
            assert!(analyzer.graph.scope(home).entry == super::super::graph::ScopeEntry::Exception);
            assert_eq!(
                analyzer.graph.entity(entity).constructor.is_some(),
                !matches!(
                    analyzer.graph.types.get(ty).kind,
                    TypeKind::Reference { .. }
                )
            );
        }
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert_eq!(analyzer.tree.bodies.mark(), bodies);
        assert!(analyzer.body_context.is_none());
    }

    #[test]
    fn exception_adjustments_are_retained_without_rewriting_syntax() {
        let source = SourceBuffer::new(
            b"void g();void f(){throw g;throw \"x\";throw 1;\
            try{}catch(int a[3]){}catch(void(*p)()){}catch(const int&n){}}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"exception-adjustment").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        let mut thrown = analyzer
            .graph
            .source_exceptions
            .thrown
            .iter()
            .map(|(&id, &ty)| (id, ty))
            .collect::<Vec<_>>();
        thrown.sort_by_key(|&(id, _)| analyzer.tree.expressions.get(id).location);
        assert_eq!(thrown.len(), 3);
        for &(id, ty) in &thrown {
            assert_eq!(
                analyzer
                    .graph
                    .expression_facts
                    .get(id)
                    .unwrap()
                    .conversion
                    .unwrap()
                    .result,
                ty
            );
        }
        let TypeKind::Pointer(function) = analyzer.graph.types.get(thrown[0].1).kind else {
            panic!("function decay");
        };
        assert!(matches!(
            analyzer.graph.types.get(function).kind,
            TypeKind::Function { .. }
        ));
        let TypeKind::Pointer(character) = analyzer.graph.types.get(thrown[1].1).kind else {
            panic!("array decay");
        };
        assert_eq!(
            analyzer.graph.types.get(character).cv,
            super::super::types::Cv::CONST
        );
        assert!(matches!(
            analyzer.graph.types.get(thrown[2].1).kind,
            TypeKind::Scalar(_)
        ));
        let mut handlers = analyzer
            .graph
            .source_exceptions
            .handlers
            .iter()
            .map(|(&id, &(ty, _))| (id, ty))
            .collect::<Vec<_>>();
        handlers.sort_by_key(|&(id, _)| analyzer.tree.statements.get(id).location);
        assert!(matches!(
            analyzer.graph.types.get(handlers[0].1).kind,
            TypeKind::Pointer(_)
        ));
        assert!(matches!(
            analyzer.graph.types.get(handlers[1].1).kind,
            TypeKind::Pointer(_)
        ));
        assert!(matches!(
            analyzer.graph.types.get(handlers[2].1).kind,
            TypeKind::Reference { .. }
        ));
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
    }

    #[test]
    fn failed_try_analysis_restores_function_context() {
        let source =
            SourceBuffer::new(b"void g();void f(){try{throw g();}catch(...){}}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"throw-rollback").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        assert!(
            matches!(analyzer.declarations(declarations, analyzer.graph.global),
            Err(Error::Diagnostic { message, .. }) if message == "a throw operand of type void (15.1p3)")
        );
        assert!(analyzer.body_context.is_none());
        assert!(analyzer.graph.source_exceptions.thrown.is_empty());
    }
}
