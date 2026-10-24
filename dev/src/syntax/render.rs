// Translated from syntax/ast_render.cpp. See NOTICE.
//! Compact syntax labels are views; never a phase transport or lookup key.
use super::ast::{InitializerKind, QualifierKind, SpecifierKind};
use super::{
    ast::{
        Body, BodyId, ClassId, Condition, DeclId, DeclKind, DeclaratorId, ExceptionDeclaration,
        ExprId, ExprKind, Handler, Initializer, Name, NameComponent, NameKind, Operator,
        ParameterClause, Qualifier, Specifier, StmtId, StmtKind, Suffix, TemplateArgumentKind,
        TemplateParameter, TemplateParameterClause, TemplateParameterKind, Tree, TypeId,
    },
    cursor::Cursor,
    storage::{List, Text},
};
use crate::lex::{identifiers::NameId, pp_lexer::Punctuator as P, vocabulary::Keyword as K};
#[derive(Clone, Copy)]
pub(super) enum Render {
    Text(&'static str),
    LambdaIntroducer(super::ast::LambdaId),
    TemplateClause(TemplateParameterClause),
    TemplateParameter(TemplateParameter),
    Declaration(DeclId),
    Statement(StmtId),
    Condition(Condition),
    Handler(Handler),
    Body(BodyId),
    Class(ClassId),
    MemberPointer(Name),
    Spelling(Text),
    Identifier(NameId),
    Name(Name, bool),
    NameRest(Name),
    Component(NameComponent, bool),
    Decltype(ExprId),
    Expression(ExprId),
    Type(TypeId),
    Specs(List<Specifier>),
    CastType(List<Specifier>),
    Specifier(Specifier),
    Declarator(DeclaratorId),
    Clause(ParameterClause),
    Qualifier(Qualifier),
    Initializer(Initializer),
    Operator(Operator),
    ParenthesizedList(List<ExprId>),
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
    pub(super) fn render(&mut self, task: Render) -> std::io::Result<()> {
        self.bytes.clear();
        self.tasks.push(task);
        while let Some(task) = self.tasks.pop() {
            self.expand(task)?;
        }
        Ok(())
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
    fn lambda_introducer(&mut self, id: super::ast::LambdaId) {
        let lambda = self.tree.lambdas.get(id);
        self.text("[");
        self.tasks.push(Render::Text("]"));
        for (index, capture) in self
            .tree
            .captures
            .get(lambda.captures)
            .iter()
            .enumerate()
            .rev()
        {
            if capture.pack {
                self.tasks.push(Render::Text("..."));
            }
            match capture.kind {
                super::ast::CaptureKind::This => self.tasks.push(Render::Text("this")),
                super::ast::CaptureKind::Copy(name) => {
                    self.tasks.push(Render::Identifier(name));
                }
                super::ast::CaptureKind::Reference(name) => {
                    self.tasks.push(Render::Identifier(name));
                    self.tasks.push(Render::Text("&"));
                }
            }
            if index != 0 || lambda.capture_default.is_some() {
                self.tasks.push(Render::Text(","));
            }
        }
        if let Some(default) = lambda.capture_default {
            self.tasks.push(Render::Text(match default {
                super::ast::CaptureDefault::Copy => "=",
                super::ast::CaptureDefault::Reference => "&",
            }));
        }
    }
    fn expand(&mut self, task: Render) -> std::io::Result<()> {
        match task {
            Render::LambdaIntroducer(id) => self.lambda_introducer(id),
            Render::TemplateClause(parameters) => {
                let parameters = parameters.parameters;
                self.text("template<");
                self.tasks.push(Render::Text(">"));
                for (index, &parameter) in self
                    .tree
                    .template_parameters
                    .get(parameters)
                    .iter()
                    .enumerate()
                    .rev()
                {
                    self.tasks.push(Render::TemplateParameter(parameter));
                    if index != 0 {
                        self.tasks.push(Render::Text(","));
                    }
                }
            }
            Render::TemplateParameter(parameter) => self.template_parameter(parameter),
            Render::Declaration(id) => self.declaration(id),
            Render::Statement(id) => self.statement(id),
            Render::Condition(condition) => self.condition(condition),
            Render::Handler(handler) => self.handler(handler),
            Render::Class(id) => self.class(id),
            Render::MemberPointer(name) => {
                self.tasks.push(Render::Text("::*"));
                self.tasks.push(Render::Name(name, false));
            }
            Render::Body(id) => {
                let Body::Parsed(body) = *self.tree.bodies.get(id) else {
                    return Err(std::io::Error::other("unparsed deferred function body"));
                };
                if let Some(handlers) = body.handlers {
                    for &handler in self.tree.handlers.get(handlers).iter().rev() {
                        self.tasks.push(Render::Handler(handler));
                    }
                }
                self.tasks.push(Render::Statement(body.compound));
                for (index, &member) in self
                    .tree
                    .mem_initializers
                    .get(body.initializers)
                    .iter()
                    .enumerate()
                    .rev()
                {
                    if member.pack {
                        self.tasks.push(Render::Text("..."));
                    }
                    self.tasks.push(Render::Initializer(member.initializer));
                    self.tasks.push(Render::Name(member.name, true));
                    self.tasks
                        .push(Render::Text(if index == 0 { ":" } else { "," }));
                }
                if body.handlers.is_some() {
                    self.tasks.push(Render::Text("try"));
                }
            }
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
                self.components(name.components, 0, name.global);
            }
            Render::NameRest(name) => {
                self.components(self.tree.names.get(name).components, 1, false);
            }
            Render::Component(component, qualified) => {
                if component.template_keyword {
                    self.text("template ");
                }
                if let Some(arguments) = component.arguments {
                    self.tasks.push(Render::Text(">"));
                    for (index, argument) in self
                        .tree
                        .template_arguments
                        .get(arguments)
                        .iter()
                        .enumerate()
                        .rev()
                    {
                        if argument.pack {
                            self.tasks.push(Render::Text("..."));
                        }
                        self.tasks.push(match argument.kind {
                            TemplateArgumentKind::Type(ty) => Render::Type(ty),
                            TemplateArgumentKind::Expression(expr) => Render::Expression(expr),
                        });
                        if index != 0 {
                            self.tasks.push(Render::Text(","));
                        }
                    }
                    self.tasks.push(Render::Text("<"));
                }
                self.component(component.kind, qualified);
            }
            Render::Decltype(operand) => {
                self.text("decltype(");
                self.tasks.push(Render::Text(")"));
                self.tasks.push(Render::Expression(operand));
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
            Render::CastType(specifiers) => {
                for (index, &specifier) in self.tree.specs.get(specifiers).iter().enumerate().rev()
                {
                    self.tasks.push(match specifier.kind {
                        SpecifierKind::Keyword(word) => Render::Text(word.spelling()),
                        SpecifierKind::Named(name) => Render::Name(name, false),
                        SpecifierKind::Decltype(operand) => Render::Decltype(operand),
                        _ => {
                            return Err(std::io::Error::other("invalid functional cast specifier"));
                        }
                    });
                    if index != 0 {
                        self.tasks.push(Render::Text(" "));
                    }
                }
            }
            Render::Specifier(specifier) => self.specifier(specifier),
            Render::Declarator(id) => self.declarator(id),
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
            Render::Qualifier(qualifier) => match qualifier.kind {
                QualifierKind::Cv(word) => {
                    self.text(word.spelling());
                    self.text(" ");
                }
                QualifierKind::Ref(op) => self.text(operator_spelling(op)),
                QualifierKind::Virt(name) => self.bytes.extend_from_slice(self.cursor.name(name)),
                QualifierKind::Trailing(ty) => {
                    self.text("->");
                    self.tasks.push(Render::Type(ty));
                }
                QualifierKind::Noexcept(operand) => {
                    self.text("noexcept");
                    if let Some(operand) = operand {
                        self.text("(");
                        self.tasks.push(Render::Text(")"));
                        self.tasks.push(Render::Expression(operand));
                    }
                }
                QualifierKind::Throw(types) => {
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
            Render::Initializer(initializer) => match initializer.kind {
                InitializerKind::Assign(expr) => {
                    self.text("=");
                    self.tasks.push(Render::Expression(expr));
                }
                InitializerKind::Braced(expr) => self.tasks.push(Render::Expression(expr)),
                InitializerKind::Parenthesized(arguments) => {
                    self.text("(");
                    self.tasks.push(Render::Text(")"));
                    self.list(arguments);
                }
                InitializerKind::Default => self.text("=default"),
                InitializerKind::Delete => self.text("=delete"),
            },
            Render::Expression(id) => self.expression(id)?,
            Render::ParenthesizedList(list) => {
                self.text("(");
                self.tasks.push(Render::Text(")"));
                self.list(list);
            }
        }
        Ok(())
    }
    fn specifier(&mut self, specifier: Specifier) {
        match specifier.kind {
            SpecifierKind::Vector { extended, operand } => {
                self.tasks.push(Render::Text(")))"));
                self.tasks.push(Render::Expression(operand));
                self.tasks.push(Render::Text(if extended {
                    "__attribute__((__ext_vector_type__("
                } else {
                    "__attribute__((__vector_size__("
                }));
            }
            SpecifierKind::Keyword(word) => {
                if matches!(word, K::Const | K::Volatile) {
                    self.tasks.push(Render::Text(" "));
                }
                self.tasks.push(Render::Text(word.spelling()));
            }
            SpecifierKind::Named(name) => self.tasks.push(Render::Name(name, true)),
            SpecifierKind::Class(id) => self.tasks.push(Render::Class(id)),
            SpecifierKind::Elaborated { key, name } => {
                self.tasks.push(Render::Name(name, true));
                self.tasks.push(Render::Text(" "));
                self.tasks.push(Render::Text(key.spelling()));
            }
            SpecifierKind::Enum(decl) => self.enumeration(decl),
            SpecifierKind::ExplicitCondition(_) => self.tasks.push(Render::Text("explicit")),
            SpecifierKind::Decltype(operand) => self.tasks.push(Render::Decltype(operand)),
        }
    }
    fn declarator(&mut self, id: DeclaratorId) {
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
            self.tasks.push(
                pointer
                    .class_name
                    .map_or(Render::Operator(pointer.op), Render::MemberPointer),
            );
        }
    }
    fn components(&mut self, components: List<NameComponent>, start: usize, global: bool) {
        let components = &self.tree.name_lists.get(components)[start..];
        let qualified = global || components.len() > 1;
        for (index, &component) in components.iter().enumerate().rev() {
            self.tasks.push(Render::Component(component, qualified));
            if index != 0 {
                self.tasks.push(Render::Text("::"));
            }
        }
    }
    fn component(&mut self, kind: NameKind, qualified: bool) {
        match kind {
            NameKind::Identifier(name) => self.tasks.push(Render::Identifier(name)),
            NameKind::LiteralOperator(name) => {
                self.text("operator\"\"");
                self.tasks.push(Render::Identifier(name));
            }
            NameKind::Operator { op, array } => {
                self.text("operator");
                let spelling = match op.token.as_punctuator() {
                    Some(P::LeftParen) => "()",
                    Some(P::LeftBracket) => "[]",
                    _ => operator_spelling(op),
                };
                if qualified
                    && spelling
                        .as_bytes()
                        .first()
                        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
                {
                    self.text(" ");
                }
                self.text(spelling);
                if array {
                    self.text("[]");
                }
            }
            NameKind::Conversion(ty) => {
                self.text("operator");
                if qualified && self.type_begins_word(ty) {
                    self.text(" ");
                }
                self.tasks.push(Render::Type(ty));
            }
            NameKind::Destructor(name) => {
                self.text("~");
                self.tasks.push(Render::Identifier(name));
            }
            NameKind::DestructorDecltype(operand) => {
                self.text("~");
                self.tasks.push(Render::Decltype(operand));
            }
            NameKind::Decltype(operand) => self.tasks.push(Render::Decltype(operand)),
        }
    }
    fn type_begins_word(&self, ty: TypeId) -> bool {
        self.tree
            .specs
            .get(self.tree.types.get(ty).specifiers)
            .first()
            .is_some_and(|specifier| match &specifier.kind {
                SpecifierKind::Vector { .. }
                | SpecifierKind::Keyword(_)
                | SpecifierKind::Enum(_)
                | SpecifierKind::ExplicitCondition(_)
                | SpecifierKind::Decltype(_)
                | SpecifierKind::Class(_)
                | SpecifierKind::Elaborated { .. } => true,
                SpecifierKind::Named(name) => {
                    let name = self.tree.names.get(*name);
                    name.typename_keyword
                        || (!name.global
                            && self
                                .tree
                                .name_lists
                                .get(name.components)
                                .first()
                                .is_some_and(|component| {
                                    matches!(
                                        component.kind,
                                        NameKind::Identifier(_) | NameKind::Decltype(_)
                                    )
                                }))
                }
            })
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
    fn builtin_expression(&mut self, spelling: &str, ty: TypeId, operand: Option<ExprId>) {
        self.text(spelling);
        self.text("(");
        self.tasks.push(Render::Text(")"));
        // Frozen builtin views write the type first, then the operand without
        // a separator, including the synthetic offsetof base object.
        if let Some(operand) = operand {
            self.tasks.push(Render::Expression(operand));
        }
        self.tasks.push(Render::Type(ty));
    }
    fn builtin_operand(&mut self, spelling: &'static str, operand: ExprId) {
        self.text(spelling);
        self.text("(");
        self.tasks.push(Render::Text(")"));
        self.tasks.push(Render::Expression(operand));
    }
    fn expression(&mut self, id: ExprId) -> std::io::Result<()> {
        match self.tree.expressions.get(id).kind {
            ExprKind::Lambda(id) => {
                let lambda = self.tree.lambdas.get(id);
                let super::ast::LambdaBody::Parsed(body) = lambda.body else {
                    return Err(std::io::Error::other("unparsed deferred lambda body"));
                };
                self.tasks.push(Render::Statement(body));
                if let Some(parameters) = lambda.parameters {
                    if let Some(trailing) = lambda.trailing {
                        self.tasks.push(Render::Type(trailing));
                        self.tasks.push(Render::Text("->"));
                    }
                    if let Some(exception) = lambda.exception {
                        self.tasks.push(Render::Qualifier(exception));
                    }
                    if lambda.is_mutable {
                        self.tasks.push(Render::Text("mutable"));
                    }
                    self.tasks.push(Render::Clause(parameters));
                }
                self.tasks.push(Render::LambdaIntroducer(id));
            }
            ExprKind::Id(name) => self.tasks.push(Render::Name(name, true)),
            ExprKind::SizeofPack(name) => {
                self.text("sizeof...(");
                self.tasks.push(Render::Text(")"));
                self.tasks.push(Render::Identifier(name));
            }
            ExprKind::PackExpansion(pattern) => {
                self.tasks.push(Render::Text("..."));
                self.tasks.push(Render::Expression(pattern));
            }
            ExprKind::Literal(literal) => self.tasks.push(Render::Spelling(literal.spelling)),
            ExprKind::Keyword(word) => self.text(word.spelling()),
            ExprKind::Statement(body) => {
                self.text("(");
                self.tasks.push(Render::Text(")"));
                self.tasks.push(Render::Statement(body));
            }
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
                    self.tasks.push(Render::Expression(operand));
                }
            }
            ExprKind::IntegerPack(operand) => self.builtin_operand("__integer_pack", operand),
            ExprKind::TypeTransform { transform, types } => {
                self.text(transform.spelling());
                self.text("(");
                self.tasks.push(Render::Text(")"));
                for (index, &ty) in self.tree.type_lists.get(types).iter().enumerate().rev() {
                    self.tasks.push(Render::Type(ty));
                    if index != 0 {
                        self.tasks.push(Render::Text(", "));
                    }
                }
            }
            ExprKind::TypePredicate { predicate, types } => {
                self.text(predicate.spelling());
                self.text("(");
                self.tasks.push(Render::Text(")"));
                for (index, &ty) in self.tree.type_lists.get(types).iter().enumerate().rev() {
                    self.tasks.push(Render::Type(ty));
                    if index != 0 {
                        self.tasks.push(Render::Text(", "));
                    }
                }
            }
            ExprKind::OffsetOf { ty, designator } => {
                self.builtin_expression("__builtin_offsetof", ty, Some(designator));
            }
            ExprKind::OffsetOfBase(ty) => {
                self.builtin_expression("__builtin_offsetof", ty, None);
            }
            ExprKind::VaArg { operand, ty } => {
                self.builtin_expression("__builtin_va_arg", ty, Some(operand));
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
            ExprKind::New {
                global,
                placement,
                parenthesized_type,
                ty,
                initializer,
            } => {
                if global {
                    self.text("::");
                }
                self.text("new");
                if let Some(initializer) = initializer {
                    self.tasks.push(Render::Initializer(initializer));
                }
                if parenthesized_type {
                    self.tasks.push(Render::Text(")"));
                }
                self.tasks.push(Render::Type(ty));
                if parenthesized_type {
                    self.tasks.push(Render::Text("("));
                }
                if let Some(placement) = placement {
                    self.tasks.push(Render::ParenthesizedList(placement));
                }
            }
            ExprKind::Delete {
                global,
                array,
                operand,
            } => {
                if global {
                    self.text("::");
                }
                self.text(if array { "delete[]" } else { "delete" });
                self.tasks.push(Render::Expression(operand));
            }
            ExprKind::Throw(operand) => {
                self.text("throw");
                if let Some(operand) = operand {
                    self.tasks.push(Render::Expression(operand));
                }
            }
        }
        Ok(())
    }
}
pub(crate) fn operator_spelling(op: Operator) -> &'static str {
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

impl Renderer<'_, '_> {
    fn class(&mut self, id: ClassId) {
        let class = *self.tree.classes.get(id);
        self.tasks.push(Render::Text("}"));
        for &member in self.tree.decl_lists.get(class.members).iter().rev() {
            self.tasks.push(Render::Declaration(member));
        }
        self.tasks.push(Render::Text("{"));
        for (index, &base) in self.tree.bases.get(class.bases).iter().enumerate().rev() {
            if base.pack {
                self.tasks.push(Render::Text("..."));
            }
            self.tasks.push(Render::Name(base.name, true));
            if base.is_virtual && !base.virtual_first {
                self.tasks.push(Render::Text("virtual"));
            }
            if let Some(access) = base.access {
                self.tasks.push(Render::Text(access.spelling()));
            }
            if base.virtual_first {
                self.tasks.push(Render::Text("virtual"));
            }
            self.tasks
                .push(Render::Text(if index == 0 { ":" } else { "," }));
        }
        if class.final_specifier {
            self.tasks.push(Render::Text("final"));
        }
        if let Some(name) = class.name {
            self.tasks.push(Render::Name(name, true));
        }
        self.tasks.push(Render::Text(" "));
        self.tasks.push(Render::Text(class.key.spelling()));
    }
    fn template_parameter(&mut self, parameter: TemplateParameter) {
        match parameter.kind {
            TemplateParameterKind::Type {
                key,
                name,
                parameters,
                default,
            } => {
                if let Some(default) = default {
                    self.tasks.push(Render::Type(default));
                    self.tasks.push(Render::Text("="));
                }
                if let Some(name) = name {
                    self.tasks.push(Render::Identifier(name));
                }
                if parameter.pack {
                    self.tasks.push(Render::Text("..."));
                }
                self.tasks.push(Render::Text(" "));
                self.tasks.push(Render::Text(key.spelling()));
                if let Some(parameters) = parameters {
                    self.tasks.push(Render::TemplateClause(parameters));
                }
            }
            TemplateParameterKind::NonType {
                specifiers,
                declarator,
                default,
            } => {
                if let Some(default) = default {
                    self.tasks.push(Render::Expression(default));
                    self.tasks.push(Render::Text("="));
                }
                if let Some(declarator) = declarator {
                    self.tasks.push(Render::Declarator(declarator));
                }
                if parameter.pack {
                    self.tasks.push(Render::Text("..."));
                }
                self.tasks.push(Render::Specs(specifiers));
            }
        }
    }
    fn declaration(&mut self, id: DeclId) {
        match self.tree.declarations.get(id).kind {
            DeclKind::Template {
                parameters,
                declaration,
            } => {
                self.tasks.push(Render::Declaration(declaration));
                self.tasks.push(Render::TemplateClause(parameters));
            }
            DeclKind::ExplicitInstantiation {
                is_extern,
                declaration,
            } => {
                self.tasks.push(Render::Declaration(declaration));
                self.tasks.push(Render::Text("template "));
                if is_extern {
                    self.tasks.push(Render::Text("extern"));
                }
            }
            DeclKind::Empty => self.tasks.push(Render::Text(";")),
            DeclKind::Class(specifier) => {
                self.tasks.push(Render::Text(";"));
                self.tasks.push(Render::Specifier(specifier));
            }
            DeclKind::Enum { .. } => {
                self.tasks.push(Render::Text(";"));
                self.enumeration(id);
            }
            DeclKind::Access(access) => {
                self.tasks.push(Render::Text(":"));
                self.tasks.push(Render::Text(access.spelling()));
            }
            DeclKind::Simple {
                specifiers,
                declarators,
            } => {
                self.tasks.push(Render::Text(";"));
                for (index, &init) in self
                    .tree
                    .init_declarators
                    .get(declarators)
                    .iter()
                    .enumerate()
                    .rev()
                {
                    if let Some(initializer) = init.initializer {
                        self.tasks.push(Render::Initializer(initializer));
                    }
                    self.tasks.push(Render::Declarator(init.declarator));
                    if index != 0 {
                        self.tasks.push(Render::Text(","));
                    }
                }
                self.tasks.push(Render::Specs(specifiers));
            }
            DeclKind::Function {
                specifiers,
                declarator,
                body,
            } => {
                self.tasks.push(Render::Body(body));
                self.tasks.push(Render::Declarator(declarator));
                self.tasks.push(Render::Specs(specifiers));
            }
            DeclKind::SpecialMember {
                specifiers,
                declarator,
                initializer,
                body,
                ..
            } => {
                if let Some(body) = body {
                    self.tasks.push(Render::Body(body));
                } else {
                    self.tasks.push(Render::Text(";"));
                    if let Some(initializer) = initializer {
                        self.tasks.push(Render::Initializer(initializer));
                    }
                }
                self.tasks.push(Render::Declarator(declarator));
                self.tasks.push(Render::Specs(specifiers));
            }
            DeclKind::BitField {
                specifiers,
                declarators,
            } => {
                self.tasks.push(Render::Text(";"));
                for (index, &field) in self
                    .tree
                    .bit_fields
                    .get(declarators)
                    .iter()
                    .enumerate()
                    .rev()
                {
                    self.tasks.push(Render::Expression(field.width));
                    self.tasks.push(Render::Text(":"));
                    if let Some(declarator) = field.declarator {
                        self.tasks.push(Render::Declarator(declarator));
                    }
                    if index != 0 {
                        self.tasks.push(Render::Text(","));
                    }
                }
                self.tasks.push(Render::Specs(specifiers));
            }
            DeclKind::Namespace {
                name,
                inline,
                declarations,
            } => {
                self.tasks.push(Render::Text("}"));
                for &member in self.tree.decl_lists.get(declarations).iter().rev() {
                    self.tasks.push(Render::Declaration(member));
                }
                self.tasks.push(Render::Text("{"));
                if let Some(name) = name {
                    self.tasks.push(Render::Identifier(name));
                }
                self.tasks.push(Render::Text(if inline {
                    "inlinenamespace"
                } else {
                    "namespace"
                }));
            }
            DeclKind::NamespaceAlias { name, target } => {
                self.tasks.push(Render::Text(";"));
                self.tasks.push(Render::Name(target, true));
                self.tasks.push(Render::Text("="));
                self.tasks.push(Render::Identifier(name));
                self.tasks.push(Render::Text("namespace"));
            }
            DeclKind::Using {
                directive,
                typename_keyword,
                target,
            } => {
                self.tasks.push(Render::Text(";"));
                self.tasks.push(Render::Name(target, true));
                self.tasks.push(Render::Text(if directive {
                    "usingnamespace"
                } else if typename_keyword {
                    "usingtypename "
                } else {
                    "using"
                }));
            }
            DeclKind::Alias { name, ty } => {
                self.tasks.push(Render::Text(";"));
                self.tasks.push(Render::Type(ty));
                self.tasks.push(Render::Text("="));
                self.tasks.push(Render::Identifier(name));
                self.tasks.push(Render::Text("using"));
            }
            DeclKind::StaticAssert { condition, message } => {
                self.tasks.push(Render::Text(");"));
                if let Some(message) = message {
                    self.tasks.push(Render::Spelling(message.spelling));
                    self.tasks.push(Render::Text(","));
                }
                self.tasks.push(Render::Expression(condition));
                self.tasks.push(Render::Text("static_assert("));
            }
            DeclKind::Linkage {
                language,
                declarations,
                ..
            } => {
                self.tasks.push(Render::Text("}"));
                for &member in self.tree.decl_lists.get(declarations).iter().rev() {
                    self.tasks.push(Render::Declaration(member));
                }
                self.tasks.push(Render::Text("\"{"));
                self.tasks.push(Render::Spelling(language));
                self.tasks.push(Render::Text("extern\""));
            }
        }
    }
    fn handler(&mut self, handler: Handler) {
        self.tasks.push(Render::Statement(handler.body));
        self.tasks.push(Render::Text(")"));
        match handler.exception {
            ExceptionDeclaration::Ellipsis => self.tasks.push(Render::Text("...")),
            ExceptionDeclaration::Typed {
                specifiers,
                declarator,
            } => {
                if let Some(declarator) = declarator {
                    self.tasks.push(Render::Declarator(declarator));
                }
                self.tasks.push(Render::Specs(specifiers));
            }
        }
        self.tasks.push(Render::Text("catch("));
    }
    fn condition(&mut self, condition: Condition) {
        match condition {
            Condition::Expression(expr) => self.tasks.push(Render::Expression(expr)),
            Condition::Declaration {
                specifiers,
                declarator,
                initializer,
            } => {
                self.tasks.push(Render::Initializer(initializer));
                self.tasks.push(Render::Declarator(declarator));
                self.tasks.push(Render::Specs(specifiers));
            }
        }
    }
    fn statement(&mut self, id: StmtId) {
        match self.tree.statements.get(id).kind {
            StmtKind::Ambiguous { readings, .. } => self.tasks.push(Render::Statement(
                self.tree.statement_readings.get(readings)[0].statement,
            )),
            StmtKind::Compound(items) => {
                self.tasks.push(Render::Text("}"));
                for &item in self.tree.stmt_lists.get(items).iter().rev() {
                    self.tasks.push(Render::Statement(item));
                }
                self.tasks.push(Render::Text("{"));
            }
            StmtKind::Declaration(decl) => self.tasks.push(Render::Declaration(decl)),
            StmtKind::Expression(expr) => {
                self.tasks.push(Render::Text(";"));
                if let Some(expr) = expr {
                    self.tasks.push(Render::Expression(expr));
                }
            }
            StmtKind::Return(expr) | StmtKind::Throw(expr) => {
                self.tasks.push(Render::Text(";"));
                if let Some(expr) = expr {
                    self.tasks.push(Render::Expression(expr));
                }
                self.tasks.push(Render::Text(
                    if matches!(self.tree.statements.get(id).kind, StmtKind::Return(_)) {
                        "return"
                    } else {
                        "throw"
                    },
                ));
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if let Some(branch) = else_branch {
                    self.tasks.push(Render::Statement(branch));
                    self.tasks.push(Render::Text("else"));
                }
                self.tasks.push(Render::Statement(then_branch));
                self.tasks.push(Render::Text(")"));
                self.tasks.push(Render::Condition(condition));
                self.tasks.push(Render::Text("if("));
            }
            StmtKind::ConditionalLoop {
                keyword,
                condition,
                body,
            } => {
                self.tasks.push(Render::Statement(body));
                self.tasks.push(Render::Text(")"));
                self.tasks.push(Render::Condition(condition));
                self.tasks.push(Render::Text("("));
                self.tasks.push(Render::Text(keyword.spelling()));
            }
            StmtKind::Do { body, condition } => {
                self.tasks.push(Render::Text(");"));
                self.tasks.push(Render::Expression(condition));
                self.tasks.push(Render::Text("while("));
                self.tasks.push(Render::Statement(body));
                self.tasks.push(Render::Text("do"));
            }
            StmtKind::For {
                declaration,
                init,
                condition,
                iteration,
                body,
            } => {
                self.tasks.push(Render::Statement(body));
                self.tasks.push(Render::Text(")"));
                if let Some(expr) = iteration {
                    self.tasks.push(Render::Expression(expr));
                }
                self.tasks.push(Render::Text(";"));
                if let Some(condition) = condition {
                    self.tasks.push(Render::Condition(condition));
                }
                if let Some(decl) = declaration {
                    self.tasks.push(Render::Declaration(decl));
                } else {
                    self.tasks.push(Render::Text(";"));
                    if let Some(expr) = init {
                        self.tasks.push(Render::Expression(expr));
                    }
                }
                self.tasks.push(Render::Text("for("));
            }
            StmtKind::RangeFor {
                specifiers,
                declarator,
                range,
                body,
            } => {
                self.tasks.push(Render::Statement(body));
                self.tasks.push(Render::Text(")"));
                self.tasks.push(Render::Expression(range));
                self.tasks.push(Render::Text(":"));
                self.tasks.push(Render::Declarator(declarator));
                self.tasks.push(Render::Specs(specifiers));
                self.tasks.push(Render::Text("for("));
            }
            StmtKind::Asm(_) => self.tasks.push(Render::Text("asm(...);")),
            StmtKind::Break => self.tasks.push(Render::Text("break;")),
            StmtKind::Continue => self.tasks.push(Render::Text("continue;")),
            StmtKind::Goto(name) => {
                self.tasks.push(Render::Text(";"));
                self.tasks.push(Render::Identifier(name));
                self.tasks.push(Render::Text("goto"));
            }
            StmtKind::Label { name, body } => {
                self.tasks.push(Render::Statement(body));
                self.tasks.push(Render::Text(":"));
                self.tasks.push(Render::Identifier(name));
            }
            StmtKind::Case { value, body } => {
                self.tasks.push(Render::Statement(body));
                self.tasks.push(Render::Text(":"));
                self.tasks.push(Render::Expression(value));
                self.tasks.push(Render::Text("case"));
            }
            StmtKind::Default(body) => {
                self.tasks.push(Render::Statement(body));
                self.tasks.push(Render::Text("default:"));
            }
            StmtKind::Try { body, handlers } => {
                for &handler in self.tree.handlers.get(handlers).iter().rev() {
                    self.tasks.push(Render::Handler(handler));
                }
                self.tasks.push(Render::Statement(body));
                self.tasks.push(Render::Text("try"));
            }
        }
    }
}
