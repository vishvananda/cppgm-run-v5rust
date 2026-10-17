// Translated from lower/symbol_lowering.cpp::declared_argument and head_declaration. See NOTICE.
//! Declaration grammar distinguishes overloaded function-template arguments.
use super::{EntityId, Error, Node, NodeId, Symbols, TypeId, TypeView};
use crate::sema::lowering::TemplateDeclaration;

impl Symbols<'_, '_> {
    pub(super) fn declared_template_argument(
        &mut self,
        instance: EntityId,
        index: usize,
        argument: TypeId,
    ) -> Result<NodeId, Error> {
        let mut node = self.abi_type(argument)?;
        let Some(parameter) = self.graph.lower_template_parameter_entity(instance, index) else {
            return Ok(node);
        };
        let (declaration, is_pack, depth) = self.graph.lower_template_declaration(parameter)?;
        let pack = self.graph.lower_argument_pack(argument);
        let empty = pack.is_some_and(<[TypeId]>::is_empty);
        let ty = match declaration {
            TemplateDeclaration::Type => return Ok(node),
            TemplateDeclaration::Value(ty) => ty,
            TemplateDeclaration::Template => {
                let first = if is_pack {
                    pack.and_then(|items| items.first().copied())
                } else {
                    Some(argument)
                };
                let head = first.and_then(|ty| self.graph.lower_argument_template(ty));
                let declaration = self.parameter_declaration(parameter)?;
                if let Some(head) = head
                    && self.graph.lower_template_head_parameters(head).is_some()
                    && self.head_declaration(
                        head,
                        is_pack,
                        depth
                            .checked_add(1)
                            .ok_or(Error::Missing("ABI template head level"))?,
                    )? == declaration
                {
                    return Ok(node);
                }
                return self
                    .abi
                    .intern(
                        Node::TypedArgument {
                            declaration,
                            argument: node,
                        },
                        &[],
                    )
                    .map_err(Error::from);
            }
        };
        if self.graph.lower_names_bound_pack(ty)
            && let Some(elements) = pack
        {
            // An expanded non-type pack declares each element's concrete
            // type. A flat declaration slice avoids recursion proportional
            // to pack arity in the ABI encoder.
            let mut declarations = Vec::with_capacity(elements.len());
            for position in 0..elements.len() {
                let element = self
                    .graph
                    .lower_argument_pack(argument)
                    .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                        "lost ABI pack",
                    )))?[position];
                let TypeView::Value { ty, .. } = self.graph.lower_type(element)? else {
                    return Err(Error::Missing(
                        "an expanded template parameter pack's argument is not a value",
                    ));
                };
                let ty = self.abi_type(ty)?;
                let declaration = self.abi.intern(Node::NonTypeParameter(ty), &[])?;
                declarations.push(declaration);
            }
            if let Some((&declaration, rest)) = declarations.split_first() {
                node = self.abi.intern(
                    Node::TypedArgument {
                        declaration,
                        argument: node,
                    },
                    rest,
                )?;
            }
            return Ok(node);
        }
        if self.declared_type_dependent(ty)? || empty {
            let declaration = self.parameter_declaration(parameter)?;
            node = self.abi.intern(
                Node::TypedArgument {
                    declaration,
                    argument: node,
                },
                &[],
            )?;
        }
        Ok(node)
    }
    fn parameter_declaration(&mut self, parameter: EntityId) -> Result<NodeId, Error> {
        self.check_stack()?;
        let key = (parameter, self.bound_pattern, self.own_level);
        if let Some(&node) = self.parameter_declarations.get(&key) {
            return Ok(node);
        }
        let (declaration, pack, _) = self.graph.lower_template_declaration(parameter)?;
        let node = match declaration {
            TemplateDeclaration::Template => self.head_declaration(parameter, pack, 0)?,
            TemplateDeclaration::Type => {
                let node = self.abi.intern(Node::TypeParameter, &[])?;
                self.parameter_pack(node, pack)?
            }
            TemplateDeclaration::Value(ty) => {
                let ty = self.abi_type(ty)?;
                let node = self.abi.intern(Node::NonTypeParameter(ty), &[])?;
                self.parameter_pack(node, pack)?
            }
        };
        self.parameter_declarations.insert(key, node);
        Ok(node)
    }

    fn parameter_pack(&mut self, node: NodeId, pack: bool) -> Result<NodeId, Error> {
        if pack {
            self.abi
                .intern(Node::ParameterPack(node), &[])
                .map_err(Error::from)
        } else {
            Ok(node)
        }
    }

    fn head_declaration(
        &mut self,
        owner: EntityId,
        pack: bool,
        level: u32,
    ) -> Result<NodeId, Error> {
        self.check_stack()?;
        let key = (owner, pack, level);
        if let Some(&node) = self.head_declarations.get(&key) {
            return Ok(node);
        }
        let saved = (self.own_level, self.bound_pattern);
        self.own_level = level;
        self.bound_pattern = true;
        let result = self.head_declaration_inner(owner, pack);
        (self.own_level, self.bound_pattern) = saved;
        let node = result?;
        self.head_declarations.insert(key, node);
        Ok(node)
    }

    fn head_declaration_inner(&mut self, owner: EntityId, pack: bool) -> Result<NodeId, Error> {
        let graph = self.graph;
        let parameters = graph
            .lower_template_head_parameters(owner)
            .ok_or(Error::Missing("ABI template declaration head"))?;
        let mut nodes = Vec::with_capacity(parameters.len());
        for &parameter in parameters {
            nodes.push(self.parameter_declaration(parameter)?);
        }
        let node = self.abi.intern(Node::TemplateParameter, &nodes)?;
        self.parameter_pack(node, pack)
    }
}
