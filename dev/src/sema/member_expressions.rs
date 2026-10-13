// Translated from sema/analyzer_expressions.cpp's member_access,
// member_qualifier and member_of_object, and analyzer_access.cpp. See NOTICE.
//! Data-member designation reads class-local lookup and publishes original-AST facts.
use super::{
    Error,
    analyzer::Analyzer,
    constants::ConstantState,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::{Access, BindingId, BindingKind, DataMemberKind, EntityId, Mutability, ScopeId},
    lookup::Filter,
    types::{Cv, TypeKind},
};
use crate::{
    lex::pp_lexer::Punctuator as P,
    support::source::SourceOffset,
    syntax::ast::{self, NameKind},
};

impl Analyzer<'_> {
    pub(super) fn data_member_expression(
        &mut self,
        id: ast::ExprId,
        base: ast::ExprId,
        op: ast::Operator,
        name: ast::Name,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let object = self.typed_expression(base, scope)?;
        self.check_resolved_function(base)?;
        let location = self.tree.expressions.get(id).location;
        let arrow = op.token.as_punctuator() == Some(P::Arrow);
        let mut ty = object.ty;
        if arrow {
            if matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)) {
                return Err(Error::Missing("class operator-> calls"));
            }
            let pointer = self.decayed_type(ty)?;
            let TypeKind::Pointer(element) = self.graph.types.get(pointer).kind else {
                return Err(Error::at(location, "-> requires a pointer to a class"));
            };
            self.convert_operand_scalar(base, pointer, false)?;
            ty = element;
        }
        let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
            return Err(Error::at(location, "member access on a non-class"));
        };
        let class_scope = self.graph.entity(class).scope;
        if class_scope.is_none()
            || !self.graph.entity(class).complete
                && !class_scope.is_some_and(|class_scope| self.graph.encloses(class_scope, scope))
        {
            return Err(Error::at(location, "member access on an incomplete class"));
        }
        let node = *self.tree.names.get(name);
        let last = *self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .ok_or(Error::Invariant("empty member name"))?;
        let NameKind::Identifier(key) = last.kind else {
            return Err(Error::Missing("operator and special member access"));
        };
        if last.arguments.is_some() {
            return Err(Error::Missing("member template specialization lookup"));
        }
        let search = self.data_member_qualifier(name, class, scope)?;
        self.lookup_location = node.location;
        let found = self.qualified_result(search, key.into(), Filter::Ordinary)?;
        let binding = found
            .binding
            .ok_or_else(|| Error::at(node.location, "no member of that name"))?;
        if self.graph.binding(binding).kind == BindingKind::Function {
            return Err(Error::Missing("bound member function designation"));
        }
        self.check_direct_member_access(binding, scope, node.location)?;
        super::record(&mut self.graph.resolved_names, name.index(), binding);
        let member = self.graph.binding(binding);
        let entity = self.graph.entity(member.entity);
        let mut facts = match member.kind {
            BindingKind::Variable => {
                let mut member_ty = entity.ty.ok_or(Error::Invariant("member without type"))?;
                let mut category = if arrow || object.category == V::Lvalue {
                    V::Lvalue
                } else {
                    V::Xvalue
                };
                if let TypeKind::Reference { element, .. } = self.graph.types.get(member_ty).kind {
                    member_ty = element;
                    category = V::Lvalue;
                } else if entity.data_member == Some(DataMemberKind::Instance) {
                    let cv = self.graph.types.get(ty).cv;
                    let cv = Cv(cv.0
                        & if entity.mutability == Mutability::Mutable {
                            !Cv::CONST.0
                        } else {
                            u8::MAX
                        });
                    member_ty = self.graph.types.qualify(member_ty, cv)?;
                }
                if self.graph.entity(member.entity).data_member == Some(DataMemberKind::Static) {
                    category = V::Lvalue;
                }
                self.new_expression_facts(member_ty, category)?
            }
            BindingKind::Enumerator => {
                let ConstantState::Value(value) = entity.constant else {
                    return Err(Error::Invariant("enumerator member without value"));
                };
                let mut facts = self.new_expression_facts(value.ty, V::Prvalue)?;
                facts.folded = Some(value.bits);
                facts
            }
            _ => {
                return Err(Error::at(
                    location,
                    "member name does not denote a data member or member function",
                ));
            }
        };
        facts.member = Some(binding);
        Ok(facts)
    }

    fn data_member_qualifier(
        &mut self,
        name: ast::Name,
        class: EntityId,
        scope: ScopeId,
    ) -> Result<ScopeId, Error> {
        let class_scope = self
            .graph
            .entity(class)
            .scope
            .ok_or(Error::Invariant("member class without scope"))?;
        let node = *self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        if !node.global && components.len() == 1 {
            return Ok(class_scope);
        }
        let mut prefix = node.global.then_some(self.graph.global);
        for component in &components[..components.len() - 1] {
            if component.arguments.is_some() {
                return Err(Error::Missing(
                    "template specialization in a member qualifier",
                ));
            }
            let NameKind::Identifier(key) = component.kind else {
                return Err(Error::at(
                    node.location,
                    "unsupported qualified member name",
                ));
            };
            self.lookup_location = node.location;
            let found = if let Some(prefix) = prefix {
                self.qualified(prefix, key, Filter::Prefix)?
            } else if let Some(binding) = self.qualified(class_scope, key, Filter::Prefix)? {
                self.check_direct_member_access(binding, scope, node.location)?;
                Some(binding)
            } else {
                self.unqualified(scope, key, Filter::Prefix)?
            };
            let binding = found
                .ok_or_else(|| Error::at(node.location, "undeclared name in a member access"))?;
            let binding = self.graph.binding(binding);
            prefix = self.graph.entity(binding.entity).scope.or_else(|| {
                binding
                    .ty
                    .and_then(|ty| match self.graph.types.get(ty).kind {
                        TypeKind::Class(class) | TypeKind::Enum(class) => {
                            self.graph.entity(class).scope
                        }
                        _ => None,
                    })
            });
            if prefix.is_none() {
                return Err(Error::at(
                    node.location,
                    "name before :: is not a class, namespace or enumeration",
                ));
            }
        }
        if prefix != Some(class_scope) {
            return Err(Error::at(
                node.location,
                "qualified member name does not name a base class",
            ));
        }
        Ok(class_scope)
    }

    pub(super) fn check_direct_member_access(
        &self,
        binding: BindingId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let home = self.graph.binding_scope(binding);
        if self.graph.binding_access(binding) != Access::Public && !self.graph.encloses(home, scope)
        {
            return Err(Error::at(location, "inaccessible member"));
        }
        Ok(())
    }
}
