// Translated from sema/template_names.cpp::alias_specialization and
// specialization_binding. See NOTICE.
//! Alias results retain canonical argument identity without demanding a class.
use super::{
    Error,
    analyzer::Analyzer,
    function_templates::Substitution,
    graph::{Binding, BindingId, BindingKind, EntityId},
    templates::HeadId,
    types::{Cv, TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset};
use std::collections::HashMap;

#[derive(PartialEq, Eq, Hash)]
struct Key {
    alias: EntityId,
    arguments: Vec<TypeId>,
}

#[derive(Default)]
pub(super) struct Aliases {
    type_pack_element: Option<EntityId>,
    specializations: HashMap<Key, TypeId>,
    bindings: HashMap<TypeId, BindingId>,
    discarded: HashMap<EntityId, u64>,
    stand_ins: HashMap<EntityId, EntityId>,
    owners: HashMap<EntityId, EntityId>,
    aliased: HashMap<EntityId, TypeId>,
}
impl Aliases {
    pub(super) fn aliased(&self, entity: EntityId) -> Option<TypeId> {
        self.aliased.get(&entity).copied()
    }
    pub(super) fn owner(&self, primary: EntityId) -> Option<EntityId> {
        self.owners.get(&primary).copied()
    }
}

impl super::graph::Graph {
    pub(super) fn unqualified_aliased_type(&self, mut ty: TypeId) -> TypeId {
        while self.types.get(ty).cv.0 == 0
            && let TypeKind::Class(entity) = self.types.get(ty).kind
            && let Some(aliased) = self.class_templates.aliases.aliased(entity)
        {
            ty = aliased;
        }
        ty
    }
}

impl Analyzer<'_> {
    pub(super) fn record_type_pack_element(
        &mut self,
        alias: EntityId,
        home: super::graph::ScopeId,
        head: HeadId,
    ) {
        if self.hosted()
            && home == self.graph.global
            && self
                .graph
                .class_templates
                .aliases
                .type_pack_element
                .is_none()
            && self
                .graph
                .entity(alias)
                .name
                .is_some_and(|name| self.identifiers.bytes(name) == b"__type_pack_element")
        {
            let parameters = self.graph.templates.parameters(head);
            if parameters.len() == 2
                && self
                    .graph
                    .entity(parameters[0])
                    .template_parameter
                    .is_some_and(|info| info.value_type.is_some())
                && self.argument_parameter_pack(head, 1)
            {
                self.graph.class_templates.aliases.type_pack_element = Some(alias);
            }
        }
    }
    /// Identify the declaration before forming its arguments or checking the
    /// terminal name's access. Elaborated types and explicit instantiations
    /// reject alias template-ids even when the aliased result is a class.
    pub(super) fn alias_template_id(
        &mut self,
        name: crate::syntax::ast::Name,
        scope: super::graph::ScopeId,
    ) -> Result<bool, Error> {
        let node = *self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        let Some(last) = components.last() else {
            return Err(Error::Invariant("empty alias template name"));
        };
        let crate::syntax::ast::NameKind::Identifier(identifier) = last.kind else {
            return Ok(false);
        };
        if last.arguments.is_none() {
            return Ok(false);
        }
        let count = components.len() - 1;
        let found = match self.name_prefix(name, scope, count, true)? {
            Some(super::lookup::Prefix::Scope(prefix)) => self.qualified_declaration_result(
                prefix,
                identifier.into(),
                super::lookup::Filter::Ordinary,
            )?,
            Some(super::lookup::Prefix::Dependent(_)) => return Ok(false),
            None => self.unqualified_declaration_result(
                scope,
                identifier.into(),
                super::lookup::Filter::Ordinary,
            )?,
        };
        Ok(found.binding.is_some_and(|binding| {
            let binding = self.graph.binding(binding);
            binding.kind == BindingKind::Alias
                && self.graph.entity(binding.entity).template_head.is_some()
        }))
    }

    pub(super) fn record_instance_declared_type(&mut self, entity: EntityId) -> Result<(), Error> {
        if self.graph.class_templates.aliases.stand_ins.is_empty() {
            return Ok(());
        }
        let instance = self
            .graph
            .class_templates
            .instance(entity)
            .ok_or(Error::Invariant("declaration identity without instance"))?;
        let primary = instance.primary;
        if !instance.dependent
            || self.graph.class_templates.aliases.owner(primary).is_some()
            || self
                .graph
                .class_templates
                .template_entities
                .name(primary)
                .is_some()
            || self
                .graph
                .class_templates
                .dependent_names
                .get(primary)
                .is_some()
        {
            return Ok(());
        }
        let patterns = instance.arguments.clone();
        let arguments = patterns
            .iter()
            .map(|&ty| self.graph.types.declared_type(ty))
            .collect::<Result<Vec<_>, _>>()?;
        if arguments != patterns {
            let representative = self.class_specialization(primary, arguments)?;
            let ty = self
                .graph
                .entity(representative)
                .ty
                .ok_or(Error::Invariant("declaration representative without type"))?;
            self.graph.types.record_declared_class(entity, ty);
        }
        Ok(())
    }

    pub(super) fn own_template_arguments(
        &mut self,
        head: HeadId,
        arguments: &[TypeId],
    ) -> Result<bool, Error> {
        if arguments.len() != self.graph.templates.parameters(head).len() {
            return Ok(false);
        }
        for (index, &argument) in arguments.iter().enumerate() {
            let parameter = self.graph.templates.parameters(head)[index];
            let ty = self
                .graph
                .entity(parameter)
                .ty
                .ok_or(Error::Invariant("template parameter without type"))?;
            let mut pattern = self.graph.types.canonical_function(ty)?;
            if self.argument_parameter_pack(head, index) {
                pattern = self
                    .graph
                    .types
                    .intern(TypeKind::Expansion(pattern), Cv::default())?;
                pattern = self
                    .graph
                    .types
                    .intern(TypeKind::Pack(vec![pattern]), Cv::default())?;
            }
            if pattern != argument {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub(super) fn alias_specialization(
        &mut self,
        alias: EntityId,
        arguments: Vec<TypeId>,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        self.alias_specialization_in(alias, arguments, location, None)
    }

    fn alias_specialization_in(
        &mut self,
        alias: EntityId,
        mut arguments: Vec<TypeId>,
        location: SourceOffset,
        context: Option<EntityId>,
    ) -> Result<TypeId, Error> {
        self.check_stack()?;
        let head = self
            .graph
            .entity(alias)
            .template_head
            .ok_or(Error::Invariant("alias template without head"))?;
        self.fill_template_arguments_in(head, &mut arguments, location, None)?;
        if let Some(failure) = self.graph.analysis.substitution.current().failure {
            return Err(Error::SubstitutionRejected(failure));
        }
        let matched = self.match_argument_list_in(
            head,
            &mut arguments,
            self.argument_match_context(location),
        );
        match matched {
            Ok(Some(())) => (),
            Ok(None) | Err(Error::SubstitutionRejected(_) | Error::SubstitutionNull) => {
                return self.reject_substitution(
                    location,
                    "template arguments that do not match the alias template's parameters",
                    false,
                );
            }
            Err(error) => return Err(error),
        }
        if arguments.len() != self.graph.templates.parameters(head).len() {
            return self.reject_substitution(
                location,
                "a pack expansion for an alias template's parameter that is no pack",
                false,
            );
        }
        let key = Key { alias, arguments };
        if let Some(&ty) = self.graph.class_templates.aliases.specializations.get(&key) {
            return Ok(ty);
        }
        let pack_element = self.graph.class_templates.aliases.type_pack_element == Some(alias);
        if pack_element
            && key
                .arguments
                .iter()
                .all(|&ty| !self.graph.dependent_type(ty))
        {
            let TypeKind::Value { bits, .. } = self.graph.types.get(key.arguments[0]).kind else {
                return self.reject_substitution(
                    location,
                    "a __type_pack_element index outside its pack",
                    false,
                );
            };
            let elements = self.graph.types.pack_elements(key.arguments[1])?;
            let Some(ty) = usize::try_from(bits)
                .ok()
                .and_then(|index| elements.get(index))
                .copied()
            else {
                return self.reject_substitution(
                    location,
                    "a __type_pack_element index outside its pack",
                    false,
                );
            };
            self.specialization_binding(ty, alias)?;
            self.graph
                .class_templates
                .aliases
                .specializations
                .insert(key, ty);
            return Ok(ty);
        }
        let pattern = self
            .graph
            .entity(alias)
            .ty
            .ok_or(Error::Invariant("alias template without type"))?;
        let ty = self.with_alias_substitution_location(location, |analyzer| {
            if analyzer.own_template_arguments(head, &key.arguments)? {
                return Ok(pattern);
            }
            let arguments: Vec<_> = key.arguments.iter().copied().map(Some).collect();
            let mut environment =
                analyzer.substitution(analyzer.graph.templates.depth(head), &arguments);
            environment.context = context;
            analyzer
                .substitute_signature_type(pattern, &mut environment)?
                .ok_or_else(|| environment.failure_error())
        })?;
        let discarded = self
            .graph
            .class_templates
            .aliases
            .discarded
            .get(&alias)
            .copied()
            .unwrap_or(0);
        let dropped = pack_element
            || key
                .arguments
                .iter()
                .take(64)
                .enumerate()
                .any(|(index, &argument)| {
                    discarded >> index & 1 != 0 && self.graph.dependent_type(argument)
                });
        let ty = if dropped {
            let primary = self.alias_stand_in(alias)?;
            let entity = self.class_specialization(primary, key.arguments.clone())?;
            self.graph
                .class_templates
                .aliases
                .aliased
                .entry(entity)
                .or_insert(ty);
            self.graph.types.record_declared_class(entity, ty);
            self.graph
                .entity(entity)
                .ty
                .ok_or(Error::Invariant("alias stand-in without type"))?
        } else {
            ty
        };
        // Internal substitution queries publish the same hidden binding as a
        // written template-id, preserving the first alias's binding ownership.
        self.specialization_binding(ty, alias)?;
        self.graph
            .class_templates
            .aliases
            .specializations
            .insert(key, ty);
        Ok(ty)
    }

    fn alias_stand_in(&mut self, alias: EntityId) -> Result<EntityId, Error> {
        if let Some(&entity) = self.graph.class_templates.aliases.stand_ins.get(&alias) {
            return Ok(entity);
        }
        let name = self.graph.entity(alias).name;
        let entity = self.graph.add_entity(None, name, Keyword::Struct)?;
        let ty = self
            .graph
            .types
            .intern(TypeKind::Class(entity), Cv::default())?;
        self.graph.entity_mut(entity).ty = Some(ty);
        self.graph.entity_mut(entity).home = Some(self.graph.global);
        self.graph
            .class_templates
            .aliases
            .stand_ins
            .insert(alias, entity);
        self.graph
            .class_templates
            .aliases
            .owners
            .insert(entity, alias);
        Ok(entity)
    }

    pub(super) fn substitute_alias_type(
        &mut self,
        entity: EntityId,
        environment: &mut Substitution<'_>,
    ) -> Result<Option<TypeId>, Error> {
        let instance = self
            .graph
            .class_templates
            .instance(entity)
            .ok_or(Error::Invariant("alias substitution without instance"))?;
        let alias = self
            .graph
            .class_templates
            .aliases
            .owner(instance.primary)
            .ok_or(Error::Invariant("alias stand-in without owner"))?;
        let patterns = instance.arguments.clone();
        let Some(arguments) = self.substitute_pack_list(&patterns, environment)? else {
            return Ok(None);
        };
        match self.alias_specialization_in(alias, arguments, SourceOffset(0), environment.context) {
            Ok(ty) => Ok(Some(ty)),
            Err(Error::SubstitutionRejected(failure)) => {
                environment.carry(Some(failure));
                Ok(None)
            }
            Err(Error::SubstitutionNull) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub(super) fn aliased_type(&mut self, mut ty: TypeId) -> Result<TypeId, Error> {
        while let TypeKind::Class(entity) = self.graph.types.get(ty).kind {
            let Some(&aliased) = self.graph.class_templates.aliases.aliased.get(&entity) else {
                break;
            };
            let cv = self.graph.types.get(ty).cv;
            ty = self.graph.types.qualify(aliased, cv)?;
        }
        Ok(ty)
    }

    /// The C++ bounded named-parameter walk treats expression placeholders and
    /// dependent members without specialization arguments as opaque. A shared
    /// DAG is visited only when a deeper path can change the 256-level answer.
    pub(super) fn record_alias_parameters(&mut self, alias: EntityId, head: HeadId, ty: TypeId) {
        let depth = self.graph.templates.depth(head);
        let mut named = 0u64;
        let mut work = vec![(ty, 0u16)];
        let mut visited = HashMap::new();
        let mut children = Vec::new();
        while let Some((ty, level)) = work.pop() {
            if level > 256 {
                return;
            }
            if visited.get(&ty).is_some_and(|&earlier| earlier >= level) {
                continue;
            }
            visited.insert(ty, level);
            children.clear();
            match &self.graph.types.get(ty).kind {
                TypeKind::TemplateParameter { entity, .. } => {
                    if self.graph.template_operands.pattern(*entity).is_some() {
                        return;
                    }
                    if let Some(name) = self.graph.class_templates.dependent_names.get(*entity) {
                        children.push(name.qualifier);
                    } else if let Some(member) = self.graph.dependent_members.get(*entity) {
                        children.push(member.qualifier);
                    } else if let Some(index) = self
                        .graph
                        .entity(*entity)
                        .template_parameter
                        .and_then(|p| p.index(depth))
                        && index < 64
                    {
                        named |= 1u64 << index;
                    }
                }
                TypeKind::Class(entity) => {
                    if let Some(instance) = self
                        .graph
                        .class_templates
                        .instance(*entity)
                        .filter(|i| i.dependent)
                    {
                        children.extend_from_slice(&instance.arguments);
                        if let Some(name) = self
                            .graph
                            .class_templates
                            .template_entities
                            .name(instance.primary)
                        {
                            children.push(name);
                        }
                        if let Some(name) = self
                            .graph
                            .class_templates
                            .dependent_names
                            .get(instance.primary)
                        {
                            children.push(name.qualifier);
                        }
                    } else if self.graph.dependent_type(ty) {
                        return;
                    }
                }
                TypeKind::Enum(_) if self.graph.dependent_type(ty) => return,
                TypeKind::Array {
                    element, operand, ..
                }
                | TypeKind::Vector {
                    element, operand, ..
                } => {
                    if operand.is_some() {
                        return;
                    }
                    children.push(*element);
                }
                TypeKind::MemberPointer { owner, element } => children.extend([*owner, *element]),
                TypeKind::Pointer(element)
                | TypeKind::Reference { element, .. }
                | TypeKind::Expansion(element) => children.push(*element),
                TypeKind::Pack(elements) => children.extend_from_slice(elements),
                TypeKind::Function {
                    result, parameters, ..
                } => {
                    children.push(*result);
                    children.extend_from_slice(parameters);
                }
                _ => (),
            }
            work.extend(children.drain(..).map(|child| (child, level + 1)));
        }
        let count = self.graph.templates.parameters(head).len();
        let mask = if count >= 64 {
            u64::MAX
        } else {
            (1u64 << count) - 1
        };
        self.graph
            .class_templates
            .aliases
            .discarded
            .insert(alias, mask & !named);
    }

    pub(super) fn specialization_binding(
        &mut self,
        ty: TypeId,
        alias: EntityId,
    ) -> Result<BindingId, Error> {
        if self.graph.types.get(ty).cv.0 == 0
            && let TypeKind::Class(class) = self.graph.types.get(ty).kind
            && self.graph.class_templates.instance(class).is_some()
        {
            return self.graph.class_templates.binding(class);
        }
        if let Some(&binding) = self.graph.class_templates.aliases.bindings.get(&ty) {
            return Ok(binding);
        }
        let owner = self.graph.entity(alias);
        let (name, home) = (owner.name, owner.home.unwrap_or(self.graph.global));
        let entity = self.graph.add_entity(Some(ty), name, Keyword::Typedef)?;
        let binding = self.graph.add_hidden_binding_in(
            home,
            Binding {
                kind: BindingKind::Alias,
                name,
                entity,
                ty: Some(ty),
                key: Keyword::Typedef,
            },
        )?;
        self.graph
            .class_templates
            .aliases
            .bindings
            .insert(ty, binding);
        Ok(binding)
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
    fn call_deduction_uses_alias_targets_without_discarding_substitution_obligations() {
        let source = SourceBuffer::new(
            b"template<class>using discard=long;template<class T>using keep=T;\
              template<class T>void f(discard<T>,keep<T>,discard<typename T::bad>);\
              struct X{using bad=int;};"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"alias-deduction").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let x = cursor.intern_name(b"X").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let binding = analyzer
            .graph
            .functions_at(global, f.into())
            .next()
            .unwrap();
        let head = analyzer.function_template_head(binding).unwrap().unwrap();
        let depth = analyzer.graph.templates.depth(head);
        let ty = analyzer.graph.binding(binding).ty.unwrap();
        let parameters: Vec<_> = (0..3)
            .map(|index| analyzer.graph.types.function_parameter(ty, index).unwrap())
            .collect();
        assert!(analyzer.graph.dependent_type(parameters[0]));
        assert!(analyzer.graph.dependent_type(parameters[2]));
        let syntax_mark = analyzer.tree.expressions.mark();
        let binding_mark = analyzer.graph.binding_limit();
        for _ in 0..4 {
            assert!(!analyzer.needs_deduction(parameters[0], depth).unwrap());
            assert!(analyzer.needs_deduction(parameters[1], depth).unwrap());
            assert!(!analyzer.needs_deduction(parameters[2], depth).unwrap());
        }
        assert_eq!(analyzer.tree.expressions.mark(), syntax_mark);
        assert_eq!(analyzer.graph.binding_limit(), binding_mark);
        let int = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(
                crate::types::FundamentalType::Int,
            ))
            .unwrap();
        let invalid = [Some(int)];
        let mut environment = analyzer.substitution(depth, &invalid);
        assert!(
            analyzer
                .with_substitution_soft(None, |analyzer| analyzer
                    .substitute_signature_type(parameters[2], &mut environment))
                .unwrap()
                .is_none()
        );
        let x = analyzer
            .graph
            .own(global, x, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let valid = [analyzer.graph.binding(x).ty];
        let mut environment = analyzer.substitution(depth, &valid);
        assert!(
            analyzer
                .substitute_signature_type(parameters[2], &mut environment)
                .unwrap()
                .is_some()
        );
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn partial_alias_access_uses_primary_friendship_and_keeps_failed_queries_uncached() {
        let source = SourceBuffer::new(
            b"template<class...>using V=void;\
              template<class T>using A=typename T::type;\
              template<class T,class=void>struct D{static const int n=1;};\
              template<class T,class=void>struct E{static const int n=3;};\
              class Hidden{typedef int type;template<class,class>friend struct D;};\
              template<class T>struct D<T,V<A<T>>>{static const int n=2;};\
              template<class T>struct E<T,V<A<T>>>{static const int n=4;};\
              static_assert(E<Hidden>::n==3,\"\");\
              static_assert(D<Hidden>::n==2,\"\");"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"alias-access").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert!(graph.root_demands.as_slice().is_empty());
        assert!(!graph.class_templates.aliases.specializations.is_empty());
    }

    #[test]
    fn defaults_and_cv_keys_reuse_hidden_results_without_definition_demand() {
        let source = SourceBuffer::new(
            b"template<class T>struct Box;\
              template<class T,class U=T*>using A=Box<U>;\
              using X=A<int>;using Y=A<int,int*>;\
              template<class T>using R=T&;\
              using C=R<char* const>;using P=R<const char*>;"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"alias-keys").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let [name_x, name_y, name_c, name_p, name_a] =
            [b"X".as_slice(), b"Y", b"C", b"P", b"A"].map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let syntax_mark = analyzer.tree.expressions.mark();
        let ty = |analyzer: &Analyzer<'_>, name| {
            let binding = analyzer
                .graph
                .own(global, name, super::super::lookup::Filter::Ordinary)
                .unwrap();
            analyzer.graph.binding(binding).ty.unwrap()
        };
        let x = ty(&analyzer, name_x);
        assert_eq!(x, ty(&analyzer, name_y));
        assert_ne!(ty(&analyzer, name_c), ty(&analyzer, name_p));
        let TypeKind::Class(class) = analyzer.graph.types.get(x).kind else {
            panic!("class alias")
        };
        assert!(analyzer.graph.entity(class).scope.is_none());
        assert!(!analyzer.graph.entity(class).complete);
        let a = analyzer
            .graph
            .own(global, name_a, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let alias = analyzer.graph.binding(a).entity;
        let int = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(
                crate::types::FundamentalType::Int,
            ))
            .unwrap();
        for _ in 0..4 {
            let binding_mark = analyzer.graph.binding_limit();
            let result = analyzer
                .alias_specialization(alias, vec![int], SourceOffset(0))
                .unwrap();
            assert_eq!(result, x);
            // The source forms this omitted default in a fresh bound frame
            // before consulting the canonical alias-result cache.
            assert!(analyzer.graph.binding_limit() > binding_mark);
            assert_eq!(
                analyzer.specialization_binding(result, alias).unwrap(),
                analyzer.graph.class_templates.binding(class).unwrap()
            );
        }
        assert_eq!(analyzer.tree.expressions.mark(), syntax_mark);
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn discarded_arguments_keep_substitution_identity_and_declared_class_equivalence() {
        let source = SourceBuffer::new(
            b"template<class T,class>using keep=T;\
              template<class>struct Box;\
              template<class T>Box<keep<T,typename T::missing>> f(T);\
              template<class U>Box<U> f(U);\
              struct X{using missing=int;};"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"alias-dependencies").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name_x = cursor.intern_name(b"X").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let facts = &analyzer.graph.class_templates.aliases;
        assert_eq!(facts.stand_ins.len(), 1);
        assert_eq!(facts.aliased.len(), 1);
        let (&entity, &aliased) = facts.aliased.iter().next().unwrap();
        let pattern = analyzer.graph.entity(entity).ty.unwrap();
        assert_ne!(pattern, aliased);
        assert_eq!(
            analyzer.graph.types.declared_type(pattern).unwrap(),
            analyzer.graph.types.canonical_function(aliased).unwrap()
        );
        let class_patterns: Vec<_> = analyzer
            .graph
            .class_templates
            .aliases
            .specializations
            .values()
            .copied()
            .collect();
        assert!(class_patterns.contains(&pattern));
        let syntax_mark = analyzer.tree.expressions.mark();
        let int = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(
                crate::types::FundamentalType::Int,
            ))
            .unwrap();
        let invalid = [Some(int)];
        let mut rejected = analyzer.substitution(0, &invalid);
        assert!(
            analyzer
                .with_substitution_soft(None, |analyzer| analyzer
                    .substitute_signature_type(pattern, &mut rejected))
                .unwrap()
                .is_none()
        );
        assert_eq!(
            rejected.failure.map(|failure| failure.message),
            Some("a dependent name's qualifier is not a complete class")
        );
        let x = analyzer
            .graph
            .own(global, name_x, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let x = analyzer.graph.binding(x).ty.unwrap();
        let valid = [Some(x)];
        for _ in 0..4 {
            let mut environment = analyzer.substitution(0, &valid);
            assert_eq!(
                analyzer
                    .substitute_signature_type(pattern, &mut environment)
                    .unwrap(),
                Some(x)
            );
        }
        assert_eq!(analyzer.tree.expressions.mark(), syntax_mark);
        assert!(analyzer.graph.entity(entity).scope.is_none());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }
}
