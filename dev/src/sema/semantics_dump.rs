// Translated from sema/semantics_dump.cpp. See NOTICE.
//! An iterative view of original syntax and recorded semantic facts.
use super::{
    expression_facts::{LvalueTransform as L, ValueCategory as V, ValueConversion as C},
    function_resolution::CallTarget,
    graph::{BindingId, BindingKind, Graph, ScopeKind},
    types::{TypeId, TypeKind},
};
use crate::{
    lex::{
        identifiers::NameId,
        vocabulary::{Keyword, SimpleToken},
    },
    syntax::{
        ast::{self, DeclKind, ExprKind, StmtKind},
        cursor::Cursor,
        storage::List,
    },
};
use std::io::{self, Write};
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
    ObjectReceiver(ast::ExprId, BindingId),
    This(TypeId),
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
                Item::ObjectReceiver(callee, binding) => {
                    printer.object_receiver(callee, binding, depth)?;
                }
                Item::This(ty) => printer.this_pointer(ty, depth)?,
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
                        if let Some(primary) = printer
                            .graph
                            .function_templates
                            .primary(printer.graph.binding(binding).entity)
                        {
                            printer.specialization_parameters(binding, primary, depth + 1)?;
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
    fn specialization_parameters(
        &mut self,
        binding: BindingId,
        primary: BindingId,
        depth: usize,
    ) -> io::Result<()> {
        let ty = self
            .graph
            .binding(binding)
            .ty
            .ok_or_else(|| io::Error::other("specialization without type"))?;
        let clause = self
            .graph
            .entity(self.graph.binding(primary).entity)
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
    fn declared(&self, id: ast::DeclaratorId) -> io::Result<BindingId> {
        self.graph
            .declared_bindings
            .get(id.index())
            .copied()
            .flatten()
            .ok_or_else(|| io::Error::other("declarator without binding"))
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
                    let binding = self.declared(init.declarator)?;
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
                self.indent(depth)?;
                self.output.write_all(b"type-alias ")?;
                self.name(Some(name))?;
                self.output.write_all(b" ")?;
                self.ty(self
                    .graph
                    .type_fact(ty)
                    .ok_or_else(|| io::Error::other("alias without type"))?)?;
                writeln!(self.output)?;
            }
            DeclKind::Function {
                declarator, body, ..
            } => {
                let binding = self.declared(declarator)?;
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
                let binding = self.declared(declarator)?;
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
            if let Some(name) = node.name {
                parts.push(name);
            }
            scope = node.parent;
        }
        for name in parts.into_iter().rev() {
            self.name(Some(name))?;
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
        } else if let Some(name) = entity.special_name {
            crate::syntax::write_member_label(self.tree, self.cursor, name, self.output)?;
        } else if let Some(key) = entity.member_function
            && key.kind == super::graph::MemberKind::Destructor
        {
            self.output.write_all(b"~")?;
            self.name(self.graph.entity(key.class).name)?;
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
        let Some(object) = facts.object else {
            return Ok(());
        };
        let converted = facts
            .conversion
            .is_some_and(|conversion| conversion.second == C::DerivedToBase);
        let at = depth + usize::from(converted);
        if converted {
            self.indent(depth)?;
            self.output.write_all(b"cast-expression prvalue ")?;
            self.object_pointer(owner, cv)?;
            writeln!(self.output)?;
        }
        match object.source {
            super::expression_facts::ObjectSource::Implicit => {
                self.this_pointer(object.ty, at)?;
            }
            super::expression_facts::ObjectSource::Explicit { expression, arrow } => {
                if !arrow {
                    self.indent(at)?;
                    self.output
                        .write_all(b"unary-expression prvalue pointer to ")?;
                    self.ty(object.ty)?;
                    writeln!(self.output, " OP_AMP:&")?;
                }
                self.expr(expression, at + usize::from(!arrow));
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
    fn function_type(&mut self, id: BindingId) -> io::Result<()> {
        let ty = self
            .graph
            .binding(id)
            .ty
            .ok_or_else(|| io::Error::other("function without type"))?;
        let ty = self.graph.types.canonical_view(ty);
        let Some((object, cv)) = self.function_object(id) else {
            return self.ty(ty);
        };
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
                    for &argument in self.tree.expr_lists.get(arguments) {
                        self.expression(argument, depth + 1, false, None)?;
                    }
                }
                ast::InitializerKind::Assign(expression)
                | ast::InitializerKind::Braced(expression) => {
                    self.expression(expression, depth + 1, false, None)?;
                }
                _ => return Err(io::Error::other("constructor initializer shape")),
            }
            return Ok(());
        }
        if self.graph.constructors.get(id).binding.is_some() {
            return Ok(());
        }
        self.line("call-expression prvalue void", depth + 1)?;
        self.indent(depth + 2)?;
        self.output.write_all(b"callee ")?;
        self.constructor_name(id, false)?;
        self.output.write_all(b" ")?;
        self.ty(self.graph.constructors.get(id).presentation_type)?;
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
        self.ty(self.graph.constructors.get(id).presentation_type)?;
        writeln!(self.output)?;
        self.indent(depth + 1)?;
        self.output.write_all(b"parameter this ")?;
        self.ty(self.graph.constructors.get(id).object_pointer)?;
        writeln!(self.output)?;
        self.line("compound-statement", depth + 1)
    }
    fn string_initializer(
        &mut self,
        ty: TypeId,
        initializer: ast::Initializer,
        depth: usize,
    ) -> io::Result<bool> {
        let TypeKind::Array { element, bound } = self.graph.types.get(ty).kind else {
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
        let ExprKind::Literal(ast::Literal {
            kind: crate::lex::PostTokenKind::String { ty: literal, .. },
            data,
            ..
        }) = self.tree.expressions.get(id).kind
        else {
            return Ok(false);
        };
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
                self.graph
                    .condition_conversion(declarator)
                    .map_err(|_| io::Error::other("condition declaration without conversion"))?;
                self.line("condition-declaration", depth + 1)?;
                let binding = self.declared(declarator)?;
                self.work
                    .push((Item::Binding(binding, Some(initializer)), depth + 2));
            }
        }
        Ok(())
    }
    fn statement(&mut self, id: ast::StmtId, depth: usize) -> io::Result<()> {
        match self.tree.statements.get(id).kind {
            StmtKind::Ambiguous { .. } => {
                let reading = self
                    .graph
                    .statement_readings
                    .get(id.index())
                    .copied()
                    .flatten()
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
    fn id_node(
        &mut self,
        name: ast::Name,
        facts: super::expression_facts::ExprFacts,
        depth: usize,
    ) -> io::Result<()> {
        self.output.write_all(b" ")?;
        if let Some(storage) = facts.anonymous_member {
            let binding = self
                .graph
                .resolved_names
                .get(name.index())
                .copied()
                .flatten()
                .ok_or_else(|| io::Error::other("anonymous member without designation"))?;
            self.name(self.graph.binding(binding).name)?;
            self.work.push((Item::Storage(storage), depth + 1));
        } else if let Some(object) = facts.object
            && matches!(
                object.source,
                super::expression_facts::ObjectSource::Implicit
            )
        {
            let binding = facts
                .member
                .ok_or_else(|| io::Error::other("implicit member without designation"))?;
            self.name(self.graph.binding(binding).name)?;
            self.work.push((Item::This(object.ty), depth + 1));
        } else {
            crate::syntax::write_expression_name(self.tree, self.cursor, name, self.output)?;
        }
        Ok(())
    }
    fn member_node(
        &mut self,
        base: ast::ExprId,
        op: ast::Operator,
        facts: super::expression_facts::ExprFacts,
        depth: usize,
    ) -> io::Result<()> {
        if matches!(
            facts.destructor,
            Some(super::expression_facts::DestructorTarget::Scalar)
        ) {
            write!(self.output, " {}:", op.token.name())?;
            self.expr(base, depth + 1);
            return Ok(());
        }
        let binding = facts
            .member
            .ok_or_else(|| io::Error::other("member without designation"))?;
        if facts.member_naming.is_none() {
            return Err(io::Error::other("member without naming class"));
        }
        write!(self.output, " {}:", op.token.name())?;
        self.name(self.graph.binding(binding).name)?;
        self.expr(base, depth + 1);
        Ok(())
    }
    fn conversion_node(
        &mut self,
        id: ast::ExprId,
        facts: super::expression_facts::ExprFacts,
        depth: usize,
        retyped: Option<(TypeId, V)>,
    ) -> io::Result<bool> {
        let conversion = facts
            .conversion
            .ok_or_else(|| io::Error::other("conversion view without facts"))?;
        if conversion.second == C::DerivedToBase
            || conversion.temporary
                && (conversion.second != C::None
                    || matches!(conversion.first, L::ArrayToPointer | L::FunctionToPointer))
        {
            self.indent(depth)?;
            if conversion.second == C::DerivedToBase && facts.base_path.is_none() {
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
        let mut facts = self
            .graph
            .expression_facts
            .get(id)
            .map_err(|_| io::Error::other("expression without facts"))?;
        if let Some(outer) = self.graph.expression_facts.user_conversion(id) {
            facts.conversion = Some(outer);
        }
        let (mut ty, mut category) = retyped.unwrap_or((facts.ty, facts.category));
        if !bare
            && let Some(conversion) = facts.conversion
            && conversion.initialization
        {
            if self.conversion_node(id, facts, depth, retyped)? {
                return Ok(());
            }
            if conversion.second == C::NullPointer && facts.null_literal {
                ty = conversion.result;
            }
        }
        let node = self.tree.expressions.get(id);
        if let Some(bits) = facts.folded
            && facts.fold_style == super::expression_facts::FoldStyle::Literal
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
            && let Some(target) = facts.reference_cast
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
        if (matches!(node.kind, ExprKind::FunctionalCast { .. }) || facts.functional_call)
            && let Some(target) = facts.reference_cast
        {
            ty = target;
        }
        if facts.call_form == super::expression_facts::CallForm::Ordinary
            && let Some(call) = facts.call
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
        let label = match node.kind {
            ExprKind::Id(_) if facts.anonymous_member.is_some() => "member-expression",
            ExprKind::Id(_)
                if facts.anonymous_member.is_some()
                    || facts.object.is_some_and(|object| {
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
            ExprKind::Sizeof { .. } | ExprKind::TypeTrait { .. } => "sizeof-expression",
            ExprKind::Cast { .. } | ExprKind::FunctionalCast { .. } => "cast-expression",
            ExprKind::Braced { .. } => "braced-init-list",
            ExprKind::Call { .. } => {
                if facts.functional_call || facts.intrinsic.is_some() {
                    "cast-expression"
                } else {
                    "call-expression"
                }
            }
            _ => return Err(io::Error::other("unsupported semantic expression view")),
        };
        self.indent(depth)?;
        write!(self.output, "{label} {} ", category.spelling())?;
        if matches!(node.kind, ExprKind::Id(_))
            && let Some(binding) = facts.function
        {
            self.function_type(binding)?;
        } else if !matches!(node.kind, ExprKind::Unary { .. }) || facts.functions.is_none() {
            // An unresolved unary overload designation has no expression type.
            // Selection rewrites its outermost node and immediate operand only.
            self.ty(ty)?;
        }
        match node.kind {
            ExprKind::Id(name) => self.id_node(name, facts, depth)?,
            ExprKind::Literal(literal) => {
                self.output.write_all(b" ")?;
                self.output
                    .write_all(self.cursor.spellings.get(literal.spelling))?;
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
            ExprKind::Call { arguments, .. }
                if facts.functional_call || facts.intrinsic.is_some() =>
            {
                for &id in self.tree.expr_lists.get(arguments).iter().rev() {
                    self.expr(id, depth + 1);
                }
            }
            ExprKind::Call { callee, arguments } => {
                return self.call_node(callee, arguments, facts, depth);
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
                self.expr(if facts.commuted { base } else { index }, depth + 1);
                self.expr(if facts.commuted { index } else { base }, depth + 1);
            }
            _ => (),
        }
        writeln!(self.output)
    }
    fn call_node(
        &mut self,
        callee: ast::ExprId,
        arguments: crate::syntax::storage::List<ast::ExprId>,
        facts: super::expression_facts::ExprFacts,
        depth: usize,
    ) -> io::Result<()> {
        writeln!(self.output)?;
        if matches!(
            facts.destructor,
            Some(super::expression_facts::DestructorTarget::Scalar)
        ) {
            self.expr(callee, depth + 1);
            return Ok(());
        }
        match facts
            .call
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
        if matches!(facts.call, Some(CallTarget::Indirect(_, _))) {
            self.expr(callee, depth + 1);
        } else if let Some(CallTarget::Direct(binding)) = facts.call {
            self.work
                .push((Item::ObjectReceiver(callee, binding), depth + 1));
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
