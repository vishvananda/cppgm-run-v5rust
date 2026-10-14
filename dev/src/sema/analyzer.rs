// Translated from sema/analyzer.cpp's PA6 AnalysisMode::Types path. See NOTICE.
use super::{
    Error,
    graph::{Binding, BindingKind, Graph, ScopeId, ScopeKind},
    types::{Scalar, TypeId},
};
use crate::{
    lex::vocabulary::Keyword,
    support::{source::SourceOffset, stack::StackBudget},
    syntax::{
        ast::{self, Tree},
        cursor::Cursor,
        storage::List,
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AnalysisMode {
    Types,
    Semantics,
    Lowering,
}

pub(crate) struct Analyzer<'tree> {
    pub(super) literal_bytes: &'tree crate::syntax::storage::TextStore,
    pub(super) tree: &'tree Tree,
    pub(super) graph: Graph,
    pub(super) stack_budget: StackBudget,
    pub(super) stack_base: usize,
    pub(super) lookup_location: SourceOffset,
    pub(super) deferred: Vec<DeferredMember>,
    pub(super) anonymous_enums: u32,
    pub(super) local_types: u32,
    pub(super) class_depth: usize,
    pub(super) template_depth: usize,
    pub(super) c_language: bool,
    pub(super) linkage_specification: Option<super::linkage::Specification>,
    pub(super) block_linkages: super::linkage::BlockLinkages,
    pub(super) template_head: Option<super::templates::HeadId>,
    pub(super) constant_spine: Vec<ast::ExprId>,
    pub(super) constant_discard_work: Vec<ast::ExprId>,
    pub(super) constant_evaluation: Option<super::constant_addresses::Evaluation>,
    pub(super) constant_scratch: super::constant_addresses::Evaluation,
    pub(super) expression_work: Vec<(ast::ExprId, bool)>,
    pub(super) suppress_implicit: bool,
    mode: AnalysisMode,
    pub(super) static_initializers: Vec<(super::graph::EntityId, ast::Initializer, ScopeId)>,
    pub(super) body_context: Option<super::procedural_statements::BodyContext>,
    pub(super) default_argument_scope: Option<ScopeId>,
    pub(super) unevaluated: u32,
    pub(super) builtin_names: super::builtin_calls::BuiltinNames,
    pub(super) call_scratch: super::procedural_calls::CallScratch,
    pub(super) copy_scratch: super::copy_initialization::CopyScratch,
    pub(super) binary_spine: Vec<ast::ExprId>,
    pub(super) specifications: Vec<super::exceptions::PendingSpecification>,
    pub(super) pending_throws: Vec<super::exceptions::SpecificationId>,
    pub(super) evaluating_noexcept: bool,
    pub(super) layout_work: Vec<(super::graph::EntityId, usize)>,
    pub(super) exception_work: Vec<(ast::ExprId, super::exception_operands::OperandStep)>,
}
#[derive(Clone, Copy)]
pub(super) struct DeferredBody {
    pub entity: super::graph::EntityId,
    pub name: Option<crate::lex::identifiers::NameId>,
    pub ty: TypeId,
    pub parameters: ast::ParameterClause,
    pub body: ast::BodyId,
    pub parent: ScopeId,
    pub print: ScopeId,
    pub template_depth: usize,
}
#[derive(Clone, Copy)]
pub(super) enum DeferredMember {
    Body(DeferredBody),
    Defaults(super::default_arguments::PendingDefaults),
}
impl<'tree> Analyzer<'tree> {
    pub(crate) fn new(
        tree: &'tree Tree,
        cursor: &'tree mut Cursor<'_>,
        stack_budget: StackBudget,
    ) -> Result<Self, Error> {
        let name = cursor
            .intern_name(b"nullptr_t")
            .map_err(|_| Error::Limit("semantic identifier limit"))?;
        let mut graph = Graph::new();
        let ty = graph.types.scalar(Scalar::Nullptr)?;
        let entity = graph.add_entity(Some(ty), Some(name), Keyword::Typedef)?;
        graph.add_binding(
            graph.global,
            None,
            Binding {
                kind: BindingKind::Alias,
                name: Some(name),
                entity,
                ty: Some(ty),
                key: Keyword::Typedef,
            },
        )?;
        let builtin_names = super::builtin_calls::BuiltinNames {
            abort: cursor
                .intern_name(b"__builtin_abort")
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            constant_p: cursor
                .intern_name(b"__builtin_constant_p")
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
        };
        let mut analyzer = Self {
            literal_bytes: &cursor.spellings,
            tree,
            graph,
            stack_budget,
            stack_base: 0,
            lookup_location: SourceOffset(0),
            deferred: Vec::new(),
            anonymous_enums: 0,
            local_types: 0,
            class_depth: 0,
            template_depth: 0,
            c_language: false,
            linkage_specification: None,
            block_linkages: super::linkage::BlockLinkages::default(),
            template_head: None,
            constant_spine: Vec::new(),
            constant_discard_work: Vec::new(),
            constant_evaluation: None,
            constant_scratch: super::constant_addresses::Evaluation::default(),
            expression_work: Vec::new(),
            suppress_implicit: false,
            mode: AnalysisMode::Types,
            static_initializers: Vec::new(),
            body_context: None,
            binary_spine: Vec::new(),
            call_scratch: super::procedural_calls::CallScratch::default(),
            copy_scratch: super::copy_initialization::CopyScratch::default(),
            builtin_names,
            default_argument_scope: None,
            unevaluated: 0,
            specifications: Vec::new(),
            pending_throws: Vec::new(),
            evaluating_noexcept: false,
            layout_work: Vec::new(),
            exception_work: Vec::new(),
        };
        analyzer.predeclare_allocations()?;
        Ok(analyzer)
    }
    pub(crate) fn set_semantics(&mut self, enabled: bool) {
        self.mode = if enabled {
            AnalysisMode::Semantics
        } else {
            AnalysisMode::Types
        };
    }
    pub(crate) fn set_lowering(&mut self) {
        self.mode = AnalysisMode::Lowering;
    }
    pub(super) fn lowering(&self) -> bool {
        self.mode == AnalysisMode::Lowering
    }
    pub(super) fn semantic(&self) -> bool {
        self.mode != AnalysisMode::Types
    }
    pub(crate) fn analyze(mut self, declarations: List<ast::DeclId>) -> Result<Graph, Error> {
        let marker = 0u8;
        self.stack_base = std::ptr::from_ref(&marker).addr();
        self.declarations(declarations, self.graph.global)?;
        self.finish_unit_exceptions()?;
        if !self.deferred.is_empty() {
            return Err(Error::Invariant("uncompleted member bodies"));
        }
        self.finish_static_initializers()?;
        Ok(self.graph)
    }
    pub(super) fn check_stack(&self) -> Result<(), Error> {
        let marker = 0u8;
        if self
            .stack_budget
            .exceeded(self.stack_base, std::ptr::from_ref(&marker).addr())
        {
            return Err(Error::Limit("constructs nested too deeply"));
        }
        Ok(())
    }
    pub(super) fn declarations(
        &mut self,
        list: List<ast::DeclId>,
        scope: ScopeId,
    ) -> Result<(), Error> {
        for &id in self.tree.decl_lists.get(list) {
            self.declaration(id, scope)?;
        }
        Ok(())
    }
    pub(super) fn declaration(&mut self, id: ast::DeclId, scope: ScopeId) -> Result<(), Error> {
        self.check_stack()?;
        let declaration = *self.tree.declarations.get(id);
        let location = declaration.location;
        if self.graph.scope(scope).kind == ScopeKind::Class && !declaration.attributes.is_empty() {
            let owner = self
                .graph
                .scope(scope)
                .owner
                .ok_or(Error::Invariant("class scope without owner"))?;
            self.graph.entity_mut(owner).layout = super::layout::LayoutState::Unsupported;
        }
        let specifiers = match declaration.kind {
            ast::DeclKind::Simple { specifiers, .. }
            | ast::DeclKind::Function { specifiers, .. }
            | ast::DeclKind::SpecialMember { specifiers, .. } => Some(specifiers),
            _ => None,
        };
        if let Some(owner) = self.graph.scope(scope).owner
            && specifiers.is_some_and(|list| {
                self.tree
                    .specs
                    .get(list)
                    .iter()
                    .any(|spec| matches!(spec.kind, ast::SpecifierKind::Keyword(Keyword::Virtual)))
            })
        {
            self.graph.entity_mut(owner).layout = super::layout::LayoutState::Unsupported;
        }
        match declaration.kind {
            ast::DeclKind::Empty => Ok(()),
            ast::DeclKind::Access(keyword) => {
                self.graph.scope_mut(scope).member_access = match keyword {
                    Keyword::Private => super::graph::Access::Private,
                    Keyword::Protected => super::graph::Access::Protected,
                    _ => super::graph::Access::Public,
                };
                Ok(())
            }
            ast::DeclKind::Namespace {
                name,
                inline,
                declarations,
            } => self.namespace_definition(name, inline, declarations, scope, location),
            ast::DeclKind::NamespaceAlias { name, target } => {
                self.namespace_alias(name, target, scope, location)
            }
            ast::DeclKind::Using {
                directive, target, ..
            } => self.using_declaration(directive, target, scope, location),
            ast::DeclKind::Alias { name, ty } => {
                let node = *self.tree.types.get(ty);
                let exceptions = self.declarator_specifies_exceptions(node.declarator);
                let specs = self.specifiers(node.specifiers, scope, false)?;
                let alias_template = self.active_template_head(scope).is_some();
                if alias_template
                    && self
                        .tree
                        .specs
                        .get(node.specifiers)
                        .iter()
                        .any(|specifier| match specifier.kind {
                            ast::SpecifierKind::Class(_) => true,
                            ast::SpecifierKind::Enum(id) => {
                                matches!(
                                    self.tree.declarations.get(id).kind,
                                    ast::DeclKind::Enum { has_body: true, .. }
                                )
                            }
                            _ => false,
                        })
                {
                    return Err(Error::at(
                        location,
                        "a class or enumeration defined in an alias template",
                    ));
                }
                let alias_type = self.declarator(node.declarator, specs.ty, scope)?.ty;
                super::record(&mut self.graph.type_facts, ty.index(), alias_type);
                if exceptions {
                    return Err(Error::at(
                        location,
                        "an exception-specification in an alias-declaration",
                    ));
                }
                if let Some(declared) = specs.declared {
                    self.name_unnamed_enum(declared, None)?;
                    self.name_unnamed_class(declared, None);
                }
                if alias_template {
                    self.declare_alias_template(name, alias_type, scope, location)
                        .map(|_| ())
                } else {
                    self.declare_alias(name, alias_type, scope, location)
                        .map(|_| ())
                }
            }
            ast::DeclKind::Simple {
                specifiers,
                declarators,
            } => {
                let specs = self.specifiers(specifiers, scope, declarators.is_empty())?;
                let first_name = self
                    .tree
                    .init_declarators
                    .get(declarators)
                    .first()
                    .map(|init| self.simple_declarator_name(init.declarator))
                    .transpose()?
                    .flatten();
                let naming = if specs.role == super::specifiers::DeclarationRole::Typedef
                    || !self.semantic() && self.graph.scope(scope).kind != ScopeKind::Class
                {
                    first_name
                } else {
                    None
                };
                if let Some(declared) = specs.declared {
                    self.name_unnamed_enum(declared, naming)?;
                    self.name_unnamed_class(declared, naming);
                    if !declarators.is_empty() {
                        self.name_local_class(declared)?;
                    }
                }
                if declarators.is_empty()
                    && let Some(storage) = self.anonymous_union(specs, scope, location, false)?
                {
                    super::record(&mut self.graph.declaration_storage, id.index(), storage);
                }
                for init in self.tree.init_declarators.get(declarators) {
                    self.declare_init(specs, *init, scope, location)?;
                }
                Ok(())
            }
            ast::DeclKind::Class(specifier) => {
                let ty = self.specifier_type(specifier, scope, true)?;
                let storage = self.anonymous_union(
                    super::specifiers::Specifiers {
                        inline: super::specifiers::InlinePreference::Unspecified,
                        ty,
                        declared: Some(ty),
                        role: super::specifiers::DeclarationRole::Ordinary,
                        is_extern: false,
                        is_static: false,
                        mutability: super::graph::Mutability::Ordinary,
                        virtuality: super::specifiers::Virtuality::NonVirtual,
                        constexpr: false,
                        explicitness: super::specifiers::Explicitness::Converting,
                        thread_storage: super::specifiers::ThreadStorage::Ordinary,
                    },
                    scope,
                    location,
                    true,
                )?;
                if let Some(storage) = storage {
                    super::record(&mut self.graph.declaration_storage, id.index(), storage);
                }
                Ok(())
            }
            ast::DeclKind::Function {
                specifiers,
                declarator,
                body,
            } => self.function_definition(id, specifiers, declarator, body, scope, location),
            ast::DeclKind::Linkage {
                language,
                declarations,
                braced,
            } => {
                let saved = self.c_language;
                let saved_specification = self.linkage_specification;
                self.c_language = self.literal_bytes.get(language) == b"C";
                self.linkage_specification = Some(if braced {
                    super::linkage::Specification::Block
                } else {
                    super::linkage::Specification::Single
                });
                let result = self.declarations(declarations, scope);
                self.c_language = saved;
                self.linkage_specification = saved_specification;
                result
            }
            ast::DeclKind::Enum { .. } => {
                let ty = self.enum_type(id, scope, true)?;
                self.name_unnamed_enum(ty, None)
            }
            ast::DeclKind::StaticAssert { condition, message } => {
                if message.is_some_and(|literal| {
                    matches!(
                        literal.kind,
                        crate::lex::PostTokenKind::String {
                            suffix: Some(_),
                            ..
                        }
                    )
                }) {
                    return Err(Error::at(
                        location,
                        "a static_assert message that is not a string literal",
                    ));
                }
                if self.template_depth != 0 {
                    return self.expression(condition, scope);
                }
                let value = self
                    .constant_value(condition, scope, true)?
                    .ok_or(Error::Invariant("missing assertion value"))?;
                if self.scoped_enum(value.ty) {
                    return Err(Error::at(
                        self.tree.expressions.get(condition).location,
                        "a scoped enumeration value does not convert to bool",
                    ));
                }
                if value.bits == 0 {
                    return Err(Error::at(location, "static assertion failed"));
                }
                Ok(())
            }
            ast::DeclKind::Template {
                parameters,
                declaration,
            } => self.template_declaration(parameters, declaration, scope, location),
            ast::DeclKind::ExplicitInstantiation { .. } => {
                Err(Error::Missing("explicit template instantiation"))
            }
            ast::DeclKind::SpecialMember { .. } => self.special_member(id, scope),
            ast::DeclKind::BitField { .. } => Err(Error::Missing("bit-field declarations")),
        }
    }
    fn function_definition(
        &mut self,
        declaration: ast::DeclId,
        specifiers: List<ast::Specifier>,
        declarator: ast::DeclaratorId,
        body: ast::BodyId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let specs = self.specifiers(specifiers, scope, false)?;
        let mut environment = self.declarator_environment(Some(declarator), scope)?;
        environment.declaration_location = Some(location);
        let mut info = self.declarator_in(Some(declarator), specs.ty, environment)?;
        if let Some((class, kind)) = self.declared_member(&info, scope)? {
            return self.complete_special_member(
                class,
                kind,
                &info,
                specs,
                scope,
                super::special_members::MemberDeclaration {
                    location,
                    name: info
                        .syntax_name
                        .ok_or(Error::Invariant("special member without name"))?,
                    declarator,
                    initializer: None,
                    body: Some(body),
                    implicit_type: false,
                },
            );
        }
        if self.conversion_declarator(&info) {
            return Err(Error::at(
                location,
                "a conversion function with a return type",
            ));
        }
        let exception_limit = self.graph.binding_limit();
        let binding = self.declare_function_value(
            &mut info,
            specs,
            scope,
            location,
            super::function_names::Definition::Body,
        )?;
        let home = info.home.unwrap_or(self.declaration_scope(scope));
        super::record(
            &mut self.graph.declared_bindings,
            declarator.index(),
            binding,
        );
        let entity = self.graph.binding(binding).entity;
        self.graph.entity_mut(entity).function_definition = Some(declaration);
        self.graph.entity_mut(entity).definition_emission =
            if self.class_depth != 0 || specs.constexpr {
                super::function_demands::DefinitionEmission::Demanded
            } else {
                super::function_demands::DefinitionEmission::Always
            };
        self.graph.entity_mut(entity).constexpr = specs.constexpr;
        self.record_function_exceptions(entity, &info, info.lookup, location, exception_limit)?;
        if self.graph.entity(entity).defined {
            return Err(Error::at(location, "redefinition of a function"));
        }
        self.graph.entity_mut(entity).defined = true;
        if self.graph.function_templates.body_demanded(entity) {
            return Err(Error::Missing(
                "demanded function template body instantiation",
            ));
        }
        let parameters = info
            .parameters
            .ok_or(Error::Invariant("function definition without parameters"))?;
        let deferred = DeferredBody {
            entity,
            name: info.name,
            ty: info.ty,
            parameters,
            body,
            template_depth: self.template_depth,
            parent: info.lookup,
            print: if info.home.is_some() { home } else { scope },
        };
        if self.class_depth == 0 {
            self.function_body(deferred)
        } else {
            self.deferred.push(DeferredMember::Body(deferred));
            Ok(())
        }
    }
    pub(super) fn function_body(&mut self, body: DeferredBody) -> Result<(), Error> {
        let saved = self.template_depth;
        let saved_class_depth = self.class_depth;
        self.template_depth = body.template_depth;
        self.class_depth = 0;
        let saved_context = self.body_context.take();
        if self.semantic() {
            let super::types::TypeKind::Function { result, .. } =
                self.graph.types.get(body.ty).kind
            else {
                return Err(Error::Invariant("body without function type"));
            };
            self.body_context = Some(super::procedural_statements::BodyContext::new(
                result,
                body.entity,
            ));
        }
        let result = self.function_body_scopes(body);
        self.body_context = saved_context;
        self.template_depth = saved;
        self.class_depth = saved_class_depth;
        result
    }
    fn function_body_scopes(&mut self, body: DeferredBody) -> Result<(), Error> {
        let scope = self.graph.add_scope(
            ScopeKind::Function,
            body.parent,
            Some(body.print),
            body.name,
        )?;
        self.graph.scope_mut(scope).owner = Some(body.entity);
        self.graph.entity_mut(body.entity).scope = Some(scope);
        let count = self.graph.types.function_parameter_count(body.ty)?;
        for (index, parameter) in self
            .tree
            .parameters
            .get(body.parameters.parameters)
            .iter()
            .take(count)
            .enumerate()
        {
            let ty = self.graph.types.function_parameter(body.ty, index)?;
            let name = self.declarator_name(parameter.declarator)?;
            if let Some(name) = name
                && self
                    .graph
                    .own(scope, name, super::lookup::Filter::Ordinary)
                    .is_some()
            {
                let location = parameter
                    .declarator
                    .map(|id| self.declarator_location(id))
                    .transpose()?
                    .unwrap_or(parameter.location);
                return Err(Error::at(location, "duplicate parameter name"));
            }
            let entity = self.graph.add_entity(Some(ty), name, Keyword::Int)?;
            self.graph.add_binding(
                scope,
                Some(scope),
                Binding {
                    kind: BindingKind::Parameter,
                    name,
                    entity,
                    ty: Some(ty),
                    key: Keyword::Int,
                },
            )?;
            self.note_nothrow_target(entity, ty, parameter.declarator, scope)?;
        }
        let ast::Body::Parsed(parsed) = self.tree.bodies.get(body.body) else {
            return Err(Error::Invariant("unparsed body at semantic boundary"));
        };
        if self.semantic() && self.template_depth == 0 {
            self.require_function_definition_types(
                body.ty,
                self.tree.statements.get(parsed.compound).location,
            )?;
        }
        if parsed.handlers.is_some() && self.graph.entity(body.entity).constexpr {
            return Err(Error::at(
                self.tree.statements.get(parsed.compound).location,
                "a constexpr function with a function-try-block",
            ));
        }
        if self.semantic()
            && self.template_depth == 0
            && let Some(key) = self.graph.entity(body.entity).member_function
            && key.kind == super::graph::MemberKind::Destructor
        {
            self.require_scalar_destruction(key.class)?;
        }
        if self.semantic()
            && self.template_depth == 0
            && self
                .graph
                .entity(body.entity)
                .member_function
                .is_some_and(|key| key.kind == super::graph::MemberKind::Constructor)
        {
            self.written_constructor_plan(body.entity, parsed, scope)?;
        } else {
            for initializer in self.tree.mem_initializers.get(parsed.initializers) {
                self.initializer_names(initializer.initializer, scope, body.ty)?;
            }
        }
        self.substatement(parsed.compound, scope)?;
        if let Some(handlers) = parsed.handlers {
            self.handlers(handlers, scope)?;
        }
        if self.semantic() {
            self.check_gotos()?;
        }
        Ok(())
    }
}
