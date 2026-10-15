// Translated from analyzer_conversions.cpp::visible_conversions/user_conversion.
// See NOTICE.
//! Conversion sets preserve path hiding; selection records one call on the original operand.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{
        Conversion, ConversionRank, ExprFacts, ObjectArgument, ObjectSource, ValueCategory,
        ValueConversion,
    },
    graph::{BindingId, EntityId, FunctionAvailability, ScopeId},
    inheritance::BasePathId,
    procedural_calls::{ArgumentSequence, UserConversion},
    specifiers::Explicitness,
    types::{TypeId, TypeKind},
    user_conversions::UserCandidate,
};
use crate::{support::source::SourceOffset, syntax::ast};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ConversionUse {
    Scalar,
    Contextual,
    Builtin,
}

#[derive(Clone, Copy)]
pub(super) enum ConversionTarget {
    Integral,
    IntegralOrEnumeration,
    ObjectPointer,
}

#[derive(Clone, Copy)]
pub(super) struct FunctionConversion {
    pub binding: BindingId,
    pub result: TypeId,
    pub category: ValueCategory,
    pub path: Option<BasePathId>,
    pub after: Conversion,
    pub after_path: Option<BasePathId>,
    pub usage: ConversionUse,
}

impl Analyzer<'_> {
    pub(super) fn unique_conversion_function(
        &mut self,
        class: EntityId,
        target: ConversionTarget,
    ) -> Result<Option<BindingId>, Error> {
        let count = self.visible_conversion_functions(class)?.len();
        let mut found = None;
        for index in 0..count {
            let binding = self.conversion_sets[&class][index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.explicitness == Explicitness::Explicit || entity.template_head.is_some() {
                continue;
            }
            let signature = entity.ty.ok_or(Error::Invariant("conversion signature"))?;
            let result = self.call_result(signature)?.ty;
            let accepts = match target {
                ConversionTarget::Integral => self.is_integral_operand(result),
                ConversionTarget::IntegralOrEnumeration => self.integral_kind(result).is_some(),
                ConversionTarget::ObjectPointer => matches!(self.graph.types.get(result).kind,
                    TypeKind::Pointer(element) if !matches!(self.graph.types.get(element).kind,
                        TypeKind::Function { .. } | TypeKind::Scalar(super::types::Scalar::Void))),
            };
            if accepts {
                if found.is_some() {
                    return Ok(None);
                }
                found = Some(binding);
            }
        }
        Ok(found)
    }

    pub(super) fn convert_unique_class_operand(
        &mut self,
        id: ast::ExprId,
        target: ConversionTarget,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let source = self.graph.expression_facts.get(id)?;
        let Some(class) = self.graph.class_of(source.ty) else {
            return Ok(source);
        };
        let Some(binding) = self.unique_conversion_function(class, target)? else {
            return Ok(source);
        };
        let location = self.tree.expressions.get(id).location;
        // unique_conversion counts declarations, not viable object sequences.
        // convert_class_operand also omits the ordinary deletion check.
        let path = self.conversion_function_receiver(source, binding, scope, location)?;
        let signature = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("conversion signature"))?;
        let produced = self.call_result(signature)?;
        let after = self
            .standard_conversion(produced, produced.ty, false)?
            .ok_or(Error::Invariant("converted operand identity"))?;
        self.graph.expression_facts.set_function_conversion(
            id,
            FunctionConversion {
                binding,
                result: produced.ty,
                category: produced.category,
                path,
                after,
                after_path: None,
                usage: ConversionUse::Builtin,
            },
        );
        Ok(produced)
    }

    pub(super) fn visible_conversion_functions(
        &mut self,
        class: EntityId,
    ) -> Result<&[BindingId], Error> {
        if !self.graph.entity(class).complete {
            return Err(Error::Missing("conversion set of an incomplete class"));
        }
        if !self.conversion_sets.contains_key(&class) {
            let mut out = Vec::new();
            let mut hiding = HashMap::<(TypeId, bool), u32>::new();
            let mut seen = HashMap::<EntityId, u8>::new();
            let mut functions = HashSet::new();
            let mut work = vec![(class, false)];
            while let Some((current, leaving)) = work.pop() {
                if !leaving {
                    let visits = seen.entry(current).or_default();
                    if *visits == 2 {
                        continue;
                    }
                    *visits += 1;
                }
                for &binding in &self.graph.entity(current).declared_conversions {
                    let entity = self.graph.entity(self.graph.binding(binding).entity);
                    let ty = entity
                        .ty
                        .ok_or(Error::Invariant("conversion without signature"))?;
                    let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind else {
                        return Err(Error::Invariant("conversion signature"));
                    };
                    let count = hiding
                        .entry((result, entity.template_head.is_some()))
                        .or_default();
                    if leaving {
                        *count -= 1;
                    } else if *count == 0 && functions.insert(self.graph.binding(binding).entity) {
                        out.push(binding);
                    }
                }
                if leaving {
                    continue;
                }
                // Own declarations hide all corresponding base declarations, even
                // when they were themselves hidden by a more derived declaration.
                for &binding in &self.graph.entity(current).declared_conversions {
                    let entity = self.graph.entity(self.graph.binding(binding).entity);
                    let ty = entity
                        .ty
                        .ok_or(Error::Invariant("conversion without signature"))?;
                    if let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind {
                        *hiding
                            .entry((result, entity.template_head.is_some()))
                            .or_default() += 1;
                    }
                }
                work.push((current, true));
                for base in self.graph.entity(current).bases.iter().rev() {
                    if self.graph.entity(base.class).complete {
                        work.push((base.class, false));
                    }
                }
            }
            self.conversion_sets.insert(class, out);
        }
        Ok(&self.conversion_sets[&class])
    }

    pub(super) fn conversion_function_sequence(
        &mut self,
        source: ExprFacts,
        target: TypeId,
        direct: bool,
    ) -> Result<Option<ArgumentSequence>, Error> {
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Reference { .. }
        ) {
            return Ok(None);
        }
        self.scalar_conversion_function_sequence(source, target, direct)
    }

    pub(super) fn scalar_conversion_function_sequence(
        &mut self,
        source: ExprFacts,
        target: TypeId,
        direct: bool,
    ) -> Result<Option<ArgumentSequence>, Error> {
        // Class value results have additional constructor competition rules.
        let object = match self.graph.types.get(target).kind {
            TypeKind::Reference { element, .. } => element,
            _ => target,
        };
        if self.graph.class_of(object).is_some() {
            return Ok(None);
        }
        let mut candidates = Vec::new();
        self.conversion_function_candidates(source, target, direct, false, &mut candidates)?;
        let Some(best) = self.best_user_candidate(&candidates)? else {
            return Ok(None);
        };
        let after = best
            .after
            .ok_or(Error::Invariant("function candidate second sequence"))?;
        let mut outer = after;
        outer.rank = ConversionRank::UserDefined;
        Ok(Some(ArgumentSequence {
            conversion: outer,
            application: Some(after),
            function: None,
            user_conversion: Some(UserConversion::Function(best.binding)),
        }))
    }

    pub(super) fn conversion_function_candidates(
        &mut self,
        source: ExprFacts,
        target: TypeId,
        direct: bool,
        direct_lvalue: bool,
        candidates: &mut Vec<UserCandidate>,
    ) -> Result<(), Error> {
        let Some(class) = self.graph.class_of(source.ty) else {
            return Ok(());
        };
        let object = match self.graph.types.get(target).kind {
            TypeKind::Reference { element, .. } => element,
            _ => target,
        };
        let destination = self.graph.class_of(object);
        let receiver = ObjectArgument {
            source: ObjectSource::Implicit,
            ty: source.ty,
            category: source.category,
        };
        let count = self.visible_conversion_functions(class)?.len();
        for index in 0..count {
            let binding = self.conversion_sets[&class][index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            let explicit = entity.explicitness == Explicitness::Explicit;
            if entity.template_head.is_some() || explicit && !direct {
                continue;
            }
            let ty = entity
                .ty
                .ok_or(Error::Invariant("conversion without signature"))?;
            let produced = self.call_result(ty)?;
            if direct_lvalue && produced.category != ValueCategory::Lvalue {
                continue;
            }
            if let Some(destination) = destination
                && object == target
                && self.graph.class_of(produced.ty).is_none_or(|made| {
                    made != destination && !self.graph.is_base_of(destination, made)
                })
            {
                continue;
            }
            let Some(first) = self.object_sequence(receiver, binding, true)? else {
                continue;
            };
            let Some(after) = self.scalar_sequence(produced, target, false)? else {
                continue;
            };
            if direct_lvalue && after.temporary
                || explicit && destination.is_none() && after.second != ValueConversion::None
            {
                continue;
            }
            candidates.push(UserCandidate {
                binding,
                first,
                after: Some(after),
            });
        }
        Ok(())
    }

    pub(super) fn select_conversion_function(
        &mut self,
        source: ExprFacts,
        sequence: ArgumentSequence,
        scope: ScopeId,
        location: SourceOffset,
        usage: ConversionUse,
    ) -> Result<FunctionConversion, Error> {
        let Some(UserConversion::Function(binding)) = sequence.user_conversion else {
            return Err(Error::Invariant("conversion function without selection"));
        };
        let entity = self.graph.binding(binding).entity;
        // convert_class_operand does not check deletion in the frozen builtin path.
        if usage != ConversionUse::Builtin
            && self.graph.entity(entity).availability == FunctionAvailability::Deleted
        {
            return Err(Error::at(location, "use of a deleted function"));
        }
        let path = self.conversion_function_receiver(source, binding, scope, location)?;
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("conversion without signature"))?;
        let produced = self.call_result(ty)?;
        let after = sequence.application.ok_or(Error::Invariant(
            "conversion function without second sequence",
        ))?;
        // The ordinary converted_value path projects the result, independently
        // of the receiver. The frozen scalar path does not check access for
        // this second step (the builtin checker does so when applying it).
        let after_path = if usage == ConversionUse::Builtin {
            None
        } else {
            self.conversion_base_path(produced, after, scope, location, false)?
        };
        Ok(FunctionConversion {
            binding,
            result: produced.ty,
            category: produced.category,
            path,
            usage,
            after_path,
            after,
        })
    }

    fn conversion_function_receiver(
        &mut self,
        source: ExprFacts,
        binding: BindingId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Option<BasePathId>, Error> {
        let class = self
            .graph
            .class_of(source.ty)
            .ok_or(Error::Invariant("conversion function without class"))?;
        let owner = self
            .member_function_owner(binding)
            .and_then(|ty| self.graph.class_of(ty))
            .ok_or(Error::Invariant("conversion function without owner"))?;
        if self.graph.base_count(class, owner) > 1 {
            return Err(Error::at(
                location,
                "a conversion function of an ambiguous base class subobject",
            ));
        }
        self.check_named_member_access(binding, class, class, scope, location)?;
        let path = if class == owner {
            None
        } else {
            Some(self.checked_base_path(class, owner, scope, location, true, false)?)
        };
        self.check_specialization_body(binding)?;
        self.demand_function(binding, true);
        Ok(path)
    }

    pub(super) fn cast_conversion_function(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
        allowed: bool,
    ) -> Result<ExprFacts, Error> {
        let source = self.graph.expression_facts.get(operand)?;
        if allowed
            && let Some(sequence) = self.conversion_function_sequence(source, target, true)?
        {
            self.apply_function_sequence(operand, sequence, scope)?;
            return self.new_expression_facts(target, ValueCategory::Prvalue);
        }
        Err(Error::at(
            self.tree.expressions.get(id).location,
            "invalid explicit conversion",
        ))
    }
    pub(super) fn apply_function_sequence(
        &mut self,
        id: ast::ExprId,
        sequence: ArgumentSequence,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let source = self.graph.expression_facts.get(id)?;
        let selected = self.select_conversion_function(
            source,
            sequence,
            scope,
            self.tree.expressions.get(id).location,
            ConversionUse::Scalar,
        )?;
        self.graph
            .expression_facts
            .set_function_conversion(id, selected);
        let mut facts = source;
        facts.conversion = Some(sequence.conversion);
        self.graph.expression_facts.set(id, facts);
        Ok(())
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
    fn unique_operands_keep_original_facts_and_only_demand_the_discovered_functions() {
        let source = SourceBuffer::new(
            b"struct S{operator volatile short&()&&;operator double();};struct X:S{};\
              struct P{operator int* const&();explicit operator long*();};struct Unused{};\
              void f(const X&x,P&p){switch(((x))){default:break;}new int[x];delete ((p));}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"unique-operands").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let nodes = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), nodes);
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
        let ast::StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
            panic!("compound");
        };
        let items = tree.stmt_lists.get(items);
        let ast::StmtKind::ConditionalLoop {
            keyword: crate::lex::vocabulary::Keyword::Switch,
            condition: ast::Condition::Expression(switch),
            ..
        } = tree.statements.get(items[0]).kind
        else {
            panic!("switch");
        };
        let ast::StmtKind::Expression(Some(new)) = tree.statements.get(items[1]).kind else {
            panic!("new");
        };
        let count = graph
            .expression_facts
            .allocation(new)
            .unwrap()
            .array
            .unwrap()
            .count;
        let ast::StmtKind::Expression(Some(delete)) = tree.statements.get(items[2]).kind else {
            panic!("delete");
        };
        let ast::ExprKind::Delete {
            operand: pointer, ..
        } = tree.expressions.get(delete).kind
        else {
            panic!("operand");
        };
        let mut selected = Vec::new();
        for id in [switch, count, pointer] {
            let facts = graph.expression_facts.get(id).unwrap();
            let step = graph.expression_facts.function_conversion(id).unwrap();
            assert!(graph.class_of(facts.ty).is_some());
            assert_eq!(facts.category, ValueCategory::Lvalue);
            assert_eq!(step.category, ValueCategory::Lvalue);
            assert!(step.usage == ConversionUse::Builtin);
            assert!(!facts.conversion.unwrap().initialization);
            assert_eq!(step.after.result, facts.conversion.unwrap().result);
            assert_eq!(step.path.is_some(), id != pointer);
            assert_eq!(
                graph
                    .expression_facts
                    .function_conversion(graph.expression_facts.root(id))
                    .unwrap()
                    .binding,
                step.binding
            );
            selected.push(step.binding);
        }
        assert_eq!(selected[0], selected[1]);
        assert_ne!(switch, graph.expression_facts.root(switch));
        for name in [b"S".as_slice(), b"P"] {
            let name = cursor.intern_name(name).unwrap();
            let class = graph
                .binding(
                    graph
                        .own(graph.global, name, super::super::lookup::Filter::Prefix)
                        .unwrap(),
                )
                .entity;
            for &binding in &graph.entity(class).declared_conversions {
                let demanded = graph.root_demands.as_slice().iter().any(|demand|
                    matches!(demand, super::super::function_demands::Demand::Function { binding: found, .. }
                        if *found == binding));
                assert_eq!(demanded, selected.contains(&binding));
            }
        }
        let unused = cursor.intern_name(b"Unused").unwrap();
        let class = graph
            .binding(
                graph
                    .own(graph.global, unused, super::super::lookup::Filter::Prefix)
                    .unwrap(),
            )
            .entity;
        assert!(matches!(
            graph.entity(class).layout,
            super::super::layout::LayoutState::NotStarted
        ));
    }

    #[test]
    fn unevaluated_result_projection_keeps_layout_and_emission_undemanded() {
        let source = SourceBuffer::new(
            b"struct B{int n;};struct D:B{int m;};struct X{operator D*()noexcept;};\
              void take(B*)noexcept;bool f(X&x){return noexcept(take(x));}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"unevaluated-result-path").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert!(super::super::function_demands::trailing(&graph).is_empty());
        for name in [b"B", b"D"] {
            let name = cursor.intern_name(name).unwrap();
            let binding = graph
                .own(graph.global, name, super::super::lookup::Filter::Prefix)
                .unwrap();
            assert!(matches!(
                graph.entity(graph.binding(binding).entity).layout,
                super::super::layout::LayoutState::NotStarted
            ));
        }
    }

    #[test]
    fn scalar_result_and_receiver_keep_separate_base_paths() {
        let source = SourceBuffer::new(
            b"struct B{int n;};struct D:B{int m;};struct S{operator D*&();};\
              struct X:S{};int take(B*);int f(X&x){return take(((x)));}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"scalar-result-path").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
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
        let ast::StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
            panic!("compound");
        };
        let ast::StmtKind::Return(Some(call)) =
            tree.statements.get(tree.stmt_lists.get(items)[0]).kind
        else {
            panic!("return");
        };
        let ast::ExprKind::Call { arguments, .. } = tree.expressions.get(call).kind else {
            panic!("call");
        };
        let operand = tree.expr_lists.get(arguments)[0];
        let selected = graph.expression_facts.function_conversion(operand).unwrap();
        assert!(selected.usage == ConversionUse::Scalar);
        assert!(selected.path.is_some());
        assert!(selected.after_path.is_some());
        assert_ne!(selected.path, selected.after_path);
        assert_eq!(selected.category, ValueCategory::Lvalue);
        assert_eq!(selected.after.second, ValueConversion::DerivedToBase);
        let facts = graph.expression_facts.get(operand).unwrap();
        assert!(graph.class_of(facts.ty).is_some());
        assert!(facts.base_path.is_none());
        assert_eq!(
            graph.lower_base_conversion_offset(operand).unwrap(),
            Some(0)
        );
        assert_ne!(operand, graph.expression_facts.root(operand));
    }

    #[test]
    fn selected_conversion_keeps_source_aliases_and_does_not_demand_hidden_or_losing_members() {
        let source = SourceBuffer::new(
            br"
            struct Base { operator int() const; operator long() const; };
            struct Box:Base { operator int() const; };
            int choose(int); int choose(char*);
            int f(const Box& b) { return choose(((b))); }
            struct Unused { operator int(); };
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"conversion-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let nodes = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), nodes);
        let mut class = |name: &[u8]| {
            let name = cursor.intern_name(name).unwrap();
            graph
                .binding(
                    graph
                        .own(graph.global, name, super::super::lookup::Filter::Prefix)
                        .unwrap(),
                )
                .entity
        };
        let base = class(b"Base");
        let boxed = class(b"Box");
        let unused = class(b"Unused");
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[4])
            .kind
        else {
            panic!("function");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("parsed body");
        };
        let ast::StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
            panic!("compound");
        };
        let ast::StmtKind::Return(Some(call)) =
            tree.statements.get(tree.stmt_lists.get(items)[0]).kind
        else {
            panic!("return");
        };
        let ast::ExprKind::Call { arguments, .. } = tree.expressions.get(call).kind else {
            panic!("call");
        };
        let original = tree.expr_lists.get(arguments)[0];
        let root = graph.expression_facts.root(original);
        assert_ne!(original, root);
        let source = graph.expression_facts.get(original).unwrap();
        assert_eq!(graph.class_of(source.ty), Some(boxed));
        assert_eq!(source.category, ValueCategory::Lvalue);
        assert_eq!(graph.expression_type(root), Some(source.ty));
        let selected = graph
            .expression_facts
            .function_conversion(original)
            .unwrap();
        assert_eq!(
            selected.binding,
            graph.entity(boxed).declared_conversions[0]
        );
        assert_eq!(
            graph
                .expression_facts
                .function_conversion(root)
                .unwrap()
                .binding,
            selected.binding
        );
        assert_eq!(source.conversion.unwrap().rank, ConversionRank::UserDefined);
        assert_eq!(selected.after.result, selected.result);
        let demanded = |binding| {
            graph.root_demands.as_slice().iter().any(|d| matches!(d, super::super::function_demands::Demand::Function { binding: found, .. } if *found == binding))
        };
        assert!(demanded(selected.binding));
        for &binding in graph
            .entity(base)
            .declared_conversions
            .iter()
            .chain(&graph.entity(unused).declared_conversions)
        {
            assert!(!demanded(binding));
        }
    }
}
