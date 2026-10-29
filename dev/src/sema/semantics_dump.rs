// Translated from sema/semantics_dump.cpp. See NOTICE.
//! An iterative view of original syntax and recorded semantic facts.
use super::{
    expression_facts::{DestructorTarget, ObjectArgument},
    expression_facts::{LvalueTransform as L, ValueCategory as V, ValueConversion as C},
    function_resolution::CallTarget,
    graph::{BindingId, BindingKind, Graph, ScopeKind},
    types::{Cv, TypeId, TypeKind},
};
use crate::{
    lex::{
        identifiers::NameId,
        pp_lexer::Punctuator as P,
        vocabulary::{Keyword, SimpleToken},
    },
    syntax::{
        ast::{self, DeclKind, ExprKind, StmtKind},
        cursor::Cursor,
        storage::List,
    },
};
use std::io::{self, Write};
// Cold, read-only presentation value. A builtin conversion projects a
// different designation without mutating the semantic unit's shared facts.
#[derive(Clone, Copy)]
struct DisplayFacts {
    core: super::expression_facts::ExprFacts,
    details: super::expression_details::Details,
}
impl std::ops::Deref for DisplayFacts {
    type Target = super::expression_facts::ExprFacts;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}
impl DisplayFacts {
    fn new(
        core: super::expression_facts::ExprFacts,
        store: &super::expression_facts::ExpressionFacts,
    ) -> Self {
        Self {
            core,
            details: core.display_details(store),
        }
    }
    fn reference_cast(self) -> Option<TypeId> {
        self.details.reference_cast
    }
    fn functions(self) -> Option<super::function_resolution::OverloadId> {
        self.details.functions
    }
    fn function(self) -> Option<super::graph::BindingId> {
        self.details.function
    }
    fn call(self) -> Option<super::function_resolution::CallTarget> {
        self.details.call
    }
    fn literal_call(self) -> Option<super::literal_calls::LiteralCall> {
        self.details.literal_call
    }
    fn destructor(self) -> Option<DestructorTarget> {
        self.details.destructor
    }
    fn anonymous_member(self) -> Option<super::graph::BindingId> {
        self.details.anonymous_member
    }
    fn member(self) -> Option<super::graph::BindingId> {
        self.details.member
    }
    fn member_naming(self) -> Option<super::graph::EntityId> {
        self.details.member_naming
    }
    fn object(self) -> Option<ObjectArgument> {
        self.details.object
    }
    fn base_path(self) -> Option<super::inheritance::BasePathId> {
        self.details.base_path
    }
}
enum Item {
    Declaration(ast::DeclId, bool),
    Statement(ast::StmtId),
    Condition(ast::Condition),
    Expression {
        id: ast::ExprId,
        bare: bool,
        retyped: Option<(TypeId, V)>,
    },
    Binding(BindingId, Option<ast::Initializer>),
    Storage(BindingId),
    ObjectReceiver(ast::ExprId, BindingId, Option<ast::ExprId>),
    ArrowCall(ast::ExprId, usize),
    This(TypeId),
    QualifiedThis(TypeId, super::types::Cv),
    Constructor(super::construction::ConstructorId),
    DemandedFunction(BindingId),
    Line(&'static str),
}
struct Printer<'a, 'source, W: Write + ?Sized> {
    graph: &'a Graph,
    tree: &'a ast::Tree,
    cursor: &'a Cursor<'source>,
    output: &'a mut W,
    work: Vec<(Item, usize)>,
}
impl Graph {
    pub(crate) fn print_semantics(
        &self,
        tree: &ast::Tree,
        cursor: &Cursor<'_>,
        declarations: List<ast::DeclId>,
        output: &mut (impl Write + ?Sized),
    ) -> io::Result<()> {
        let mut printer = Printer {
            graph: self,
            tree,
            cursor,
            output,
            work: Vec::new(),
        };
        printer.line("translation-unit", 0)?;
        printer.work.extend(
            super::function_demands::trailing(self)
                .into_iter()
                .rev()
                .map(|demand| {
                    (
                        match demand {
                            super::function_demands::Demand::Constructor(id) => {
                                Item::Constructor(id)
                            }
                            super::function_demands::Demand::Function { binding, .. } => {
                                Item::DemandedFunction(binding)
                            }
                        },
                        1,
                    )
                }),
        );
        printer.declarations(declarations, 1, false);
        while let Some((item, depth)) = printer.work.pop() {
            match item {
                Item::Declaration(id, block) => printer.declaration(id, depth, block)?,
                Item::Statement(id) => printer.statement(id, depth)?,
                Item::Condition(condition) => printer.condition(condition, depth)?,
                Item::Expression { id, bare, retyped } => {
                    printer.expression(id, depth, bare, retyped)?;
                }
                Item::Binding(id, initializer) => printer.binding(id, initializer, depth)?,
                Item::Storage(id) => printer.storage_expression(id, depth)?,
                Item::ObjectReceiver(callee, binding, call) => {
                    printer.object_receiver(callee, binding, call, depth)?;
                }
                Item::ArrowCall(base, index) => printer.arrow_call(base, index, depth)?,
                Item::This(ty) => printer.this_pointer(ty, depth)?,
                Item::QualifiedThis(ty, cv) => {
                    printer.indent(depth)?;
                    printer.output.write_all(b"id-expression prvalue ")?;
                    printer.object_pointer(ty, cv)?;
                    writeln!(printer.output, " this")?;
                }
                Item::Constructor(id) => printer.constructor_definition(id, depth)?,
                Item::DemandedFunction(binding) => {
                    let binding = printer
                        .graph
                        .function_origin(printer.graph.binding(binding).entity)
                        .map_err(|_| io::Error::other("demanded function without origin"))?;
                    if let Some(key) = printer
                        .graph
                        .entity(printer.graph.binding(binding).entity)
                        .member_function
                        && printer.graph.entity(key.class).implicit_destructor == Some(binding)
                    {
                        printer.implicit_destructor_definition(binding, key.class, depth)?;
                    } else if let Some(
                        super::special_members::SpecialDefinition::Body(body)
                        | super::special_members::SpecialDefinition::Defaulted {
                            body: Some(body),
                            ..
                        },
                    ) = printer
                        .graph
                        .entity(printer.graph.binding(binding).entity)
                        .special_definition
                    {
                        printer.function_definition(binding, Some(body), depth)?;
                    } else if let Some(body) = printer
                        .graph
                        .entity(printer.graph.binding(binding).entity)
                        .analyzed_body
                        && printer
                            .graph
                            .entity(printer.graph.binding(binding).entity)
                            .home
                            .and_then(|home| printer.graph.scope(home).owner)
                            .is_some_and(|class| {
                                printer.graph.closure_call(class)
                                    == Some(printer.graph.binding(binding).entity)
                            })
                    {
                        printer.function_definition(binding, Some(body), depth)?;
                    } else if let Some(body) = printer
                        .graph
                        .function_templates
                        .bodies
                        .body(printer.graph.binding(binding).entity)
                    {
                        printer.function_definition(binding, Some(body), depth)?;
                    } else if let Some(declaration) = printer
                        .graph
                        .entity(printer.graph.binding(binding).entity)
                        .function_definition
                        && printer
                            .graph
                            .entity(printer.graph.binding(binding).entity)
                            .special_definition
                            .is_none()
                    {
                        let DeclKind::Function { body, .. } =
                            printer.tree.declarations.get(declaration).kind
                        else {
                            return Err(io::Error::other(
                                "demanded definition without function syntax",
                            ));
                        };
                        printer.function_definition(binding, Some(body), depth)?;
                    } else {
                        printer.binding_header(binding, "function-declaration", depth)?;
                        if printer
                            .graph
                            .function_templates
                            .primary(printer.graph.binding(binding).entity)
                            .is_some()
                        {
                            printer.specialization_parameters(binding, depth + 1)?;
                        }
                    }
                }
                Item::Line(text) => printer.line(text, depth)?,
            }
        }
        Ok(())
    }
}
impl<W: Write + ?Sized> Printer<'_, '_, W> {
    fn specialization_parameters(&mut self, binding: BindingId, depth: usize) -> io::Result<()> {
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or_else(|| io::Error::other("specialization without type"))?;
        let clause = self
            .graph
            .entity(self.graph.binding(binding).entity)
            .function_parameters;
        let count = self
            .graph
            .types
            .function_parameter_count(ty)
            .map_err(|_| io::Error::other("specialization without function type"))?;
        for index in 0..count {
            self.indent(depth)?;
            self.output.write_all(b"parameter ")?;
            let mut declarator = clause
                .and_then(|clause| self.tree.parameters.get(clause.parameters).get(index))
                .and_then(|parameter| parameter.declarator);
            let mut name = None;
            while let Some(id) = declarator {
                let node = self.tree.declarators.get(id);
                if let Some(syntax_name) = node.name {
                    name = self
                        .tree
                        .name_lists
                        .get(self.tree.names.get(syntax_name).components)
                        .last()
                        .and_then(|component| match component.kind {
                            ast::NameKind::Identifier(name) => Some(name),
                            _ => None,
                        });
                    break;
                }
                declarator = node.nested;
            }
            self.name(name)?;
            self.output.write_all(b" ")?;
            self.ty(self
                .graph
                .types
                .function_parameter(ty, index)
                .map_err(|_| io::Error::other("specialization parameter outside type"))?)?;
            writeln!(self.output)?;
        }
        Ok(())
    }
    fn indent(&mut self, depth: usize) -> io::Result<()> {
        for _ in 0..depth {
            self.output.write_all(b"  ")?;
        }
        Ok(())
    }
    fn line(&mut self, text: &str, depth: usize) -> io::Result<()> {
        self.indent(depth)?;
        writeln!(self.output, "{text}")
    }
    fn name(&mut self, name: Option<NameId>) -> io::Result<()> {
        if let Some(name) = name {
            self.output.write_all(self.cursor.name(name))?;
        }
        Ok(())
    }
    fn ty(&mut self, ty: TypeId) -> io::Result<()> {
        self.graph.write_semantic_type(
            self.graph.types.canonical_view(ty),
            self.cursor,
            self.output,
        )
    }
    fn declarations(&mut self, declarations: List<ast::DeclId>, depth: usize, block: bool) {
        self.work.extend(
            self.tree
                .decl_lists
                .get(declarations)
                .iter()
                .rev()
                .map(|&id| (Item::Declaration(id, block), depth)),
        );
    }
    fn expr(&mut self, id: ast::ExprId, depth: usize) {
        self.work.push((
            Item::Expression {
                id,
                bare: false,
                retyped: None,
            },
            depth,
        ));
    }
    fn declared(&self, id: ast::DeclaratorId) -> Option<BindingId> {
        self.graph
            .declared_bindings
            .get(id.index())
            .copied()
            .flatten()
    }
    fn declaration(&mut self, id: ast::DeclId, depth: usize, block: bool) -> io::Result<()> {
        match self.tree.declarations.get(id).kind {
            DeclKind::Namespace {
                name, declarations, ..
            } => {
                self.indent(depth)?;
                self.output.write_all(b"namespace-definition ")?;
                if name.is_some() {
                    self.name(name)?;
                } else {
                    self.output.write_all(b"<unnamed>")?;
                }
                writeln!(self.output)?;
                self.declarations(declarations, depth + 1, false);
            }
            DeclKind::Simple { declarators, .. } => {
                if block {
                    self.line("simple-declaration", depth)?;
                }
                let inner = depth + usize::from(block);
                for init in self.tree.init_declarators.get(declarators).iter().rev() {
                    let Some(binding) = self
                        .graph
                        .declared_bindings
                        .get(init.declarator.index())
                        .copied()
                        .flatten()
                    else {
                        continue;
                    };
                    if self
                        .graph
                        .lower_binding_alias(self.graph.binding(binding).entity)
                        .is_some()
                    {
                        continue;
                    }
                    self.work
                        .push((Item::Binding(binding, init.initializer), inner));
                }
                if let Some(storage) = self
                    .graph
                    .declaration_storage
                    .get(id.index())
                    .copied()
                    .flatten()
                {
                    self.work.push((Item::Binding(storage, None), inner));
                }
            }
            DeclKind::Alias { name, ty } => {
                let Some(ty) = self.graph.type_fact(ty) else {
                    return Ok(());
                };
                self.indent(depth)?;
                self.output.write_all(b"type-alias ")?;
                self.name(Some(name))?;
                self.output.write_all(b" ")?;
                self.ty(ty)?;
                writeln!(self.output)?;
            }
            DeclKind::Function {
                declarator, body, ..
            } => {
                let Some(binding) = self.declared(declarator) else {
                    return Ok(());
                };
                if self
                    .graph
                    .entity(self.graph.binding(binding).entity)
                    .definition_emission
                    == super::function_demands::DefinitionEmission::Always
                {
                    self.function_definition(binding, Some(body), depth)?;
                }
            }
            DeclKind::SpecialMember {
                declarator,
                initializer,
                body,
                ..
            } => {
                let Some(binding) = self.declared(declarator) else {
                    return Ok(());
                };
                if body.is_some()
                    || initializer
                        .is_some_and(|init| matches!(init.kind, ast::InitializerKind::Default))
                {
                    self.function_definition(binding, body, depth)?;
                } else if block {
                    self.binding_header(binding, "function-declaration", depth)?;
                }
            }
            DeclKind::Linkage { declarations, .. } => self.declarations(declarations, depth, block),
            DeclKind::Class(specifier) => {
                if block && !matches!(specifier.kind, ast::SpecifierKind::Class(_)) {
                    self.line("simple-declaration", depth)?;
                }
                if let Some(storage) = self
                    .graph
                    .declaration_storage
                    .get(id.index())
                    .copied()
                    .flatten()
                {
                    if block && matches!(specifier.kind, ast::SpecifierKind::Class(_)) {
                        self.line("simple-declaration", depth)?;
                    }
                    self.work
                        .push((Item::Binding(storage, None), depth + usize::from(block)));
                }
            }
            DeclKind::Enum { .. } if block => {
                self.line("simple-declaration", depth)?;
            }
            _ => (),
        }
        Ok(())
    }
    fn function_definition(
        &mut self,
        binding: BindingId,
        body: Option<ast::BodyId>,
        depth: usize,
    ) -> io::Result<()> {
        let b = self.graph.binding(binding);
        self.binding_header(binding, "function-definition", depth)?;
        if let Some((object, cv)) = self.function_object(binding) {
            self.indent(depth + 1)?;
            self.output.write_all(b"parameter this ")?;
            self.object_pointer(object, cv)?;
            writeln!(self.output)?;
        }
        let Some(body) = body else {
            return self.line("compound-statement", depth + 1);
        };
        let scope = self
            .graph
            .entity(b.entity)
            .scope
            .ok_or_else(|| io::Error::other("function without scope"))?;
        let ty = self
            .graph
            .types
            .canonical_view(b.ty.ok_or_else(|| io::Error::other("function without type"))?);
        let TypeKind::Function { parameters, .. } = &self.graph.types.get(ty).kind else {
            return Err(io::Error::other("definition without function type"));
        };
        let mut index = 0;
        for &binding in &self.graph.scope(scope).bindings {
            let binding = self.graph.binding(binding);
            if binding.kind != BindingKind::Parameter {
                continue;
            }
            self.indent(depth + 1)?;
            self.output.write_all(b"parameter ")?;
            self.name(binding.name)?;
            self.output.write_all(b" ")?;
            let parameter = *parameters
                .get(index)
                .ok_or_else(|| io::Error::other("parameter count mismatch"))?;
            self.ty(parameter)?;
            writeln!(self.output)?;
            index += 1;
        }
        let ast::Body::Parsed(body) = self.tree.bodies.get(body) else {
            return Err(io::Error::other("unparsed semantic body"));
        };
        let special = self
            .graph
            .entity(b.entity)
            .member_function
            .is_some_and(|key| {
                matches!(
                    key.kind,
                    super::graph::MemberKind::Constructor | super::graph::MemberKind::Destructor
                )
            });
        if special {
            self.line("compound-statement", depth + 1)?;
        }
        self.work.push((
            Item::Statement(body.compound),
            depth + 1 + usize::from(special),
        ));
        Ok(())
    }
    fn function_name(&mut self, id: BindingId) -> io::Result<()> {
        let binding = self.graph.binding(id);
        let entity = self.graph.entity(binding.entity);
        if let Some(key) = entity.member_function
            && key.kind == super::graph::MemberKind::Constructor
        {
            return self
                .graph
                .write_constructor_name(key.class, self.cursor, self.output, true);
        }
        let mut parts = Vec::new();
        let mut scope = Some(self.graph.binding_scope(id));
        while let Some(id) = scope {
            let node = self.graph.scope(id);
            if !matches!(node.kind, ScopeKind::Namespace | ScopeKind::Class) {
                break;
            }
            if node.name.is_some()
                || node.owner.is_some_and(|class| {
                    self.graph.lower_closure(class).is_some()
                        || self.graph.entity(class).local_type_ordinal.is_some()
                })
            {
                parts.push(id);
            }
            scope = if node.kind == ScopeKind::Class {
                node.owner.and_then(|owner| self.graph.entity(owner).home)
            } else {
                node.parent
            };
        }
        for scope in parts.into_iter().rev() {
            let node = self.graph.scope(scope);
            if let Some(closure) = node.owner.and_then(|class| self.graph.lower_closure(class)) {
                write!(self.output, "__lambda{}", closure.ordinal)?;
            } else if let Some(ordinal) = node
                .owner
                .and_then(|class| self.graph.entity(class).local_type_ordinal)
            {
                write!(self.output, "__local_type{ordinal}")?;
            } else {
                self.name(node.name)?;
            }
            self.output.write_all(b"::")?;
        }
        if let Some(super::function_names::SpecialName::Operator { token, array }) = entity
            .function
            .filter(|function| function.allocation.is_some())
            .and_then(|function| function.special)
        {
            write!(
                self.output,
                "operator {}{}",
                token.spelling(),
                if array { "[]" } else { "" }
            )?;
        } else if self.graph.lower_closure_converter(binding.entity).is_some() {
            self.output.write_all(b"operator <function pointer>")?;
        } else if let Some(name) = entity.special_name {
            crate::syntax::write_member_label(self.tree, self.cursor, name, self.output)?;
        } else if let Some(key) = entity.member_function
            && key.kind == super::graph::MemberKind::Destructor
        {
            self.output.write_all(b"~")?;
            self.name(self.graph.entity(key.class).name)?;
        } else if let Some(super::function_names::SpecialName::Operator { token, array }) =
            entity.function.and_then(|f| f.special)
        {
            write!(
                self.output,
                "operator{}{}",
                token.spelling(),
                if array { "[]" } else { "" }
            )?;
            if token.as_punctuator() == Some(P::LeftParen) {
                self.output.write_all(b")")?;
            }
        } else {
            self.name(binding.name)?;
        }
        Ok(())
    }
    fn implicit_destructor_definition(
        &mut self,
        binding: BindingId,
        class: super::graph::EntityId,
        depth: usize,
    ) -> io::Result<()> {
        self.binding_header(binding, "function-definition", depth)?;
        self.indent(depth + 1)?;
        self.output.write_all(b"parameter this pointer to ")?;
        self.ty(self
            .graph
            .entity(class)
            .ty
            .ok_or_else(|| io::Error::other("implicit destructor class without type"))?)?;
        writeln!(self.output)?;
        self.line("compound-statement", depth + 1)
    }
    fn binding_header(&mut self, id: BindingId, label: &str, depth: usize) -> io::Result<()> {
        let binding = self.graph.binding(id);
        self.indent(depth)?;
        write!(self.output, "{label} ")?;
        if binding.kind == BindingKind::Function {
            self.function_name(id)?;
        } else {
            self.variable_name(id)?;
        }
        self.output.write_all(b" ")?;
        if binding.kind == BindingKind::Function {
            self.function_type(id)?;
        } else {
            self.ty(binding
                .ty
                .ok_or_else(|| io::Error::other("binding without type"))?)?;
        }
        writeln!(self.output)
    }
    fn function_object(&self, id: BindingId) -> Option<(TypeId, super::types::Cv)> {
        let entity = self.graph.entity(self.graph.binding(id).entity);
        let home = entity.home?;
        if self.graph.scope(home).kind != ScopeKind::Class
            || entity
                .function
                .is_some_and(|properties| properties.is_static)
        {
            return None;
        }
        let object = self
            .graph
            .scope(home)
            .owner
            .and_then(|class| self.graph.entity(class).ty)?;
        let TypeKind::Function { cv, .. } = self.graph.types.get(entity.ty?).kind else {
            return None;
        };
        Some((object, cv))
    }
    fn object_pointer(&mut self, object: TypeId, cv: super::types::Cv) -> io::Result<()> {
        self.output.write_all(b"pointer to ")?;
        if cv.0 & 1 != 0 {
            self.output.write_all(b"const ")?;
        }
        if cv.0 & 2 != 0 {
            self.output.write_all(b"volatile ")?;
        }
        self.ty(object)
    }
    fn object_receiver(
        &mut self,
        callee: ast::ExprId,
        binding: BindingId,
        call: Option<ast::ExprId>,
        depth: usize,
    ) -> io::Result<()> {
        let Some((owner, cv)) = self.function_object(binding) else {
            return Ok(());
        };
        let facts = self
            .graph
            .expression_facts
            .get(callee)
            .map_err(|_| io::Error::other("receiver without facts"))?;
        let callable = call.and_then(|id| self.graph.expression_facts.callable_receiver(id));
        let object = if callable.is_some() {
            let ExprKind::Member { base, op, .. } = self
                .tree
                .expressions
                .get(self.graph.expression_facts.root(callee))
                .kind
            else {
                return Ok(());
            };
            let base_facts = self
                .graph
                .expression_facts
                .get(base)
                .map_err(|_| io::Error::other("member base without facts"))?;
            let arrow = op.token.as_punctuator() == Some(P::Arrow);
            let ty = if arrow {
                match self.graph.types.get(base_facts.ty).kind {
                    TypeKind::Pointer(element) | TypeKind::Array { element, .. } => element,
                    _ => base_facts.ty,
                }
            } else {
                base_facts.ty
            };
            Some(super::expression_facts::ObjectArgument {
                source: super::expression_facts::ObjectSource::Explicit {
                    expression: base,
                    arrow,
                },
                ty,
                category: base_facts.category,
            })
        } else {
            facts.object(&self.graph.expression_facts)
        };
        let Some(object) = object else {
            return Ok(());
        };
        let converted = callable.map_or_else(
            || {
                facts
                    .conversion
                    .is_some_and(|conversion| conversion.second == C::DerivedToBase)
            },
            |receiver| receiver.path.is_some(),
        );
        let at = depth + usize::from(converted);
        if converted {
            self.indent(depth)?;
            self.output.write_all(b"cast-expression prvalue ")?;
            self.object_pointer(owner, cv)?;
            writeln!(self.output)?;
        }
        match object.source {
            super::expression_facts::ObjectSource::Implicit => {
                if let Item::QualifiedThis(ty, cv) = self.implicit_this(callee, object.ty) {
                    self.indent(at)?;
                    self.output.write_all(b"id-expression prvalue ")?;
                    self.object_pointer(ty, cv)?;
                    writeln!(self.output, " this")?;
                } else {
                    self.this_pointer(object.ty, at)?;
                }
            }
            super::expression_facts::ObjectSource::Explicit { expression, arrow } => {
                if !arrow {
                    self.indent(at)?;
                    self.output
                        .write_all(b"unary-expression prvalue pointer to ")?;
                    self.ty(object.ty)?;
                    writeln!(self.output, " OP_AMP:&")?;
                }
                if arrow {
                    self.arrow_operand(expression, at);
                } else {
                    self.expr(expression, at + 1);
                }
            }
        }
        Ok(())
    }
    fn this_pointer(&mut self, object: TypeId, depth: usize) -> io::Result<()> {
        self.indent(depth)?;
        self.output
            .write_all(b"id-expression prvalue pointer to ")?;
        self.ty(object)?;
        writeln!(self.output, " this")
    }
    fn implicit_this(&self, id: ast::ExprId, object: TypeId) -> Item {
        let qualified = self
            .graph
            .expression_scopes
            .get(self.graph.expression_facts.root(id).index())
            .copied()
            .flatten()
            .and_then(|scope| self.graph.lower_scope_function(scope))
            .and_then(|function| self.graph.function_origin(function).ok())
            .and_then(|binding| self.function_object(binding));
        qualified.map_or(Item::This(object), |(ty, cv)| Item::QualifiedThis(ty, cv))
    }
    fn function_type(&mut self, id: BindingId) -> io::Result<()> {
        let binding = self.graph.binding(id);
        // Return deduction publishes the signature on the shared entity and
        // its origin binding. A later declaration's binding can still contain
        // the placeholder; it must display the entity's deduced signature.
        let ty = if self
            .graph
            .predefined_variables
            .placeholder_result(binding.entity)
        {
            self.graph.entity(binding.entity).ty
        } else {
            binding.ty
        }
        .ok_or_else(|| io::Error::other("function without type"))?;
        let ty = self.graph.types.canonical_view(ty);
        let Some((object, cv)) = self.function_object(id) else {
            return self.ty(ty);
        };
        self.function_type_with_object(ty, object, cv)
    }
    fn function_type_with_object(&mut self, ty: TypeId, object: TypeId, cv: Cv) -> io::Result<()> {
        let TypeKind::Function {
            result,
            parameters,
            variadic,
            ..
        } = &self.graph.types.get(ty).kind
        else {
            return Err(io::Error::other("function without type"));
        };
        self.output.write_all(b"function of (")?;
        self.object_pointer(object, cv)?;
        for &parameter in parameters {
            self.output.write_all(b", ")?;
            self.ty(parameter)?;
        }
        if *variadic {
            self.output.write_all(b", ...")?;
        }
        self.output.write_all(b") returning ")?;
        self.ty(*result)
    }
    fn binding(
        &mut self,
        id: BindingId,
        initializer: Option<ast::Initializer>,
        depth: usize,
    ) -> io::Result<()> {
        let binding = self.graph.binding(id);
        let label = match binding.kind {
            BindingKind::Variable => "variable",
            BindingKind::Alias => "type-alias",
            BindingKind::Function => "function-declaration",
            _ => return Ok(()),
        };
        if initializer.is_some_and(|init| matches!(init.kind, ast::InitializerKind::Delete)) {
            return Ok(());
        }
        self.binding_header(id, label, depth)?;
        if binding.kind != BindingKind::Variable {
            return Ok(());
        }
        if let Some(constructor) = self.graph.entity(binding.entity).constructor
            && !self
                .graph
                .entity(binding.entity)
                .variable
                .is_some_and(|properties| properties.is_extern)
        {
            self.constructor_action(id, constructor, initializer, depth + 1)?;
            return Ok(());
        }
        if let Some(init) = initializer {
            if self.copy_constructor_action(id, init, depth + 1)? {
                return Ok(());
            }
            if self.string_initializer(
                binding
                    .ty
                    .ok_or_else(|| io::Error::other("variable without type"))?,
                init,
                depth + 1,
            )? {
                return Ok(());
            }
            let retype = |id| {
                let facts = self.graph.expression_facts.get(id).ok()?;
                let conversion = facts.conversion?;
                if self.graph.entity(binding.entity).constexpr
                    && conversion.second == C::None
                    && !matches!(
                        self.graph.types.get(conversion.target).kind,
                        TypeKind::Reference { .. }
                    )
                {
                    Some((conversion.target, facts.category))
                } else {
                    None
                }
            };
            match init.kind {
                ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                    let retyped = retype(id);
                    self.work.push((
                        Item::Expression {
                            id,
                            bare: false,
                            retyped,
                        },
                        depth + 1,
                    ));
                }
                ast::InitializerKind::Parenthesized(arguments) => {
                    for &id in self.tree.expr_lists.get(arguments).iter().rev() {
                        let retyped = retype(id);
                        self.work.push((
                            Item::Expression {
                                id,
                                bare: false,
                                retyped,
                            },
                            depth + 1,
                        ));
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }
    fn variable_name(&mut self, id: BindingId) -> io::Result<()> {
        if let Some(name) = self.graph.binding(id).name {
            self.name(Some(name))
        } else {
            self.graph.write_storage_name(id, self.output)
        }
    }
    fn storage_expression(&mut self, id: BindingId, depth: usize) -> io::Result<()> {
        let ty = self
            .graph
            .binding(id)
            .ty
            .ok_or_else(|| io::Error::other("storage without type"))?;
        self.indent(depth)?;
        self.output.write_all(b"id-expression lvalue ")?;
        self.ty(ty)?;
        self.output.write_all(b" ")?;
        self.variable_name(id)?;
        writeln!(self.output)
    }
    fn copy_constructor_action(
        &mut self,
        variable: BindingId,
        initializer: ast::Initializer,
        depth: usize,
    ) -> io::Result<bool> {
        let Some(ty) = self.graph.binding(variable).ty else {
            return Ok(false);
        };
        let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
            return Ok(false);
        };
        let expression = match initializer.kind {
            ast::InitializerKind::Assign(id) => id,
            ast::InitializerKind::Parenthesized(arguments)
                if self.tree.expr_lists.get(arguments).len() == 1 =>
            {
                self.tree.expr_lists.get(arguments)[0]
            }
            _ => return Ok(false),
        };
        if self
            .graph
            .expression_facts
            .copy_action(expression)
            .is_none()
        {
            return Ok(false);
        }
        self.indent(depth)?;
        self.output.write_all(b"constructor-action ")?;
        self.graph
            .write_constructor_name(class, self.cursor, self.output, false)?;
        writeln!(self.output)?;
        let mut facts = self
            .graph
            .expression_facts
            .get(expression)
            .map_err(|_| io::Error::other("copy source without facts"))?;
        if let Some(selected) = facts.class_copy(&self.graph.expression_facts)
            && selected.binding.second == C::DerivedToBase
        {
            // The constructor action shows the selected reference binding,
            // while the source occurrence retains its original derived type.
            facts.conversion = Some(selected.binding);
            if self.conversion_node(
                expression,
                DisplayFacts::new(facts, &self.graph.expression_facts),
                depth + 1,
                None,
            )? {
                return Ok(true);
            }
        }
        self.work.push((
            Item::Expression {
                id: expression,
                bare: false,
                retyped: None,
            },
            depth + 1,
        ));
        Ok(true)
    }
    fn constructor_name(
        &mut self,
        id: super::construction::ConstructorId,
        declaration: bool,
    ) -> io::Result<()> {
        let class = self.graph.constructors.get(id).class;
        self.graph
            .write_constructor_name(class, self.cursor, self.output, declaration)
    }
    fn constructor_action(
        &mut self,
        variable: BindingId,
        id: super::construction::ConstructorId,
        initializer: Option<ast::Initializer>,
        depth: usize,
    ) -> io::Result<()> {
        let ty = self
            .graph
            .binding(variable)
            .ty
            .ok_or_else(|| io::Error::other("constructed variable without type"))?;
        self.indent(depth)?;
        if matches!(self.graph.types.get(ty).kind, TypeKind::Array { .. }) {
            self.output.write_all(b"constructor-array-action ")?;
            self.ty(ty)?;
            writeln!(self.output)?;
            self.indent(depth + 1)?;
            self.output.write_all(b"constructor-action ")?;
            self.constructor_name(id, false)?;
            return writeln!(self.output);
        }
        self.output.write_all(b"constructor-action ")?;
        self.constructor_name(id, false)?;
        writeln!(self.output)?;
        if let Some(initializer) = initializer {
            match initializer.kind {
                ast::InitializerKind::Parenthesized(arguments) => {
                    for &id in self.tree.expr_lists.get(arguments).iter().rev() {
                        self.work.push((
                            Item::Expression {
                                id,
                                bare: false,
                                retyped: None,
                            },
                            depth + 1,
                        ));
                    }
                }
                ast::InitializerKind::Assign(expression)
                | ast::InitializerKind::Braced(expression) => {
                    self.work.push((
                        Item::Expression {
                            id: expression,
                            bare: false,
                            retyped: None,
                        },
                        depth + 1,
                    ));
                }
                _ => return Err(io::Error::other("constructor initializer shape")),
            }
            return Ok(());
        }
        if self
            .graph
            .constructors
            .get(id)
            .binding
            .is_some_and(|binding| {
                self.graph
                    .inherited_constructors
                    .source(self.graph.binding(binding).entity)
                    .is_none()
            })
        {
            return Ok(());
        }
        self.line("call-expression prvalue void", depth + 1)?;
        self.indent(depth + 2)?;
        self.output.write_all(b"callee ")?;
        self.constructor_name(id, false)?;
        self.output.write_all(b" ")?;
        self.constructor_type(id)?;
        writeln!(self.output)?;
        self.indent(depth + 2)?;
        self.output
            .write_all(b"unary-expression prvalue pointer to ")?;
        self.ty(ty)?;
        writeln!(self.output, " OP_AMP:&")?;
        self.storage_expression(variable, depth + 3)
    }
    fn constructor_definition(
        &mut self,
        id: super::construction::ConstructorId,
        depth: usize,
    ) -> io::Result<()> {
        self.indent(depth)?;
        self.output.write_all(b"function-definition ")?;
        self.constructor_name(id, true)?;
        self.output.write_all(b" ")?;
        self.constructor_type(id)?;
        writeln!(self.output)?;
        self.indent(depth + 1)?;
        self.output.write_all(b"parameter this ")?;
        let object = self
            .graph
            .entity(self.graph.constructors.get(id).class)
            .ty
            .ok_or_else(|| io::Error::other("constructor class without type"))?;
        self.object_pointer(object, Cv::default())?;
        writeln!(self.output)?;
        self.line("compound-statement", depth + 1)
    }
    fn constructor_type(&mut self, id: super::construction::ConstructorId) -> io::Result<()> {
        let plan = self.graph.constructors.get(id);
        let object = self
            .graph
            .entity(plan.class)
            .ty
            .ok_or_else(|| io::Error::other("constructor class without type"))?;
        self.function_type_with_object(
            plan.callable_type
                .ok_or_else(|| io::Error::other("anonymous action is not callable"))?,
            object,
            Cv::default(),
        )
    }
    fn string_initializer(
        &mut self,
        ty: TypeId,
        initializer: ast::Initializer,
        depth: usize,
    ) -> io::Result<bool> {
        let TypeKind::Array { element, bound, .. } = self.graph.types.get(ty).kind else {
            return Ok(false);
        };
        let (ast::InitializerKind::Assign(mut id) | ast::InitializerKind::Braced(mut id)) =
            initializer.kind
        else {
            return Ok(false);
        };
        id = self.graph.expression_facts.root(id);
        if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind {
            let elements = self.tree.expr_lists.get(elements);
            if elements.len() != 1 {
                return Ok(false);
            }
            id = self.graph.expression_facts.root(elements[0]);
        }
        let ExprKind::Literal(value) = self.tree.expressions.get(id).kind else {
            return Ok(false);
        };
        let crate::lex::PostTokenKind::String { ty: literal, .. } =
            value.kind(&self.cursor.spellings)
        else {
            return Ok(false);
        };
        let data = value.data(&self.cursor.spellings);
        let bytes = self.cursor.spellings.get(data);
        let kind = self
            .graph
            .integral_kind(element)
            .ok_or_else(|| io::Error::other("character array without integral kind"))?;
        self.indent(depth)?;
        self.output.write_all(b"braced-init-list lvalue ")?;
        self.ty(ty)?;
        writeln!(self.output)?;
        let mut units = bytes.chunks_exact(literal.size());
        for _ in 0..bound {
            let mut bytes = [0u8; 8];
            if let Some(unit) = units.next() {
                bytes[..unit.len()].copy_from_slice(unit);
            }
            let bits = kind.convert(u64::from_le_bytes(bytes));
            self.indent(depth + 1)?;
            self.output.write_all(b"literal prvalue ")?;
            self.ty(element)?;
            if kind.signed {
                writeln!(self.output, " {}", super::integral::signed(bits))?;
            } else {
                writeln!(self.output, " {bits}")?;
            }
        }
        Ok(true)
    }
    fn condition(&mut self, condition: ast::Condition, depth: usize) -> io::Result<()> {
        self.line("condition", depth)?;
        match condition {
            ast::Condition::Expression(id) => self.expr(id, depth + 1),
            ast::Condition::Declaration {
                declarator,
                initializer,
                ..
            } => {
                self.line("condition-declaration", depth + 1)?;
                if let Some(binding) = self
                    .graph
                    .declared_bindings
                    .get(declarator.index())
                    .copied()
                    .flatten()
                {
                    self.graph.condition_conversion(declarator).map_err(|_| {
                        io::Error::other("condition declaration without conversion")
                    })?;
                    self.work
                        .push((Item::Binding(binding, Some(initializer)), depth + 2));
                }
            }
        }
        Ok(())
    }
    fn statement(&mut self, id: ast::StmtId, depth: usize) -> io::Result<()> {
        match self.tree.statements.get(id).kind {
            StmtKind::Ambiguous { readings, .. } => {
                let reading = self
                    .graph
                    .statement_readings
                    .get(id.index())
                    .copied()
                    .flatten()
                    .or_else(|| {
                        self.tree
                            .statement_readings
                            .get(readings)
                            .first()
                            .map(|reading| reading.statement)
                    })
                    .ok_or_else(|| io::Error::other("statement without reading"))?;
                self.work.push((Item::Statement(reading), depth));
            }
            StmtKind::Compound(items) => {
                self.line("compound-statement", depth)?;
                self.work.extend(
                    self.tree
                        .stmt_lists
                        .get(items)
                        .iter()
                        .rev()
                        .map(|&id| (Item::Statement(id), depth + 1)),
                );
            }
            StmtKind::Declaration(id) => self.work.push((Item::Declaration(id, true), depth)),
            StmtKind::Expression(operand) | StmtKind::Return(operand) => {
                self.line(
                    if matches!(self.tree.statements.get(id).kind, StmtKind::Return(_)) {
                        "return-statement"
                    } else {
                        "expression-statement"
                    },
                    depth,
                )?;
                if let Some(id) = operand {
                    self.expr(id, depth + 1);
                }
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.line("if-statement", depth)?;
                if let Some(id) = else_branch {
                    self.work.push((Item::Statement(id), depth + 2));
                    self.work.push((Item::Line("else"), depth + 1));
                }
                self.work.push((Item::Statement(then_branch), depth + 2));
                self.work.push((Item::Line("then"), depth + 1));
                self.work.push((Item::Condition(condition), depth + 1));
            }
            StmtKind::ConditionalLoop {
                keyword,
                condition,
                body,
            } => {
                self.line(
                    if keyword == Keyword::Switch {
                        "switch-statement"
                    } else {
                        "while-statement"
                    },
                    depth,
                )?;
                self.work.push((Item::Statement(body), depth + 1));
                self.work.push((Item::Condition(condition), depth + 1));
            }
            StmtKind::Do { body, condition } => {
                self.line("do-statement", depth)?;
                self.expr(condition, depth + 2);
                self.work.push((Item::Line("condition"), depth + 1));
                self.work.push((Item::Statement(body), depth + 1));
            }
            StmtKind::For {
                declaration,
                init,
                condition,
                iteration,
                body,
            } => {
                self.line("for-statement", depth)?;
                self.work.push((Item::Statement(body), depth + 1));
                if let Some(id) = iteration {
                    self.expr(id, depth + 2);
                    self.work.push((Item::Line("iteration"), depth + 1));
                }
                if let Some(condition) = condition {
                    self.work.push((Item::Condition(condition), depth + 1));
                }
                if let Some(id) = declaration {
                    self.work.push((Item::Declaration(id, true), depth + 2));
                } else if let Some(id) = init {
                    self.expr(id, depth + 2);
                }
                self.work
                    .push((Item::Line("for-init-statement"), depth + 1));
            }
            StmtKind::Case { value, body } => {
                self.line("case-statement", depth)?;
                self.work.push((Item::Statement(body), depth + 1));
                self.expr(value, depth + 1);
            }
            StmtKind::Default(body) => {
                self.line("default-statement", depth)?;
                self.work.push((Item::Statement(body), depth + 1));
            }
            StmtKind::Break => self.line("break-statement", depth)?,
            StmtKind::Continue => self.line("continue-statement", depth)?,
            StmtKind::Label { body, .. } => self.work.push((Item::Statement(body), depth)),
            _ => (),
        }
        Ok(())
    }
    fn predefined_id(
        &mut self,
        kind: ExprKind,
        ty: TypeId,
        category: V,
        depth: usize,
    ) -> io::Result<bool> {
        if let ExprKind::Id(name) = kind
            && let Some(binding) = self.graph.resolved_name(name)
        {
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if self.graph.binding(binding).kind == BindingKind::Variable
                && entity
                    .home
                    .is_some_and(|home| self.graph.scope(home).kind == ScopeKind::Function)
                && entity
                    .name
                    .is_some_and(|name| self.cursor.name(name) == b"__func__")
                && let Some(function) = entity.home.and_then(|home| self.graph.scope(home).owner)
            {
                self.indent(depth)?;
                write!(self.output, "literal {} ", category.spelling())?;
                self.ty(ty)?;
                self.output.write_all(b" \"")?;
                // Printer::id writes the owner's label, including token-joined operators.
                let binding = self
                    .graph
                    .function_origin(function)
                    .map_err(|_| io::Error::other("predefined owner without binding"))?;
                let function = self.graph.entity(function);
                if self
                    .graph
                    .lower_call_closure(self.graph.binding(binding).entity)
                    .is_some()
                {
                    self.output.write_all(b"operator()")?;
                } else if let Some(member) = function
                    .member_function
                    .filter(|member| member.kind == super::graph::MemberKind::Constructor)
                {
                    self.name(self.graph.entity(member.class).name)?;
                } else if let Some(super::function_names::SpecialName::Operator { token, array }) =
                    function
                        .function
                        .filter(|properties| properties.allocation.is_some())
                        .and_then(|properties| properties.special)
                {
                    write!(
                        self.output,
                        "operator {}{}",
                        token.spelling(),
                        if array { "[]" } else { "" }
                    )?;
                } else if let Some(name) = function.special_name {
                    crate::syntax::write_member_label(self.tree, self.cursor, name, self.output)?;
                } else if function
                    .member_function
                    .is_some_and(|member| member.kind == super::graph::MemberKind::Destructor)
                {
                    self.output.write_all(b"~")?;
                    self.name(function.name)?;
                } else {
                    self.name(self.graph.binding(binding).name)?;
                }
                writeln!(self.output, "\"")?;
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn id_node(
        &mut self,
        id: ast::ExprId,
        name: ast::Name,
        facts: DisplayFacts,
        depth: usize,
    ) -> io::Result<()> {
        self.output.write_all(b" ")?;
        if let Some(storage) = facts.anonymous_member() {
            let binding = self
                .graph
                .resolved_names
                .get(name.index())
                .copied()
                .flatten()
                .ok_or_else(|| io::Error::other("anonymous member without designation"))?;
            self.name(self.graph.binding(binding).name)?;
            self.work.push((Item::Storage(storage), depth + 1));
        } else if let Some(object) = facts.object()
            && matches!(
                object.source,
                super::expression_facts::ObjectSource::Implicit
            )
        {
            let binding = facts
                .member()
                .ok_or_else(|| io::Error::other("implicit member without designation"))?;
            self.name(self.graph.binding(binding).name)?;
            self.work
                .push((self.implicit_this(id, object.ty), depth + 1));
        } else if let Some(binding) = facts.member()
            && self
                .graph
                .entity(self.graph.binding(binding).entity)
                .data_member
                == Some(super::graph::DataMemberKind::Instance)
        {
            // A qualified data-member address retains its selected declaration
            // without an object; the oracle prints that declaration's own name.
            self.name(self.graph.binding(binding).name)?;
        } else {
            crate::syntax::write_expression_name(self.tree, self.cursor, name, self.output)?;
        }
        Ok(())
    }
    fn member_node(
        &mut self,
        base: ast::ExprId,
        op: ast::Operator,
        facts: DisplayFacts,
        depth: usize,
    ) -> io::Result<()> {
        if matches!(
            facts.destructor(),
            Some(super::expression_facts::DestructorTarget::Scalar)
        ) {
            write!(self.output, " {}:", op.token.name())?;
            self.arrow_operand(base, depth + 1);
            return Ok(());
        }
        let binding = facts
            .member()
            .ok_or_else(|| io::Error::other("member without designation"))?;
        if facts.function() == Some(binding)
            && self
                .graph
                .entity(self.graph.binding(binding).entity)
                .member_function
                .is_some_and(|member| member.kind == super::graph::MemberKind::Conversion)
        {
            write!(self.output, " {}:operator <conversion>", op.token.name())?;
            self.arrow_operand(base, depth + 1);
            return Ok(());
        }
        if facts.member_naming().is_none() {
            return Err(io::Error::other("member without naming class"));
        }
        write!(self.output, " {}:", op.token.name())?;
        self.name(self.graph.binding(binding).name)?;
        self.arrow_operand(base, depth + 1);
        Ok(())
    }
    fn arrow_operand(&mut self, base: ast::ExprId, depth: usize) {
        if let Some(last) = self
            .graph
            .expression_facts
            .arrow_calls(base)
            .len()
            .checked_sub(1)
        {
            self.work.push((Item::ArrowCall(base, last), depth));
        } else {
            self.expr(base, depth);
        }
    }
    fn arrow_call(&mut self, base: ast::ExprId, index: usize, depth: usize) -> io::Result<()> {
        let step = self.graph.expression_facts.arrow_calls(base)[index];
        let binding = self
            .graph
            .function_origin(self.graph.binding(step.binding).entity)
            .map_err(|_| io::Error::other("arrow without origin"))?;
        let function = self
            .graph
            .binding(binding)
            .ty
            .ok_or_else(|| io::Error::other("arrow without type"))?;
        let TypeKind::Function { result, .. } = self.graph.types.get(function).kind else {
            return Err(io::Error::other("arrow without signature"));
        };
        self.indent(depth)?;
        write!(self.output, "call-expression {} ", step.category.spelling())?;
        self.ty(result)?;
        writeln!(self.output)?;
        self.indent(depth + 1)?;
        self.output.write_all(b"callee ")?;
        self.function_name(binding)?;
        self.output.write_all(b" ")?;
        self.function_type(binding)?;
        writeln!(self.output)?;
        let (owner, cv) = self
            .function_object(binding)
            .ok_or_else(|| io::Error::other("arrow without owner"))?;
        let at = depth + 1 + usize::from(step.path.is_some());
        if step.path.is_some() {
            self.indent(depth + 1)?;
            self.output.write_all(b"cast-expression prvalue ")?;
            self.object_pointer(owner, cv)?;
            writeln!(self.output)?;
        }
        let ty = if let Some(previous) = index.checked_sub(1) {
            self.graph.expression_facts.arrow_calls(base)[previous].result
        } else {
            self.graph
                .expression_facts
                .get(base)
                .map_err(|_| io::Error::other("arrow base without facts"))?
                .ty
        };
        self.indent(at)?;
        self.output
            .write_all(b"unary-expression prvalue pointer to ")?;
        self.ty(ty)?;
        writeln!(self.output, " OP_AMP:&")?;
        if let Some(previous) = index.checked_sub(1) {
            self.work.push((Item::ArrowCall(base, previous), at + 1));
        } else {
            self.expr(base, at + 1);
        }
        Ok(())
    }
    fn conversion_node(
        &mut self,
        id: ast::ExprId,
        facts: DisplayFacts,
        depth: usize,
        retyped: Option<(TypeId, V)>,
    ) -> io::Result<bool> {
        let conversion = facts
            .conversion
            .ok_or_else(|| io::Error::other("conversion view without facts"))?;
        // record_copy presents its outer standard constructor conversion, not
        // the derived-to-base binding used for the constructor's argument.
        // Keep that retained binding available to lowering.
        if self.graph.expression_facts.copy_action(id).is_some()
            && matches!(
                self.graph.types.get(conversion.target).kind,
                TypeKind::Class(_)
            )
        {
            return Ok(false);
        }
        if conversion.second == C::DerivedToBase
            || conversion.temporary
                && (conversion.second != C::None
                    || matches!(conversion.first, L::ArrayToPointer | L::FunctionToPointer))
        {
            self.indent(depth)?;
            if conversion.second == C::DerivedToBase && facts.base_path().is_none() {
                return Err(io::Error::other("base conversion without retained path"));
            }
            self.output.write_all(b"cast-expression ")?;
            let category = if conversion.temporary {
                V::Prvalue
            } else {
                match self.graph.types.get(conversion.target).kind {
                    TypeKind::Reference {
                        kind: super::types::RefKind::Lvalue,
                        ..
                    } => V::Lvalue,
                    TypeKind::Reference { .. } => V::Xvalue,
                    _ => V::Prvalue,
                }
            };
            write!(self.output, "{} ", category.spelling())?;
            self.ty(conversion.result)?;
            writeln!(self.output)?;
            self.work.push((
                Item::Expression {
                    id,
                    bare: true,
                    retyped,
                },
                depth + 1,
            ));
            return Ok(true);
        }
        Ok(false)
    }

    fn expression(
        &mut self,
        id: ast::ExprId,
        depth: usize,
        bare: bool,
        retyped: Option<(TypeId, V)>,
    ) -> io::Result<()> {
        let id = self.graph.expression_facts.root(id);
        let Some(facts) = self.graph.expression_facts.known(id) else {
            return Ok(());
        };
        let mut facts = DisplayFacts::new(facts, &self.graph.expression_facts);
        if let Some(converted) = self.graph.expression_facts.function_conversion(id)
            && matches!(
                converted.usage,
                super::conversion_functions::ConversionUse::Builtin
                    | super::conversion_functions::ConversionUse::Contextual
            )
        {
            // convert_class_operand replaces the C++ occurrence's facts with
            // its selected call. Rust keeps the receiver facts for lowering;
            // project that same presentation from the retained typed edge.
            let conversion = facts.conversion;
            facts = DisplayFacts::new(
                super::expression_facts::ExprFacts::new(converted.result, converted.category),
                &self.graph.expression_facts,
            );
            facts.details.function = Some(converted.binding);
            facts.details.member = Some(converted.binding);
            facts.details.call = Some(CallTarget::Direct(converted.binding));
            facts.core.conversion = conversion;
            facts.details.base_path = converted.after_path;
        }
        if let Some(outer) = self.graph.expression_facts.user_conversion(id) {
            facts.core.conversion = Some(outer);
        }
        let (mut ty, mut category) = retyped.unwrap_or((facts.ty, facts.category));
        if !bare
            && let Some(conversion) = facts.conversion
            && conversion.initialization
        {
            if self.conversion_node(id, facts, depth, retyped)? {
                return Ok(());
            }
            if conversion.second == C::NullPointer && facts.null_literal() {
                ty = conversion.result;
            }
        }
        let node = self.tree.expressions.get(id);
        if let Some(bits) = facts.folded()
            && facts.fold_style() == super::expression_facts::FoldStyle::Literal
        {
            self.indent(depth)?;
            self.output.write_all(b"literal prvalue ")?;
            self.ty(retyped.map_or(facts.ty, |(ty, _)| ty))?;
            let signed = self
                .graph
                .integral_kind(facts.ty)
                .is_some_and(|kind| kind.signed);
            if signed {
                writeln!(self.output, " {}", super::integral::signed(bits))?;
            } else {
                writeln!(self.output, " {bits}")?;
            }
            return Ok(());
        }
        if let ExprKind::Cast { operand, .. } = node.kind
            && let Some(target) = facts.reference_cast()
        {
            self.work.push((
                Item::Expression {
                    id: operand,
                    bare: true,
                    retyped: Some((target, facts.category)),
                },
                depth,
            ));
            return Ok(());
        }
        if (matches!(node.kind, ExprKind::FunctionalCast { .. }) || facts.functional_call())
            && let Some(target) = facts.reference_cast()
        {
            ty = target;
        }
        if facts.call_form() != super::expression_facts::CallForm::Operator
            && facts.literal_call().is_none()
            && self
                .graph
                .expression_facts
                .function_conversion(id)
                .is_none_or(|conversion| {
                    conversion.usage == super::conversion_functions::ConversionUse::Scalar
                })
            && let Some(call) = facts.call()
        {
            let function = match call {
                CallTarget::Direct(binding) => self
                    .graph
                    .binding(binding)
                    .ty
                    .ok_or_else(|| io::Error::other("call without function type"))?,
                CallTarget::Indirect(ty, _) => ty,
            };
            let function = self.graph.types.canonical_view(function);
            let TypeKind::Function { result, .. } = self.graph.types.get(function).kind else {
                return Err(io::Error::other("call without function type"));
            };
            ty = result;
            // Printer::call uses the call's own category and declared result,
            // including when a reference cast retypes its presentation item.
            category = facts.category;
        }
        if self.graph.expression_facts.member_data_call_view(id) {
            let ExprKind::Call { callee, arguments } = node.kind else {
                return Err(io::Error::other("member data call view without call"));
            };
            let callable = self.member_data_call_header(callee, facts.category, depth)?;
            for &argument in self.tree.expr_lists.get(arguments).iter().rev() {
                self.expr(argument, depth + 1);
            }
            self.expr(callable, depth + 1);
            return Ok(());
        }
        let intrinsic = self.graph.expression_facts.is_intrinsic(id, facts.core)
            || self.graph.expression_facts.cast_call_view(id);
        if intrinsic && matches!(node.kind, ExprKind::Call { .. }) {
            // Printer::call reads an expanded builtin's own result facts,
            // independently of the surrounding initialization's retyped item.
            ty = facts.reference_cast().unwrap_or(facts.ty);
            category = facts.category;
        }
        if self.predefined_id(node.kind, ty, category, depth)? {
            return Ok(());
        }
        let label = expression_label(node.kind, facts, intrinsic)?;
        self.indent(depth)?;
        write!(self.output, "{label} {} ", category.spelling())?;
        if matches!(node.kind, ExprKind::Id(_))
            && let Some(binding) = facts.function()
        {
            self.function_type(binding)?;
        } else if !matches!(node.kind, ExprKind::Unary { .. }) || facts.functions().is_none() {
            // An unresolved unary overload designation has no expression type.
            // Selection rewrites its outermost node and immediate operand only.
            self.ty(ty)?;
        }
        match node.kind {
            ExprKind::Id(name) => self.id_node(id, name, facts, depth)?,
            ExprKind::Literal(literal) => {
                self.output.write_all(b" ")?;
                self.output.write_all(
                    self.cursor
                        .spellings
                        .get(literal.spelling(&self.cursor.spellings)),
                )?;
            }
            ExprKind::Keyword(Keyword::This) => self.output.write_all(b" this")?,
            ExprKind::Keyword(keyword) => write!(
                self.output,
                " {}:{}",
                SimpleToken::keyword(keyword).name(),
                keyword.spelling()
            )?,
            ExprKind::Unary { op, operand } | ExprKind::Postfix { op, operand } => {
                self.token(op)?;
                self.expr(operand, depth + 1);
            }
            ExprKind::Binary {
                op, left, right, ..
            } => {
                self.token(op)?;
                self.expr(right, depth + 1);
                self.expr(left, depth + 1);
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                self.expr(else_expr, depth + 1);
                self.expr(then_expr, depth + 1);
                self.expr(condition, depth + 1);
            }
            ExprKind::Cast {
                keyword, operand, ..
            } => {
                if let Some(keyword) = keyword {
                    write!(
                        self.output,
                        " {}:{}",
                        SimpleToken::keyword(keyword).name(),
                        keyword.spelling()
                    )?;
                } else {
                    self.output.write_all(b" OP_LPAREN:")?;
                }
                self.expr(operand, depth + 1);
            }
            ExprKind::Call { arguments, .. } if facts.functional_call() || intrinsic => {
                for &id in self.tree.expr_lists.get(arguments).iter().rev() {
                    self.expr(id, depth + 1);
                }
            }
            ExprKind::Call { callee, arguments } => {
                return self.call_node(id, callee, arguments, facts, depth);
            }
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => {
                let args = if let Some(braced) = braced {
                    let ExprKind::Braced { elements, .. } = self.tree.expressions.get(braced).kind
                    else {
                        return Err(io::Error::other("cast without list"));
                    };
                    elements
                } else {
                    arguments
                };
                for &id in self.tree.expr_lists.get(args).iter().rev() {
                    self.expr(id, depth + 1);
                }
            }
            ExprKind::Braced { elements, .. } => {
                for &id in self.tree.expr_lists.get(elements).iter().rev() {
                    self.expr(id, depth + 1);
                }
            }
            ExprKind::Member { base, op, .. } => self.member_node(base, op, facts, depth)?,
            ExprKind::Subscript { base, index } => {
                self.expr(if facts.commuted() { base } else { index }, depth + 1);
                self.expr(if facts.commuted() { index } else { base }, depth + 1);
            }
            _ => (),
        }
        writeln!(self.output)
    }
    fn member_data_call_header(
        &mut self,
        mut callee: ast::ExprId,
        category: V,
        depth: usize,
    ) -> io::Result<ast::ExprId> {
        // Printer::call strips the untyped outer parentheses, then reads the
        // data member pointer's element as a function signature. Preserve its
        // element (if any) as the displayed return type, independent of the
        // actual access type that lowering consumes.
        while let ExprKind::Paren(inner) = self.tree.expressions.get(callee).kind {
            callee = inner;
        }
        let facts = self
            .graph
            .expression_facts
            .get(callee)
            .map_err(|_| io::Error::other("member data callee without facts"))?;
        let TypeKind::MemberPointer { element, .. } = self.graph.types.get(facts.ty).kind else {
            return Err(io::Error::other("member data callee without pointer type"));
        };
        let displayed = match self.graph.types.get(element).kind {
            TypeKind::Pointer(element)
            | TypeKind::Array { element, .. }
            | TypeKind::MemberPointer { element, .. } => Some(element),
            _ => None,
        };
        self.indent(depth)?;
        write!(self.output, "call-expression {} ", category.spelling())?;
        if let Some(ty) = displayed {
            self.ty(ty)?;
        }
        writeln!(self.output)?;
        Ok(callee)
    }

    fn call_node(
        &mut self,
        id: ast::ExprId,
        callee: ast::ExprId,
        arguments: crate::syntax::storage::List<ast::ExprId>,
        facts: DisplayFacts,
        depth: usize,
    ) -> io::Result<()> {
        writeln!(self.output)?;
        if matches!(
            facts.destructor(),
            Some(super::expression_facts::DestructorTarget::Scalar)
        ) {
            self.expr(callee, depth + 1);
            return Ok(());
        }
        match facts
            .call()
            .ok_or_else(|| io::Error::other("call without target"))?
        {
            CallTarget::Direct(binding) => {
                self.binding_header(binding, "callee", depth + 1)?;
                let entity = self.graph.binding(binding).entity;
                for default in self
                    .graph
                    .entity(entity)
                    .defaults
                    .iter()
                    .skip(self.tree.expr_lists.get(arguments).len())
                    .rev()
                    .flatten()
                {
                    self.expr(default.expression, depth + 1);
                }
            }
            CallTarget::Indirect(_, _) => (),
        }
        for &id in self.tree.expr_lists.get(arguments).iter().rev() {
            self.expr(id, depth + 1);
        }
        if matches!(facts.call(), Some(CallTarget::Indirect(_, _))) {
            self.expr(callee, depth + 1);
        } else if let Some(CallTarget::Direct(binding)) = facts.call() {
            let call = (facts.call_form() == super::expression_facts::CallForm::CallableObject)
                .then_some(id);
            self.work
                .push((Item::ObjectReceiver(callee, binding, call), depth + 1));
        }
        Ok(())
    }
    fn token(&mut self, op: ast::Operator) -> io::Result<()> {
        write!(
            self.output,
            " {}:{}",
            op.token.name(),
            crate::syntax::operator_spelling(op)
        )
    }
}

fn expression_label(
    kind: ExprKind,
    facts: DisplayFacts,
    intrinsic: bool,
) -> io::Result<&'static str> {
    Ok(match kind {
        ExprKind::Id(_) if facts.anonymous_member().is_some() => "member-expression",
        ExprKind::Id(_)
            if facts.anonymous_member().is_some()
                || facts.object().is_some_and(|object| {
                    matches!(
                        object.source,
                        super::expression_facts::ObjectSource::Implicit
                    )
                }) =>
        {
            "member-expression"
        }
        ExprKind::Id(_) | ExprKind::Keyword(Keyword::This) => "id-expression",
        ExprKind::Literal(_) | ExprKind::Keyword(_) => "literal",
        ExprKind::Unary { .. } => "unary-expression",
        ExprKind::Postfix { .. } => "postfix-expression",
        ExprKind::Binary { assignment, .. } => {
            if assignment {
                "assignment-expression"
            } else {
                "binary-expression"
            }
        }
        ExprKind::Conditional { .. } => "conditional-expression",
        ExprKind::Subscript { .. } => "subscript-expression",
        ExprKind::Member { .. } => "member-expression",
        ExprKind::Typeof { .. }
        | ExprKind::TypeTransform { .. }
        | ExprKind::TypePredicate { .. }
        | ExprKind::OffsetOf { .. }
        | ExprKind::OffsetOfBase(_)
        | ExprKind::Sizeof { .. }
        | ExprKind::TypeTrait { .. }
        | ExprKind::ComplexPart { .. }
        | ExprKind::BuiltinValue { .. }
        | ExprKind::VaArg { .. } => "sizeof-expression",
        ExprKind::Cast { .. } | ExprKind::FunctionalCast { .. } => "cast-expression",
        ExprKind::Braced { .. } => "braced-init-list",
        ExprKind::New { .. }
        | ExprKind::Delete { .. }
        | ExprKind::SizeofPack(_)
        | ExprKind::Lambda(_)
        | ExprKind::Statement(_) => "expression",
        ExprKind::Call { .. } => {
            if facts.functional_call() || intrinsic {
                "cast-expression"
            } else {
                "call-expression"
            }
        }
        _ => return Err(io::Error::other("unsupported semantic expression view")),
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{conversion_functions::ConversionUse, template_bodies::analyze_unit},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn deferred_wrapper_views_consume_only_published_facts() {
        let source = SourceBuffer::new(
            b"inline int f(){int x=__builtin_unprovided();return x;}\
              inline int g(){auto x=__builtin_unprovided();using Ignored=int;void ignored();missing();}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"deferred-view").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let mark = tree.expressions.mark();
        let mut output = Vec::new();
        graph
            .print_semantics(tree, cursor, declarations, &mut output)
            .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("function-definition f "));
        assert!(output.contains("simple-declaration"));
        assert!(!output.contains("__builtin_unprovided"));
        assert!(!output.contains("missing"));
        assert!(!output.contains("Ignored"));
        assert!(!output.contains("ignored"));
        assert_eq!(tree.expressions.mark(), mark);
    }

    #[test]
    fn scalar_conversion_keeps_the_receivers_declared_reference_call_view() {
        let source = SourceBuffer::new(
            b"struct A{operator int();};A& effect();int f(){return effect();}".to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"reference-call-view").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let declaration = *tree.decl_lists.get(declarations).last().unwrap();
        let crate::syntax::ast::DeclKind::Function { body, .. } =
            tree.declarations.get(declaration).kind
        else {
            panic!("function")
        };
        let crate::syntax::ast::Body::Parsed(body) = *tree.bodies.get(body) else {
            panic!("parsed body")
        };
        let crate::syntax::ast::StmtKind::Compound(items) = tree.statements.get(body.compound).kind
        else {
            panic!("compound")
        };
        let crate::syntax::ast::StmtKind::Return(Some(call)) =
            tree.statements.get(tree.stmt_lists.get(items)[0]).kind
        else {
            panic!("return")
        };
        let facts = graph.expression_facts.get(call).unwrap();
        assert!(graph.class_of(facts.ty).is_some());
        assert!(
            graph
                .expression_facts
                .function_conversion(call)
                .unwrap()
                .usage
                == ConversionUse::Scalar
        );
        let mut output = Vec::new();
        graph
            .print_semantics(tree, cursor, declarations, &mut output)
            .unwrap();
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("call-expression lvalue lvalue-reference to struct A")
        );
        assert_eq!(graph.expression_facts.get(call).unwrap().ty, facts.ty);
    }

    #[test]
    fn builtin_conversion_view_preserves_receiver_facts_and_selected_demand() {
        for (operand, expected) in [
            (
                "a + 1",
                "id-expression prvalue function of (pointer to struct A) returning int a",
            ),
            (
                "((a)) + 1",
                "id-expression prvalue function of (pointer to struct A) returning int a",
            ),
            (
                "b.value + 1",
                "member-expression prvalue int OP_DOT:operator <conversion>",
            ),
            (
                "make() + 1",
                "call-expression prvalue int\n            callee A::operatorint function of (pointer to struct A) returning int",
            ),
            (
                "+a",
                "id-expression prvalue function of (pointer to struct A) returning int a",
            ),
        ] {
            let source = SourceBuffer::new(
                format!(
                    "struct A{{operator int(){{return 1;}}}};struct B{{A value;}};\
                 A make(){{return A();}}\
                 template<class T>int operator+(int,T){{return T::missing;}}\
                 int main(){{A a;B b;return {operand};}}"
                )
                .into_bytes(),
            )
            .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"converted-view").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let graph = analyze_unit(
                tree,
                cursor,
                declarations,
                StackBudget::default(),
                true,
                true,
            )
            .unwrap();
            let mut receivers = Vec::new();
            // Dense expression ordinals belong to the original unit slabs.
            // Collect the retained edges through syntax, without manufacturing IDs.
            let declaration = *tree.decl_lists.get(declarations).last().unwrap();
            let crate::syntax::ast::DeclKind::Function { body, .. } =
                tree.declarations.get(declaration).kind
            else {
                panic!("main definition");
            };
            let crate::syntax::ast::Body::Parsed(body) = *tree.bodies.get(body) else {
                panic!("parsed body");
            };
            let crate::syntax::ast::StmtKind::Compound(statements) =
                tree.statements.get(body.compound).kind
            else {
                panic!("compound body");
            };
            for &statement in tree.stmt_lists.get(statements) {
                if let crate::syntax::ast::StmtKind::Return(Some(expression)) =
                    tree.statements.get(statement).kind
                {
                    let operand = match tree.expressions.get(expression).kind {
                        crate::syntax::ast::ExprKind::Binary { left, .. } => left,
                        crate::syntax::ast::ExprKind::Unary { operand, .. } => operand,
                        _ => panic!("operator expression"),
                    };
                    let original = graph.expression_facts.get(operand).unwrap();
                    let selected = graph.expression_facts.function_conversion(operand).unwrap();
                    assert!(selected.usage == ConversionUse::Builtin);
                    assert!(graph.class_of(original.ty).is_some());
                    receivers.push((operand, original.ty, original.category, selected.binding));
                }
            }
            assert_eq!(receivers.len(), 1);
            let mut output = Vec::new();
            graph
                .print_semantics(tree, cursor, declarations, &mut output)
                .unwrap();
            let output = String::from_utf8(output).unwrap();
            assert!(output.contains(expected), "{operand}: {output}");
            for (operand, ty, category, binding) in receivers {
                let facts = graph.expression_facts.get(operand).unwrap();
                assert_eq!(facts.ty, ty);
                assert_eq!(facts.category, category);
                assert_eq!(
                    graph
                        .expression_facts
                        .function_conversion(operand)
                        .unwrap()
                        .binding,
                    binding
                );
            }
            for demand in crate::sema::function_demands::trailing(&graph) {
                if let crate::sema::function_demands::Demand::Function { binding, .. } = demand {
                    assert!(
                        graph
                            .function_templates
                            .primary(graph.binding(binding).entity)
                            .is_none()
                    );
                }
            }
        }
    }
}
