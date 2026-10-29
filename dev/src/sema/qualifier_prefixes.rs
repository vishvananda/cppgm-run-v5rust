// Translated from sema/class_templates.cpp::qualifier_arguments. See NOTICE.
//! Demanded bound qualifier defaults are semantic facts, consumed read-only by ABI lowering.
use super::{
    Error,
    analyzer::Analyzer,
    expression_keys::{KeyNode, KeyWord},
    graph::{EntityId, ScopeId},
    templates::HeadId,
    types::{TypeId, TypeKind},
};
use crate::support::hash::HashMap;
use crate::support::hash::HashSet;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum AbiDemand {
    Type(TypeId),
    Function(EntityId),
    Scope(ScopeId),
}

enum ArgumentCount {
    Spelled(usize),
    Failed(Error),
}

#[derive(Default)]
pub(super) struct Qualifiers {
    requested: HashSet<TypeId>,
    pending: Vec<TypeId>,
    counts: HashMap<EntityId, ArgumentCount>,
    defaults: HashMap<(HeadId, usize), TypeId>,
    abi_seen: HashSet<AbiDemand>,
    abi_pending: Vec<AbiDemand>,
}

impl Qualifiers {
    fn demand(&mut self, demand: AbiDemand) {
        if self.abi_seen.insert(demand) {
            self.abi_pending.push(demand);
        }
    }
    pub(super) fn request(&mut self, ty: TypeId) {
        if self.requested.insert(ty) {
            self.pending.push(ty);
        }
    }
    pub(super) fn count(&self, entity: EntityId) -> Result<Option<usize>, Error> {
        match self.counts.get(&entity) {
            Some(ArgumentCount::Spelled(count)) => Ok(Some(*count)),
            Some(ArgumentCount::Failed(error)) => Err(error.clone()),
            None => Ok(None),
        }
    }
}
impl Analyzer<'_> {
    pub(super) fn request_qualifier_signature(&mut self, entity: EntityId) {
        self.graph
            .class_templates
            .qualifiers
            .demand(AbiDemand::Function(entity));
    }

    fn collect_qualifier_demands(&mut self) -> Result<(), Error> {
        while let Some(demand) = self.graph.class_templates.qualifiers.abi_pending.pop() {
            match demand {
                AbiDemand::Type(ty) => self.qualifier_type_demands(ty),
                AbiDemand::Function(entity) => {
                    let entity = self.graph.lower_function_entity(entity);
                    let node = self.graph.entity(entity);
                    let (ty, home) = (node.ty, node.home);
                    if let Some(ty) = ty {
                        self.graph
                            .class_templates
                            .qualifiers
                            .demand(AbiDemand::Type(ty));
                    }
                    if let Some(home) = home {
                        self.graph
                            .class_templates
                            .qualifiers
                            .demand(AbiDemand::Scope(home));
                    }
                    if self.graph.function_templates.primary(entity).is_some() {
                        let (pattern, arguments) = self.graph.lower_template_signature(entity)?;
                        let arguments = arguments.to_vec();
                        self.graph
                            .class_templates
                            .qualifiers
                            .demand(AbiDemand::Type(pattern));
                        for argument in arguments {
                            self.graph
                                .class_templates
                                .qualifiers
                                .demand(AbiDemand::Type(argument));
                        }
                    }
                }
                AbiDemand::Scope(scope) => {
                    let node = self.graph.scope(scope);
                    let (parent, owner, kind) = (node.parent, node.owner, node.kind);
                    if let Some(parent) = parent {
                        self.graph
                            .class_templates
                            .qualifiers
                            .demand(AbiDemand::Scope(parent));
                    }
                    if let Some(owner) = owner {
                        if kind == super::graph::ScopeKind::Function {
                            self.request_qualifier_signature(owner);
                        } else if let Some(ty) = self.graph.entity(owner).ty {
                            self.graph
                                .class_templates
                                .qualifiers
                                .demand(AbiDemand::Type(ty));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn qualifier_type_demands(&mut self, ty: TypeId) {
        let mut types = Vec::new();
        if let Some((_, words)) = self.graph.lower_template_operand(ty) {
            let mut qualifiers = Vec::new();
            for window in words.windows(4) {
                if let [
                    KeyWord::Node(KeyNode::Name),
                    KeyWord::Count(count),
                    KeyWord::Flag(false),
                    KeyWord::Type(root),
                ] = window
                    && *count > 1
                {
                    qualifiers.push(*root);
                }
            }
            let mut functions = Vec::new();
            let mut homes = Vec::new();
            for word in words {
                match word {
                    KeyWord::Type(ty)
                    | KeyWord::Constant(ty, _)
                    | KeyWord::FloatingLiteral { ty, .. } => types.push(*ty),
                    KeyWord::ResolvedFunction(entity) => functions.push(*entity),
                    KeyWord::Entity(entity) => {
                        let node = self.graph.entity(*entity);
                        if node.callable().function.is_some() {
                            functions.push(*entity);
                        } else if let Some(ty) = self.graph.lower_operand_type_name(*entity) {
                            types.push(ty);
                        }
                        if let Some(home) = node.home {
                            homes.push(home);
                        }
                    }
                    _ => (),
                }
            }
            for qualifier in qualifiers {
                self.graph.class_templates.qualifiers.request(qualifier);
            }
            for function in functions {
                self.request_qualifier_signature(function);
            }
            for home in homes {
                self.graph
                    .class_templates
                    .qualifiers
                    .demand(AbiDemand::Scope(home));
            }
        }
        if let Some((qualifier, _)) = self.graph.lower_dependent_member(ty) {
            types.push(qualifier);
        }
        match &self.graph.types.get(ty).kind {
            TypeKind::Class(entity) => {
                if let Some(aliased) = self.graph.class_templates.aliases.aliased(*entity) {
                    types.push(aliased);
                } else if let Some(instance) = self.graph.class_templates.instance(*entity) {
                    types.extend_from_slice(&instance.arguments);
                }
                if let Some(home) = self.graph.entity(*entity).home {
                    self.graph
                        .class_templates
                        .qualifiers
                        .demand(AbiDemand::Scope(home));
                }
            }
            TypeKind::Pack(elements) => types.extend_from_slice(elements),
            TypeKind::Expansion(element)
            | TypeKind::Pointer(element)
            | TypeKind::Reference { element, .. }
            | TypeKind::Value { ty: element, .. }
            | TypeKind::Address { ty: element, .. } => types.push(*element),
            TypeKind::BoundPack { arguments, .. } => types.push(*arguments),
            TypeKind::MemberPointer { owner, element } => types.extend([*owner, *element]),
            TypeKind::Array {
                element, operand, ..
            } => {
                types.push(*element);
                types.extend(operand);
            }
            TypeKind::Function {
                result, parameters, ..
            } => {
                types.push(*result);
                types.extend_from_slice(parameters);
            }
            _ => (),
        }
        for ty in types {
            self.graph
                .class_templates
                .qualifiers
                .demand(AbiDemand::Type(ty));
        }
    }

    pub(super) fn finish_qualifier_arguments(&mut self) -> Result<(), Error> {
        self.collect_qualifier_demands()?;
        // Only queued names are visited, after declaration defaults have settled.
        // Default analysis may itself introduce another bound qualifier request.
        while let Some(mut root) = self.graph.class_templates.qualifiers.pending.pop() {
            while let Some((qualifier, _)) = self.graph.lower_dependent_member(root) {
                root = qualifier;
            }
            if let TypeKind::Class(entity) = self.graph.types.get(root).kind
                && let Some(aliased) = self.graph.class_templates.aliases.aliased(entity)
            {
                root = aliased;
            }
            let TypeKind::Class(entity) = self.graph.types.get(root).kind else {
                continue;
            };
            if self
                .graph
                .class_templates
                .qualifiers
                .counts
                .contains_key(&entity)
            {
                continue;
            }
            let Some(instance) = self.graph.class_templates.instance(entity) else {
                continue;
            };
            if self
                .graph
                .class_templates
                .template_entities
                .name(instance.primary)
                .is_some()
                || self.graph.entity(entity).name.is_none()
            {
                continue;
            }
            let head = self
                .graph
                .entity(instance.primary)
                .context()
                .template_head
                .ok_or(Error::Invariant("qualifier specialization without head"))?;
            let arguments = instance.arguments.clone();
            // Preserve lowering's diagnostic order: default substitution is an
            // ABI query, and a failed fact is reported only when its level is read.
            let count = match self.qualifier_arguments(head, &arguments) {
                Ok(count) => ArgumentCount::Spelled(count),
                Err(
                    error @ (Error::Diagnostic { .. }
                    | Error::NonConstant { .. }
                    | Error::InstantiationDiagnostic { .. }
                    | Error::UnconditionalDiagnostic { .. }
                    | Error::SubstitutionRejected(_)
                    | Error::SubstitutionNull),
                ) => ArgumentCount::Failed(error),
                Err(error) => return Err(error),
            };
            self.graph
                .class_templates
                .qualifiers
                .counts
                .insert(entity, count);
        }
        Ok(())
    }

    fn qualifier_arguments(&mut self, head: HeadId, arguments: &[TypeId]) -> Result<usize, Error> {
        let mut count = arguments.len();
        while count != 0 {
            let index = count - 1;
            let Some(&parameter) = self.graph.templates.parameters(head).get(index) else {
                break;
            };
            let info = self
                .graph
                .entity(parameter)
                .context()
                .template_parameter
                .ok_or(Error::Invariant("qualifier head without parameter facts"))?;
            if info.is_pack() || info.inner.is_some() {
                break;
            }
            let Some(default) = self.graph.templates.default_argument(head, index) else {
                break;
            };
            let pattern = if let Some(&pattern) = self
                .graph
                .class_templates
                .qualifiers
                .defaults
                .get(&(head, index))
            {
                pattern
            } else {
                let target = self.argument_target(head, index, arguments)?.known();
                let copy = self.copy_template_pattern(default.syntax)?;
                let saved = self.graph.lookup_limit.replace(default.limit);
                self.template_depth += 1;
                let result = self.template_argument_at(
                    copy,
                    default.scope,
                    self.default_location(default),
                    target,
                    None,
                    false,
                );
                self.template_depth -= 1;
                self.graph.lookup_limit = saved;
                let pattern = result?;
                self.graph
                    .class_templates
                    .qualifiers
                    .defaults
                    .insert((head, index), pattern);
                pattern
            };
            let mut depth = info.depth();
            for &binding in &self.graph.scope(default.scope).bindings {
                if let Some(info) = self
                    .graph
                    .entity(self.graph.binding(binding).entity)
                    .context()
                    .template_parameter
                {
                    depth = info.depth();
                    break;
                }
            }
            let bound = if index == 0 || !self.graph.dependent_type(pattern) {
                pattern
            } else {
                let given: Vec<_> = arguments[..index].iter().copied().map(Some).collect();
                let mut environment = self.substitution(depth, &given);
                self.substitute_signature_type(pattern, &mut environment)?
                    .ok_or_else(|| {
                        environment.failure.map_or(
                            Error::Invariant(
                                "qualifier default substitution without failure reason",
                            ),
                            Error::SubstitutionRejected,
                        )
                    })?
            };
            if self.graph.types.canonical_function(bound)?
                != self.graph.types.canonical_function(arguments[index])?
            {
                break;
            }
            count -= 1;
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{lookup::Filter, types::Scalar},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
        types::FundamentalType,
    };

    #[test]
    fn demanded_defaults_reuse_patterns_without_completing_class_shells() {
        let source = SourceBuffer::new(
            b"template<class T,class U=T*,class V=U*>struct Box; \
              template<class T,class... U>struct Pack;"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"qualifier-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"Box").unwrap();
        let pack = cursor.intern_name(b"Pack").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let primary = analyzer
            .graph
            .binding(analyzer.graph.own(global, name, Filter::Ordinary).unwrap())
            .entity;
        let pack = analyzer
            .graph
            .binding(analyzer.graph.own(global, pack, Filter::Ordinary).unwrap())
            .entity;
        let int = analyzer
            .graph
            .types
            .scalar(Scalar::Basic(FundamentalType::Int))
            .unwrap();
        let pointer = analyzer
            .graph
            .types
            .intern(TypeKind::Pointer(int), super::super::types::Cv::default())
            .unwrap();
        let pointers = analyzer
            .graph
            .types
            .intern(
                TypeKind::Pointer(pointer),
                super::super::types::Cv::default(),
            )
            .unwrap();
        let empty = analyzer
            .graph
            .types
            .intern(
                TypeKind::Pack(Vec::new()),
                super::super::types::Cv::default(),
            )
            .unwrap();
        let matching = analyzer
            .class_specialization(primary, vec![int, pointer, pointers])
            .unwrap();
        let different = analyzer
            .class_specialization(primary, vec![int, int, pointer])
            .unwrap();
        let pack = analyzer
            .class_specialization(pack, vec![int, empty])
            .unwrap();
        for entity in [matching, different, pack, matching] {
            let ty = analyzer.graph.entity(entity).ty.unwrap();
            analyzer.graph.class_templates.qualifiers.request(ty);
        }
        assert_eq!(analyzer.graph.class_templates.qualifiers.pending.len(), 3);
        analyzer.finish_qualifier_arguments().unwrap();
        assert_eq!(
            analyzer
                .graph
                .class_templates
                .qualifiers
                .count(matching)
                .unwrap(),
            Some(1)
        );
        assert_eq!(
            analyzer
                .graph
                .class_templates
                .qualifiers
                .count(different)
                .unwrap(),
            Some(2)
        );
        assert_eq!(
            analyzer
                .graph
                .class_templates
                .qualifiers
                .count(pack)
                .unwrap(),
            Some(2)
        );
        assert_eq!(analyzer.graph.class_templates.qualifiers.defaults.len(), 2);
        let defaults = analyzer.graph.class_templates.qualifiers.defaults.clone();
        analyzer.finish_qualifier_arguments().unwrap();
        assert_eq!(analyzer.graph.class_templates.qualifiers.defaults, defaults);
        let head = analyzer
            .graph
            .entity(primary)
            .context()
            .template_head
            .unwrap();
        for index in 1..3 {
            let default = analyzer
                .graph
                .templates
                .default_argument(head, index)
                .unwrap();
            let crate::syntax::ast::TemplateArgumentKind::Type(ty) = default.syntax else {
                panic!("type default")
            };
            assert!(analyzer.graph.type_fact(ty).is_none());
        }
        for entity in [matching, different, pack] {
            let node = analyzer.graph.entity(entity);
            assert!(node.context().scope.is_none() && !node.complete && !node.defined);
        }
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn invalid_overridden_default_reports_its_failure_without_publishing_a_count() {
        let source =
            SourceBuffer::new(b"template<class T,class U=typename T::missing>struct Box;".to_vec())
                .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"qualifier-failure").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"Box").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let primary = analyzer
            .graph
            .binding(analyzer.graph.own(global, name, Filter::Ordinary).unwrap())
            .entity;
        let int = analyzer
            .graph
            .types
            .scalar(Scalar::Basic(FundamentalType::Int))
            .unwrap();
        let instance = analyzer
            .class_specialization(primary, vec![int, int])
            .unwrap();
        let ty = analyzer.graph.entity(instance).ty.unwrap();
        analyzer.graph.class_templates.qualifiers.request(ty);
        analyzer.finish_qualifier_arguments().unwrap();
        assert!(matches!(
            analyzer.graph.class_templates.qualifiers.count(instance),
            Err(Error::UnconditionalDiagnostic {
                location: crate::support::source::SourceOffset(0),
                message: "a dependent name's qualifier is not a complete class",
            })
        ));
        assert!(analyzer.graph.lookup_limit.is_none());
        assert_eq!(analyzer.template_depth, 0);
        assert!(analyzer.graph.entity(instance).context().scope.is_none());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }
}
