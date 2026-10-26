// Translated from sema/analyzer_expressions.cpp's member_access,
// member_qualifier and member_of_object, and analyzer_access.cpp. See NOTICE.
//! Member designation retains lookup, receiver type/category and naming-class facts.
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

pub(super) enum AccessFailure {
    Inaccessible,
    ProtectedObject,
}

impl AccessFailure {
    pub(super) fn message(&self) -> &'static str {
        match self {
            Self::Inaccessible => "inaccessible member",
            Self::ProtectedObject => "protected member accessed through an object of another class",
        }
    }
}

impl Analyzer<'_> {
    // analyzer_access.cpp::check_function_access: selection identifies an
    // entity, but access belongs to the declaration that introduced it into
    // this lookup set. Specialization identity must not erase a using's path.
    pub(super) fn selected_function_declaration(
        &self,
        selected: BindingId,
        functions: &[BindingId],
    ) -> Option<BindingId> {
        let function = self.graph.binding(selected).entity;
        let declared = self
            .graph
            .function_templates
            .primary(function)
            .map_or(function, |binding| self.graph.binding(binding).entity);
        functions.iter().copied().find(|&binding| {
            let entity = self.graph.binding(binding).entity;
            entity == function || entity == declared
        })
    }

    // analyzer_expressions.cpp::member_templates filters the visible lookup
    // before argument formation. Ordinary declarations can hide base templates.
    fn member_templates(
        &self,
        found: &mut super::lookup::LookupResult,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let templates = if found
            .binding
            .is_some_and(|binding| self.graph.binding(binding).kind == BindingKind::Function)
        {
            found
                .functions
                .iter()
                .copied()
                .filter(|&binding| {
                    self.graph
                        .entity(self.graph.binding(binding).entity)
                        .template_head
                        .is_some()
                })
                .collect::<super::lookup::FunctionCandidates>()
        } else {
            super::lookup::FunctionCandidates::default()
        };
        found.binding = templates.as_slice().first().copied();
        if found.binding.is_none() {
            return Err(Error::at(
                location,
                "a template-id names no member function template",
            ));
        }
        found.functions = templates;
        Ok(())
    }

    pub(super) fn current_object_type(&mut self) -> Result<Option<super::types::TypeId>, Error> {
        if self.default_argument_scope.is_some() {
            return Ok(None);
        }
        if let Some(ty) = self
            .signature_object
            .filter(|_| self.body_context.is_none())
        {
            return Ok(Some(ty));
        }
        if let Some(class) = self.member_initializer_class {
            return Ok(Some(self.graph.lower_entity_type(class)?));
        }
        let Some(body) = &self.body_context else {
            return Ok(None);
        };
        let entity = self.graph.entity(body.entity);
        if let Some(frame) = self.capture_frames.get(body.entity) {
            return Ok(frame.object_type());
        }
        if entity.function.is_some_and(|function| function.is_static) {
            return Ok(None);
        }
        let Some(class) = entity
            .home
            .and_then(|home| self.graph.scope(home).owner)
            .filter(|&class| {
                self.graph
                    .entity(class)
                    .ty
                    .is_some_and(|ty| matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)))
            })
        else {
            return Ok(None);
        };
        let ty = self
            .graph
            .entity(class)
            .ty
            .ok_or(Error::Invariant("object class without type"))?;
        let function = entity
            .ty
            .ok_or(Error::Invariant("member function without type"))?;
        let TypeKind::Function { cv, .. } = self.graph.types.get(function).kind else {
            return Err(Error::Invariant("member without function type"));
        };
        Ok(Some(self.graph.types.qualify(ty, cv)?))
    }

    pub(super) fn implicit_member_object(
        &mut self,
        binding: BindingId,
    ) -> Result<Option<super::expression_facts::ObjectArgument>, Error> {
        let Some(ty) = self.current_object_type()? else {
            return Ok(None);
        };
        let Some(class) = self.graph.class_of(ty) else {
            return Ok(None);
        };
        let member = self.graph.binding(binding).entity;
        let Some(mut home) = self
            .graph
            .entity(member)
            .home
            .and_then(|home| self.graph.scope(home).owner)
        else {
            return Ok(None);
        };
        if self.graph.entity(member).data_member == Some(DataMemberKind::Instance) {
            home = self.graph.member_holder(member, Some(class))?;
        }
        Ok(
            (home == class || self.graph.is_base_of(home, class)).then_some(
                super::expression_facts::ObjectArgument {
                    source: super::expression_facts::ObjectSource::Implicit,
                    ty,
                    category: V::Lvalue,
                },
            ),
        )
    }

    pub(super) fn implicit_data_member(
        &mut self,
        id: ast::ExprId,
        facts: &mut ExprFacts,
        binding: BindingId,
        qualifier: Option<ScopeId>,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let location = self.tree.expressions.get(id).location;
        let Some(object) = self.implicit_member_object(binding)? else {
            return self.evaluated_error(location, "non-static data member used without an object");
        };
        self.capture_this(location)?;
        facts.object = Some(object);
        facts.member = Some(binding);
        let class = self
            .graph
            .class_of(object.ty)
            .ok_or(Error::Invariant("implicit object without class"))?;
        let naming = qualifier
            .filter(|&home| self.graph.scope(home).kind == super::graph::ScopeKind::Class)
            .and_then(|home| self.graph.scope(home).owner)
            .unwrap_or(class);
        self.check_member_subobject(binding, class, naming, scope, location)?;
        facts.member_naming = Some(naming);
        facts.member_path = self.data_member_path(binding, naming, location)?;
        if class != naming {
            facts.member_naming_path = Some(self.graph.retain_base_path(class, naming)?);
        }
        let member = self.graph.entity(self.graph.binding(binding).entity);
        let ty = member.ty.ok_or(Error::Invariant("member without type"))?;
        if !matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
            let cv = self.graph.types.get(object.ty).cv;
            let cv = Cv(cv.0
                & if member.mutability == Mutability::Mutable {
                    !Cv::CONST.0
                } else {
                    u8::MAX
                });
            facts.ty = self.graph.types.qualify(facts.ty, cv)?;
        }
        Ok(())
    }

    pub(super) fn data_member_expression(
        &mut self,
        id: ast::ExprId,
        base: ast::ExprId,
        op: ast::Operator,
        name: ast::Name,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let mut object = self.typed_expression(base, scope)?;
        self.check_resolved_function(base)?;
        let location = self.tree.expressions.get(id).location;
        self.check_temporary_destruction(object, scope, location)?;
        let arrow = op.token.as_punctuator() == Some(P::Arrow);
        let mut ty = object.ty;
        if arrow {
            if matches!(self.graph.types.get(ty).kind, TypeKind::Class(_)) {
                object = self.arrow_calls(id, base, object, scope)?;
                ty = object.ty;
            }
            let pointer = self.decayed_type(ty)?;
            let TypeKind::Pointer(element) = self.graph.types.get(pointer).kind else {
                return Err(Error::at(location, "-> requires a pointer to a class"));
            };
            if self.graph.expression_facts.arrow_calls(base).is_empty() {
                self.convert_operand_scalar(base, pointer, false)?;
            }
            ty = element;
        }
        let node = *self.tree.names.get(name);
        let last = *self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .ok_or(Error::Invariant("empty member name"))?;
        if matches!(
            last.kind,
            NameKind::Destructor(_) | NameKind::DestructorDecltype(_)
        ) {
            return self.destructor_designation(
                id,
                super::expression_facts::ObjectArgument {
                    source: super::expression_facts::ObjectSource::Explicit {
                        expression: base,
                        arrow,
                    },
                    ty,
                    category: if arrow { V::Lvalue } else { object.category },
                },
                name,
                scope,
            );
        }
        let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
            return Err(Error::at(location, "member access on a non-class"));
        };
        self.complete_class(class)?;
        let class_scope = self.graph.entity(class).scope;
        if class_scope.is_none()
            || !self.graph.entity(class).complete
                && !class_scope.is_some_and(|class_scope| self.graph.encloses(class_scope, scope))
        {
            return Err(Error::at(location, "member access on an incomplete class"));
        }
        let search = self.data_member_qualifier(name, class, scope)?;
        self.lookup_location = node.location;
        let mut found = if let NameKind::Conversion(target) = last.kind {
            let target = self.type_syntax(target, scope)?;
            let naming = self
                .graph
                .scope(search)
                .owner
                .ok_or(Error::Invariant("conversion search without class"))?;
            self.named_conversion_lookup(naming, target, node.location)?
        } else {
            let key = Self::expression_name(last.kind)?;
            self.prepare_assignment_lookup(search, key, false)?;
            self.qualified_result(search, key, Filter::Ordinary)
                .map_err(|error| match error {
                    Error::Diagnostic {
                        message: "ambiguous name",
                        ..
                    } => Error::at(node.location, "ambiguous member name"),
                    other => other,
                })?
        };
        if found.binding.is_none() {
            return Err(Error::at(node.location, "no member of that name"));
        }
        if last.arguments.is_some() {
            self.member_templates(&mut found, node.location)?;
            self.explicit_function_specializations(
                ast::NameComponent { location, ..last },
                &mut found,
                scope,
                true,
            )?;
        }
        let binding = found
            .binding
            .ok_or_else(|| Error::at(node.location, "no member of that name"))?;
        let naming = self
            .graph
            .scope(search)
            .owner
            .ok_or(Error::Invariant("member search without class"))?;
        let entity = self.graph.entity(self.graph.binding(binding).entity);
        let shared = !matches!(last.kind, NameKind::Conversion(_))
            || found.functions.iter().all(|&other| {
                self.graph.entity(self.graph.binding(other).entity).home == entity.home
            });
        if shared
            && (entity.data_member == Some(DataMemberKind::Instance)
                || self.member_function_owner(binding).is_some())
        {
            self.check_member_subobject(binding, class, naming, scope, node.location)?;
        }
        if self.graph.binding(binding).kind == BindingKind::Function {
            self.require_named_result(id, found.functions.as_slice())?;
            let mut facts = self.function_designation(binding, found.functions.as_slice())?;
            facts.category = if found.functions.as_slice().len() == 1
                && self.member_function_owner(binding).is_none()
            {
                V::Lvalue
            } else {
                V::Prvalue
            };
            facts.object = Some(super::expression_facts::ObjectArgument {
                source: super::expression_facts::ObjectSource::Explicit {
                    expression: base,
                    arrow,
                },
                ty,
                category: if arrow { V::Lvalue } else { object.category },
            });
            facts.member_naming = Some(naming);
            super::record(&mut self.graph.resolved_names, name.index(), binding);
            return Ok(facts);
        }
        self.check_named_member_access(binding, naming, class, scope, node.location)?;
        if self.lowering()
            && self
                .graph
                .entity(self.graph.binding(binding).entity)
                .data_member
                == Some(DataMemberKind::Instance)
        {
            self.demand_storage_layout(class, location)?;
        }
        super::record(&mut self.graph.resolved_names, name.index(), binding);
        let member = self.graph.binding(binding);
        if member.kind == BindingKind::Variable {
            self.use_instance_variable(member.entity)?;
        }
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
                facts.folded = self.held_integral_constant(value).then_some(value.bits);
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
        facts.set_bit_field(self.graph.entity(member.entity).bit_field.is_some());
        facts.field_identity = self.graph.entity(member.entity).bit_field;
        self.static_member_constant(&mut facts, member.entity);
        facts.member_naming = Some(naming);
        facts.member_path = self.data_member_path(binding, naming, location)?;
        if class != naming
            && self.graph.entity(member.entity).data_member == Some(DataMemberKind::Instance)
        {
            facts.member_naming_path = Some(self.graph.retain_base_path(class, naming)?);
        }
        Ok(facts)
    }

    pub(super) fn data_member_path(
        &mut self,
        binding: BindingId,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<Option<super::inheritance::BasePathId>, Error> {
        let entity = self.graph.binding(binding).entity;
        let member = self.graph.entity(entity);
        if member.data_member != Some(DataMemberKind::Instance) {
            return Ok(None);
        }
        let holder = self.graph.member_holder(entity, Some(class))?;
        if class == holder {
            return Ok(None);
        }
        let path = self.graph.retain_base_path(class, holder)?;
        if self.lowering() {
            self.demand_storage_layout(class, location)?;
        }
        Ok(Some(path))
    }

    // analyzer_member_pointers.cpp::member_subobject: uniqueness is required
    // on each side of the naming class, not directly from object to member.
    // A qualified name can select one of several copies of the member's home.
    pub(super) fn check_member_subobject(
        &mut self,
        binding: BindingId,
        class: EntityId,
        naming: EntityId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let member = self.graph.binding(binding).entity;
        let home = if self.graph.entity(member).data_member == Some(DataMemberKind::Instance) {
            self.graph.member_holder(member, Some(naming))?
        } else {
            self.member_function_owner(binding)
                .and_then(|ty| self.graph.class_of(ty))
                .ok_or(Error::Invariant("nonstatic member without class"))?
        };
        if self.graph.base_count(class, naming) > 1 || self.graph.base_count(naming, home) > 1 {
            return Err(Error::at(
                location,
                "a non-static member of an ambiguous base class subobject",
            ));
        }
        if class != naming {
            let path = self.graph.retain_base_path(class, naming)?;
            self.check_base_path_access(path, scope, location)?;
        }
        Ok(())
    }

    pub(super) fn data_member_qualifier(
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
        let component_count = self.tree.name_lists.get(node.components).len();
        if !node.global && component_count == 1 {
            return Ok(class_scope);
        }
        let mut prefix = node.global.then_some(self.graph.global);
        for index in 0..component_count - 1 {
            let component = self.tree.name_lists.get(node.components)[index];
            let NameKind::Identifier(key) = component.kind else {
                return Err(Error::at(
                    node.location,
                    "unsupported qualified member name",
                ));
            };
            self.lookup_location = node.location;
            let found = if let Some(prefix) = prefix {
                self.qualified_declaration_result(prefix, key.into(), Filter::Prefix)?
                    .binding
            } else {
                let found =
                    self.qualified_declaration_result(class_scope, key.into(), Filter::Prefix)?;
                if let Some(binding) = found.binding {
                    if !found.ambiguous {
                        self.check_named_member_access(
                            binding,
                            class,
                            class,
                            scope,
                            node.location,
                        )?;
                    }
                    Some(binding)
                } else {
                    self.unqualified_declaration_result(scope, key.into(), Filter::Prefix)?
                        .binding
                }
            };
            let mut binding = found
                .ok_or_else(|| Error::at(node.location, "undeclared name in a member access"))?;
            if component.arguments.is_some() {
                binding = self
                    .class_template_id(binding, component, scope)?
                    .ok_or_else(|| {
                        Error::at(
                            component.location,
                            "a template-id in a nested-name-specifier that names no class template",
                        )
                    })?;
            }
            prefix = Some(self.binding_qualifier(binding, node.location)?);
        }
        if !prefix
            .and_then(|scope| self.graph.scope(scope).owner)
            .is_some_and(|base| base == class || self.graph.is_base_of(base, class))
        {
            return Err(Error::at(
                node.location,
                "qualified member name does not name a base class",
            ));
        }
        prefix.ok_or(Error::Invariant("member qualifier without scope"))
    }

    pub(super) fn check_direct_member_access(
        &self,
        binding: BindingId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if !self.direct_member_accessible(binding, scope) {
            return Err(Error::at(location, "inaccessible member"));
        }
        Ok(())
    }

    pub(super) fn check_special_member_access(
        &self,
        binding: BindingId,
        object: EntityId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_direct_member_access(binding, scope, location)?;
        let owner = self
            .graph
            .scope(self.graph.binding_scope(binding))
            .owner
            .ok_or(Error::Invariant("special member without class"))?;
        if self.graph.binding_access(binding) == Access::Protected
            && !self.protected_object_accessible(scope, owner, object)
        {
            return Err(Error::at(
                location,
                "protected member accessed through an object of another class",
            ));
        }
        Ok(())
    }

    pub(super) fn direct_member_accessible(&self, binding: BindingId, scope: ScopeId) -> bool {
        let home = self.graph.binding_scope(binding);
        let owner = self.graph.scope(home).owner;
        self.graph.binding_access(binding) == Access::Public
            || owner.is_some_and(|class| self.member_or_friend(scope, class))
            || self.graph.binding_access(binding) == Access::Protected
                && owner.is_some_and(|class| self.class_context(scope, class))
    }

    pub(super) fn check_named_member_access(
        &mut self,
        binding: BindingId,
        naming: EntityId,
        object: EntityId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_declaration_member_access(binding, naming, Some(object), scope, location)
    }
    pub(super) fn validate_member_access(
        &self,
        binding: BindingId,
        naming: EntityId,
        object: Option<EntityId>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if let Some(failure) = self.member_access_failure(binding, naming, object, scope) {
            return Err(Error::at(location, failure.message()));
        }
        Ok(())
    }
    pub(super) fn member_access_failure(
        &self,
        binding: BindingId,
        naming: EntityId,
        object: Option<EntityId>,
        scope: ScopeId,
    ) -> Option<AccessFailure> {
        self.imported_member_access_failure(binding, naming, object, scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
        types::FundamentalType,
    };

    #[test]
    fn rejected_address_selection_keeps_use_location_without_body_demand() {
        for (declaration, expression, at) in [
            ("template<class T>static int f(T);", "((&B::f))", "((&"),
            ("template<class T>static int f(T);", "((&B::f<int>))", "&B"),
            ("static int f(int);static int f(long);", "((&B::f))", "((&"),
            ("static int f(int);", "((&B::f))", "B::f"),
        ] {
            let text = format!("struct B{{private:{declaration}}};int(*p)(int)={expression};");
            let expected = text.find(at).unwrap();
            let source = SourceBuffer::new(text.into_bytes()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"address-access").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let class_name = cursor.intern_name(b"B").unwrap();
            let function_name = cursor.intern_name(b"f").unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let result = analyzer.declarations(declarations, analyzer.graph.global);
            assert!(
                matches!(result, Err(Error::Diagnostic { location, message })
                if location.index() == expected && message == "inaccessible member")
            );
            let binding = analyzer
                .graph
                .own(analyzer.graph.global, class_name, Filter::Elaborated)
                .unwrap();
            let class = analyzer.graph.binding(binding).entity;
            let scope = analyzer.graph.entity(class).scope.unwrap();
            let function = analyzer
                .graph
                .own(scope, function_name, Filter::Ordinary)
                .unwrap();
            assert!(
                !analyzer
                    .graph
                    .function_templates
                    .body_demanded(analyzer.graph.binding(function).entity)
            );
            assert!(analyzer.graph.root_demands.as_slice().is_empty());
        }
    }

    #[test]
    fn shared_specialization_keeps_lookup_local_using_access_and_receiver_owner() {
        let source = SourceBuffer::new(
            b"struct Base { template<class T> int f(T); };\
              struct Public : private Base { public: using Base::f; };\
              struct Private : private Base { private: using Base::f; };"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"using-access-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let names = [b"Base".as_slice(), b"Public", b"Private", b"f"]
            .map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        for declaration in analyzer.tree.decl_lists.get(declarations).to_vec() {
            analyzer
                .declaration(declaration, analyzer.graph.global)
                .unwrap();
        }
        let classes = [names[0], names[1], names[2]].map(|name| {
            let binding = analyzer
                .graph
                .own(analyzer.graph.global, name, Filter::Elaborated)
                .unwrap();
            analyzer.graph.binding(binding).entity
        });
        let int = analyzer
            .graph
            .types
            .scalar(super::super::types::Scalar::Basic(FundamentalType::Int))
            .unwrap();
        let mut specialization = None;
        for (index, class) in classes.into_iter().enumerate() {
            let scope = analyzer.graph.entity(class).scope.unwrap();
            let found = analyzer
                .qualified_result(scope, names[3].into(), Filter::Ordinary)
                .unwrap();
            let selected = analyzer
                .complete_signature(found.binding.unwrap(), &mut [Some(int)])
                .unwrap()
                .unwrap();
            assert_eq!(*specialization.get_or_insert(selected), selected);
            let declaration = analyzer
                .selected_function_declaration(selected, found.functions.as_slice())
                .unwrap();
            assert_eq!(analyzer.graph.binding_scope(declaration), scope);
            assert!(
                analyzer.graph.binding_access(declaration)
                    == if index == 2 {
                        Access::Private
                    } else {
                        Access::Public
                    }
            );
            assert_eq!(
                analyzer
                    .member_access_failure(declaration, class, Some(class), analyzer.graph.global)
                    .is_none(),
                index != 2
            );
            let owner = analyzer.member_function_owner(selected).unwrap();
            assert_eq!(analyzer.graph.class_of(owner), Some(classes[0]));
            assert!(matches!(
                analyzer.graph.entity(class).layout,
                super::super::layout::LayoutState::NotStarted
            ));
        }
        assert_eq!(expressions, analyzer.tree.expressions.mark());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }
}
