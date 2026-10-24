// Translated from class_templates.cpp::match_template_arguments and
// template_packs.cpp::pack_expansion_type/bind_pack/pack_size. See NOTICE.
//! Canonical argument partitions and frame bindings, independent of body copies.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{Binding, BindingKind, Graph, ScopeId},
    lookup::Filter,
    templates::HeadId,
    types::{Cv, TypeId, TypeKind},
};
use crate::{lex::identifiers::NameId, support::source::SourceOffset};

impl super::types::Types {
    /// A specialization's canonical partitions as a written template-id list.
    /// Only the outer argument packs flatten; a nested pack remains an element.
    pub(super) fn flattened_arguments(&self, arguments: &[TypeId]) -> Vec<TypeId> {
        let mut result = Vec::with_capacity(arguments.len());
        for &argument in arguments {
            if let TypeKind::Pack(elements) = &self.get(argument).kind {
                result.extend_from_slice(elements);
            } else {
                result.push(argument);
            }
        }
        result
    }
}

impl Graph {
    pub(super) fn instance_levels(&self, mut scope: Option<ScopeId>) -> Result<u32, Error> {
        let mut levels = 0u32;
        while let Some(current) = scope {
            let node = self.scope(current);
            if node.owner.is_some_and(|owner| {
                !self.class_templates.definitions.explicit(owner)
                    && self
                        .class_templates
                        .instance(owner)
                        .is_some_and(|instance| !instance.dependent)
            }) {
                levels = levels
                    .checked_add(1)
                    .ok_or(Error::Limit("instance levels"))?;
            }
            scope = node.parent;
        }
        Ok(levels)
    }
}

impl Analyzer<'_> {
    pub(super) fn template_argument_parameter(
        &self,
        head: HeadId,
        index: usize,
    ) -> Option<super::templates::ParameterInfo> {
        let parameters = self.graph.templates.parameters(head);
        let parameter = parameters.get(index).or_else(|| {
            parameters.last().filter(|&&parameter| {
                self.graph
                    .entity(parameter)
                    .template_parameter
                    .is_some_and(super::templates::ParameterInfo::is_pack)
            })
        })?;
        self.graph.entity(*parameter).template_parameter
    }

    pub(super) fn argument_parameter_pack(&self, head: HeadId, index: usize) -> bool {
        self.graph
            .templates
            .parameters(head)
            .get(index)
            .is_some_and(|&parameter| {
                self.graph
                    .entity(parameter)
                    .template_parameter
                    .is_some_and(super::templates::ParameterInfo::is_pack)
            })
    }
    pub(super) fn trailing_argument_pack(&self, head: HeadId) -> bool {
        self.graph
            .templates
            .parameters(head)
            .len()
            .checked_sub(1)
            .is_some_and(|index| self.argument_parameter_pack(head, index))
    }

    pub(super) fn pack_expansion_type(
        &mut self,
        pattern: TypeId,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        if let TypeKind::BoundPack { arguments, .. } = self.graph.types.get(pattern).kind {
            return Ok(arguments);
        }
        self.check_pack_pattern(pattern, location)?;
        self.graph
            .types
            .intern(TypeKind::Expansion(pattern), Cv::default())
    }

    #[inline]
    pub(super) fn match_argument_list(
        &mut self,
        head: HeadId,
        given: &mut Vec<TypeId>,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let count = self.graph.templates.parameters(head).len();
        if (0..count).any(|index| self.argument_parameter_pack(head, index))
            || given
                .iter()
                .any(|&ty| matches!(self.graph.types.get(ty).kind, TypeKind::Expansion(_)))
        {
            *given = self.match_pack_arguments(head, given, location, false)?;
        } else {
            if given.len() > count {
                return Err(Error::at(location, "too many template arguments"));
            }
            for index in 0..given.len() {
                given[index] = self.match_template_argument(
                    head,
                    index,
                    given[index],
                    &given[..index],
                    location,
                )?;
            }
        }
        Ok(())
    }

    // Keep partition scratch off the recursive, pack-free argument path.
    #[inline(never)]
    pub(super) fn match_pack_arguments(
        &mut self,
        head: HeadId,
        given: &[TypeId],
        location: SourceOffset,
        partial: bool,
    ) -> Result<Vec<TypeId>, Error> {
        let count = self.graph.templates.parameters(head).len();
        let mut result = Vec::with_capacity(count);
        let mut next = 0;
        for index in 0..count {
            let parameter = self.graph.templates.parameters(head)[index];
            let info = self
                .graph
                .entity(parameter)
                .template_parameter
                .ok_or(Error::Invariant("argument without parameter"))?;
            if info.is_pack() {
                if partial && next == given.len() {
                    break;
                }
                let formed = if let Some(&argument) = given.get(next)
                    && let TypeKind::Pack(elements) = &self.graph.types.get(argument).kind
                {
                    next += 1;
                    Some((argument, elements.len()))
                } else {
                    None
                };
                let size = formed.map_or(given.len() - next, |(_, size)| size);
                let mut converted = Vec::with_capacity(size);
                let pattern = info
                    .value_type
                    .filter(|&ty| self.graph.dependent_type(ty))
                    .map(|ty| match self.graph.types.get(ty).kind {
                        TypeKind::Expansion(pattern) => pattern,
                        _ => ty,
                    });
                let earlier = pattern.map(|_| result.iter().copied().map(Some).collect::<Vec<_>>());
                for position in 0..size {
                    let element = if let Some((pack, _)) = formed {
                        self.graph.types.pack_elements(pack)?[position]
                    } else {
                        given[next + position]
                    };
                    let target = if let Some((pattern, arguments)) = pattern.zip(earlier.as_deref())
                    {
                        let selection = super::function_templates::PackSelection {
                            elements: &[],
                            bound_element: Some(position),
                        };
                        let mut environment = super::function_templates::Substitution::new(
                            self.graph.templates.depth(head),
                            arguments,
                        );
                        environment.selection = Some(&selection);
                        Some(
                            self.substitute_signature_type(pattern, &mut environment)?
                                .ok_or(Error::at(
                                    location,
                                    "a template argument does not match its template parameter",
                                ))?,
                        )
                    } else {
                        None
                    };
                    converted.push(self.match_template_argument_to(
                        head, index, element, &result, location, target,
                    )?);
                }
                if formed.is_none() {
                    next = given.len();
                }
                result.push(
                    self.graph
                        .types
                        .intern(TypeKind::Pack(converted), Cv::default())?,
                );
            } else if let Some(&argument) = given.get(next) {
                if let TypeKind::Expansion(pattern) = self.graph.types.get(argument).kind {
                    // The frozen owner checks only value versus non-value
                    // here. Expansion positions and template-name kind checks
                    // remain deferred until the concrete list is substituted.
                    if self.value_argument(pattern) != info.value_type.is_some() {
                        return Err(Error::at(
                            location,
                            "a pack expansion of the wrong kind for its template parameter",
                        ));
                    }
                    // Positions remain unknown until this list is substituted.
                    return given
                        .iter()
                        .map(|&ty| self.graph.types.canonical_function(ty))
                        .collect();
                }
                result
                    .push(self.match_template_argument(head, index, argument, &result, location)?);
                next += 1;
            } else {
                break;
            }
        }
        if next != given.len() {
            return Err(Error::at(location, "too many template arguments"));
        }
        Ok(result)
    }

    pub(super) fn value_argument(&self, mut argument: TypeId) -> bool {
        if let TypeKind::Expansion(pattern) = self.graph.types.get(argument).kind {
            argument = pattern;
        }
        match self.graph.types.get(argument).kind {
            TypeKind::Value { .. } | TypeKind::Address { .. } => true,
            TypeKind::TemplateParameter { entity, .. }
            | TypeKind::BoundPack {
                parameter: entity, ..
            } => {
                self.graph
                    .entity(entity)
                    .template_parameter
                    .is_some_and(|info| info.value_type.is_some())
                    || self
                        .graph
                        .template_operands
                        .pattern(entity)
                        .is_some_and(|pattern| pattern.value)
            }
            _ => false,
        }
    }

    #[inline(never)]
    pub(super) fn bind_argument_pack(
        &mut self,
        scope: ScopeId,
        binding: Binding,
        arguments: TypeId,
    ) -> Result<(), Error> {
        let info = self
            .graph
            .entity(binding.entity)
            .template_parameter
            .ok_or(Error::Invariant("pack binding without parameter"))?;
        let levels = self.graph.instance_levels(self.graph.scope(scope).parent)?;
        let parameter_type = self
            .graph
            .entity(binding.entity)
            .ty
            .ok_or(Error::Invariant("bound pack parameter without type"))?;
        let canonical = self.graph.types.canonical_function(parameter_type)?;
        let TypeKind::TemplateParameter {
            entity: parameter, ..
        } = self.graph.types.get(canonical).kind
        else {
            return Err(Error::Invariant(
                "bound pack parameter without positional type",
            ));
        };
        let ty = self.graph.types.intern(
            TypeKind::BoundPack {
                arguments,
                parameter,
                depth: info
                    .depth()
                    .checked_add(levels)
                    .ok_or(Error::Limit("bound pack depth"))?,
            },
            Cv::default(),
        )?;
        self.graph.add_binding(
            scope,
            None,
            Binding {
                kind: BindingKind::Alias,
                ty: Some(ty),
                ..binding
            },
        )?;
        let count = self.graph.types.pack_elements(arguments)?.len();
        if let Some(name) = binding.name {
            for index in 0..count {
                let element = self.graph.types.pack_elements(arguments)?[index];
                let mut spelling = self.identifiers.bytes(name).to_vec();
                spelling.extend_from_slice(format!("__pack{index}").as_bytes());
                let name = self.identifiers.intern(&spelling).map_err(Error::Limit)?;
                self.bind_template_argument(
                    scope,
                    Binding {
                        name: Some(name),
                        ..binding
                    },
                    element,
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn pack_size(&self, name: NameId, scope: ScopeId) -> Result<Option<u64>, Error> {
        let Some(binding) = self
            .unqualified_result(scope, name.into(), Filter::Ordinary)?
            .binding
        else {
            return Ok(None);
        };
        let binding = self.graph.binding(binding);
        if binding.kind != BindingKind::Alias {
            return Ok(None);
        }
        let Some(mut ty) = binding.ty else {
            return Ok(None);
        };
        if let TypeKind::BoundPack { arguments, .. } = self.graph.types.get(ty).kind {
            ty = arguments;
        }
        if let TypeKind::Pack(elements) = &self.graph.types.get(ty).kind {
            Ok(Some(
                u64::try_from(elements.len()).map_err(|_| Error::Limit("pack size"))?,
            ))
        } else {
            Ok(None)
        }
    }

    pub(super) fn check_sizeof_pack_name(
        &self,
        name: NameId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let found = self
            .unqualified_result(scope, name.into(), Filter::Ordinary)?
            .binding;
        let declared_pack = found.is_some_and(|id| {
            let binding = self.graph.binding(id);
            self.graph
                .entity(binding.entity)
                .template_parameter
                .is_some_and(super::templates::ParameterInfo::is_pack)
                || binding.kind == BindingKind::Parameter
                    && binding.ty.is_some_and(|ty| {
                        matches!(self.graph.types.get(ty).kind, TypeKind::Expansion(_))
                    })
        });
        if declared_pack || self.pack_size(name, scope)?.is_some() {
            Ok(())
        } else {
            Err(Error::at(
                location,
                "sizeof... of a name that is no parameter pack",
            ))
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
    fn partitions_are_canonical_and_definition_demand_keeps_pattern_facts_private() {
        let source = SourceBuffer::new(
            b"template<class T=int,class... U>struct Box{\
            static const unsigned long count=sizeof...(U);void unused(){T::missing;}};\
            using X=Box<int,char,long>;using Y=Box<int,char,long>;using Z=Box<>;\
            template<int... N>struct Values;using A=Values<3>;using B=Values<3L>;\
            using Empty=Values<>;"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"pack-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [
            b"X".as_slice(),
            b"Y",
            b"Z",
            b"A",
            b"B",
            b"Empty",
            b"U",
            b"U__pack0",
            b"count",
        ]
        .map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let classes = names[..6]
            .iter()
            .map(|&name| {
                let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
                let TypeKind::Class(class) = analyzer
                    .graph
                    .types
                    .get(analyzer.graph.binding(binding).ty.unwrap())
                    .kind
                else {
                    panic!("class alias")
                };
                class
            })
            .collect::<Vec<_>>();
        assert_eq!(classes[0], classes[1]);
        assert_eq!(classes[3], classes[4]);
        assert_ne!(classes[3], classes[5]);
        for &class in &classes {
            assert!(analyzer.graph.entity(class).scope.is_none());
        }
        let instance = analyzer.graph.class_templates.instance(classes[0]).unwrap();
        assert_eq!(instance.arguments.len(), 2);
        let pack = instance.arguments[1];
        assert!(
            matches!(&analyzer.graph.types.get(pack).kind, TypeKind::Pack(elements) if elements.len() == 2)
        );
        let head = analyzer
            .graph
            .entity(instance.primary)
            .template_head
            .unwrap();
        let parameter = analyzer.graph.templates.parameters(head)[1];
        let parameter_ty = analyzer.graph.entity(parameter).ty;
        assert!(analyzer.complete_class(classes[0]).unwrap());
        assert!(analyzer.complete_class(classes[0]).unwrap());
        assert!(analyzer.graph.entity(classes[2]).scope.is_none());
        assert_eq!(analyzer.graph.entity(parameter).ty, parameter_ty);
        let class_scope = analyzer.graph.entity(classes[0]).scope.unwrap();
        let frame = analyzer.graph.scope(class_scope).parent.unwrap();
        let bound = analyzer
            .graph
            .own(frame, names[6], Filter::Ordinary)
            .unwrap();
        let binding = analyzer.graph.binding(bound);
        assert_eq!(binding.kind, BindingKind::Alias);
        assert!(
            matches!(analyzer.graph.types.get(binding.ty.unwrap()).kind, TypeKind::BoundPack {arguments, ..} if arguments == pack)
        );
        assert_eq!(analyzer.pack_size(names[6], frame).unwrap(), Some(2));
        assert!(
            analyzer
                .graph
                .own(frame, names[7], Filter::Ordinary)
                .is_some()
        );
        let count = analyzer
            .graph
            .own(class_scope, names[8], Filter::Ordinary)
            .unwrap();
        assert!(
            matches!(analyzer.graph.entity(analyzer.graph.binding(count).entity).constant,
            super::super::constants::ConstantState::Value(value) if value.bits == 2)
        );
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }
}
