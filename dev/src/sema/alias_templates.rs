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
        mut arguments: Vec<TypeId>,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        self.check_stack()?;
        let head = self
            .graph
            .entity(alias)
            .template_head
            .ok_or(Error::Invariant("alias template without head"))?;
        self.complete_template_arguments(head, &mut arguments, location)?;
        if arguments.len() != self.graph.templates.parameters(head).len() {
            return Err(Error::at(
                location,
                "a pack expansion for an alias template's parameter that is no pack",
            ));
        }
        let key = Key { alias, arguments };
        if let Some(&ty) = self.graph.class_templates.aliases.specializations.get(&key) {
            return Ok(ty);
        }
        let pattern = self
            .graph
            .entity(alias)
            .ty
            .ok_or(Error::Invariant("alias template without type"))?;
        let ty = if self.own_template_arguments(head, &key.arguments)? {
            pattern
        } else {
            let arguments: Vec<_> = key.arguments.iter().copied().map(Some).collect();
            let mut environment = Substitution::new(self.graph.templates.depth(head), &arguments);
            self.substitute_signature_type(pattern, &mut environment)?
                .ok_or_else(|| {
                    environment.failure.map_or(
                        Error::Invariant("alias substitution without failure reason"),
                        |message| {
                            Error::at(
                                if environment.failure_location.0 == 0 {
                                    location
                                } else {
                                    environment.failure_location
                                },
                                message,
                            )
                        },
                    )
                })?
        };
        let discarded = self
            .graph
            .class_templates
            .aliases
            .discarded
            .get(&alias)
            .copied()
            .unwrap_or(0);
        let dropped = key
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
        match self.alias_specialization(alias, arguments, SourceOffset(0)) {
            Ok(ty) => Ok(Some(ty)),
            Err(
                Error::Diagnostic { location, message } | Error::NonConstant { location, message },
            ) => Ok(environment.reject_at(location, message)),
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
                    if let Some(member) = self.graph.dependent_members.get(*entity) {
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
        let binding_mark = analyzer.graph.binding_limit();
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
            let result = analyzer
                .alias_specialization(alias, vec![int], SourceOffset(0))
                .unwrap();
            assert_eq!(result, x);
            assert_eq!(
                analyzer.specialization_binding(result, alias).unwrap(),
                analyzer.graph.class_templates.binding(class).unwrap()
            );
        }
        assert_eq!(analyzer.tree.expressions.mark(), syntax_mark);
        assert_eq!(analyzer.graph.binding_limit(), binding_mark);
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
        let mut rejected = Substitution::new(0, &invalid);
        assert!(
            analyzer
                .substitute_signature_type(pattern, &mut rejected)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            rejected.failure,
            Some("a dependent name's qualifier is not a complete class")
        );
        let x = analyzer
            .graph
            .own(global, name_x, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let x = analyzer.graph.binding(x).ty.unwrap();
        let valid = [Some(x)];
        for _ in 0..4 {
            let mut environment = Substitution::new(0, &valid);
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
