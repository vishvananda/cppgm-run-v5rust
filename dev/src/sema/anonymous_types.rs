// Translated from sema/analyzer_classes.cpp's unnamed type/union declarations. See NOTICE.
//! Declaration-point naming is presentation; injected members share entity identity.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{Binding, BindingKind, EntityId, Graph, ScopeId, ScopeKind},
    specifiers::Specifiers,
    types::{TypeId, TypeKind},
};
use crate::{
    lex::{identifiers::NameId, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::ast,
};

#[derive(Clone, Copy)]
pub(super) enum AnonymousClass {
    Pending(ast::TokenSpan),
    Labeled(ast::TokenSpan),
}
impl AnonymousClass {
    fn tokens(self) -> ast::TokenSpan {
        match self {
            Self::Pending(tokens) | Self::Labeled(tokens) => tokens,
        }
    }
    pub(super) fn label(self) -> Option<ast::TokenSpan> {
        match self {
            Self::Pending(_) => None,
            Self::Labeled(tokens) => Some(tokens),
        }
    }
}

#[derive(Clone, Copy)]
pub enum ClassSpelling {
    Closure(u32),
    Name(NameId),
    Local(u32),
    Anonymous(ast::TokenSpan),
}
#[derive(Clone, Copy)]
pub enum UnnamedNumbering {
    Members(ScopeId),
    Local(u32),
    Private(Option<EntityId>),
}
impl Graph {
    pub(super) fn injected_member_storage(
        &self,
        member: EntityId,
    ) -> Result<Option<super::graph::BindingId>, Error> {
        let Some(home) = self.entity(member).home else {
            return Ok(None);
        };
        if self.scope(home).kind != ScopeKind::Class {
            return Ok(None);
        }
        let holder = self.member_holder(member, None)?;
        Ok(self
            .entity(holder)
            .class()
            .anonymous_storage
            .filter(|&storage| self.scope(self.binding_scope(storage)).kind != ScopeKind::Class))
    }
    #[must_use]
    pub fn lower_declaration_storage(&self, id: ast::DeclId) -> Option<EntityId> {
        self.declaration_storage
            .get(id.index())
            .copied()
            .flatten()
            .map(|binding| self.binding(binding).entity)
    }

    pub fn lower_injected_member(&self, id: ast::ExprId) -> Result<Option<(EntityId, u64)>, Error> {
        let facts = self.expression_facts.get(id)?;
        let Some(storage) = facts.anonymous_member(&self.expression_facts) else {
            return Ok(None);
        };
        let member = self
            .binding(
                facts
                    .member(&self.expression_facts)
                    .ok_or(Error::Invariant("injected member without designation"))?,
            )
            .entity;
        let offset = self
            .lower_member_offset(member)?
            .wrapping_add(self.lower_anonymous_member_offset(member, None)?);
        Ok(Some((self.binding(storage).entity, offset)))
    }

    pub fn lower_injected_storage(&self, id: ast::ExprId) -> Result<Option<EntityId>, Error> {
        Ok(self
            .expression_facts
            .get(id)?
            .anonymous_member(&self.expression_facts)
            .map(|binding| self.binding(binding).entity))
    }

    pub fn lower_union_assignment(&self, id: ast::ExprId) -> Result<bool, Error> {
        let Some(member) = self
            .expression_facts
            .get(id)?
            .member(&self.expression_facts)
        else {
            return Ok(false);
        };
        Ok(self.member_in_union(self.binding(member).entity))
    }

    pub(super) fn member_in_union(&self, member: EntityId) -> bool {
        let mut owner = self
            .entity(member)
            .home
            .and_then(|scope| self.scope(scope).owner);
        while let Some(class) = owner {
            if self.entity(class).key == Keyword::Union {
                return true;
            }
            owner = self
                .anonymous_member_storage(class)
                .and_then(|object| self.entity(object).home)
                .and_then(|scope| self.scope(scope).owner);
        }
        false
    }

    #[must_use]
    pub fn lower_first_named_member(&self, ty: TypeId) -> Option<NameId> {
        let class = self.class_of(ty)?;
        let mut work = vec![self.entity(class).class().members.iter()];
        while let Some(members) = work.last_mut() {
            let Some(&member) = members.next() else {
                work.pop();
                continue;
            };
            let member = self.entity(member);
            if let Some(name) = member.name {
                return Some(name);
            }
            if let Some(class) = member.ty.and_then(|ty| self.class_of(ty))
                && self.entity(class).class().anonymous_class.is_some()
            {
                work.push(self.entity(class).class().members.iter());
            }
        }
        None
    }

    pub fn lower_anonymous_scope(
        &self,
        scope: ScopeId,
    ) -> Result<Option<(EntityId, ast::TokenSpan, UnnamedNumbering)>, Error> {
        let node = self.scope(scope);
        if node.kind != ScopeKind::Class || node.name.is_some() {
            return Ok(None);
        }
        let Some((class, tokens)) = node.owner.and_then(|class| {
            self.entity(class)
                .class()
                .anonymous_class?
                .label()
                .map(|tokens| (class, tokens))
        }) else {
            return Ok(None);
        };
        Ok(Some((class, tokens, self.lower_unnamed_numbering(class)?)))
    }
    pub fn lower_unnamed_numbering(&self, entity: EntityId) -> Result<UnnamedNumbering, Error> {
        if let Some(home) = self.entity(entity).home {
            if self.scope(home).kind == ScopeKind::Class {
                // Unnumbered closures use a private unit name even in a
                // visible class (for example, a static member initializer).
                // They are not unnamed declarations in the class's member list.
                if self.lower_closure(entity).is_some() {
                    return Ok(UnnamedNumbering::Private(None));
                }
                if self
                    .scope(home)
                    .owner
                    .map(|owner| self.lower_visible_type(owner))
                    .transpose()?
                    .unwrap_or(false)
                {
                    return Ok(UnnamedNumbering::Members(home));
                }
                return Ok(UnnamedNumbering::Private(None));
            }
            if self.lower_visible_type(entity)?
                && let Some(ordinal) = self.unnamed_local_ordinal(entity)
            {
                return Ok(UnnamedNumbering::Local(ordinal));
            }
            return Ok(UnnamedNumbering::Private(self.lower_local_scope(home)));
        }
        Ok(UnnamedNumbering::Private(None))
    }
    pub fn lower_class_spelling(&self, class: EntityId) -> Result<ClassSpelling, Error> {
        let entity = self.entity(class);
        if let Some(closure) = self.lower_closure(class) {
            return Ok(ClassSpelling::Closure(closure.ordinal));
        }
        if let Some(name) = entity.name {
            return Ok(ClassSpelling::Name(name));
        }
        if let Some(ordinal) = entity.class().local_type_ordinal {
            return Ok(ClassSpelling::Local(ordinal));
        }
        entity
            .class()
            .anonymous_class
            .and_then(AnonymousClass::label)
            .map(ClassSpelling::Anonymous)
            .ok_or(Error::Invariant("class without presentation provenance"))
    }
    pub fn lower_unnamed_scope_types(&self, scope: ScopeId) -> impl Iterator<Item = EntityId> + '_ {
        self.scope(scope).bindings.iter().filter_map(move |&id| {
            let binding = self.binding(id);
            let entity = self.entity(binding.entity);
            (binding.kind == BindingKind::Type
                && entity.name.is_none()
                && entity.home == Some(scope)
                && entity.ty.is_some_and(|ty| {
                    matches!(
                        self.types.get(ty).kind,
                        TypeKind::Class(_) | TypeKind::Enum(_)
                    )
                }))
            .then_some(binding.entity)
        })
    }

    // Anonymous injection changes the lookup declaration, but the entity keeps
    // its original class. Follow its owned storage to the object being named;
    // only then can the caller select a base route or a union memory fact.
    pub(super) fn anonymous_member_storage(&self, class: EntityId) -> Option<EntityId> {
        let object = self
            .binding(self.entity(class).class().anonymous_storage?)
            .entity;
        let home = self.entity(object).home?;
        (self.scope(home).kind == ScopeKind::Class).then_some(object)
    }

    pub(super) fn member_holder(
        &self,
        member: EntityId,
        within: Option<EntityId>,
    ) -> Result<EntityId, Error> {
        let mut holder = self
            .entity(member)
            .home
            .and_then(|home| self.scope(home).owner)
            .ok_or(Error::Invariant("member without holder"))?;
        while Some(holder) != within {
            let Some(object) = self.anonymous_member_storage(holder) else {
                break;
            };
            holder = self
                .entity(object)
                .home
                .and_then(|home| self.scope(home).owner)
                .ok_or(Error::Invariant("anonymous member without holder"))?;
        }
        Ok(holder)
    }
}

impl Analyzer<'_> {
    pub(super) fn name_local_class(&mut self, ty: TypeId) -> Result<(), Error> {
        let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
            return Ok(());
        };
        if self.semantic()
            && self.graph.entity(class).class().anonymous_class.is_some()
            && self
                .graph
                .entity(class)
                .class()
                .local_type_ordinal
                .is_none()
        {
            self.graph.local_types = self
                .graph
                .local_types
                .checked_add(1)
                .ok_or(Error::Limit("unnamed class ordinal limit"))?;
            self.graph.entity_mut(class).class_mut().local_type_ordinal =
                Some(self.graph.local_types);
        }
        Ok(())
    }
    pub(super) fn simple_declarator_name(
        &self,
        mut id: ast::DeclaratorId,
    ) -> Result<Option<NameId>, Error> {
        loop {
            let node = self.tree.declarators.get(id);
            if let Some(name) = node.name {
                let name = self.tree.names.get(name);
                let components = self.tree.name_lists.get(name.components);
                return Ok(if !name.global && components.len() == 1 {
                    if let ast::NameKind::Identifier(name) = components[0].kind {
                        Some(name)
                    } else {
                        None
                    }
                } else {
                    None
                });
            }
            id = node
                .nested
                .ok_or(Error::Invariant("declarator without name"))?;
        }
    }

    pub(super) fn name_unnamed_class(&mut self, ty: TypeId, name: Option<NameId>) {
        let TypeKind::Class(entity) = self.graph.types.get(ty).kind else {
            return;
        };
        let Some(anonymous) = self.graph.entity(entity).class().anonymous_class else {
            return;
        };
        if self.graph.entity(entity).name.is_some() {
            return;
        }
        if let Some(name) = name {
            self.graph.entity_mut(entity).name = Some(name);
            if let Some(scope) = self.graph.entity(entity).context().scope {
                self.graph.scope_mut(scope).name = Some(name);
            }
        } else {
            self.graph.entity_mut(entity).class_mut().anonymous_class =
                Some(AnonymousClass::Labeled(anonymous.tokens()));
        }
        // As in name_unnamed_type, the placeholder's printed name changes,
        // without publishing a class-name lookup binding or injected-class-name.
    }

    pub(super) fn anonymous_union(
        &mut self,
        specs: Specifiers,
        scope: ScopeId,
        location: SourceOffset,
        after_semicolon: bool,
    ) -> Result<Option<super::graph::BindingId>, Error> {
        let Some(declared) = specs.declared else {
            return Ok(None);
        };
        let TypeKind::Class(entity) = self.graph.types.get(declared).kind else {
            return Ok(None);
        };
        let Some(anonymous) = self.graph.entity(entity).class().anonymous_class else {
            return Ok(None);
        };
        if self.graph.entity(entity).name.is_some() {
            return Ok(None);
        }
        if specs.virtuality == super::specifiers::Virtuality::Virtual {
            return Err(Error::at(
                location,
                "virtual in a declaration that declares no function",
            ));
        }
        if specs.constexpr {
            return Err(Error::at(
                location,
                "constexpr in a declaration that declares no variable or function",
            ));
        }
        let kind = self.graph.scope(scope).kind;
        if self.graph.entity(entity).key != Keyword::Union && kind != ScopeKind::Class {
            return Err(Error::at(location, "declaration does not declare anything"));
        }
        let unnamed_namespace = kind == ScopeKind::Namespace
            && scope != self.graph.global
            && self.graph.scope(scope).name.is_none();
        if kind == ScopeKind::Namespace && !specs.is_static && !unnamed_namespace {
            return Err(Error::at(
                location,
                "anonymous union in a named or the global namespace must be static",
            ));
        }
        let mut tokens = anonymous.tokens();
        if after_semicolon {
            // The class-key-only declaration's label is assigned after its ';'.
            tokens.end = tokens
                .end
                .checked_add(1)
                .ok_or(Error::Limit("token ordinal limit"))?;
        }
        self.graph.entity_mut(entity).class_mut().anonymous_class =
            Some(AnonymousClass::Labeled(tokens));
        let class = self
            .graph
            .entity(entity)
            .context()
            .scope
            .ok_or(Error::Invariant("anonymous class without scope"))?;
        // Read only this class's source-order bindings. No enclosing-scope scan.
        for index in 0..self.graph.scope(class).bindings.len() {
            let member = self.graph.binding(self.graph.scope(class).bindings[index]);
            if member.kind == BindingKind::Type && self.graph.entity(member.entity).name.is_none() {
                continue;
            }
            if member.kind != BindingKind::Variable {
                return Err(Error::at(
                    location,
                    "an anonymous union can only have non-static data members",
                ));
            }
            if member
                .name
                .is_some_and(|name| self.graph.scope(scope).index.contains_key(&name))
            {
                return Err(Error::at(
                    location,
                    "anonymous union member conflicts with a declaration in the enclosing scope",
                ));
            }
            self.graph.add_binding(scope, Some(scope), member)?;
        }
        if kind == ScopeKind::Class {
            let owner = self
                .graph
                .scope(scope)
                .owner
                .ok_or(Error::Invariant("class without owner"))?;
            let storage = self.graph.add_entity(Some(specs.ty), None, Keyword::Int)?;
            self.graph.entity_mut(storage).member_access = self.graph.scope(scope).member_access;
            self.graph
                .entity_mut(owner)
                .class_mut()
                .members
                .push(storage);
            if self.graph.scope(scope).member_access != super::graph::Access::Public {
                self.graph.entity_mut(owner).class_mut().layout_category =
                    super::layout::LayoutCategory::NonPod;
            }
            let binding = self.graph.add_binding(
                scope,
                None,
                Binding {
                    kind: BindingKind::Variable,
                    name: None,
                    entity: storage,
                    ty: Some(specs.ty),
                    key: Keyword::Int,
                },
            )?;
            self.graph.entity_mut(entity).class_mut().anonymous_storage = Some(binding);
            return Ok(None);
        }
        if !self.semantic() || self.template_depth != 0 {
            return Ok(None);
        }
        // Hidden storage has a typed binding, but no ordinary-name or print entry.
        // Its label is derived only in dump views from the class's token provenance.
        let storage = self.graph.add_entity(Some(specs.ty), None, Keyword::Int)?;
        self.graph.entity_mut(storage).defined = true;
        self.graph.entity_mut(storage).object_mut().variable =
            Some(super::graph::VariableProperties {
                is_extern: false,
                internal_linkage: kind == ScopeKind::Namespace,
                // The frozen anonymous_union owner creates this hidden object
                // without inheriting a surrounding linkage specification.
                language: super::linkage::Language::Cpp,
                // analyzer_classes.cpp publishes no thread-storage flag on this hidden object.
                thread_storage: super::specifiers::ThreadStorage::Ordinary,
                static_specifier: if specs.is_static {
                    super::specifiers::StaticSpecifier::Present
                } else {
                    super::specifiers::StaticSpecifier::Absent
                },
                automatic: kind == ScopeKind::Block && !specs.is_static,
            });
        let binding = self.graph.add_binding(
            scope,
            None,
            Binding {
                kind: BindingKind::Variable,
                name: None,
                entity: storage,
                ty: Some(specs.ty),
                key: Keyword::Int,
            },
        )?;
        self.graph.entity_mut(entity).class_mut().anonymous_storage = Some(binding);
        self.default_initialize(binding, location)?;
        if self.lowering() {
            self.request_union_storage(storage, location);
        }
        Ok(Some(binding))
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
    fn injected_members_retain_one_storage_owner_without_reparsing_or_unused_layouts() {
        let source = SourceBuffer::new(b"static union { struct { char c; union{long value;int second;}; }; int other; }; long*const p=&value; int f(){value=7;return value;} inline int unused(){static union{long hidden=3;};return hidden;}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"union-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let syntax_declarations = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), syntax_declarations);
        let storage = graph
            .lower_declaration_storage(tree.decl_lists.get(declarations)[0])
            .unwrap();
        let facts = graph.lower_variable(storage).unwrap();
        assert!(facts.name.is_none());
        assert_eq!(
            cursor.name(graph.lower_first_named_member(facts.ty).unwrap()),
            b"c"
        );
        let mut uses = Vec::new();
        let mut hidden = None;
        let pointer = cursor.intern_name(b"p").unwrap();
        let pointer = graph
            .binding(
                graph
                    .own(
                        graph.global,
                        pointer,
                        super::super::lookup::Filter::Ordinary,
                    )
                    .unwrap(),
            )
            .entity;
        assert!(matches!(graph.lower_variable(pointer).unwrap().initializer,
            Some(super::super::static_initialization::Value::Address(object, 8)) if object == storage));
        for &decl in &tree.decl_lists.get(declarations)[2..] {
            let ast::DeclKind::Function { body, .. } = tree.declarations.get(decl).kind else {
                panic!("function")
            };
            let ast::Body::Parsed(body) = tree.bodies.get(body) else {
                panic!("parsed body")
            };
            let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind
            else {
                panic!("compound")
            };
            for &stmt in tree.stmt_lists.get(statements) {
                match tree.statements.get(stmt).kind {
                    ast::StmtKind::Expression(Some(expr)) => {
                        let ast::ExprKind::Binary { left, .. } = tree.expressions.get(expr).kind
                        else {
                            panic!("assignment")
                        };
                        uses.push(left);
                    }
                    ast::StmtKind::Return(Some(expr)) if uses.len() == 1 => uses.push(expr),
                    ast::StmtKind::Return(Some(expr)) => hidden = Some(expr),
                    _ => {}
                }
            }
        }
        for id in uses {
            assert_eq!(graph.lower_injected_member(id).unwrap(), Some((storage, 8)));
            assert!(graph.lower_union_assignment(id).unwrap());
        }
        let object = graph
            .lower_injected_storage(hidden.unwrap())
            .unwrap()
            .unwrap();
        assert_ne!(object, storage);
        assert!(graph.lower_array_image(object).is_none());
        let class = graph
            .class_of(graph.lower_entity_type(object).unwrap())
            .unwrap();
        assert!(matches!(
            graph.entity(class).class().layout,
            super::super::layout::LayoutState::NotStarted
        ));
    }
}
