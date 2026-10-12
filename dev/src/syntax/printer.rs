// Translated from syntax/ast_printer.cpp. See NOTICE.
//! Iterative tree view, including arbitrarily long left-associated chains.
use super::{
    ast::{
        Base, BitField, Body, BodyId, ClassId, Condition, DeclId, DeclKind, DeclaratorId,
        Enumerator, ExceptionDeclaration, ExprId, ExprKind, FunctionBody, Handler, InitDeclarator,
        Initializer, Literal, MemInitializer, Name, NameKind, Operator, Parameter, ParameterClause,
        Qualifier, Specifier, StmtId, StmtKind, Suffix, TemplateParameter, TemplateParameterClause,
        TemplateParameterKind, Tree, TypeId,
    },
    cursor::Cursor,
    render::{Render, Renderer, operator_spelling},
    storage::{List, Text},
};
use crate::lex::{identifiers::NameId, vocabulary::Keyword as K};
use std::io::{self, Write};
#[derive(Clone, Copy)]
enum Payload {
    None,
    LiteralToken(Literal),
    Text(&'static str),
    Spelling(Text),
    Identifier(NameId),
    Name(Name),
    Keyword(K),
    Operator(Operator),
    NamedSpecifier(Name),
    Render(Render),
    Cast(Option<K>),
    Virt(NameId),
}
#[derive(Clone, Copy)]
enum Task {
    Close,
    TemplateClause(TemplateParameterClause),
    TemplateParameter(TemplateParameter),
    Line(&'static str, Payload, bool),
    Declaration(DeclId),
    Expression(ExprId),
    Type(TypeId),
    Specs(List<Specifier>, bool),
    Declarator(Option<DeclaratorId>, bool),
    Suffix(Suffix),
    Clause(ParameterClause),
    Parameter(Parameter, bool),
    Qualifier(Qualifier),
    Initializer(Initializer),
    Statement(StmtId),
    Condition(Condition),
    ExpressionList(&'static str, List<ExprId>),
    InitDeclarator(InitDeclarator),
    Enumerator(Enumerator),
    Handler(Handler),
    Body(BodyId),
    OwnedSpecifier(Specifier),
    Base(Base),
    BitField(BitField),
    MemInitializer(MemInitializer),
}
pub(super) fn print(
    tree: &Tree,
    cursor: &Cursor<'_>,
    declarations: List<DeclId>,
    output: &mut (impl Write + ?Sized),
) -> io::Result<()> {
    let mut printer = Printer {
        renderer: Renderer::new(tree, cursor),
        tasks: Vec::new(),
        output,
        depth: 0,
    };
    printer.line("translation-unit", Payload::None, true)?;
    printer.tasks.push(Task::Close);
    for &declaration in tree.decl_lists.get(declarations).iter().rev() {
        printer.tasks.push(Task::Declaration(declaration));
    }
    while let Some(task) = printer.tasks.pop() {
        printer.execute(task)?;
    }
    Ok(())
}
struct Printer<'a, 'source, W: Write + ?Sized> {
    renderer: Renderer<'a, 'source>,
    tasks: Vec<Task>,
    output: &'a mut W,
    depth: usize,
}
impl<W: Write + ?Sized> Printer<'_, '_, W> {
    fn line(&mut self, label: &str, payload: Payload, open: bool) -> io::Result<()> {
        // A small fixed initialized buffer handles indentation without an allocation per line.
        const SPACES: &[u8; 128] = &[b' '; 128];
        let mut spaces = self.depth * 2;
        while spaces > 0 {
            let count = spaces.min(SPACES.len());
            self.output.write_all(&SPACES[..count])?;
            spaces -= count;
        }
        self.output.write_all(label.as_bytes())?;
        let task = match payload {
            Payload::None => None,
            Payload::LiteralToken(literal) => {
                self.output.write_all(b" TT_LITERAL:")?;
                self.renderer.render(Render::Spelling(literal.spelling))?;
                self.output.write_all(&self.renderer.bytes)?;
                None
            }
            Payload::Text(text) => Some(Render::Text(text)),
            Payload::Spelling(text) => Some(Render::Spelling(text)),
            Payload::Identifier(name) => Some(Render::Identifier(name)),
            Payload::Name(name) => Some(Render::Name(name, false)),
            Payload::Render(task) => Some(task),
            Payload::Keyword(word) | Payload::Cast(Some(word)) => {
                write!(self.output, " {}:{}", word.name(), word.spelling())?;
                None
            }
            Payload::Operator(op) => {
                write!(
                    self.output,
                    " {}:{}",
                    op.token.name(),
                    operator_spelling(op)
                )?;
                None
            }
            Payload::NamedSpecifier(name) => {
                let node = self.renderer.tree.names.get(name);
                if !node.global
                    && !node.typename_keyword
                    && self.renderer.tree.name_lists.get(node.components).len() == 1
                    && matches!(
                        self.renderer.tree.name_lists.get(node.components)[0].kind,
                        NameKind::Identifier(_)
                    )
                    && self.renderer.tree.name_lists.get(node.components)[0]
                        .arguments
                        .is_none()
                {
                    self.output.write_all(b" TT_IDENTIFIER:")?;
                    self.renderer.render(Render::Name(name, false))?;
                    self.output.write_all(&self.renderer.bytes)?;
                    None
                } else {
                    Some(Render::Name(name, false))
                }
            }
            Payload::Cast(None) => Some(Render::Text("OP_LPAREN:")),
            Payload::Virt(name) => {
                self.output.write_all(b" TT_IDENTIFIER:")?;
                Some(Render::Identifier(name))
            }
        };
        if let Some(task) = task {
            self.renderer.render(task)?;
            if !self.renderer.bytes.is_empty() {
                if !matches!(payload, Payload::Virt(_)) {
                    self.output.write_all(b" ")?;
                }
                self.output.write_all(&self.renderer.bytes)?;
            }
        }
        self.output.write_all(b"\n")?;
        if open {
            self.depth += 1;
        }
        Ok(())
    }
    fn open(&mut self, label: &str, payload: Payload) -> io::Result<()> {
        self.line(label, payload, true)?;
        self.tasks.push(Task::Close);
        Ok(())
    }
    fn execute(&mut self, task: Task) -> io::Result<()> {
        match task {
            Task::TemplateClause(parameters) => {
                let parameters = parameters.parameters;
                self.open("template-parameter-clause", Payload::None)?;
                if !parameters.is_empty() {
                    self.tasks.push(Task::Close);
                    for &parameter in self
                        .renderer
                        .tree
                        .template_parameters
                        .get(parameters)
                        .iter()
                        .rev()
                    {
                        self.tasks.push(Task::TemplateParameter(parameter));
                    }
                    self.tasks
                        .push(Task::Line("template-parameter-list", Payload::None, true));
                }
            }
            Task::TemplateParameter(parameter) => self.template_parameter(parameter)?,
            Task::Body(id) => {
                let Body::Parsed(body) = *self.renderer.tree.bodies.get(id) else {
                    return Err(io::Error::other("unparsed deferred function body"));
                };
                self.function_body(body)?;
            }
            Task::OwnedSpecifier(specifier) => match specifier {
                Specifier::Class(id) => self.class(id)?,
                Specifier::Elaborated { key, name } => {
                    self.open("class-forward-declaration", Payload::Name(name))?;
                    self.tasks
                        .push(Task::Line("class-key", Payload::Keyword(key), false));
                }
                _ => return Err(io::Error::other("invalid owned class specifier")),
            },
            Task::Base(base) => {
                self.open("base-specifier", Payload::None)?;
                if base.pack {
                    self.tasks.push(Task::Line(
                        "pack-expansion",
                        Payload::Operator(
                            super::parser::Parser::make_operator(
                                crate::lex::pp_lexer::Punctuator::Ellipsis,
                            )
                            .map_err(|_| io::Error::other("invalid ellipsis operator"))?,
                        ),
                        false,
                    ));
                }
                self.tasks
                    .push(Task::Line("base-name", Payload::Name(base.name), false));
                if base.is_virtual && !base.virtual_first {
                    self.tasks
                        .push(Task::Line("virtual", Payload::Keyword(K::Virtual), false));
                }
                if let Some(access) = base.access {
                    self.tasks.push(Task::Line(
                        "access-specifier",
                        Payload::Keyword(access),
                        false,
                    ));
                }
                if base.virtual_first {
                    self.tasks
                        .push(Task::Line("virtual", Payload::Keyword(K::Virtual), false));
                }
            }
            Task::BitField(field) => {
                self.open("bit-field-declarator", Payload::None)?;
                self.tasks.push(Task::Expression(field.width));
                if let Some(declarator) = field.declarator {
                    self.tasks.push(Task::Declarator(Some(declarator), false));
                }
            }
            Task::MemInitializer(member) => {
                self.open("mem-initializer", Payload::None)?;
                if member.pack {
                    self.tasks.push(Task::Line(
                        "pack-expansion",
                        Payload::Text("OP_DOTS:..."),
                        false,
                    ));
                }
                match member.initializer {
                    Initializer::Braced(expr) => self.tasks.push(Task::Expression(expr)),
                    Initializer::Parenthesized(args) => self
                        .tasks
                        .push(Task::ExpressionList("paren-argument-list", args)),
                    _ => return Err(io::Error::other("invalid constructor initializer")),
                }
                self.tasks.push(Task::Line(
                    "mem-initializer-id",
                    Payload::Name(member.name),
                    false,
                ));
            }
            Task::Close => self.depth -= 1,
            Task::Line(label, payload, open) => self.line(label, payload, open)?,
            Task::Declaration(id) => self.declaration(id)?,
            Task::Expression(id) => self.expression(id)?,
            Task::Statement(id) => self.statement(id)?,
            Task::Type(id) => {
                self.open("type-id", Payload::None)?;
                let ty = self.renderer.tree.types.get(id);
                if let Some(id) = ty.declarator {
                    self.tasks.push(Task::Declarator(Some(id), false));
                }
                self.tasks.push(Task::Specs(ty.specifiers, true));
            }
            Task::Specs(specifiers, types) => self.specifiers(specifiers, types)?,
            Task::Declarator(id, trailing) => self.declarator(id, trailing)?,
            Task::Suffix(suffix) => self.suffix(suffix)?,
            Task::Clause(clause) => self.clause(clause)?,
            Task::Parameter(parameter, trailing) => self.parameter(parameter, trailing)?,
            Task::Qualifier(qualifier) => self.qualifier(qualifier)?,
            Task::Initializer(initializer) => {
                self.open("initializer", Payload::None)?;
                match initializer {
                    Initializer::Assign(expr) | Initializer::Braced(expr) => {
                        self.tasks.push(Task::Expression(expr));
                    }
                    Initializer::Parenthesized(args) => self
                        .tasks
                        .push(Task::ExpressionList("paren-initializer", args)),
                    Initializer::Default => self.tasks.push(Task::Line(
                        "special-initializer",
                        Payload::Text("default"),
                        false,
                    )),
                    Initializer::Delete => self.tasks.push(Task::Line(
                        "special-initializer",
                        Payload::Text("delete"),
                        false,
                    )),
                }
            }
            Task::ExpressionList(label, list) => {
                self.open(label, Payload::None)?;
                for &expr in self.renderer.tree.expr_lists.get(list).iter().rev() {
                    self.tasks.push(Task::Expression(expr));
                }
            }
            Task::InitDeclarator(init) => {
                self.open("init-declarator", Payload::None)?;
                if let Some(initializer) = init.initializer {
                    self.tasks.push(Task::Initializer(initializer));
                }
                self.tasks
                    .push(Task::Declarator(Some(init.declarator), false));
            }
            Task::Enumerator(enumerator) => {
                self.open("enumerator", Payload::Identifier(enumerator.name))?;
                if let Some(value) = enumerator.value {
                    self.tasks.push(Task::Expression(value));
                }
            }
            Task::Handler(handler) => {
                self.open("handler", Payload::None)?;
                self.tasks.push(Task::Statement(handler.body));
                self.tasks.push(Task::Close);
                match handler.exception {
                    ExceptionDeclaration::Ellipsis => {
                        self.tasks
                            .push(Task::Line("ellipsis", Payload::Text("..."), false));
                    }
                    ExceptionDeclaration::Typed {
                        specifiers,
                        declarator,
                    } => {
                        if let Some(declarator) = declarator {
                            self.tasks.push(Task::Declarator(Some(declarator), false));
                        }
                        self.tasks.push(Task::Specs(specifiers, false));
                    }
                }
                self.tasks
                    .push(Task::Line("exception-declaration", Payload::None, true));
            }
            Task::Condition(condition) => {
                self.open("condition", Payload::None)?;
                match condition {
                    Condition::Expression(expr) => self.tasks.push(Task::Expression(expr)),
                    Condition::Declaration {
                        specifiers,
                        declarator,
                        initializer,
                    } => {
                        self.tasks.push(Task::Close);
                        self.tasks.push(Task::Initializer(initializer));
                        self.tasks.push(Task::Declarator(Some(declarator), false));
                        self.tasks.push(Task::Specs(specifiers, false));
                        self.tasks
                            .push(Task::Line("condition-declaration", Payload::None, true));
                    }
                }
            }
        }
        Ok(())
    }
    fn template_parameter(&mut self, parameter: TemplateParameter) -> io::Result<()> {
        match parameter.kind {
            TemplateParameterKind::Type {
                key,
                name,
                parameters,
                default,
            } => {
                self.open("type-parameter", Payload::None)?;
                if let Some(default) = default {
                    self.tasks.push(Task::Close);
                    self.tasks.push(Task::Type(default));
                    self.tasks
                        .push(Task::Line("default-template-argument", Payload::None, true));
                }
                if let Some(name) = name {
                    self.tasks
                        .push(Task::Line("identifier", Payload::Identifier(name), false));
                }
                if parameter.pack {
                    self.tasks
                        .push(Task::Line("parameter-pack", Payload::Text("..."), false));
                }
                self.tasks
                    .push(Task::Line("parameter-key", Payload::Keyword(key), false));
                if let Some(parameters) = parameters {
                    self.tasks.push(Task::TemplateClause(parameters));
                    self.tasks.push(Task::Line(
                        "template-template-parameter",
                        Payload::None,
                        false,
                    ));
                }
            }
            TemplateParameterKind::NonType {
                specifiers,
                declarator,
                default,
            } => {
                self.open("non-type-template-parameter", Payload::None)?;
                if let Some(default) = default {
                    self.tasks.push(Task::Close);
                    self.tasks.push(Task::Expression(default));
                    self.tasks
                        .push(Task::Line("default-template-argument", Payload::None, true));
                }
                if let Some(declarator) = declarator {
                    self.tasks.push(Task::Declarator(Some(declarator), false));
                }
                if parameter.pack {
                    self.tasks
                        .push(Task::Line("parameter-pack", Payload::Text("..."), false));
                }
                self.tasks.push(Task::Specs(specifiers, false));
            }
        }
        Ok(())
    }
    fn declaration(&mut self, id: DeclId) -> io::Result<()> {
        match self.renderer.tree.declarations.get(id).kind {
            DeclKind::Template {
                parameters,
                declaration,
            } => {
                self.open("template-declaration", Payload::None)?;
                self.tasks.push(Task::Declaration(declaration));
                self.tasks.push(Task::TemplateClause(parameters));
            }
            DeclKind::ExplicitInstantiation {
                is_extern,
                declaration,
            } => {
                self.open(
                    if is_extern {
                        "explicit-instantiation-declaration"
                    } else {
                        "explicit-instantiation-definition"
                    },
                    Payload::None,
                )?;
                self.tasks.push(Task::Declaration(declaration));
            }
            DeclKind::Class(specifier) => self.tasks.push(Task::OwnedSpecifier(specifier)),
            DeclKind::Access(access) => {
                self.line("access-specifier", Payload::Keyword(access), false)?;
            }
            DeclKind::BitField {
                specifiers,
                declarators,
            } => {
                self.open("bit-field-declaration", Payload::None)?;
                for &field in self.renderer.tree.bit_fields.get(declarators).iter().rev() {
                    self.tasks.push(Task::BitField(field));
                }
                self.tasks.push(Task::Specs(specifiers, false));
            }
            DeclKind::SpecialMember {
                specifiers,
                name,
                declarator,
                initializer,
                body,
            } => {
                self.open(
                    if body.is_some() {
                        "special-member-definition"
                    } else {
                        "special-member-declaration"
                    },
                    Payload::Name(name),
                )?;
                if let Some(body) = body {
                    self.tasks.push(Task::Body(body));
                }
                if let Some(initializer) = initializer {
                    self.tasks.push(Task::Initializer(initializer));
                }
                self.tasks.push(Task::Declarator(Some(declarator), false));
                if !specifiers.is_empty() {
                    self.tasks.push(Task::Close);
                    for &specifier in self.renderer.tree.specs.get(specifiers).iter().rev() {
                        let Specifier::Keyword(word) = specifier else {
                            return Err(io::Error::other("invalid member specifier"));
                        };
                        self.tasks.push(Task::Line(
                            "specifier",
                            if word == K::Explicit {
                                Payload::Text("explicit")
                            } else {
                                Payload::Keyword(word)
                            },
                            false,
                        ));
                    }
                    self.tasks
                        .push(Task::Line("member-specifiers", Payload::None, true));
                }
            }
            DeclKind::Empty => self.line("empty-declaration", Payload::None, false)?,
            DeclKind::Simple {
                specifiers,
                declarators,
            } => {
                self.open("simple-declaration", Payload::None)?;
                if !declarators.is_empty() {
                    self.tasks.push(Task::Close);
                    for &init in self
                        .renderer
                        .tree
                        .init_declarators
                        .get(declarators)
                        .iter()
                        .rev()
                    {
                        self.tasks.push(Task::InitDeclarator(init));
                    }
                    self.tasks
                        .push(Task::Line("init-declarator-list", Payload::None, true));
                }
                self.tasks.push(Task::Specs(specifiers, false));
            }
            DeclKind::Function {
                specifiers,
                declarator,
                body,
            } => {
                self.open("function-definition", Payload::None)?;
                self.tasks.push(Task::Body(body));
                self.tasks.push(Task::Declarator(Some(declarator), false));
                self.tasks.push(Task::Specs(specifiers, false));
            }
            DeclKind::Namespace {
                name,
                inline,
                declarations,
            } => {
                self.open(
                    "namespace-definition",
                    name.map_or(Payload::Text("<unnamed>"), Payload::Identifier),
                )?;
                for &decl in self.renderer.tree.decl_lists.get(declarations).iter().rev() {
                    self.tasks.push(Task::Declaration(decl));
                }
                if inline {
                    self.tasks.push(Task::Line("inline", Payload::None, false));
                }
            }
            DeclKind::NamespaceAlias { name, target } => {
                self.open("namespace-alias-definition", Payload::Identifier(name))?;
                self.tasks
                    .push(Task::Line("target", Payload::Name(target), false));
            }
            DeclKind::Using { directive, target } => {
                self.open(
                    if directive {
                        "using-directive"
                    } else {
                        "using-declaration"
                    },
                    Payload::None,
                )?;
                self.tasks
                    .push(Task::Line("target", Payload::Name(target), false));
            }
            DeclKind::Alias { name, ty } => {
                self.open("alias-declaration", Payload::Identifier(name))?;
                self.tasks.push(Task::Type(ty));
            }
            DeclKind::Enum {
                name,
                key,
                underlying,
                enumerators,
                ..
            } => {
                self.open("enum-specifier", name.map_or(Payload::None, Payload::Name))?;
                for &enumerator in self.renderer.tree.enumerators.get(enumerators).iter().rev() {
                    self.tasks.push(Task::Enumerator(enumerator));
                }
                if let Some(ty) = underlying {
                    self.tasks.push(Task::Type(ty));
                }
                if let Some(key) = key {
                    self.tasks
                        .push(Task::Line("enum-key", Payload::Keyword(key), false));
                }
            }
            DeclKind::StaticAssert { condition, message } => {
                self.open("static-assert-declaration", Payload::None)?;
                if let Some(message) = message {
                    self.tasks.push(Task::Line(
                        "message",
                        Payload::Spelling(message.spelling),
                        false,
                    ));
                }
                self.tasks.push(Task::Expression(condition));
            }
            DeclKind::Linkage {
                language,
                declarations,
                ..
            } => {
                self.open("linkage-specification", Payload::Spelling(language))?;
                for &decl in self.renderer.tree.decl_lists.get(declarations).iter().rev() {
                    self.tasks.push(Task::Declaration(decl));
                }
            }
        }
        Ok(())
    }
    fn specifiers(&mut self, specifiers: List<Specifier>, types: bool) -> io::Result<()> {
        self.open(
            if types {
                "type-specifier-seq"
            } else {
                "decl-specifier-seq"
            },
            Payload::None,
        )?;
        for &specifier in self.renderer.tree.specs.get(specifiers).iter().rev() {
            let task = match specifier {
                Specifier::Vector { operand, .. } => {
                    self.tasks.push(Task::Close);
                    self.tasks.push(Task::Expression(operand));
                    Task::Line(
                        "vector-attribute",
                        Payload::Render(Render::Expression(operand)),
                        true,
                    )
                }
                Specifier::Keyword(word) => Task::Line(
                    if !types {
                        "decl-specifier"
                    } else if matches!(word, K::Const | K::Volatile) {
                        "cv-qualifier"
                    } else {
                        "type-specifier"
                    },
                    Payload::Keyword(word),
                    false,
                ),
                Specifier::Named(name) => {
                    let node = self.renderer.tree.names.get(name);
                    let components = self.renderer.tree.name_lists.get(node.components);
                    if !node.global
                        && components.len() >= 2
                        && let NameKind::Decltype(operand) = components[0].kind
                    {
                        self.tasks.push(Task::Close);
                        self.tasks.push(Task::Line(
                            "qualified-type-name",
                            Payload::Render(Render::NameRest(name)),
                            false,
                        ));
                        self.tasks.push(Task::Expression(operand));
                        Task::Line(
                            if types {
                                "decltype-specifier"
                            } else {
                                "decl-specifier"
                            },
                            Payload::Name(name),
                            true,
                        )
                    } else {
                        Task::Line(
                            if types { "type-name" } else { "decl-specifier" },
                            if types {
                                Payload::Name(name)
                            } else {
                                Payload::NamedSpecifier(name)
                            },
                            false,
                        )
                    }
                }
                Specifier::Class(_) | Specifier::Elaborated { .. } => {
                    Task::OwnedSpecifier(specifier)
                }
                Specifier::Enum(decl) => Task::Declaration(decl),
                Specifier::Decltype(operand) => {
                    self.tasks.push(Task::Close);
                    self.tasks.push(Task::Expression(operand));
                    Task::Line(
                        if types {
                            "decltype-specifier"
                        } else {
                            "decl-specifier"
                        },
                        Payload::Render(Render::Decltype(operand)),
                        true,
                    )
                }
            };
            self.tasks.push(task);
        }
        Ok(())
    }
    fn declarator(&mut self, id: Option<DeclaratorId>, trailing: bool) -> io::Result<()> {
        let Some(id) = id else {
            self.open("declarator", Payload::None)?;
            self.tasks
                .push(Task::Line("parameter-pack", Payload::Text("..."), false));
            return Ok(());
        };
        let declarator = *self.renderer.tree.declarators.get(id);
        self.open(
            if declarator.abstract_form {
                "abstract-declarator"
            } else {
                "declarator"
            },
            Payload::None,
        )?;
        if trailing {
            self.tasks
                .push(Task::Line("parameter-pack", Payload::Text("..."), false));
        }
        for &suffix in self
            .renderer
            .tree
            .suffixes
            .get(declarator.suffixes)
            .iter()
            .rev()
        {
            self.tasks.push(Task::Suffix(suffix));
        }
        if let Some(nested) = declarator.nested {
            self.tasks.push(Task::Close);
            self.tasks.push(Task::Declarator(Some(nested), false));
            self.tasks
                .push(Task::Line("nested-declarator", Payload::None, true));
        }
        if let Some(name) = declarator.name {
            self.tasks
                .push(Task::Line("identifier", Payload::Name(name), false));
        }
        if declarator.pack {
            self.tasks
                .push(Task::Line("parameter-pack", Payload::Text("..."), false));
        }
        for pointer in self
            .renderer
            .tree
            .pointers
            .get(declarator.pointers)
            .iter()
            .rev()
        {
            for &cv in self.renderer.tree.cvs.get(pointer.cv).iter().rev() {
                self.tasks
                    .push(Task::Line("cv-qualifier", Payload::Keyword(cv), false));
            }
            self.tasks.push(Task::Line(
                "ptr-operator",
                pointer
                    .class_name
                    .map_or(Payload::Operator(pointer.op), |name| {
                        Payload::Render(Render::MemberPointer(name))
                    }),
                false,
            ));
        }
        Ok(())
    }
    fn class(&mut self, id: ClassId) -> io::Result<()> {
        let class = *self.renderer.tree.classes.get(id);
        self.open(
            "class-specifier",
            class.name.map_or(Payload::None, Payload::Name),
        )?;
        for &member in self
            .renderer
            .tree
            .decl_lists
            .get(class.members)
            .iter()
            .rev()
        {
            self.tasks.push(Task::Declaration(member));
        }
        if !class.bases.is_empty() {
            self.tasks.push(Task::Close);
            for &base in self.renderer.tree.bases.get(class.bases).iter().rev() {
                self.tasks.push(Task::Base(base));
            }
            self.tasks
                .push(Task::Line("base-clause", Payload::None, true));
        }
        if class.final_specifier {
            self.tasks.push(Task::Line(
                "class-virt-specifier",
                Payload::Text("TT_IDENTIFIER:final"),
                false,
            ));
        }
        self.tasks
            .push(Task::Line("class-key", Payload::Keyword(class.key), false));
        Ok(())
    }
    fn function_body(&mut self, body: FunctionBody) -> io::Result<()> {
        if body.handlers.is_some() {
            self.open("function-try-block", Payload::None)?;
        }
        if let Some(handlers) = body.handlers {
            for &handler in self.renderer.tree.handlers.get(handlers).iter().rev() {
                self.tasks.push(Task::Handler(handler));
            }
        }
        self.tasks.push(Task::Statement(body.compound));
        if !body.initializers.is_empty() {
            self.tasks.push(Task::Close);
            for &member in self
                .renderer
                .tree
                .mem_initializers
                .get(body.initializers)
                .iter()
                .rev()
            {
                self.tasks.push(Task::MemInitializer(member));
            }
            self.tasks
                .push(Task::Line("ctor-initializer", Payload::None, true));
        }
        Ok(())
    }
    fn suffix(&mut self, suffix: Suffix) -> io::Result<()> {
        match suffix {
            Suffix::Array(bound) => {
                self.open("array-suffix", Payload::None)?;
                if let Some(bound) = bound {
                    self.tasks.push(Task::Expression(bound));
                }
            }
            Suffix::Function {
                parameters,
                qualifiers,
            } => {
                for &qualifier in self.renderer.tree.qualifiers.get(qualifiers).iter().rev() {
                    self.tasks.push(Task::Qualifier(qualifier));
                }
                self.tasks.push(Task::Clause(parameters));
            }
        }
        Ok(())
    }
    fn clause(&mut self, clause: ParameterClause) -> io::Result<()> {
        self.open("parameter-clause", Payload::None)?;
        let parameters = self.renderer.tree.parameters.get(clause.parameters);
        let trailing = clause.ellipsis
            && clause.ellipsis_without_comma
            && parameters.last().is_some_and(|parameter| {
                parameter.default.is_none() && !self.has_declarator_id(parameter.declarator)
            });
        if clause.ellipsis && !trailing {
            self.tasks
                .push(Task::Line("parameter-pack", Payload::Text("..."), false));
        }
        for (index, &parameter) in parameters.iter().enumerate().rev() {
            self.tasks.push(Task::Parameter(
                parameter,
                trailing && index + 1 == parameters.len(),
            ));
        }
        Ok(())
    }
    fn has_declarator_id(&self, mut id: Option<DeclaratorId>) -> bool {
        while let Some(current) = id {
            let declarator = self.renderer.tree.declarators.get(current);
            if declarator.name.is_some() {
                return true;
            }
            id = declarator.nested;
        }
        false
    }
    fn parameter(&mut self, parameter: Parameter, trailing: bool) -> io::Result<()> {
        self.open("parameter-declaration", Payload::None)?;
        if let Some(default) = parameter.default {
            self.tasks.push(Task::Close);
            self.tasks.push(Task::Initializer(default));
            self.tasks
                .push(Task::Line("default-argument", Payload::None, true));
        }
        if parameter.declarator.is_some() || trailing {
            self.tasks
                .push(Task::Declarator(parameter.declarator, trailing));
        }
        self.tasks.push(Task::Specs(parameter.specifiers, false));
        Ok(())
    }
    fn qualifier(&mut self, qualifier: Qualifier) -> io::Result<()> {
        match qualifier {
            Qualifier::Cv(word) => self.line("cv-qualifier", Payload::Keyword(word), false)?,
            Qualifier::Ref(op) => self.line("ref-qualifier", Payload::Operator(op), false)?,
            Qualifier::Virt(name) => self.line("virt-specifier", Payload::Virt(name), false)?,
            Qualifier::Trailing(ty) => {
                self.open("trailing-return-type", Payload::Render(Render::Type(ty)))?;
                self.tasks.push(Task::Type(ty));
            }
            Qualifier::Noexcept(operand) => {
                self.open(
                    "function-qualifier",
                    Payload::Render(Render::Qualifier(qualifier)),
                )?;
                if let Some(operand) = operand {
                    self.tasks.push(Task::Expression(operand));
                }
            }
            Qualifier::Throw(_) => self.open(
                "function-qualifier",
                Payload::Render(Render::Qualifier(qualifier)),
            )?,
        }
        Ok(())
    }
    fn lambda(&mut self, id: super::ast::LambdaId) -> io::Result<()> {
        let lambda = *self.renderer.tree.lambdas.get(id);
        let super::ast::LambdaBody::Parsed(body) = lambda.body else {
            return Err(io::Error::other("unparsed deferred lambda body"));
        };
        self.open("lambda-expression", Payload::None)?;
        self.tasks.push(Task::Statement(body));
        if let Some(parameters) = lambda.parameters {
            self.tasks.push(Task::Close);
            if let Some(trailing) = lambda.trailing {
                self.tasks.push(Task::Close);
                self.tasks.push(Task::Type(trailing));
                self.tasks
                    .push(Task::Line("trailing-return-type", Payload::None, true));
            }
            if let Some(exception) = lambda.exception {
                match exception {
                    Qualifier::Noexcept(operand) => {
                        self.tasks.push(Task::Close);
                        if let Some(operand) = operand {
                            self.tasks.push(Task::Expression(operand));
                        }
                        self.tasks
                            .push(Task::Line("noexcept-specification", Payload::None, true));
                    }
                    Qualifier::Throw(_) => self.tasks.push(Task::Qualifier(exception)),
                    _ => {
                        return Err(io::Error::other("invalid lambda exception specification"));
                    }
                }
            }
            if lambda.is_mutable {
                self.tasks.push(Task::Line(
                    "lambda-specifier",
                    Payload::Keyword(K::Mutable),
                    false,
                ));
            }
            self.tasks.push(Task::Clause(parameters));
            self.tasks
                .push(Task::Line("lambda-declarator", Payload::None, true));
        }
        self.tasks.push(Task::Line(
            "lambda-introducer",
            Payload::Render(Render::LambdaIntroducer(id)),
            false,
        ));
        Ok(())
    }
    fn expression(&mut self, id: ExprId) -> io::Result<()> {
        match self.renderer.tree.expressions.get(id).kind {
            ExprKind::Lambda(id) => self.lambda(id)?,
            ExprKind::SizeofPack(name) => {
                self.line("sizeof-pack-expression", Payload::Identifier(name), false)?;
            }
            ExprKind::PackExpansion(pattern) => {
                self.open("pack-expansion-expression", Payload::None)?;
                self.tasks.push(Task::Expression(pattern));
            }
            ExprKind::Id(name) => self.line("id-expression", Payload::Name(name), false)?,
            ExprKind::Literal(literal) => {
                self.line(
                    "literal",
                    if literal.token_form {
                        Payload::LiteralToken(literal)
                    } else {
                        Payload::Spelling(literal.spelling)
                    },
                    false,
                )?;
            }
            ExprKind::Keyword(word) => {
                self.line("keyword-literal", Payload::Keyword(word), false)?;
            }
            ExprKind::Paren(inner) => {
                self.open("parenthesized-expression", Payload::None)?;
                self.tasks.push(Task::Expression(inner));
            }
            ExprKind::Unary { op, operand } | ExprKind::Postfix { op, operand } => {
                self.open(
                    if matches!(
                        self.renderer.tree.expressions.get(id).kind,
                        ExprKind::Unary { .. }
                    ) {
                        "unary-expression"
                    } else {
                        "postfix-expression"
                    },
                    Payload::Operator(op),
                )?;
                self.tasks.push(Task::Expression(operand));
            }
            ExprKind::Binary {
                op,
                left,
                right,
                assignment,
            } => {
                self.open(
                    if assignment {
                        "assignment-expression"
                    } else {
                        "binary-expression"
                    },
                    Payload::Operator(op),
                )?;
                self.tasks.push(Task::Expression(right));
                self.tasks.push(Task::Expression(left));
            }
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                self.open("conditional-expression", Payload::None)?;
                self.tasks.push(Task::Expression(else_expr));
                self.tasks.push(Task::Expression(then_expr));
                self.tasks.push(Task::Expression(condition));
            }
            ExprKind::Call { callee, arguments } => {
                self.open("call-expression", Payload::None)?;
                self.tasks
                    .push(Task::ExpressionList("argument-list", arguments));
                self.tasks.push(Task::Expression(callee));
            }
            ExprKind::Subscript { base, index } => {
                self.open("subscript-expression", Payload::None)?;
                self.tasks.push(Task::Expression(index));
                self.tasks.push(Task::Expression(base));
            }
            ExprKind::Member { base, op, member } => {
                self.open("member-expression", Payload::Operator(op))?;
                self.tasks
                    .push(Task::Line("identifier", Payload::Name(member), false));
                self.tasks.push(Task::Expression(base));
            }
            ExprKind::Braced { elements, .. } => self
                .tasks
                .push(Task::ExpressionList("braced-init-list", elements)),
            ExprKind::FunctionalCast {
                specifiers,
                arguments,
                braced,
            } => {
                self.open("call-expression", Payload::None)?;
                let keywords = matches!(
                    self.renderer.tree.specs.get(specifiers),
                    [Specifier::Keyword(_)]
                );
                if let Some(braced) = braced {
                    self.tasks.push(Task::Expression(braced));
                } else {
                    self.tasks.push(Task::ExpressionList(
                        if keywords {
                            "paren-argument-list"
                        } else {
                            "argument-list"
                        },
                        arguments,
                    ));
                }
                self.tasks.push(Task::Line(
                    "id-expression",
                    Payload::Render(Render::CastType(specifiers)),
                    false,
                ));
            }
            ExprKind::Cast {
                keyword,
                ty,
                operand,
            } => {
                self.open("cast-expression", Payload::Cast(keyword))?;
                self.tasks.push(Task::Expression(operand));
                self.tasks.push(Task::Type(ty));
            }
            ExprKind::Sizeof { ty, operand, .. } => {
                self.open("sizeof-expression", Payload::None)?;
                if let Some(ty) = ty {
                    self.tasks.push(Task::Type(ty));
                } else if let Some(operand) = operand {
                    self.tasks.push(Task::Expression(operand));
                }
            }
            ExprKind::TypeTrait {
                keyword,
                ty,
                operand,
            } => {
                self.open("type-trait-expression", Payload::Keyword(keyword))?;
                if let Some(ty) = ty {
                    self.tasks.push(Task::Type(ty));
                } else if let Some(operand) = operand {
                    self.tasks.push(Task::Expression(operand));
                }
            }
            ExprKind::New {
                global,
                placement,
                ty,
                initializer,
                ..
            } => {
                self.open("new-expression", Payload::None)?;
                if let Some(initializer) = initializer {
                    self.tasks.push(Task::Initializer(initializer));
                }
                self.tasks.push(Task::Type(ty));
                if let Some(placement) = placement {
                    self.tasks.push(Task::Close);
                    self.tasks
                        .push(Task::ExpressionList("paren-argument-list", placement));
                    self.tasks.push(Task::Line(
                        "placement",
                        Payload::Render(Render::ParenthesizedList(placement)),
                        true,
                    ));
                }
                if global {
                    self.tasks
                        .push(Task::Line("global-scope", Payload::None, false));
                }
            }
            ExprKind::Delete {
                global,
                array,
                operand,
            } => {
                self.open("delete-expression", Payload::None)?;
                self.tasks.push(Task::Expression(operand));
                if array {
                    self.tasks
                        .push(Task::Line("array-delete", Payload::None, false));
                }
                if global {
                    self.tasks
                        .push(Task::Line("global-scope", Payload::None, false));
                }
            }
            ExprKind::Throw(operand) => {
                self.open("throw-expression", Payload::None)?;
                if let Some(operand) = operand {
                    self.tasks.push(Task::Expression(operand));
                }
            }
        }
        Ok(())
    }
    fn statement(&mut self, id: StmtId) -> io::Result<()> {
        match self.renderer.tree.statements.get(id).kind {
            StmtKind::Compound(items) => {
                self.open("compound-statement", Payload::None)?;
                for &item in self.renderer.tree.stmt_lists.get(items).iter().rev() {
                    self.tasks.push(Task::Statement(item));
                }
            }
            StmtKind::Declaration(decl) => self.tasks.push(Task::Declaration(decl)),
            StmtKind::Expression(expr) | StmtKind::Return(expr) | StmtKind::Throw(expr) => {
                let label = match self.renderer.tree.statements.get(id).kind {
                    StmtKind::Return(_) => "return-statement",
                    StmtKind::Throw(_) => "throw-statement",
                    _ => "expression-statement",
                };
                self.open(label, Payload::None)?;
                if let Some(expr) = expr {
                    self.tasks.push(Task::Expression(expr));
                }
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.open("if-statement", Payload::None)?;
                if let Some(else_branch) = else_branch {
                    self.tasks.push(Task::Close);
                    self.tasks.push(Task::Statement(else_branch));
                    self.tasks.push(Task::Line("else", Payload::None, true));
                }
                self.tasks.push(Task::Close);
                self.tasks.push(Task::Statement(then_branch));
                self.tasks.push(Task::Line("then", Payload::None, true));
                self.tasks.push(Task::Condition(condition));
            }
            StmtKind::ConditionalLoop {
                keyword,
                condition,
                body,
            } => {
                self.open(
                    if keyword == K::While {
                        "while-statement"
                    } else {
                        "switch-statement"
                    },
                    Payload::None,
                )?;
                self.tasks.push(Task::Statement(body));
                self.tasks.push(Task::Condition(condition));
            }
            StmtKind::Do { body, condition } => {
                self.open("do-statement", Payload::None)?;
                self.tasks.push(Task::Close);
                self.tasks.push(Task::Expression(condition));
                self.tasks
                    .push(Task::Line("condition", Payload::None, true));
                self.tasks.push(Task::Statement(body));
            }
            StmtKind::For {
                declaration,
                init,
                condition,
                iteration,
                body,
            } => {
                self.open("for-statement", Payload::None)?;
                self.tasks.push(Task::Statement(body));
                if let Some(iteration) = iteration {
                    self.tasks.push(Task::Close);
                    self.tasks.push(Task::Expression(iteration));
                    self.tasks
                        .push(Task::Line("iteration", Payload::None, true));
                }
                if let Some(condition) = condition {
                    self.tasks.push(Task::Condition(condition));
                }
                self.tasks.push(Task::Close);
                if let Some(decl) = declaration {
                    self.tasks.push(Task::Declaration(decl));
                } else if let Some(init) = init {
                    self.tasks.push(Task::Expression(init));
                }
                self.tasks
                    .push(Task::Line("for-init-statement", Payload::None, true));
            }
            StmtKind::RangeFor {
                specifiers,
                declarator,
                range,
                body,
            } => {
                self.open("range-for-statement", Payload::None)?;
                self.tasks.push(Task::Statement(body));
                self.tasks.push(Task::Close);
                self.tasks.push(Task::Expression(range));
                self.tasks
                    .push(Task::Line("range-initializer", Payload::None, true));
                self.tasks.push(Task::Close);
                self.tasks.push(Task::Declarator(Some(declarator), false));
                self.tasks.push(Task::Specs(specifiers, false));
                self.tasks
                    .push(Task::Line("range-declaration", Payload::None, true));
            }
            StmtKind::Break => self.line("break-statement", Payload::None, false)?,
            StmtKind::Continue => self.line("continue-statement", Payload::None, false)?,
            StmtKind::Goto(name) => {
                self.line("goto-statement", Payload::Identifier(name), false)?;
            }
            StmtKind::Label { name, body } => {
                self.open("labeled-statement", Payload::Identifier(name))?;
                self.tasks.push(Task::Statement(body));
            }
            StmtKind::Case { value, body } => {
                self.open("case-statement", Payload::None)?;
                self.tasks.push(Task::Statement(body));
                self.tasks.push(Task::Expression(value));
            }
            StmtKind::Default(body) => {
                self.open("default-statement", Payload::None)?;
                self.tasks.push(Task::Statement(body));
            }
            StmtKind::Try { body, handlers } => {
                self.open("try-block", Payload::None)?;
                for &handler in self.renderer.tree.handlers.get(handlers).iter().rev() {
                    self.tasks.push(Task::Handler(handler));
                }
                self.tasks.push(Task::Statement(body));
            }
        }
        Ok(())
    }
}
