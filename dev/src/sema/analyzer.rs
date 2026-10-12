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

pub(crate) struct Analyzer<'tree> {
    pub(super) literal_bytes: &'tree crate::syntax::storage::TextStore,
    pub(super) tree: &'tree Tree,
    pub(super) graph: Graph,
    pub(super) stack_budget: StackBudget,
    pub(super) stack_base: usize,
    pub(super) lookup_location: SourceOffset,
    pub(super) deferred: Vec<DeferredBody>,
    pub(super) anonymous_enums: u32,
    pub(super) class_depth: usize,
    pub(super) constant_spine: Vec<ast::ExprId>,
    pub(super) expression_work: Vec<(ast::ExprId, bool)>,
    pub(super) suppress_implicit: bool,
    pub(super) specifications: Vec<super::exceptions::PendingSpecification>,
    pub(super) pending_throws: Vec<super::exceptions::SpecificationId>,
    pub(super) evaluating_noexcept: bool,
    pub(super) layout_work: Vec<(super::graph::EntityId, usize)>,
    pub(super) exception_work: Vec<(ast::ExprId, super::exception_operands::OperandStep)>,
}
#[derive(Clone, Copy)]
pub(super) struct DeferredBody {
    pub name: crate::lex::identifiers::NameId,
    pub ty: TypeId,
    pub parameters: ast::ParameterClause,
    pub body: ast::BodyId,
    pub parent: ScopeId,
    pub print: ScopeId,
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
        Ok(Self {
            literal_bytes: &cursor.spellings,
            tree,
            graph,
            stack_budget,
            stack_base: 0,
            lookup_location: SourceOffset(0),
            deferred: Vec::new(),
            anonymous_enums: 0,
            class_depth: 0,
            constant_spine: Vec::new(),
            expression_work: Vec::new(),
            suppress_implicit: false,
            specifications: Vec::new(),
            pending_throws: Vec::new(),
            evaluating_noexcept: false,
            layout_work: Vec::new(),
            exception_work: Vec::new(),
        })
    }
    pub(crate) fn analyze(mut self, declarations: List<ast::DeclId>) -> Result<Graph, Error> {
        let marker = 0u8;
        self.stack_base = std::ptr::from_ref(&marker).addr();
        self.declarations(declarations, self.graph.global)?;
        self.finish_unit_exceptions()?;
        if !self.deferred.is_empty() {
            return Err(Error::Invariant("uncompleted member bodies"));
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
            | ast::DeclKind::Function { specifiers, .. } => Some(specifiers),
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
            ast::DeclKind::Empty | ast::DeclKind::Access(_) => Ok(()),
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
                let exceptions =
                    self.declarator_specifies_exceptions(self.tree.types.get(ty).declarator);
                let ty = self.type_syntax(ty, scope)?;
                if exceptions {
                    return Err(Error::at(
                        location,
                        "an exception-specification in an alias-declaration",
                    ));
                }
                self.declare_alias(name, ty, scope, location).map(|_| ())
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
                    .map(|init| self.declarator_name(Some(init.declarator)))
                    .transpose()?
                    .flatten();
                let naming = if self.graph.scope(scope).kind != ScopeKind::Class
                    || specs.role == super::specifiers::DeclarationRole::Typedef
                {
                    first_name
                } else {
                    None
                };
                self.name_unnamed_enum(specs.ty, naming, scope)?;
                for init in self.tree.init_declarators.get(declarators) {
                    self.declare_init(specs, *init, scope, location)?;
                }
                Ok(())
            }
            ast::DeclKind::Class(specifier) => {
                self.specifier_type(specifier, scope, true).map(|_| ())
            }
            ast::DeclKind::Function {
                specifiers,
                declarator,
                body,
            } => {
                let specs = self.specifiers(specifiers, scope, false)?;
                let info = self.declarator(Some(declarator), specs.ty, scope)?;
                let name = info
                    .name
                    .ok_or(Error::Invariant("function definition without a name"))?;
                let home = info.home.unwrap_or(scope);
                let location = self.declarator_location(declarator)?;
                self.check_member_function_redeclaration(&info, home, location)?;
                if info.home.is_some()
                    && self
                        .graph
                        .own(home, name, super::lookup::Filter::Ordinary)
                        .is_none()
                {
                    return Err(Error::at(
                        location,
                        "qualified definition does not match a declaration",
                    ));
                }
                let exception_limit = self.graph.binding_limit();
                let binding = self.declare_value(
                    name,
                    info.ty,
                    home,
                    home,
                    BindingKind::Function,
                    self.declarator_location(declarator)?,
                )?;
                super::record(
                    &mut self.graph.declared_bindings,
                    declarator.index(),
                    binding,
                );
                let entity = self.graph.binding(binding).entity;
                self.graph.entity_mut(entity).constexpr = specs.constexpr;
                self.record_function_exceptions(entity, &info, home, location, exception_limit)?;
                if self.graph.entity(entity).defined {
                    return Err(Error::at(location, "redefinition of a function"));
                }
                self.graph.entity_mut(entity).defined = true;
                let parameters = info
                    .parameters
                    .ok_or(Error::Invariant("function definition without parameters"))?;
                let deferred = DeferredBody {
                    name,
                    ty: info.ty,
                    parameters,
                    body,
                    parent: home,
                    print: home,
                };
                if self.class_depth == 0 {
                    self.function_body(deferred)
                } else {
                    self.deferred.push(deferred);
                    Ok(())
                }
            }
            ast::DeclKind::Linkage { declarations, .. } => self.declarations(declarations, scope),
            ast::DeclKind::Enum { .. } => {
                let ty = self.enum_type(id, scope, true)?;
                self.name_unnamed_enum(ty, None, scope)
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
            ast::DeclKind::Template { .. } | ast::DeclKind::ExplicitInstantiation { .. } => {
                Err(Error::Missing("template parameter scopes and declarations"))
            }
            ast::DeclKind::SpecialMember { .. } | ast::DeclKind::BitField { .. } => {
                Err(Error::Missing("special member and bit-field declarations"))
            }
        }
    }
    pub(super) fn function_body(&mut self, body: DeferredBody) -> Result<(), Error> {
        let scope = self.graph.add_scope(
            ScopeKind::Function,
            body.parent,
            Some(body.print),
            Some(body.name),
        )?;
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
        }
        let ast::Body::Parsed(parsed) = self.tree.bodies.get(body.body) else {
            return Err(Error::Invariant("unparsed body at semantic boundary"));
        };
        self.statement(parsed.compound, scope)
    }
    fn statement(&mut self, id: ast::StmtId, scope: ScopeId) -> Result<(), Error> {
        self.check_stack()?;
        match self.tree.statements.get(id).kind {
            ast::StmtKind::Compound(list) => {
                let block = self
                    .graph
                    .add_scope(ScopeKind::Block, scope, Some(scope), None)?;
                for &id in self.tree.stmt_lists.get(list) {
                    self.statement(id, block)?;
                }
                Ok(())
            }
            ast::StmtKind::Declaration(decl) => self.declaration(decl, scope),
            ast::StmtKind::Expression(operand)
            | ast::StmtKind::Return(operand)
            | ast::StmtKind::Throw(operand) => {
                operand.map_or(Ok(()), |id| self.expression(id, scope))
            }
            ast::StmtKind::Break | ast::StmtKind::Continue | ast::StmtKind::Goto(_) => Ok(()),
            ast::StmtKind::Label { body, .. }
            | ast::StmtKind::Case { body, .. }
            | ast::StmtKind::Default(body) => self.statement(body, scope),
            _ => Err(Error::Missing(
                "selection, iteration, handler and ambiguous statement scopes",
            )),
        }
    }
}
