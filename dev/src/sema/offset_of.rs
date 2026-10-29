// Translated from sema/analyzer_traits.cpp::offsetof_value/member_offset. See NOTICE.
//! A parsed designator uses ordinary member/subscript facts and demand-only layout.
use super::{
    Error,
    analyzer::Analyzer,
    constants::Constant,
    expression_facts::{ExprFacts, ValueCategory},
    graph::{BindingKind, EntityId, ScopeId},
    types::Scalar,
};
use crate::{
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind},
    types::FundamentalType,
};

impl Analyzer<'_> {
    pub(super) fn offsetof_expression(
        &mut self,
        id: ast::ExprId,
        designator: ast::ExprId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let offset = self.offsetof_value(designator, scope)?;
        let ty = self
            .graph
            .types
            .scalar(Scalar::Basic(FundamentalType::UnsignedLongInt))?;
        let mut facts = self.new_expression_facts(ty, ValueCategory::Prvalue)?;
        facts.set_folded(Some(offset));
        // Types-mode noexcept still types its operand before constant demand.
        // Retain that value as the frozen typed_value/converted_constant path does.
        super::record(
            &mut self.graph.constant_facts,
            id.index(),
            Constant {
                state: crate::sema::constant_addresses::ScalarState::Value,
                high: 0,
                bits: offset,
                ty,
            },
        );
        Ok(facts)
    }

    pub(super) fn offsetof_constant(
        &mut self,
        ty: ast::TypeId,
        designator: ast::ExprId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Constant, Error> {
        let ty = self.type_syntax(ty, scope)?;
        if self.graph.dependent_type(ty) || !self.semantic() || self.template_depth != 0 {
            return Err(Error::NonConstant {
                failure: super::ConstantFailure::Reported,
                location,
                message: "a type trait of a dependent type",
            });
        }
        let bits = self.offsetof_value(designator, scope)?;
        Ok(Constant {
            state: crate::sema::constant_addresses::ScalarState::Value,
            high: 0,
            bits,
            ty: self
                .graph
                .types
                .scalar(Scalar::Basic(FundamentalType::UnsignedLongInt))?,
        })
    }

    fn offsetof_value(&mut self, mut id: ast::ExprId, scope: ScopeId) -> Result<u64, Error> {
        self.typed_expression(id, scope)?;
        let mut offset = 0u64;
        loop {
            let node = *self.tree.expressions.get(id);
            match node.kind {
                ExprKind::OffsetOfBase(_) => return Ok(offset),
                ExprKind::Subscript { base, index } => {
                    // integral_constant uses the ordinary integral evaluator in
                    // types mode, even inside a noexcept specification. It must
                    // not enter the specification adapter's call interpreter.
                    let saved = self.evaluating_noexcept;
                    self.evaluating_noexcept = false;
                    let value = self.constant_value(index, scope, false);
                    self.evaluating_noexcept = saved;
                    let index = value?.ok_or_else(|| {
                        Error::at(node.location, "an offsetof subscript that is no constant")
                    })?;
                    let ty = self.graph.expression_facts.get(id)?.ty;
                    let (size, _) = self.type_size(ty, node.location)?;
                    offset = offset.wrapping_add(index.bits.wrapping_mul(size));
                    id = base;
                }
                ExprKind::Member { base, .. } => {
                    let facts = self.graph.expression_facts.get(id)?;
                    let class = self
                        .graph
                        .class_of(self.graph.expression_facts.get(base)?.ty);
                    let member = facts
                        .member(&self.graph.expression_facts)
                        .filter(|&binding| {
                            self.graph.binding(binding).kind == BindingKind::Variable
                        })
                        .map(|binding| self.graph.binding(binding).entity);
                    let bytes = if let (Some(class), Some(member)) = (class, member) {
                        self.member_offset(class, member, node.location)?
                    } else {
                        None
                    };
                    offset = offset.wrapping_add(bytes.ok_or_else(|| {
                        Error::at(
                            node.location,
                            "an offsetof designator naming no data member",
                        )
                    })?);
                    id = base;
                }
                _ => return Err(Error::Invariant("invalid offsetof designator")),
            }
        }
    }

    fn member_offset(
        &mut self,
        class: EntityId,
        member: EntityId,
        location: SourceOffset,
    ) -> Result<Option<u64>, Error> {
        self.check_stack()?;
        self.class_layout(class, location)?;
        for index in 0..self.graph.entity(class).class().members.len() {
            let data = self.graph.entity(class).class().members[index];
            let entity = self.graph.entity(data);
            if data == member {
                if entity.object().bit_field.is_some() {
                    return Err(Error::at(location, "offsetof of a bit-field"));
                }
                return Ok(entity.object().member_offset);
            }
            // An anonymous union/struct's object owns its injected members.
            let anonymous = entity
                .name
                .is_none()
                .then_some(entity.ty)
                .flatten()
                .and_then(|ty| self.graph.class_of(ty));
            if let Some(anonymous) = anonymous
                && let Some(inner) = self.member_offset(anonymous, member, location)?
            {
                return Ok(Some(
                    self.graph.lower_member_offset(data)?.wrapping_add(inner),
                ));
            }
        }
        for index in 0..self.graph.entity(class).class().bases.len() {
            let base = self.graph.entity(class).class().bases[index];
            if !base.is_virtual
                && let Some(inner) = self.member_offset(base.class, member, location)?
            {
                return Ok(Some(
                    base.offset
                        .ok_or(Error::Invariant("unlaid offsetof base"))?
                        .wrapping_add(inner),
                ));
            }
        }
        Ok(None)
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
    fn offset_queries_reuse_layout_and_fold_without_emitting_or_rewriting_designators() {
        let source = SourceBuffer::new(
            b"struct C{char c;int x[4];template<class T>void unused(){T::missing();}};\
              static_assert(__builtin_offsetof(C,x[2])==12,\"element\");\
              static_assert(__builtin_offsetof(C,x[-2])==18446744073709551612UL,\"wrap\");"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"offset-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let name = analyzer.identifiers.intern(b"C").unwrap();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        let binding = graph
            .own(graph.global, name, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let class = graph.binding(binding).entity;
        assert!(matches!(
            graph.entity(class).class().layout,
            super::super::layout::LayoutState::Value {
                size: 20,
                alignment: 4,
                ..
            }
        ));
        assert!(graph.storage_layouts.is_empty());
        assert!(graph.root_demands.as_slice().is_empty());
        assert!(super::super::function_demands::trailing(&graph).is_empty());
        for (&declaration, expected) in tree.decl_lists.get(declarations)[1..]
            .iter()
            .zip([12, u64::MAX - 3])
        {
            let ast::DeclKind::StaticAssert { condition, .. } =
                tree.declarations.get(declaration).kind
            else {
                panic!("assertion");
            };
            let ExprKind::Binary { left, .. } = tree.expressions.get(condition).kind else {
                panic!("comparison");
            };
            let facts = graph.expression_facts.get(left).unwrap();
            assert_eq!(facts.folded(), Some(expected));
            assert_eq!(facts.category, ValueCategory::Prvalue);
            assert!(matches!(
                graph.types.get(facts.ty).kind,
                super::super::types::TypeKind::Scalar(Scalar::Basic(
                    FundamentalType::UnsignedLongInt
                ))
            ));
            let ExprKind::OffsetOf { designator, .. } = tree.expressions.get(left).kind else {
                panic!("query");
            };
            let ExprKind::Subscript { base, .. } = tree.expressions.get(designator).kind else {
                panic!("subscript");
            };
            let ExprKind::Member { base, .. } = tree.expressions.get(base).kind else {
                panic!("member");
            };
            assert!(matches!(
                tree.expressions.get(base).kind,
                ExprKind::OffsetOfBase(_)
            ));
            assert_eq!(
                graph.expression_facts.get(base).unwrap().category,
                ValueCategory::Lvalue
            );
        }
    }
    #[test]
    fn types_mode_noexcept_retains_typed_offset_values() {
        let source = SourceBuffer::new(
            b"struct C{char c;int x[4];};\
              template<class T>void f()noexcept(__builtin_offsetof(C,x[2])==12);\
              int g()noexcept(sizeof((__builtin_offsetof(C,x[2])))==8);"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"offset-types").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let name = analyzer.identifiers.intern(b"g").unwrap();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        let binding = graph
            .own(graph.global, name, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let entity = graph.binding(binding).entity;
        assert!(matches!(
            graph.entity(entity).callable().exceptions,
            super::super::exceptions::ExceptionState::Value(
                super::super::exceptions::ExceptionSet::None
            )
        ));
        assert!(graph.storage_layouts.is_empty());
        assert!(graph.root_demands.as_slice().is_empty());
    }
    #[test]
    fn noexcept_specifications_demand_calls_but_query_operands_do_not() {
        for expression in [
            "index()==2",
            "__builtin_offsetof(C,a[index()])==8",
            "noexcept(index())",
        ] {
            let source = SourceBuffer::new(format!(
                "constexpr int index(){{return 2;}}struct C{{int a[4];}};void f()noexcept({expression});"
            ).into_bytes()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"offset-specification").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_semantics(true);
            let name = analyzer.identifiers.intern(b"index").unwrap();
            let graph = analyzer.analyze(declarations).unwrap();
            let binding = graph
                .own(graph.global, name, super::super::lookup::Filter::Ordinary)
                .unwrap();
            let demanded = super::super::function_demands::trailing(&graph).iter().any(|demand| {
                matches!(demand, super::super::function_demands::Demand::Function { binding: used, .. } if graph.binding(*used).entity == graph.binding(binding).entity)
            });
            assert_eq!(demanded, expression != "noexcept(index())");
        }
    }
}
