// Translated from symbol_lowering.cpp::function_encoding. See NOTICE.
//! Callable ABI names consume canonical signatures and specialization facts.
use super::{Error, Symbols};
use crate::lower::construction::Variant;
use crate::{
    abi::{
        self,
        graph::{Node, NodeId},
    },
    sema::lowering::{EntityId, FunctionName, TypeView},
};
impl Symbols<'_, '_> {
    pub(in crate::lower) fn function_encoding(
        &mut self,
        entity: EntityId,
    ) -> Result<NodeId, Error> {
        let facts = self.function_facts(entity)?;
        self.encode_function(&facts, Variant::Complete)
    }
    pub(super) fn encode_function(
        &mut self,
        facts: &crate::sema::lowering::FunctionFacts,
        variant: Variant,
    ) -> Result<NodeId, Error> {
        let inherited = facts
            .inherited_constructor
            .map(|root| self.function_facts(root))
            .transpose()?;
        let specialization = inherited.as_ref().unwrap_or(facts);
        let level = if specialization.template_instance.is_some() {
            self.graph.lower_instance_levels(specialization.home)?
        } else {
            0
        };
        let inherited_base = inherited
            .as_ref()
            .map(|root| {
                let class = root.object.ok_or(crate::sema::Error::Invariant(
                    "inherited constructor root without class",
                ))?;
                self.abi_type(self.graph.lower_entity_type(class)?)
            })
            .transpose()?;
        let saved = (self.state.own_level, self.state.bound_pattern);
        self.state.own_level = level;
        self.state.bound_pattern = false;
        let result = self.encode_function_inner(facts, specialization, inherited_base, variant);
        (self.state.own_level, self.state.bound_pattern) = saved;
        let encoding = result?;
        if !self.graph.lower_has_abi_tags() || facts.c_linkage {
            return Ok(encoding);
        }
        let TypeView::Function { result, .. } = self.graph.lower_type(specialization.ty)? else {
            return Err(crate::sema::Error::Invariant("ABI encoding without function type").into());
        };
        let scope_encoding = variant == Variant::Complete;
        if scope_encoding && let Some(owner) = facts.abi_owner {
            self.state.encodings_underway.push((owner, encoding));
        }
        let derived = self.with_implicit_tags(encoding, result);
        if scope_encoding && facts.abi_owner.is_some() {
            self.state.encodings_underway.pop();
        }
        derived
    }

    fn encode_function_inner(
        &mut self,
        facts: &crate::sema::lowering::FunctionFacts,
        specialization: &crate::sema::lowering::FunctionFacts,
        inherited_base: Option<NodeId>,
        variant: Variant,
    ) -> Result<NodeId, Error> {
        if facts.c_linkage
            || matches!(facts.name,
            FunctionName::Identifier(identifier) if self.cursor.name(identifier) == b"main"
                && self.graph.lower_presentation_scope(facts.home).0.is_none())
        {
            let spelling = self.source_function_spelling(facts.name)?;
            let terminal = self.state.abi.string(spelling.as_bytes())?;
            return Ok(self.state.abi.intern(
                Node::Name {
                    parent: None,
                    terminal,
                    internal: false,
                    hidden_tags: false,
                    data_member: false,
                    occurrence: 0,
                },
                &[],
            )?);
        }
        let parent = self.scope(facts.home)?;
        // The frozen encoder uses the primary's written parameter count for
        // operator arity, just as it uses that signature for parameter types.
        // An empty expansion must not turn a binary terminal into a unary one.
        let template = specialization
            .template_instance
            .map(|instance| self.graph.lower_template_signature(instance))
            .transpose()?;
        let signature = template.map_or(facts.ty, |(pattern, _)| pattern);
        let terminal = if matches!(
            facts.name,
            FunctionName::Constructor(_) | FunctionName::Destructor(_)
        ) {
            self.state.abi.intern(
                Node::Terminal {
                    kind: match (facts.name, variant) {
                        (FunctionName::Destructor(_), Variant::Base) => {
                            abi::vocabulary::Terminal::DestructorBase
                        }
                        (FunctionName::Destructor(_), Variant::Deleting) => {
                            abi::vocabulary::Terminal::DestructorDeleting
                        }
                        (FunctionName::Destructor(_), _) => {
                            abi::vocabulary::Terminal::DestructorComplete
                        }
                        (_, Variant::Base) => abi::vocabulary::Terminal::ConstructorBase,
                        _ => abi::vocabulary::Terminal::ConstructorComplete,
                    },
                    inherited: inherited_base,
                },
                &[],
            )?
        } else {
            let name = if matches!(facts.name, FunctionName::Conversion(_)) {
                let TypeView::Function { result, .. } = self.graph.lower_type(signature)? else {
                    return Err(Error::Semantic(crate::sema::Error::Invariant(
                        "conversion without function type",
                    )));
                };
                FunctionName::Conversion(result)
            } else {
                facts.name
            };
            self.function_terminal(name, signature, facts.object.is_some())?
        };
        let name = self.tagged_entity_name(
            Node::Name {
                parent,
                terminal,
                internal: facts.declared_linkage
                    == crate::sema::lowering::DeclaredLinkage::Internal
                    && !self.graph.lower_anonymous_namespace(facts.home),
                hidden_tags: false,
                data_member: false,
                occurrence: 0,
            },
            facts
                .abi_owner
                .map(|owner| self.graph.lower_function_abi_tag_owner(owner)),
        )?;
        let mut name = name;
        if let Some((instance, (_, arguments))) = specialization.template_instance.zip(template) {
            let mut nodes = Vec::with_capacity(arguments.len());
            for (index, &argument) in arguments.iter().enumerate() {
                nodes.push(self.declared_template_argument(instance, index, argument)?);
            }
            name = self.state.abi.intern(Node::Template(name), &nodes)?;
        }
        let TypeView::Function {
            result,
            parameters,
            variadic,
        } = self.graph.lower_type(signature)?
        else {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "function without function type",
            )));
        };
        let parameters = parameters.to_vec();
        let parameters = parameters.as_slice();
        let mut args = Vec::with_capacity(parameters.len());
        for (index, &parameter) in parameters.iter().enumerate() {
            let parameter = if specialization.template_instance.is_some()
                && self.unmangled_placeholder(parameter)?
            {
                let TypeView::Function { parameters, .. } =
                    self.graph.lower_type(specialization.ty)?
                else {
                    return Err(Error::Missing("specialization without function type"));
                };
                let parameters = parameters.to_vec();
                let parameters = parameters.as_slice();
                *parameters
                    .get(index)
                    .ok_or(Error::Missing("specialization parameter count"))?
            } else {
                parameter
            };
            args.push(self.abi_type(parameter)?);
        }
        let result = if specialization.template_instance.is_some()
            && !matches!(
                facts.name,
                FunctionName::Constructor(_)
                    | FunctionName::Destructor(_)
                    | FunctionName::Conversion(_)
            ) {
            let result = if self.unmangled_placeholder(result)? {
                let TypeView::Function { result, .. } = self.graph.lower_type(specialization.ty)?
                else {
                    return Err(Error::Missing("specialization without function type"));
                };
                result
            } else {
                result
            };
            Some(self.abi_type(result)?)
        } else {
            None
        };
        let function = self
            .state
            .abi
            .intern(
                Node::Function {
                    name,
                    result,
                    qualifiers: facts.qualifiers,
                    variadic,
                },
                &args,
            )
            .map_err(Error::from)?;
        Ok(function)
    }
}
