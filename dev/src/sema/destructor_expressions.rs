// Translated from analyzer_expressions.cpp::destructor_access and
// analyzer_calls.cpp's bound destructor call. See NOTICE.
//! Destructor designation and invocation retain the original object and selected type.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{DestructorTarget, ExprFacts, ObjectArgument, ValueCategory},
    function_resolution::CallTarget,
    graph::{BindingKind, ScopeId},
    lookup::Filter,
    types::{Cv, Scalar, TypeKind},
};
use crate::syntax::{ast, storage::List};

impl Analyzer<'_> {
    pub(super) fn destructor_designation(
        &mut self,
        id: ast::ExprId,
        object: ObjectArgument,
        name: ast::Name,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let location = self.tree.expressions.get(id).location;
        let node = *self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        let last = *components
            .last()
            .ok_or(Error::Invariant("empty destructor name"))?;
        let ast::NameKind::Destructor(key) = last.kind else {
            return Err(Error::at(
                location,
                "a destructor name outside the supported forms",
            ));
        };
        let qualified = node.global || components.len() > 1;
        let object_class = self.graph.class_of(object.ty);
        if let Some(class) = object_class {
            self.complete_class(class)?;
        }
        let complete = object_class.filter(|&class| {
            self.graph.entity(class).complete && self.graph.entity(class).scope.is_some()
        });
        if qualified && complete.is_none() {
            return Err(Error::at(
                location,
                "a qualified destructor name for an object of no complete class type",
            ));
        }
        let naming = if let Some(class) = complete {
            let search = self.data_member_qualifier(name, class, scope)?;
            self.graph.scope(search).owner
        } else {
            object_class
        };
        self.lookup_location = location;
        let found = if let Some(home) = complete
            .and(naming)
            .and_then(|class| self.graph.entity(class).scope)
        {
            self.qualified_declaration_result(home, key.into(), Filter::Elaborated)?
                .binding
        } else {
            None
        };
        let mut binding = if let Some(binding) = found {
            Some(binding)
        } else {
            self.unqualified_declaration_result(scope, key.into(), Filter::Elaborated)?
                .binding
        }
        .filter(|&binding| {
            matches!(
                self.graph.binding(binding).kind,
                BindingKind::Type | BindingKind::Alias
            )
        })
        .ok_or(Error::at(
            location,
            "the destructor name does not name a type",
        ))?;
        if last.arguments.is_some() {
            binding = self
                .class_template_id(binding, last, scope)?
                .ok_or_else(|| {
                    Error::at(
                        location,
                        "the destructor name's template-id names no class template",
                    )
                })?;
        }
        let named_type = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("destructor type name without type"))?;
        let target = if qualified {
            self.graph.lower_entity_type(
                naming.ok_or(Error::Invariant("qualified destructor without class"))?,
            )?
        } else {
            object.ty
        };
        if self.bare_type(named_type)? != self.bare_type(target)? {
            return Err(Error::at(
                location,
                "the destructor name does not name the object's type",
            ));
        }
        let void = self.graph.types.scalar(Scalar::Void)?;
        let target = if let Some(class) = naming {
            if !self.graph.entity(class).complete {
                return Err(Error::at(location, "a class without a destructor"));
            }
            let binding = self.explicit_destructor_binding(class)?;
            self.check_function_not_deleted(self.graph.binding(binding).entity, location)?;
            self.check_special_use_access(
                binding,
                object_class.ok_or(Error::Invariant("class destructor without object class"))?,
                scope,
                location,
            )?;
            DestructorTarget::Class(binding)
        } else {
            DestructorTarget::Scalar
        };
        let ty = match target {
            DestructorTarget::Class(binding) => self.graph.types.canonical_function(
                self.graph
                    .binding(binding)
                    .ty
                    .ok_or(Error::Invariant("destructor without type"))?,
            )?,
            DestructorTarget::Scalar => self.graph.types.intern(
                TypeKind::Function {
                    result: void,
                    parameters: Vec::new(),
                    variadic: false,
                    cv: Cv::default(),
                    reference: None,
                },
                Cv::default(),
            )?,
        };
        let mut facts = self.new_expression_facts(ty, ValueCategory::Prvalue)?;
        facts.set_destructor(Some(target), &mut self.graph.expression_facts)?;
        if let DestructorTarget::Class(binding) = target {
            facts.set_dispatch(
                if !qualified
                    && self
                        .graph
                        .lower_virtual_slot(self.graph.binding(binding).entity)
                        .is_some()
                {
                    super::expression_facts::Dispatch::Virtual
                } else {
                    super::expression_facts::Dispatch::Direct
                },
            );
        }
        facts.set_object(Some(object), &mut self.graph.expression_facts)?;
        facts.set_member_naming(naming, &mut self.graph.expression_facts)?;
        if let (Some(object), Some(naming)) = (object_class, naming)
            && object != naming
        {
            if self.lowering() {
                self.request_storage_layout(object, location)?;
            }
            facts.set_member_naming_path(
                Some(self.graph.retain_base_path(object, naming)?),
                &mut self.graph.expression_facts,
            )?;
        }
        Ok(facts)
    }

    fn explicit_destructor_binding(
        &mut self,
        class: super::graph::EntityId,
    ) -> Result<super::graph::BindingId, Error> {
        self.prepare_destruction(class)?;
        if let Some(binding) = self
            .destructor_callable(class)?
            .or(self.graph.entity(class).implicit_destructor)
        {
            return Ok(binding);
        }
        let declaration = self
            .graph
            .entity(class)
            .destruction
            .ok_or(Error::Invariant(
                "explicit destructor without completed declaration",
            ))?
            .declaration;
        self.graph.bind_implicit_destructor(class, declaration)
    }

    pub(super) fn destructor_call(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        target: DestructorTarget,
    ) -> Result<ExprFacts, Error> {
        if !self.tree.expr_lists.get(arguments).is_empty() {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "a destructor call with arguments",
            ));
        }
        let void = self.graph.types.scalar(Scalar::Void)?;
        let mut facts = self.new_expression_facts(void, ValueCategory::Prvalue)?;
        facts.set_destructor(Some(target), &mut self.graph.expression_facts)?;
        if let DestructorTarget::Class(binding) = target {
            facts.set_call(
                Some(CallTarget::Direct(
                    self.graph
                        .function_origin(self.graph.binding(binding).entity)?,
                )),
                &mut self.graph.expression_facts,
            )?;
            self.graph.record_call_binding(id, binding);
            self.demand_function(binding, true)?;
        }
        Ok(facts)
    }
}

#[cfg(test)]
mod tests {
    use super::{Analyzer, DestructorTarget};
    use crate::{
        preprocess::unit::Preprocessor,
        sema::lookup::Filter,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{SyntaxUnit, ast},
    };

    #[test]
    fn destructor_designators_complete_one_class_and_reuse_its_selected_member() {
        let source = SourceBuffer::new(b"template<class T>struct A{T n;~A(){}void unused(){typename T::missing x;}};void f(const A<int>*p){p->~A<int>();p->~A();}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"template-destructor").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(*tree.decl_lists.get(declarations).last().unwrap())
            .kind
        else {
            panic!("function");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("body");
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("compound");
        };
        let mut selected = None;
        for &statement in tree.stmt_lists.get(statements) {
            let ast::StmtKind::Expression(Some(call)) = tree.statements.get(statement).kind else {
                panic!("statement");
            };
            let ast::ExprKind::Call { callee, .. } = tree.expressions.get(call).kind else {
                panic!("call");
            };
            let facts = graph.expression_facts.get(callee).unwrap();
            let Some(DestructorTarget::Class(binding)) = facts.destructor(&graph.expression_facts)
            else {
                panic!("class destructor");
            };
            let object = facts.object(&graph.expression_facts).unwrap();
            let class = graph.class_of(object.ty).unwrap();
            assert!(graph.entity(class).complete);
            assert_eq!(graph.types.get(object.ty).cv, super::Cv::CONST);
            assert_eq!(graph.entity(class).declared_destructor, Some(binding));
            assert_eq!(graph.call_binding(call).unwrap(), binding);
            assert_eq!(facts.member_naming(&graph.expression_facts), Some(class));
            if let Some(previous) = selected {
                assert_eq!(binding, previous);
            }
            selected = Some(binding);
            let unused = cursor.intern_name(b"unused").unwrap();
            let unused = graph
                .binding(
                    graph
                        .own(graph.entity(class).scope.unwrap(), unused, Filter::Ordinary)
                        .unwrap(),
                )
                .entity;
            assert!(graph.function_templates.bodies.pending_member(unused));
            assert!(graph.function_templates.bodies.body(unused).is_none());
        }
    }
}
