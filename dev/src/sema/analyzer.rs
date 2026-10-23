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

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum SpecializationMode {
    #[default]
    Ordinary,
    Explicit,
}

pub(crate) struct Analyzer<'tree> {
    pub(super) int128_name: crate::lex::identifiers::NameId,
    pub(super) decltype_name: crate::lex::identifiers::NameId,
    pub(super) override_name: crate::lex::identifiers::NameId,
    pub(super) exception_overrides: Vec<super::virtual_exceptions::PendingOverride>,
    pub(super) deferred_exception_overrides: Vec<super::virtual_exceptions::PendingOverride>,
    pub(super) declaration_access: super::declaration_access::DeclarationAccess,
    pub(super) literal_bytes: &'tree crate::syntax::storage::TextStore,
    pub(super) literal_scanner: Box<crate::types::floating::FloatingScanner>,
    pub(super) identifiers: &'tree mut crate::lex::identifiers::Identifiers,
    pub(super) tree: &'tree mut Tree,
    pub(super) graph: Graph,
    pub(super) stack_budget: StackBudget,
    pub(super) stack_base: usize,
    pub(super) lookup_location: SourceOffset,
    pub(super) deferred: Vec<DeferredMember>,
    pub(super) deferred_constexpr: std::collections::HashMap<super::graph::EntityId, DeferredBody>,
    pub(super) class_depth: usize,
    pub(super) member_initializer_class: Option<super::graph::EntityId>,
    pub(super) base_object: Option<super::graph::EntityId>,
    pub(super) template_depth: usize,
    pub(super) deduced_pack_element: Option<usize>,
    pub(super) specialization_levels: u32,
    pub(super) specialization_mode: SpecializationMode,
    pub(super) instantiation_depth: u32,
    pub(super) incomplete_class_observations: u64,
    pub(super) dependent_uses: std::cell::Cell<u64>,
    pub(super) value_parameter_uses: std::cell::Cell<u64>,
    pub(super) c_language: bool,
    pub(super) linkage_specification: Option<super::linkage::Specification>,
    pub(super) template_head: Option<super::templates::HeadId>,
    pub(super) template_owner: Option<super::graph::EntityId>,
    pub(super) operand_home: Option<ScopeId>,
    pub(super) namespace_friend_class: Option<ScopeId>,
    pub(super) constant_spine: Vec<ast::ExprId>,
    pub(super) constant_discard_work: Vec<ast::ExprId>,
    pub(super) constant_evaluation: Option<super::constant_addresses::Evaluation>,
    pub(super) constexpr_frames: super::constexpr_frames::Frames,
    pub(super) constant_scratch: super::constant_addresses::Evaluation,
    pub(super) expression_work: Vec<(ast::ExprId, bool)>,
    pub(super) fixed_validation_work: Vec<(ast::ExprId, u16)>,
    pub(super) suppress_implicit: bool,
    mode: AnalysisMode,
    pub(super) initializing: Option<super::graph::EntityId>,
    pub(super) initializing_declarator: Option<ast::DeclaratorId>,
    // Origin of storage-only initializer analysis. Unlike expression context,
    // this survives nested instantiations until an actual body owns the use.
    pub(super) initializer_demand_owner: Option<super::graph::EntityId>,
    pub(super) parenthesized_elements: std::collections::HashSet<ast::ExprId>,
    pub(super) static_addresses: super::static_addresses::Queries,
    pub(super) static_initializers:
        Vec<(super::graph::EntityId, Option<ast::Initializer>, ScopeId)>,
    pub(super) body_context: Option<super::procedural_statements::BodyContext>,
    pub(super) enclosing_bodies: Vec<super::graph::EntityId>,
    pub(super) capture_frames: Box<super::captures::Frames>,
    pub(super) signature_object: Option<super::types::TypeId>,
    pub(super) default_argument_scope: Option<ScopeId>,
    pub(super) closure_default_argument: Option<(super::graph::EntityId, u32)>,
    pub(super) unevaluated: u32,
    pub(super) type_queries: super::type_queries::QueryContext,
    pub(super) template_arguments: u32,
    pub(super) decltype_call: Option<ast::ExprId>,
    pub(super) builtin_names: super::builtin_calls::BuiltinNames,
    pub(super) library_nans: [Option<super::graph::BindingId>; 3],
    pub(super) pretyped_callable: Option<ast::ExprId>,
    pub(super) callee_name: Option<ast::ExprId>,
    pub(super) operator_scratch: super::operator_resolution::OperatorScratch,
    pub(super) operator_lookup: super::operator_candidates::OperatorLookup,
    pub(super) association_scratch: super::call_lookup::AssociationScratch,
    pub(super) user_conversion_scratch: super::user_conversions::UserScratch,
    pub(super) call_scratch: super::procedural_calls::CallScratch,
    pub(super) copy_scratch: super::copy_initialization::CopyScratch,
    pub(super) assignment_scratch: super::assignment::Scratch,
    pub(super) binary_spine: Vec<ast::ExprId>,
    pub(super) specifications: Vec<super::exceptions::PendingSpecification>,
    pub(super) pending_throws: Vec<super::exceptions::SpecificationId>,
    pub(super) evaluating_noexcept: bool,
    pub(super) layout_work: Vec<(super::graph::EntityId, usize)>,
    pub(super) bit_layout: super::bit_layout::BitLayout,
    pub(super) exception_work: Vec<(ast::ExprId, super::exception_operands::OperandStep)>,
    pub(super) throw_work: Vec<(ast::ExprId, u16)>,
    pub(super) deduced_initializer: Option<ast::ExprId>,
    pub(super) placeholder_scratch: super::placeholders::PlaceholderScratch,
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
    pub definition_head: Option<super::templates::HeadId>,
}
#[derive(Clone, Copy)]
pub(super) enum DeferredMember {
    Initializer(super::graph::EntityId),
    Constant(super::graph::EntityId, ast::Initializer, ScopeId),
    AggregateConstant(super::graph::EntityId, Option<ast::Initializer>, ScopeId),
    Body(DeferredBody),
    Defaults(super::default_arguments::PendingDefaults),
}
impl<'tree> Analyzer<'tree> {
    pub(super) fn specializing(&self) -> bool {
        self.specialization_mode == SpecializationMode::Explicit
    }

    pub(crate) fn new(
        tree: &'tree mut Tree,
        cursor: &'tree mut Cursor<'_>,
        stack_budget: StackBudget,
    ) -> Result<Self, Error> {
        Self::with_graph(tree, cursor, stack_budget, None)
    }
    pub(super) fn with_graph(
        tree: &'tree mut Tree,
        cursor: &'tree mut Cursor<'_>,
        stack_budget: StackBudget,
        previous: Option<Graph>,
    ) -> Result<Self, Error> {
        let fresh = previous.is_none();
        let mut graph = previous.unwrap_or_else(Graph::new);
        let int128_name = cursor
            .intern_name(b"__int128")
            .map_err(|_| Error::Limit("semantic identifier limit"))?;
        if fresh {
            for (spelling, scalar) in [
                (b"nullptr_t".as_slice(), Scalar::Nullptr),
                (
                    b"__int128".as_slice(),
                    Scalar::Basic(crate::types::FundamentalType::Int128),
                ),
                (
                    b"__int128_t".as_slice(),
                    Scalar::Basic(crate::types::FundamentalType::Int128),
                ),
                (
                    b"__uint128_t".as_slice(),
                    Scalar::Basic(crate::types::FundamentalType::UnsignedInt128),
                ),
            ] {
                let name = cursor
                    .intern_name(spelling)
                    .map_err(|_| Error::Limit("semantic identifier limit"))?;
                let ty = graph.types.scalar(scalar)?;
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
            }
        }
        let builtin_names = super::builtin_calls::BuiltinNames {
            abort: cursor
                .intern_name(b"__builtin_abort")
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            expect: cursor
                .intern_name(b"__builtin_expect")
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            constant_p: cursor
                .intern_name(b"__builtin_constant_p")
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            invoke: cursor
                .intern_name(b"__builtin_invoke")
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            memcpy: cursor
                .intern_name(b"__builtin_memcpy")
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            memmove: cursor
                .intern_name(b"__builtin_memmove")
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            memcmp: cursor
                .intern_name(b"__builtin_memcmp")
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            memset: cursor
                .intern_name(b"__builtin_memset")
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            strlen: cursor
                .intern_name(b"__builtin_strlen")
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            unreachable: cursor
                .intern_name(b"__builtin_unreachable")
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            floating: super::floating_intrinsics::names(cursor)?,
        };
        let mut analyzer = Self {
            library_nans: [None; 3],
            deduced_pack_element: None,
            decltype_name: cursor
                .intern_name(b"decltype(...)")
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            override_name: cursor
                .intern_name(b"override")
                .map_err(|_| Error::Limit("semantic identifier limit"))?,
            exception_overrides: Vec::new(),
            deferred_exception_overrides: Vec::new(),
            declaration_access: super::declaration_access::DeclarationAccess::default(),
            literal_bytes: &cursor.spellings,
            literal_scanner: Box::default(),
            identifiers: cursor.post.input_mut().identifiers_mut(),
            deduced_initializer: None,
            placeholder_scratch: super::placeholders::PlaceholderScratch::default(),
            tree,
            graph,
            stack_budget,
            stack_base: 0,
            lookup_location: SourceOffset(0),
            deferred: Vec::new(),
            deferred_constexpr: std::collections::HashMap::new(),
            class_depth: 0,
            member_initializer_class: None,
            base_object: None,
            template_depth: 0,
            specialization_levels: 0,
            specialization_mode: SpecializationMode::Ordinary,
            instantiation_depth: 0,
            incomplete_class_observations: 0,
            dependent_uses: std::cell::Cell::new(0),
            value_parameter_uses: std::cell::Cell::new(0),
            c_language: false,
            linkage_specification: None,
            template_head: None,
            template_owner: None,
            operand_home: None,
            namespace_friend_class: None,
            constant_spine: Vec::new(),
            constant_discard_work: Vec::new(),
            constant_evaluation: None,
            constexpr_frames: super::constexpr_frames::Frames::default(),
            constant_scratch: super::constant_addresses::Evaluation::default(),
            expression_work: Vec::new(),
            fixed_validation_work: Vec::new(),
            suppress_implicit: false,
            mode: AnalysisMode::Types,
            initializing: None,
            initializing_declarator: None,
            initializer_demand_owner: None,
            parenthesized_elements: std::collections::HashSet::new(),
            static_addresses: super::static_addresses::Queries::default(),
            static_initializers: Vec::new(),
            body_context: None,
            enclosing_bodies: Vec::new(),
            capture_frames: Box::default(),
            signature_object: None,
            binary_spine: Vec::new(),
            operator_scratch: super::operator_resolution::OperatorScratch::default(),
            operator_lookup: super::operator_candidates::OperatorLookup::default(),
            association_scratch: super::call_lookup::AssociationScratch::default(),
            user_conversion_scratch: super::user_conversions::UserScratch::default(),
            call_scratch: super::procedural_calls::CallScratch::default(),
            copy_scratch: super::copy_initialization::CopyScratch::default(),
            assignment_scratch: super::assignment::Scratch::default(),
            builtin_names,
            int128_name,
            pretyped_callable: None,
            callee_name: None,
            default_argument_scope: None,
            closure_default_argument: None,
            unevaluated: 0,
            type_queries: super::type_queries::QueryContext::default(),
            template_arguments: 0,
            decltype_call: None,
            specifications: Vec::new(),
            pending_throws: Vec::new(),
            evaluating_noexcept: false,
            layout_work: Vec::new(),
            bit_layout: super::bit_layout::BitLayout::default(),
            exception_work: Vec::new(),
            throw_work: Vec::new(),
        };
        if fresh {
            analyzer.predeclare_allocations()?;
        }
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
    pub(super) fn hosted(&self) -> bool {
        self.graph.hosted
    }
    pub(super) fn set_hosted(&mut self, hosted: bool) {
        self.graph.hosted = hosted;
    }
    pub(super) fn semantic(&self) -> bool {
        self.mode != AnalysisMode::Types
    }
    pub(crate) fn analyze(self, declarations: List<ast::DeclId>) -> Result<Graph, Error> {
        self.analyze_declarations(declarations, true)
    }
    pub(super) fn analyze_declarations(
        mut self,
        declarations: List<ast::DeclId>,
        boundaries: bool,
    ) -> Result<Graph, Error> {
        let marker = 0u8;
        self.stack_base = std::ptr::from_ref(&marker).addr();
        self.declarations(declarations, self.graph.global)?;
        self.finish_analysis(boundaries)
    }
    pub(super) fn finish_analysis(mut self, boundaries: bool) -> Result<Graph, Error> {
        self.finish_unit_exceptions()?;
        if !self.deferred.is_empty() {
            return Err(Error::Invariant("uncompleted member bodies"));
        }
        if self.lowering() {
            self.finish_instance_variables()?;
        }
        self.finish_static_initializers()?;
        if boundaries && self.lowering() {
            self.finish_lowering_boundaries()?;
        }
        if self.lowering() {
            self.finish_qualifier_arguments()?;
        }
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
        for syntax_index in 0..self.tree.decl_lists.get(list).len() {
            let id = self.tree.decl_lists.get(list)[syntax_index];
            self.declaration(id, scope)?;
        }
        Ok(())
    }
    pub(super) fn declaration(&mut self, id: ast::DeclId, scope: ScopeId) -> Result<(), Error> {
        let enabled = matches!(
            self.tree.declarations.get(id).kind,
            ast::DeclKind::Simple { .. } | ast::DeclKind::Function { .. }
        );
        let mark = self.begin_declaration_access(scope, enabled);
        let result = self.declaration_in(id, scope);
        self.finish_declaration_access(mark);
        result.map_err(|error| match error {
            Error::Limit("constructs nested too deeply") => Error::at(
                self.tree.declarations.get(id).location,
                "constructs nested too deeply",
            ),
            other => other,
        })
    }
    fn declaration_in(&mut self, id: ast::DeclId, scope: ScopeId) -> Result<(), Error> {
        self.check_stack()?;
        let declaration = *self.tree.declarations.get(id);
        let location = declaration.location;
        if self.graph.scope(scope).kind == ScopeKind::Class
            && self
                .tree
                .attributes
                .get(declaration.attributes)
                .iter()
                .any(|attribute| attribute.kind == ast::AttributeKind::NoUniqueAddress)
        {
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
            // Types-only queries still need the physical dynamic-class fact;
            // virtual declaration diagnostics and slot completion are semantic work.
            self.graph.entity_mut(owner).polymorphism =
                super::virtual_members::Polymorphism::Dynamic;
            self.graph.entity_mut(owner).aggregation =
                super::aggregate_initialization::Aggregation::Ineligible;
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
                let info = self.declarator(node.declarator, specs.ty, scope)?;
                self.check_virtual_alias(&info, specs, location)?;
                let alias_type = info.ty;
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
                    let binding = self.declare_alias(name, alias_type, scope, location)?;
                    self.alias_declaration_alignment(binding, specs, declaration.attributes, scope)
                }
            }
            ast::DeclKind::Simple {
                specifiers,
                declarators,
            } => self.simple_declaration(id, specifiers, declarators, scope),
            ast::DeclKind::Class(specifier) => {
                let ty = self.specifier_type(specifier, scope, true)?;
                let storage = self.anonymous_union(
                    super::specifiers::Specifiers {
                        dependent_uses: self.dependent_uses.get(),
                        alias_alignment: 0,
                        friendship: super::specifiers::Friendship::Ordinary,
                        auto_cv: super::types::Cv::default(),
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
                        explicit_condition: None,
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
                self.static_assertion(condition, message, scope, location)
            }
            ast::DeclKind::Template {
                parameters,
                declaration,
            } => self.template_declaration(parameters, declaration, scope, location),
            ast::DeclKind::ExplicitInstantiation {
                is_extern,
                declaration,
            } => self.explicit_instantiation(declaration, is_extern, scope, location),
            ast::DeclKind::SpecialMember { .. } => self.special_member(id, scope),
            ast::DeclKind::BitField {
                specifiers,
                declarators,
            } => self.bit_field_declaration(id, specifiers, declarators, scope),
        }
    }
    // analyzer.cpp::static_assertion: semantic mode records value typing and
    // contextual conversion before the evaluator consumes the original nodes.
    // Types mode retains its integral-only rules; patterns retain only names.
    fn static_assertion(
        &mut self,
        condition: ast::ExprId,
        message: Option<ast::Literal>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
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
        let truth = if self.semantic() {
            let facts = self.typed_expression(condition, scope)?;
            self.check_resolved_function(condition)?;
            if self.scoped_enum(facts.ty) {
                return Err(Error::at(
                    self.tree.expressions.get(condition).location,
                    "a scoped enumeration value does not convert to bool",
                ));
            }
            self.convert_bool(condition)?;
            self.in_constant_evaluation(|analyzer| analyzer.constant_truth(condition, scope, false))
                .map_err(|error| match error {
                    Error::NonConstant { location, message } => Error::at(location, message),
                    other => other,
                })?
        } else {
            let value = self
                .constant_value(condition, scope, true)?
                .ok_or(Error::Invariant("missing assertion value"))?;
            if self.scoped_enum(value.ty) {
                return Err(Error::at(
                    self.tree.expressions.get(condition).location,
                    "a scoped enumeration value does not convert to bool",
                ));
            }
            value.bits != 0
        };
        if !truth {
            return Err(Error::at(location, "static assertion failed"));
        }
        Ok(())
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
        let specs = self.declaration_specifiers(specifiers, scope, Some(declarator), false)?;
        let mut environment = self.declarator_environment(Some(declarator), scope)?;
        environment.declaration_location = Some(location);
        self.friend_environment(specs, scope, &mut environment);
        self.placeholder_environment(Some(declarator), specs, &mut environment);
        let declared = if environment.placeholder_function.is_some() {
            self.graph.types.qualify(specs.ty, specs.auto_cv)?
        } else {
            specs.ty
        };
        let mut info = self.declarator_in(Some(declarator), declared, environment)?;
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
        let home = self.graph.binding_scope(binding);
        super::record(
            &mut self.graph.declared_bindings,
            declarator.index(),
            binding,
        );
        let entity = self.graph.binding(binding).entity;
        self.graph.entity_mut(entity).function_definition = Some(declaration);
        // The frozen definition owner evaluates only leading layout attributes
        // here; unlike a simple declaration it does not reject alignas.
        let attributes = self.layout_attributes(
            self.tree.declarations.get(declaration).attributes,
            scope,
            false,
        )?;
        self.graph.note_no_inline(entity, attributes.no_inline)?;
        self.declarator_symbol_attributes(
            declarator,
            self.tree.declarations.get(declaration).attributes,
        )?;
        self.note_instantiation_policy(entity, declarator, attributes.instantiation);
        self.graph.entity_mut(entity).definition_emission = if self.class_depth != 0
            || specs.constexpr
            || self
                .graph
                .function_templates
                .specializations
                .contains(entity)
                && self.graph.entity(entity).inline == super::specifiers::InlinePreference::Inline
        {
            super::function_demands::DefinitionEmission::Demanded
        } else {
            super::function_demands::DefinitionEmission::Always
        };
        self.graph.entity_mut(entity).constexpr = specs.constexpr;
        if self.class_depth != 0 || specs.constexpr {
            self.graph.entity_mut(entity).inline = super::specifiers::InlinePreference::Inline;
        }
        if self.graph.entity(entity).template_head.is_none()
            && !self.class_template_member_redeclaration(&info)
        {
            self.record_function_exceptions(entity, &info, info.lookup, location, exception_limit)?;
        }
        self.virtual_declaration(binding, &info, specs, location)?;
        self.settle_declaration_access(Some(binding))?;
        if self.graph.entity(entity).defined {
            return Err(Error::at(location, "redefinition of a function"));
        }
        self.graph.entity_mut(entity).defined = true;
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
            definition_head: self.template_head,
            parent: info.lookup,
            print: if info.home.is_some() { home } else { scope },
        };
        if self.class_depth == 0 {
            self.function_body(deferred)
        } else {
            self.defer_function_body(deferred);
            Ok(())
        }
    }
    pub(super) fn defer_function_body(&mut self, body: DeferredBody) {
        if self.shared_instance_body(body)
            && let Some(head) = self.graph.entity(body.entity).template_head
        {
            // The copied member template is a definition pattern even while
            // its enclosing instance is still declaring later members.
            self.graph
                .function_templates
                .record_pattern(body, head, self.graph.binding_limit());
        }
        if self.graph.entity(body.entity).constexpr && body.template_depth == 0 {
            self.deferred_constexpr.insert(body.entity, body);
        }
        self.deferred.push(DeferredMember::Body(body));
    }
    pub(super) fn function_body(&mut self, body: DeferredBody) -> Result<(), Error> {
        self.deferred_constexpr.remove(&body.entity);
        if body.template_depth != 0 && self.graph.entity(body.entity).template_head.is_none() {
            self.graph
                .function_templates
                .bodies
                .record_member_definition(body, self.graph.binding_limit());
        }
        if self.defer_instance_member(body) {
            return Ok(());
        }
        if self.lowering()
            && body.template_depth == 0
            && self.graph.entity(body.entity).definition_emission
                == super::function_demands::DefinitionEmission::Always
            && self.graph.entity(body.entity).inline != super::specifiers::InlinePreference::Inline
        {
            self.graph.boundary_roots.push(body.entity);
        }
        if body.template_depth != 0 && self.graph.entity(body.entity).template_head.is_some() {
            self.graph.function_templates.record_pattern(
                body,
                body.definition_head
                    .ok_or(Error::Invariant("template body without definition head"))?,
                self.graph.binding_limit(),
            );
        }
        let saved = self.template_depth;
        let saved_class_depth = self.class_depth;
        self.template_depth = body.template_depth;
        self.class_depth = 0;
        let saved_context = self.body_context.take();
        if let Some(context) = &saved_context {
            self.enclosing_bodies.push(context.entity);
        }
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
            let placeholder = self.holds_auto(result).then_some(result);
            if (placeholder.is_some() || self.graph.closure_deduces_result(body.entity))
                && self.template_depth == 0
                && let Some(context) = self.body_context.as_mut()
            {
                context.placeholder = placeholder;
                context.return_deduction = super::return_deduction::ReturnDeduction::Pending;
            }
        }
        let result = self.function_body_scopes(body);
        if saved_context.is_some() {
            self.enclosing_bodies.pop();
        }
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
        self.graph.entity_mut(body.entity).analyzed_body = Some(body.body);
        self.graph
            .entity_mut(body.entity)
            .parameter_entities
            .clear();
        let count = self.graph.types.function_parameter_count(body.ty)?;
        let pattern = self.parameter_dependence(body.entity, count)?;
        for index in 0..self
            .tree
            .parameters
            .get(body.parameters.parameters)
            .len()
            .min(count)
        {
            let syntax_item = self.tree.parameters.get(body.parameters.parameters)[index];
            let parameter = &syntax_item;
            let mut ty = self.graph.types.function_parameter(body.ty, index)?;
            if self.graph.function_templates.primary(body.entity).is_some() {
                // The canonical signature discards top-level cv. Parameter
                // objects keep the definition's declared type, substituted in
                // its immutable argument frame (an array/function adjusts).
                let specs = self.specifiers(parameter.specifiers, scope, false)?;
                let info = self.declarator(parameter.declarator, specs.ty, scope)?;
                ty = self.graph.types.parameter_object(info.ty)?;
            }
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
            self.graph
                .entity_mut(body.entity)
                .parameter_entities
                .push(entity);
            let binding = self.graph.add_binding(
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
            let dependent = if let Some(pattern) = pattern {
                self.graph
                    .dependent_type(self.graph.types.function_parameter(pattern, index)?)
            } else {
                self.graph.lookup_limit.is_some()
            };
            self.graph.set_binding_dependence(binding, dependent);
            self.note_nothrow_target(entity, ty, parameter.declarator, scope)?;
        }
        let syntax_body = *self.tree.bodies.get(body.body);
        let ast::Body::Parsed(parsed) = &syntax_body else {
            return Err(Error::Invariant("unparsed body at semantic boundary"));
        };
        if self.semantic() && self.template_depth == 0 {
            self.check_constexpr_scalar_signature(
                body.entity,
                body.ty,
                self.tree.statements.get(parsed.compound).location,
            )?;
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
        // An instance's member template still shares its definition's body.
        // Its bound class packs are expanded only in the demanded body's
        // private copy; walking the shared syntax here would read pack aliases
        // as values. Match analyzer_bodies.cpp::function_body's shared boundary.
        if self.shared_instance_body(body) {
            return Ok(());
        }
        if self.semantic()
            && self.template_depth == 0
            && let Some(key) = self.graph.entity(body.entity).member_function
            && key.kind == super::graph::MemberKind::Destructor
        {
            self.check_subobject_destruction(
                key.class,
                scope,
                self.tree.statements.get(parsed.compound).location,
            )?;
            self.prepare_destruction(key.class)?;
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
            self.check_constant_constructor_plan(
                body.entity,
                self.tree.statements.get(parsed.compound).location,
            )?;
        } else {
            for syntax_index in 0..self.tree.mem_initializers.get(parsed.initializers).len() {
                let syntax_item = self.tree.mem_initializers.get(parsed.initializers)[syntax_index];
                let initializer = &syntax_item;
                self.initializer_names(initializer.initializer, scope, body.ty)?;
            }
        }
        self.substatement(parsed.compound, scope)?;
        if let Some(handlers) = parsed.handlers {
            let constructor = self
                .graph
                .entity(body.entity)
                .member_function
                .is_some_and(|key| key.kind == super::graph::MemberKind::Constructor);
            if let Some(context) = self.body_context.as_mut() {
                context.constructor_handlers = constructor;
            }
            let result = self.handlers(handlers, scope);
            if let Some(context) = self.body_context.as_mut() {
                context.constructor_handlers = false;
            }
            result?;
        }
        if self.semantic() {
            self.check_gotos()?;
            self.finish_return_deduction(self.tree.statements.get(parsed.compound).location)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor, support::source::SourceBuffer, syntax::SyntaxUnit,
    };

    #[test]
    fn assertion_queries_retain_original_facts_without_body_or_emission_demand() {
        let source = SourceBuffer::new(
            b"template<class T>struct Q{static int f()noexcept{T::missing;}\
                void unused(){static_assert(noexcept(T::bad()),\"\");}};\
              static_assert(noexcept(Q<int>::f()),\"\");"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"assertion-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let bodies = tree.bodies.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let assertion = *analyzer.tree.decl_lists.get(declarations).last().unwrap();
        let ast::DeclKind::StaticAssert { condition, .. } =
            analyzer.tree.declarations.get(assertion).kind
        else {
            panic!("assertion");
        };
        let ast::ExprKind::TypeTrait {
            operand: Some(call),
            ..
        } = analyzer.tree.expressions.get(condition).kind
        else {
            panic!("query");
        };
        let facts = analyzer.graph.expression_facts.get(condition).unwrap();
        assert_eq!(facts.folded, Some(1));
        assert_eq!(facts.conversion.unwrap().target, facts.ty);
        assert!(
            analyzer
                .graph
                .expression_facts
                .get(call)
                .unwrap()
                .call
                .is_some()
        );
        assert_eq!(analyzer.tree.bodies.mark(), bodies);
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert!(analyzer.body_context.is_none());
        assert_eq!(analyzer.unevaluated, 0);
        assert_eq!(analyzer.template_depth, 0);
        assert!(analyzer.constant_evaluation.is_none());
        let marks = (
            analyzer.tree.expressions.mark(),
            analyzer.tree.types.mark(),
            analyzer.tree.declarations.mark(),
        );
        for _ in 0..3 {
            assert!(analyzer.constant_truth(condition, global, false).unwrap());
        }
        assert_eq!(
            marks,
            (
                analyzer.tree.expressions.mark(),
                analyzer.tree.types.mark(),
                analyzer.tree.declarations.mark(),
            )
        );
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn rejected_assertion_restores_unevaluated_and_constant_storage_contexts() {
        for (source, message) in [
            (
                b"static_assert(noexcept(missing()),\"\");".as_slice(),
                "undeclared function name",
            ),
            (
                b"int x;static_assert(x,\"\");",
                "a variable is not usable in a constant expression",
            ),
        ] {
            let source = SourceBuffer::new(source.to_vec()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"assertion-failure").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let global = analyzer.graph.global;
            assert!(matches!(analyzer.declarations(declarations, global),
                Err(Error::Diagnostic { message: actual, .. }) if actual == message));
            assert_eq!(analyzer.unevaluated, 0);
            assert!(analyzer.body_context.is_none());
            assert!(analyzer.constant_evaluation.is_none());
        }
    }
}
