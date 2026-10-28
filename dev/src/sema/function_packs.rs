// Translated from function_templates.cpp::deduce_for_call and template_packs.cpp.
// See NOTICE.
//! Explicit prefixes, trailing deduction and named function packs share partitions.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::ExprFacts,
    graph::BindingId,
    templates::HeadId,
    types::{Cv, TypeId, TypeKind},
};
use crate::syntax::ast;

impl Analyzer<'_> {
    #[inline(never)]
    pub(super) fn deduce_pack_function_call(
        &mut self,
        binding: BindingId,
        head: HeadId,
        args: &[ast::ExprId],
        prefix: Option<ExprFacts>,
    ) -> Result<Option<BindingId>, Error> {
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or(Error::Invariant("pack function without type"))?;
        let TypeKind::Function {
            result,
            parameters,
            variadic,
            ..
        } = self.graph.types.get(ty).kind.clone()
        else {
            return Err(Error::Invariant("pack function without signature"));
        };
        let depth = self.graph.templates.depth(head);
        let mut arguments = self.signature_arguments(binding, head)?;
        let offset = usize::from(prefix.is_some());
        let supplied = args.len() + offset;
        // analyzer_calls.cpp::resolve filters the declared fixed prefix before
        // any substitution can demand a return type or a class definition.
        let trailing = parameters
            .last()
            .is_some_and(|&ty| matches!(self.graph.types.get(ty).kind, TypeKind::Expansion(_)));
        let fixed = parameters.len() - usize::from(trailing);
        let inner = parameters[..fixed]
            .iter()
            .any(|&ty| matches!(self.graph.types.get(ty).kind, TypeKind::Expansion(_)));
        if !inner
            && (supplied > fixed && !variadic && !trailing
                || (supplied..fixed).any(|index| {
                    !self.function_has_default(self.graph.binding(binding).entity, index)
                }))
        {
            return Ok(None);
        }
        let explicit = self.graph.function_templates.requires_target(binding);
        let mut prefixes = vec![None; arguments.len()];
        let mut explicit_count = 0;
        if explicit
            && let Some(&last) = parameters.last()
            && let TypeKind::Expansion(pattern) = self.graph.types.get(last).kind
        {
            let packs = self.type_pack_pattern(pattern, true)?;
            let indices: Vec<_> = packs
                .parameters
                .iter()
                .filter_map(|&entity| {
                    self.graph
                        .entity(entity)
                        .template_parameter
                        .and_then(|info| info.index(depth))
                })
                .collect();
            let mut length = None;
            let mut known = !indices.is_empty();
            for &index in &indices {
                if let Some(pack) = arguments[index]
                    && let TypeKind::Pack(elements) = &self.graph.types.get(pack).kind
                {
                    known &= length.is_none_or(|length| length == elements.len());
                    length = Some(elements.len());
                } else {
                    known = false;
                }
            }
            if known
                && let Some(length) = length
                && supplied > parameters.len() - 1 + length
            {
                explicit_count = length;
                for index in indices {
                    prefixes[index] = arguments[index].take();
                }
            }
        }
        let parameters = if explicit {
            // C++ SoftSubstitution covers the leading result and the entire
            // parameter list, including expansion length checks.
            let owner = self.graph.binding(binding).entity;
            let substituted = self.with_substitution_soft(Some(owner), |analyzer| {
                let mut environment = analyzer.substitution(depth, &arguments);
                environment.context = Some(owner);
                let substituted = (|| {
                    if !analyzer.graph.function_templates.trailing_result(owner)
                        && analyzer
                            .substitute_signature_type(result, &mut environment)?
                            .is_none()
                    {
                        return Ok(None);
                    }
                    analyzer.substitute_pack_list(&parameters, &mut environment)
                })();
                let failed = analyzer
                    .graph
                    .analysis
                    .substitution
                    .current()
                    .failure
                    .is_some();
                let substituted = substituted?;
                Ok(if failed { None } else { substituted })
            });
            match substituted {
                Ok(Some(parameters)) => parameters,
                Ok(None)
                | Err(
                    Error::Diagnostic { .. }
                    | Error::NonConstant { .. }
                    | Error::SubstitutionRejected(_)
                    | Error::SubstitutionNull,
                ) => {
                    return Ok(None);
                }
                Err(error) => return Err(error),
            }
        } else {
            parameters
        };
        let trailing = parameters
            .last()
            .and_then(|&ty| match self.graph.types.get(ty).kind {
                TypeKind::Expansion(pattern) => Some(pattern),
                _ => None,
            });
        let fixed = parameters.len() - usize::from(trailing.is_some());
        for (index, &pattern) in parameters.iter().take(fixed.min(supplied)).enumerate() {
            if matches!(self.graph.types.get(pattern).kind, TypeKind::Expansion(_))
                || !self.needs_deduction(pattern, depth)?
            {
                continue;
            }
            if index >= offset
                && matches!(
                    self.tree.expressions.get(args[index - offset]).kind,
                    ast::ExprKind::Braced { .. }
                )
            {
                if !self.deduce_list_elements(
                    pattern,
                    args[index - offset],
                    depth,
                    &mut arguments,
                )? {
                    return Ok(None);
                }
                continue;
            }
            let facts = if index < offset {
                prefix.ok_or(Error::Invariant("missing call prefix"))?
            } else {
                self.graph.expression_facts.get(args[index - offset])?
            };
            if !self.deduce_pack_call_fact(pattern, facts, depth, &mut arguments)? {
                return Ok(None);
            }
        }
        if let Some(pattern) = trailing
            && supplied > fixed + explicit_count
        {
            let first = fixed + explicit_count;
            if !self.deduce_pack_elements(
                pattern,
                supplied - first,
                true,
                depth,
                &mut arguments,
                |this, position, trial| {
                    let index = first + position;
                    if index >= offset
                        && matches!(
                            this.tree.expressions.get(args[index - offset]).kind,
                            ast::ExprKind::Braced { .. }
                        )
                    {
                        return this.deduce_list_elements(
                            pattern,
                            args[index - offset],
                            depth,
                            trial,
                        );
                    }
                    let facts = if index < offset {
                        prefix.ok_or(Error::Invariant("missing call prefix"))?
                    } else {
                        this.graph.expression_facts.get(args[index - offset])?
                    };
                    this.deduce_pack_call_fact(pattern, facts, depth, trial)
                },
            )? {
                return Ok(None);
            }
        }
        for (index, prefix) in prefixes.into_iter().enumerate() {
            if let Some(prefix) = prefix {
                let mut elements = self.graph.types.pack_elements(prefix)?.to_vec();
                if let Some(pack) = arguments[index] {
                    elements.extend_from_slice(self.graph.types.pack_elements(pack)?);
                }
                arguments[index] = Some(
                    self.graph
                        .types
                        .intern(TypeKind::Pack(elements), Cv::default())?,
                );
            }
        }
        if !self.deduced_values_typed(head, ty, &arguments)? {
            return Ok(None);
        }
        self.complete_signature(binding, &mut arguments)
    }

    pub(super) fn function_pack_scope(
        &mut self,
        parent: super::graph::ScopeId,
        clause: ast::ParameterClause,
        packs: &[(Option<crate::lex::identifiers::NameId>, usize)],
        ty: TypeId,
    ) -> Result<super::graph::ScopeId, Error> {
        use super::graph::{Binding, BindingKind, ScopeKind};
        if packs.is_empty() {
            return Ok(parent);
        }
        let scope = self
            .graph
            .add_scope(ScopeKind::TemplateParameters, parent, None, None)?;
        let mut position = 0;
        let mut next = 0;
        for index in 0..self.tree.parameters.get(clause.parameters).len() {
            let parameter = self.tree.parameters.get(clause.parameters)[index];
            if !parameter
                .declarator
                .is_some_and(|id| self.tree.declarators.get(id).pack)
                || next >= packs.len()
            {
                position += 1;
                continue;
            }
            let (name, count) = packs[next];
            next += 1;
            if let Some(name) = name
                && position + count <= self.graph.types.function_parameter_count(ty)?
            {
                let mut elements = Vec::with_capacity(count);
                for index in position..position + count {
                    elements.push(self.graph.types.function_parameter(ty, index)?);
                }
                let pack = self
                    .graph
                    .types
                    .intern(TypeKind::Pack(elements), Cv::default())?;
                let entity = self.graph.add_entity(
                    Some(pack),
                    Some(name),
                    crate::lex::vocabulary::Keyword::Typedef,
                )?;
                self.graph.add_binding(
                    scope,
                    None,
                    Binding {
                        kind: BindingKind::Alias,
                        name: Some(name),
                        entity,
                        ty: Some(pack),
                        key: crate::lex::vocabulary::Keyword::Typedef,
                    },
                )?;
            }
            position += count;
        }
        Ok(scope)
    }

    fn deduce_pack_call_fact(
        &mut self,
        pattern: TypeId,
        facts: ExprFacts,
        depth: u32,
        arguments: &mut [Option<TypeId>],
    ) -> Result<bool, Error> {
        if facts.object.is_some()
            && (facts.functions.is_some() || facts.function.is_some() || facts.destructor.is_some())
        {
            return Ok(false);
        }
        if let Some(set) = facts.functions {
            self.deduce_overloaded_argument(pattern, set, depth, arguments)?;
            Ok(true)
        } else {
            self.deduce_call_argument(pattern, facts.ty, facts.category, depth, arguments)
        }
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
    fn an_explicit_pack_rejection_charges_its_own_soft_scope_once() {
        let source = SourceBuffer::new(
            b"template<class...T>int f(T*...);template<class...T>int f(int);int main(){return f<int&>(0);}".to_vec(),
        ).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"pack-failure-charge").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        assert_eq!(graph.work.totals.substitution_failures, 1);
        assert_eq!(graph.function_templates.arguments.len(), 1);
        assert!(graph.analysis.substitution.current().failure.is_none());
    }

    #[test]
    fn identity_nontemplate_match_does_not_form_a_template_signature() {
        let source = SourceBuffer::new(b"template<class T>struct Trap{static_assert(sizeof(T)==0,\"unused\");using type=int;};int f(int n){return n;}template<class T>typename Trap<T>::type f(T);int main(){return f(1);}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"perfect-call-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        assert!(graph.function_templates.arguments.is_empty());
    }
    #[test]
    fn expression_expansion_selects_elements_but_keeps_the_whole_pack_for_sizeof() {
        let source = SourceBuffer::new(b"template<int...N>struct S{};template<int...N>S<(int(sizeof...(N))-N)...>f(S<N...>); S<0,1> a;".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"signature-pack-selection").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let a = cursor.intern_name(b"a").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let f = analyzer
            .graph
            .own(global, f, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let a = analyzer
            .graph
            .own(global, a, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let head = analyzer.function_template_head(f).unwrap().unwrap();
        let pattern = analyzer
            .graph
            .types
            .function_parameter(analyzer.graph.binding(f).ty.unwrap(), 0)
            .unwrap();
        let actual = analyzer.graph.binding(a).ty.unwrap();
        let mut args = vec![None];
        assert!(
            analyzer
                .deduce_signature_type(
                    pattern,
                    actual,
                    analyzer.graph.templates.depth(head),
                    &mut args,
                    false
                )
                .unwrap()
        );
        assert!(args[0].is_some(), "missing deduced pack");
        let specialization = analyzer.complete_signature(f, &mut args).unwrap();
        let specialization = specialization.unwrap();
        let TypeKind::Function { result, .. } = analyzer
            .graph
            .types
            .get(analyzer.graph.binding(specialization).ty.unwrap())
            .kind
        else {
            panic!("not a function");
        };
        let class = analyzer.graph.class_of(result).unwrap();
        let result = analyzer
            .graph
            .class_templates
            .instance(class)
            .unwrap()
            .arguments[0];
        let values: Vec<_> = analyzer
            .graph
            .types
            .pack_elements(result)
            .unwrap()
            .iter()
            .map(|&ty| {
                let TypeKind::Value { bits, .. } = analyzer.graph.types.get(ty).kind else {
                    panic!("not a value");
                };
                bits
            })
            .collect();
        assert_eq!(values, [2, 1]);
        let expressions = analyzer.tree.expressions.mark();
        assert_eq!(
            analyzer.complete_signature(f, &mut args).unwrap(),
            Some(specialization)
        );
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
    }
    #[test]
    fn partitions_reuse_signatures_and_copy_only_demanded_parameter_objects() {
        let source = SourceBuffer::new(b"template<class...>struct S{};template<class...U,class...T>int f(S<U...>,T...t){return sizeof...(U)+sizeof...(t);}int main(){return f(S<int,long>(),1)+f(S<int,long>(),1)+f(S<int>(),1L,2);}static_assert(sizeof(f(S<char>(),1,2,3))==4,\"\");".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"function-pack-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = super::super::template_bodies::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        assert_eq!(graph.function_templates.arguments.len(), 3);
        let mut signatures = std::collections::HashSet::new();
        let mut bodies = std::collections::HashSet::new();
        for (&entity, arguments) in &graph.function_templates.arguments {
            assert_eq!(arguments.len(), 2);
            let primary = graph
                .binding(graph.function_templates.primary(entity).unwrap())
                .entity;
            assert!(signatures.insert(entity));
            let original = graph.entity(primary).function_parameters.unwrap();
            let parameters = tree.parameters.get(original.parameters);
            assert_eq!(parameters.len(), 2);
            assert!(tree.declarators.get(parameters[1].declarator.unwrap()).pack);
            let count = graph.types.pack_elements(arguments[1]).unwrap().len();
            let Some(body) = graph.function_templates.bodies.body(entity) else {
                assert_eq!(count, 3);
                continue;
            };
            assert!(bodies.insert(body));
            let parameters = graph.entity(entity).function_parameters.unwrap();
            assert_eq!(tree.parameters.get(parameters.parameters).len(), count + 1);
            for parameter in tree.parameters.get(parameters.parameters) {
                assert!(
                    !parameter
                        .declarator
                        .is_some_and(|id| tree.declarators.get(id).pack)
                );
            }
        }
        assert_eq!(bodies.len(), 2);
    }
}
