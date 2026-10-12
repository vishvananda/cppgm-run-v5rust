// Translated from syntax/ast_render.cpp. See NOTICE.
//! Compact syntax labels are views; never a phase transport or lookup key.
use super::{
    ast::{
        DeclKind, DeclaratorId, ExprId, ExprKind, Initializer, Name, Operator, ParameterClause,
        Qualifier, Specifier, Suffix, Tree, TypeId,
    },
    cursor::Cursor,
    storage::{List, Text},
};
use crate::lex::{identifiers::NameId, pp_lexer::Punctuator as P, vocabulary::Keyword as K};
#[derive(Clone, Copy)]
pub(super) enum Render {
    Text(&'static str),
    Spelling(Text),
    Identifier(NameId),
    Name(Name, bool),
    Expression(ExprId),
    Type(TypeId),
    Specs(List<Specifier>),
    Specifier(Specifier),
    Declarator(DeclaratorId),
    Clause(ParameterClause),
    Qualifier(Qualifier),
    Initializer(Initializer),
    Operator(Operator),
}
pub(super) struct Renderer<'a, 'source> {
    pub tree: &'a Tree,
    pub cursor: &'a Cursor<'source>,
    pub bytes: Vec<u8>,
    tasks: Vec<Render>,
}
impl<'a, 'source> Renderer<'a, 'source> {
    pub(super) fn new(tree: &'a Tree, cursor: &'a Cursor<'source>) -> Self {
        Self {
            tree,
            cursor,
            bytes: Vec::new(),
            tasks: Vec::new(),
        }
    }
    pub(super) fn render(&mut self, task: Render) {
        self.bytes.clear();
        self.tasks.push(task);
        while let Some(task) = self.tasks.pop() {
            self.expand(task);
        }
    }
    fn text(&mut self, text: &str) {
        self.bytes.extend_from_slice(text.as_bytes());
    }
    fn list(&mut self, list: List<ExprId>) {
        for (index, &expr) in self.tree.expr_lists.get(list).iter().enumerate().rev() {
            self.tasks.push(Render::Expression(expr));
            if index != 0 {
                self.tasks.push(Render::Text(","));
            }
        }
    }
    fn expand(&mut self, task: Render) {
        match task {
            Render::Text(text) => self.text(text),
            Render::Spelling(text) => self
                .bytes
                .extend_from_slice(self.cursor.spellings.get(text)),
            Render::Identifier(name) => self.bytes.extend_from_slice(self.cursor.name(name)),
            Render::Name(name, typename) => {
                let name = self.tree.names.get(name);
                if typename && name.typename_keyword {
                    self.text("typename ");
                }
                if name.global {
                    self.text("::");
                }
                for (index, &component) in
                    self.tree.name_lists.get(name.components).iter().enumerate()
                {
                    if index != 0 {
                        self.text("::");
                    }
                    self.bytes.extend_from_slice(self.cursor.name(component));
                }
            }
            Render::Operator(op) => self.text(operator_spelling(op)),
            Render::Type(ty) => {
                let ty = self.tree.types.get(ty);
                if let Some(declarator) = ty.declarator {
                    self.tasks.push(Render::Declarator(declarator));
                }
                self.tasks.push(Render::Specs(ty.specifiers));
            }
            Render::Specs(specifiers) => {
                for &specifier in self.tree.specs.get(specifiers).iter().rev() {
                    self.tasks.push(Render::Specifier(specifier));
                }
            }
            Render::Specifier(specifier) => self.specifier(specifier),
            Render::Declarator(id) => {
                let node = self.tree.declarators.get(id);
                for &suffix in self.tree.suffixes.get(node.suffixes).iter().rev() {
                    match suffix {
                        Suffix::Array(bound) => {
                            self.tasks.push(Render::Text("]"));
                            if let Some(bound) = bound {
                                self.tasks.push(Render::Expression(bound));
                            }
                            self.tasks.push(Render::Text("["));
                        }
                        Suffix::Function {
                            parameters,
                            qualifiers,
                        } => {
                            for &qualifier in self.tree.qualifiers.get(qualifiers).iter().rev() {
                                self.tasks.push(Render::Qualifier(qualifier));
                            }
                            self.tasks.push(Render::Clause(parameters));
                        }
                    }
                }
                if let Some(nested) = node.nested {
                    self.tasks.push(Render::Text(")"));
                    self.tasks.push(Render::Declarator(nested));
                    self.tasks.push(Render::Text("("));
                }
                if let Some(name) = node.name {
                    self.tasks.push(Render::Name(name, true));
                }
                if node.pack {
                    self.tasks.push(Render::Text("..."));
                }
                for pointer in self.tree.pointers.get(node.pointers).iter().rev() {
                    for &cv in self.tree.cvs.get(pointer.cv).iter().rev() {
                        self.tasks.push(Render::Text(" "));
                        self.tasks.push(Render::Text(cv.spelling()));
                    }
                    self.tasks.push(Render::Operator(pointer.op));
                }
            }
            Render::Clause(clause) => {
                self.tasks.push(Render::Text(")"));
                if clause.ellipsis {
                    self.tasks.push(Render::Text("..."));
                    if !clause.parameters.is_empty() && !clause.ellipsis_without_comma {
                        self.tasks.push(Render::Text(","));
                    }
                }
                for (index, parameter) in self
                    .tree
                    .parameters
                    .get(clause.parameters)
                    .iter()
                    .enumerate()
                    .rev()
                {
                    if let Some(default) = parameter.default {
                        self.tasks.push(Render::Initializer(default));
                    }
                    if let Some(declarator) = parameter.declarator {
                        self.tasks.push(Render::Declarator(declarator));
                    }
                    self.tasks.push(Render::Specs(parameter.specifiers));
                    if index != 0 {
                        self.tasks.push(Render::Text(","));
                    }
                }
                self.tasks.push(Render::Text("("));
            }
            Render::Qualifier(qualifier) => match qualifier {
                Qualifier::Cv(word) => {
                    self.text(word.spelling());
                    self.text(" ");
                }
                Qualifier::Ref(op) => self.text(operator_spelling(op)),
                Qualifier::Virt(name) => self.bytes.extend_from_slice(self.cursor.name(name)),
                Qualifier::Trailing(ty) => {
                    self.text("->");
                    self.tasks.push(Render::Type(ty));
                }
                Qualifier::Noexcept(operand) => {
                    self.text("noexcept");
                    if let Some(operand) = operand {
                        self.text("(");
                        self.tasks.push(Render::Text(")"));
                        self.tasks.push(Render::Expression(operand));
                    }
                }
                Qualifier::Throw(types) => {
                    self.text("throw(");
                    self.tasks.push(Render::Text(")"));
                    for (index, &ty) in self.tree.type_lists.get(types).iter().enumerate().rev() {
                        self.tasks.push(Render::Type(ty));
                        if index != 0 {
                            self.tasks.push(Render::Text(","));
                        }
                    }
                }
            },
            Render::Initializer(initializer) => match initializer {
                Initializer::Assign(expr) => {
                    self.text("=");
                    self.tasks.push(Render::Expression(expr));
                }
                Initializer::Braced(expr) => self.tasks.push(Render::Expression(expr)),
                Initializer::Parenthesized(arguments) => {
                    self.text("(");
                    self.tasks.push(Render::Text(")"));
                    self.list(arguments);
                }
                Initializer::Default => self.text("=default"),
                Initializer::Delete => self.text("=delete"),
            },
            Render::Expression(id) => self.expression(id),
        }
    }
    fn specifier(&mut self, specifier: Specifier) {
        match specifier {
            Specifier::Keyword(word) => {
                if matches!(word, K::Const | K::Volatile) {
                    self.tasks.push(Render::Text(" "));
                }
                self.tasks.push(Render::Text(word.spelling()));
            }
            Specifier::Named(name) => self.tasks.push(Render::Name(name, true)),
            Specifier::Enum(decl) => self.enumeration(decl),
        }
    }
    fn enumeration(&mut self, decl: super::ast::DeclId) {
        if let DeclKind::Enum {
            name,
            key,
            underlying,
            enumerators,
            has_body,
            trailing_comma,
            ..
        } = self.tree.declarations.get(decl).kind
        {
            if has_body {
                self.tasks.push(Render::Text("}"));
                if trailing_comma {
                    self.tasks.push(Render::Text(","));
                }
                for (index, enumerator) in self
                    .tree
                    .enumerators
                    .get(enumerators)
                    .iter()
                    .enumerate()
                    .rev()
                {
                    if let Some(value) = enumerator.value {
                        self.tasks.push(Render::Expression(value));
                        self.tasks.push(Render::Text("="));
                    }
                    self.tasks.push(Render::Identifier(enumerator.name));
                    if index != 0 {
                        self.tasks.push(Render::Text(","));
                    }
                }
                self.tasks.push(Render::Text("{"));
            }
            if let Some(underlying) = underlying {
                self.tasks.push(Render::Type(underlying));
                self.tasks.push(Render::Text(":"));
            }
            if let Some(name) = name {
                self.tasks.push(Render::Name(name, true));
            }
            if let Some(key) = key {
                self.tasks.push(Render::Text(" "));
                self.tasks.push(Render::Text(key.spelling()));
            }
            self.tasks.push(Render::Text("enum "));
        }
    }
    fn expression(&mut self, id: ExprId) {
        match self.tree.expressions.get(id).kind {
            ExprKind::Id(name) => self.tasks.push(Render::Name(name, true)),
            ExprKind::Literal(literal) => self.tasks.push(Render::Spelling(literal.spelling)),
            ExprKind::Keyword(word) => self.text(word.spelling()),
            ExprKind::Paren(inner) => {
                self.text("(");
                self.tasks.push(Render::Text(")"));
                self.tasks.push(Render::Expression(inner));
            }
            ExprKind::Unary { op, operand } => {
                self.text(operator_spelling(op));
                self.tasks.push(Render::Expression(operand));
            }
            ExprKind::Postfix { op, operand } => {
                self.tasks.push(Render::Operator(op));
                self.tasks.push(Render::Expression(operand));
            }
            ExprKind::Binary {
                op, left, right, ..
            } => {
                self.tasks.push(Render::Expression(right));
                self.tasks.push(Render::Operator(op));
                self.tasks.push(Render::Expression(left));
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                self.tasks.push(Render::Expression(else_expr));
                self.tasks.push(Render::Text(":"));
                self.tasks.push(Render::Expression(then_expr));
                self.tasks.push(Render::Text("?"));
                self.tasks.push(Render::Expression(condition));
            }
            ExprKind::Call { callee, arguments } => {
                self.tasks.push(Render::Text(")"));
                self.list(arguments);
                self.tasks.push(Render::Text("("));
                self.tasks.push(Render::Expression(callee));
            }
            ExprKind::Subscript { base, index } => {
                self.tasks.push(Render::Text("]"));
                self.tasks.push(Render::Expression(index));
                self.tasks.push(Render::Text("["));
                self.tasks.push(Render::Expression(base));
            }
            ExprKind::Member { base, op, member } => {
                self.tasks.push(Render::Name(member, true));
                self.tasks.push(Render::Operator(op));
                self.tasks.push(Render::Expression(base));
            }
            ExprKind::Braced {
                elements,
                trailing_comma,
            } => {
                self.text("{");
                self.tasks.push(Render::Text("}"));
                if trailing_comma {
                    self.tasks.push(Render::Text(","));
                }
                self.list(elements);
            }
            ExprKind::FunctionalCast {
                specifiers,
                arguments,
                braced,
            } => {
                if let Some(braced) = braced {
                    self.tasks.push(Render::Expression(braced));
                } else {
                    self.tasks.push(Render::Text(")"));
                    self.list(arguments);
                    self.tasks.push(Render::Text("("));
                }
                self.tasks.push(Render::Specs(specifiers));
            }
            ExprKind::Cast {
                keyword,
                ty,
                operand,
            } => {
                if let Some(keyword) = keyword {
                    self.text(keyword.spelling());
                    self.text("<");
                    self.tasks.push(Render::Text(")"));
                    self.tasks.push(Render::Expression(operand));
                    self.tasks.push(Render::Text(">("));
                    self.tasks.push(Render::Type(ty));
                } else {
                    self.text("(");
                    self.tasks.push(Render::Expression(operand));
                    self.tasks.push(Render::Text(")"));
                    self.tasks.push(Render::Type(ty));
                }
            }
            ExprKind::Sizeof {
                ty,
                operand,
                parenthesized,
            } => {
                self.text("sizeof");
                if parenthesized || ty.is_some() {
                    self.text("(");
                    self.tasks.push(Render::Text(")"));
                }
                if let Some(ty) = ty {
                    self.tasks.push(Render::Type(ty));
                } else if let Some(operand) = operand {
                    if parenthesized
                        && let ExprKind::Paren(inner) = self.tree.expressions.get(operand).kind
                    {
                        self.tasks.push(Render::Expression(inner));
                    } else {
                        self.tasks.push(Render::Expression(operand));
                    }
                }
            }
            ExprKind::TypeTrait {
                keyword,
                ty,
                operand,
            } => {
                self.text(keyword.spelling());
                self.text("(");
                self.tasks.push(Render::Text(")"));
                if let Some(ty) = ty {
                    self.tasks.push(Render::Type(ty));
                } else if let Some(operand) = operand {
                    self.tasks.push(Render::Expression(operand));
                }
            }
        }
    }
}
pub(super) fn operator_spelling(op: Operator) -> &'static str {
    if op.alternative {
        match op.token.as_punctuator() {
            Some(P::LogicalAnd) => "and",
            Some(P::LogicalOr) => "or",
            Some(P::LogicalNot) => "not",
            Some(P::BitAnd) => "bitand",
            Some(P::BitOr) => "bitor",
            Some(P::Xor) => "xor",
            Some(P::Complement) => "compl",
            Some(P::AndAssign) => "and_eq",
            Some(P::OrAssign) => "or_eq",
            Some(P::XorAssign) => "xor_eq",
            Some(P::NotEqual) => "not_eq",
            _ => op.token.spelling(),
        }
    } else {
        op.token.spelling()
    }
}
