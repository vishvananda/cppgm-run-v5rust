// Translated from sema/semantics_dump.cpp. See NOTICE.
//! An iterative view of original syntax and recorded semantic facts.
use super::{
    expression_facts::{LvalueTransform as L, ValueCategory as V, ValueConversion as C},
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
                Item::Line(text) => printer.line(text, depth)?,
            }
        }
        Ok(())
    }
}
impl<W: Write + ?Sized> Printer<'_, '_, W> {
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
        self.graph.write_type(
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
                let b = self.graph.binding(binding);
                self.binding_header(binding, "function-definition", depth)?;
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
                self.work.push((Item::Statement(body.compound), depth + 1));
            }
            DeclKind::Linkage { declarations, .. } => self.declarations(declarations, depth, block),
            DeclKind::Enum { .. } | DeclKind::Class(_) if block => {
                self.line("simple-declaration", depth)?;
            }
            _ => (),
        }
        Ok(())
    }
    fn binding_header(&mut self, id: BindingId, label: &str, depth: usize) -> io::Result<()> {
        let binding = self.graph.binding(id);
        self.indent(depth)?;
        write!(self.output, "{label} ")?;
        if binding.kind == BindingKind::Function {
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
            self.name(binding.name)?;
        } else {
            self.name(binding.name)?;
        }
        self.output.write_all(b" ")?;
        self.ty(binding
            .ty
            .ok_or_else(|| io::Error::other("binding without type"))?)?;
        writeln!(self.output)
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
        if let Some(init) = initializer {
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
            _ => (),
        }
        Ok(())
    }
    fn expression(
        &mut self,
        id: ast::ExprId,
        depth: usize,
        bare: bool,
        retyped: Option<(TypeId, V)>,
    ) -> io::Result<()> {
        let id = self.graph.expression_facts.root(id);
        let facts = self
            .graph
            .expression_facts
            .get(id)
            .map_err(|_| io::Error::other("expression without facts"))?;
        let (mut ty, category) = retyped.unwrap_or((facts.ty, facts.category));
        if !bare
            && let Some(conversion) = facts.conversion
            && conversion.initialization
        {
            if conversion.temporary
                && (conversion.second != C::None
                    || matches!(conversion.first, L::ArrayToPointer | L::FunctionToPointer))
            {
                self.indent(depth)?;
                self.output.write_all(b"cast-expression prvalue ")?;
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
                return Ok(());
            }
            if conversion.second == C::NullPointer && facts.null_literal {
                ty = conversion.result;
            }
        }
        let node = self.tree.expressions.get(id);
        if let Some(bits) = facts.folded {
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
        let label = match node.kind {
            ExprKind::Id(_) => "id-expression",
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
            ExprKind::Sizeof { .. } => "sizeof-expression",
            ExprKind::Cast { .. } | ExprKind::FunctionalCast { .. } => "cast-expression",
            ExprKind::Call { .. } if facts.functional_call => "cast-expression",
            _ => return Err(io::Error::other("unsupported semantic expression view")),
        };
        self.indent(depth)?;
        write!(self.output, "{label} {} ", category.spelling())?;
        self.ty(ty)?;
        match node.kind {
            ExprKind::Id(name) => {
                self.output.write_all(b" ")?;
                super::dump::write_syntax_name(self.tree, self.cursor, name, self.output)?;
            }
            ExprKind::Literal(literal) => {
                self.output.write_all(b" ")?;
                self.output
                    .write_all(self.cursor.spellings.get(literal.spelling))?;
            }
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
            ExprKind::Call { arguments, .. } if facts.functional_call => {
                for &id in self.tree.expr_lists.get(arguments).iter().rev() {
                    self.expr(id, depth + 1);
                }
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
            ExprKind::Subscript { base, index } => {
                self.expr(if facts.commuted { base } else { index }, depth + 1);
                self.expr(if facts.commuted { index } else { base }, depth + 1);
            }
            _ => (),
        }
        writeln!(self.output)
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
