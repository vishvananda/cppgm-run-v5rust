//! Source-faithful syntax, owned once in typed flat stores. See NOTICE.
use super::scopes::{Binding, Category, Guesses, ScopeId};
use super::storage::{Id, List, ListMark, Lists, Store, Text};
use crate::lex::{
    PostTokenKind,
    identifiers::NameId,
    vocabulary::{Keyword, SimpleToken},
};
use crate::support::source::SourceOffset;

pub(crate) type DeclId = Id<Declaration>;
pub(crate) type ExprId = Id<Expression>;
pub(crate) type DeclaratorId = Id<Declarator>;
pub(crate) type TypeId = Id<TypeSyntax>;
pub(crate) type Name = Id<QualifiedName>;
pub(crate) type BodyId = Id<Body>;
pub(crate) type ClassId = Id<ClassSyntax>;
pub(crate) type StmtId = Id<Statement>;
pub(crate) type LambdaId = Id<Lambda>;
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TokenSpan<T = u64> {
    pub begin: T,
    pub end: T,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum CaptureKind {
    This,
    Copy(NameId),
    Reference(NameId),
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct LambdaCapture {
    pub kind: CaptureKind,
    pub pack: bool,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum CaptureDefault {
    Copy,
    Reference,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum LambdaBody {
    Deferred,
    Parsed(StmtId),
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Lambda {
    pub capture_default: Option<CaptureDefault>,
    pub captures: List<LambdaCapture>,
    pub parameters: Option<ParameterClause>,
    pub pattern_parameters: Option<ParameterClause>,
    pub parameter_packs: List<(Option<NameId>, usize)>,
    pub is_mutable: bool,
    pub exception: Option<Qualifier>,
    pub trailing: Option<TypeId>,
    pub attributes: List<Attribute>,
    pub clause_token: u32,
    pub body_token: u32,
    pub body: LambdaBody,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AttributeKind {
    Alignas,
    NoInline,
    Packed,
    AsmLabel,
    Weak,
    Section,
    AbiTag,
    Pure,
    Const,
    UsingIfExists,
    NoUniqueAddress,
    VectorSize,
    ExtVectorType,
    ExcludeFromExplicitInstantiation,
    AlwaysInline,
    GnuInline,
    Used,
    NoReturn,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AttributeForm {
    Standard,
    Gnu,
    Alignment,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AttributeSite {
    Entity,
    Type,
    Class,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Attribute {
    pub kind: AttributeKind,
    pub location: SourceOffset,
    pub form: AttributeForm,
    pub site: AttributeSite,
    pub continued: bool,
    pub expression: Option<ExprId>,
    pub ty: Option<TypeId>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct QualifiedName {
    pub location: SourceOffset,
    pub global: bool,
    pub typename_keyword: bool,
    pub components: List<NameComponent>,
    pub(super) prefix_scope: Option<ScopeId>,
    pub prefix_state: PrefixState,
    pub prefix_template: TemplatePrefix,
    pub(super) prefix_guesses: Guesses,
    pub mentions_parameters: bool,
    pub(super) binding: Option<Binding>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrefixState {
    Unqualified,
    Known,
    Unknown,
    Dependent,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TemplatePrefix {
    #[default]
    Ordinary,
    Specialization,
    Dependent,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct NameComponent {
    pub location: SourceOffset,
    pub kind: NameKind,
    pub template_keyword: bool,
    pub expanded: bool,
    pub arguments: Option<List<TemplateArgument>>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum NameKind {
    Identifier(NameId),
    Operator { op: Operator, array: bool },
    LiteralOperator(NameId),
    Conversion(TypeId),
    Destructor(NameId),
    DestructorDecltype(ExprId),
    Decltype(ExprId),
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum TemplateArgumentKind {
    Type(TypeId),
    Expression(ExprId),
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct TemplateArgument {
    pub kind: TemplateArgumentKind,
    pub pack: bool,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct TemplateParameter {
    pub location: SourceOffset,
    pub pack: bool,
    pub kind: TemplateParameterKind,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct TemplateParameterClause {
    #[expect(
        dead_code,
        reason = "Template list location is retained for later specialization diagnostics"
    )]
    pub location: SourceOffset,
    pub parameters: List<TemplateParameter>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum TemplateParameterKind {
    Type {
        key: Keyword,
        name: Option<NameId>,
        parameters: Option<TemplateParameterClause>,
        default: Option<TypeId>,
    },
    NonType {
        specifiers: List<Specifier>,
        declarator: Option<DeclaratorId>,
        default: Option<ExprId>,
    },
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Operator {
    pub token: SimpleToken,
    pub alternative: bool,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Literal {
    pub kind: PostTokenKind,
    pub spelling: Text,
    pub token_form: bool,
    pub data: Text,
    pub suffix_name: Option<NameId>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Specifier {
    pub location: SourceOffset,
    pub kind: SpecifierKind,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum SpecifierKind {
    Keyword(Keyword),
    Named(Name),
    Enum(DeclId),
    Class(ClassId),
    Elaborated { key: Keyword, name: Name },
    Decltype(ExprId),
    ExplicitCondition(ExprId),
    Vector { extended: bool, operand: ExprId },
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct TypeSyntax {
    pub specialization: u8,
    pub location: SourceOffset,
    pub specifiers: List<Specifier>,
    pub declarator: Option<DeclaratorId>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Pointer {
    pub op: Operator,
    pub class_name: Option<Name>,
    pub cv: List<Keyword>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Declarator {
    pub(super) member_scope: Option<ScopeId>,
    pub attributes: List<Attribute>,
    pub location: SourceOffset,
    pub abstract_form: bool,
    pub pointers: List<Pointer>,
    pub pack: bool,
    pub name: Option<Name>,
    pub nested: Option<DeclaratorId>,
    pub suffixes: List<Suffix>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum Suffix {
    Array(Option<ExprId>),
    Function {
        parameters: ParameterClause,
        qualifiers: List<Qualifier>,
    },
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct ParameterClause {
    pub location: SourceOffset,
    pub parameters: List<Parameter>,
    pub ellipsis: bool,
    pub ellipsis_without_comma: bool,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Parameter {
    pub location: SourceOffset,
    pub specifiers: List<Specifier>,
    pub declarator: Option<DeclaratorId>,
    pub default: Option<Initializer>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Qualifier {
    pub location: SourceOffset,
    pub kind: QualifierKind,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum QualifierKind {
    Cv(Keyword),
    Ref(Operator),
    Noexcept(Option<ExprId>),
    Throw(List<TypeId>),
    Trailing(TypeId),
    Virt(NameId),
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Initializer {
    pub location: SourceOffset,
    pub kind: InitializerKind,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum InitializerKind {
    Assign(ExprId),
    Braced(ExprId),
    Parenthesized(List<ExprId>),
    Default,
    Delete,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct InitDeclarator {
    pub tokens: TokenSpan<u32>,
    pub declarator: DeclaratorId,
    pub initializer: Option<Initializer>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Enumerator {
    pub location: SourceOffset,
    pub name: NameId,
    pub value: Option<ExprId>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum Body {
    Deferred,
    Parsed(FunctionBody),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BaseDependency {
    Fixed,
    Dependent,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Base {
    pub location: SourceOffset,
    pub name: Name,
    pub access: Option<Keyword>,
    pub is_virtual: bool,
    pub virtual_first: bool,
    pub pack: bool,
    // Definition-time provenance retained when copying a template instance.
    pub dependency: BaseDependency,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClassSyntax {
    pub packing: u32,
    pub tokens: TokenSpan,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "The presence of an empty base clause is distinct from its owned base list"
        )
    )]
    pub has_base_clause: bool,
    pub attributes: List<Attribute>,
    pub location: SourceOffset,
    pub key: Keyword,
    pub name: Option<Name>,
    pub(super) scope: ScopeId,
    pub final_specifier: bool,
    pub bases: List<Base>,
    pub members: List<DeclId>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct BitField {
    pub declarator: Option<DeclaratorId>,
    pub width: ExprId,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct MemInitializer {
    pub location: SourceOffset,
    pub name: Name,
    pub initializer: Initializer,
    pub pack: bool,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct FunctionBody {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "The ctor-initializer colon location is retained for later constructor diagnostics"
        )
    )]
    pub initializer_location: Option<SourceOffset>,
    pub initializers: List<MemInitializer>,
    pub compound: StmtId,
    pub handlers: Option<List<Handler>>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum ExceptionDeclaration {
    Ellipsis,
    Typed {
        specifiers: List<Specifier>,
        declarator: Option<DeclaratorId>,
    },
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Handler {
    pub exception: ExceptionDeclaration,
    pub body: StmtId,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Declaration {
    pub attributes: List<Attribute>,
    pub location: SourceOffset,
    pub kind: DeclKind,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum DeclKind {
    Template {
        parameters: TemplateParameterClause,
        declaration: DeclId,
    },
    ExplicitInstantiation {
        is_extern: bool,
        declaration: DeclId,
    },
    Empty,
    Simple {
        specifiers: List<Specifier>,
        declarators: List<InitDeclarator>,
    },
    Function {
        specifiers: List<Specifier>,
        declarator: DeclaratorId,
        body: BodyId,
    },
    Class(Specifier),
    SpecialMember {
        specifiers: List<Specifier>,
        name: Name,
        declarator: DeclaratorId,
        initializer: Option<Initializer>,
        body: Option<BodyId>,
    },
    BitField {
        specifiers: List<Specifier>,
        declarators: List<BitField>,
    },
    Access(Keyword),
    Namespace {
        name: Option<NameId>,
        inline: bool,
        declarations: List<DeclId>,
    },
    NamespaceAlias {
        name: NameId,
        target: Name,
    },
    Using {
        directive: bool,
        typename_keyword: bool,
        target: Name,
    },
    Alias {
        name: NameId,
        ty: TypeId,
    },
    Enum {
        scope: Option<ScopeId>,
        name: Option<Name>,
        key: Option<Keyword>,
        underlying: Option<TypeId>,
        enumerators: List<Enumerator>,
        has_body: bool,
        trailing_comma: bool,
    },
    StaticAssert {
        condition: ExprId,
        message: Option<Literal>,
    },
    Linkage {
        language: Text,
        braced: bool,
        declarations: List<DeclId>,
    },
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Expression {
    pub location: SourceOffset,
    pub kind: ExprKind,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum ExprKind {
    Lambda(LambdaId),
    PackExpansion(ExprId),
    SizeofPack(NameId),
    Id(Name),
    Literal(Literal),
    Keyword(Keyword),
    Paren(ExprId),
    Unary {
        op: Operator,
        operand: ExprId,
    },
    Binary {
        op: Operator,
        left: ExprId,
        right: ExprId,
        assignment: bool,
    },
    Conditional {
        condition: ExprId,
        then_expr: ExprId,
        else_expr: ExprId,
    },
    Call {
        callee: ExprId,
        arguments: List<ExprId>,
    },
    Subscript {
        base: ExprId,
        index: ExprId,
    },
    Member {
        base: ExprId,
        op: Operator,
        member: Name,
    },
    Postfix {
        op: Operator,
        operand: ExprId,
    },
    Braced {
        elements: List<ExprId>,
        trailing_comma: bool,
    },
    FunctionalCast {
        specifiers: List<Specifier>,
        arguments: List<ExprId>,
        braced: Option<ExprId>,
    },
    Cast {
        keyword: Option<Keyword>,
        ty: TypeId,
        operand: ExprId,
    },
    Sizeof {
        ty: Option<TypeId>,
        operand: Option<ExprId>,
        parenthesized: bool,
    },
    VaArg {
        operand: ExprId,
        ty: TypeId,
    },
    TypeTrait {
        keyword: Keyword,
        ty: Option<TypeId>,
        operand: Option<ExprId>,
    },
    New {
        global: bool,
        placement: Option<List<ExprId>>,
        parenthesized_type: bool,
        ty: TypeId,
        initializer: Option<Initializer>,
    },
    Delete {
        global: bool,
        array: bool,
        operand: ExprId,
    },
    Throw(Option<ExprId>),
    Statement(StmtId),
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Statement {
    pub location: SourceOffset,
    pub kind: StmtKind,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum Condition {
    Expression(ExprId),
    Declaration {
        specifiers: List<Specifier>,
        declarator: DeclaratorId,
        initializer: Initializer,
    },
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum StmtKind {
    Ambiguous {
        name: Name,
        component: u32,
        readings: List<StatementReading>,
    },
    Compound(List<StmtId>),
    Expression(Option<ExprId>),
    Declaration(DeclId),
    Return(Option<ExprId>),
    Throw(Option<ExprId>),
    If {
        condition: Condition,
        then_branch: StmtId,
        else_branch: Option<StmtId>,
    },
    ConditionalLoop {
        keyword: Keyword,
        condition: Condition,
        body: StmtId,
    },
    Do {
        body: StmtId,
        condition: ExprId,
    },
    For {
        declaration: Option<DeclId>,
        init: Option<ExprId>,
        condition: Option<Condition>,
        iteration: Option<ExprId>,
        body: StmtId,
    },
    RangeFor {
        specifiers: List<Specifier>,
        declarator: DeclaratorId,
        range: ExprId,
        body: StmtId,
    },
    Break,
    Continue,
    Goto(NameId),
    Label {
        name: NameId,
        body: StmtId,
    },
    Case {
        value: ExprId,
        body: StmtId,
    },
    Default(StmtId),
    Try {
        body: StmtId,
        handlers: List<Handler>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadingCategory {
    Value,
    Type,
    Template,
    Unknown,
}
impl ReadingCategory {
    pub(super) fn of(category: Option<Category>) -> Self {
        match category {
            Some(Category::Type) => Self::Type,
            Some(Category::TypeTemplate | Category::FunctionTemplate) => Self::Template,
            Some(Category::Value | Category::Namespace) => Self::Value,
            None => Self::Unknown,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct StatementReading {
    pub statement: StmtId,
    pub category: ReadingCategory,
}

#[derive(Default)]
pub(crate) struct Tree {
    pub lambdas: Store<Lambda>,
    pub captures: Lists<LambdaCapture>,
    pub lambda_parameter_packs: Lists<(Option<NameId>, usize)>,
    pub declarations: Store<Declaration>,
    pub expressions: Store<Expression>,
    pub declarators: Store<Declarator>,
    pub types: Store<TypeSyntax>,
    pub names: Store<QualifiedName>,
    pub classes: Store<ClassSyntax>,
    pub bodies: Store<Body>,
    pub statements: Store<Statement>,
    pub statement_readings: Lists<StatementReading>,
    pub decl_lists: Lists<DeclId>,
    pub expr_lists: Lists<ExprId>,
    pub stmt_lists: Lists<StmtId>,
    pub name_lists: Lists<NameComponent>,
    pub specs: Lists<Specifier>,
    pub pointers: Lists<Pointer>,
    pub cvs: Lists<Keyword>,
    pub suffixes: Lists<Suffix>,
    pub parameters: Lists<Parameter>,
    pub qualifiers: Lists<Qualifier>,
    pub type_lists: Lists<TypeId>,
    pub init_declarators: Lists<InitDeclarator>,
    pub enumerators: Lists<Enumerator>,
    pub bases: Lists<Base>,
    pub bit_fields: Lists<BitField>,
    pub mem_initializers: Lists<MemInitializer>,
    pub handlers: Lists<Handler>,
    pub template_parameters: Lists<TemplateParameter>,
    pub template_arguments: Lists<TemplateArgument>,
    pub attributes: Lists<Attribute>,
}
#[derive(Clone, Copy)]
pub(crate) struct TreeMark {
    lambdas: usize,
    captures: ListMark,
    lambda_parameter_packs: ListMark,
    declarations: usize,
    expressions: usize,
    declarators: usize,
    types: usize,
    names: usize,
    classes: usize,
    bodies: usize,
    statements: usize,
    statement_readings: ListMark,
    decl_lists: ListMark,
    expr_lists: ListMark,
    stmt_lists: ListMark,
    name_lists: ListMark,
    specs: ListMark,
    pointers: ListMark,
    cvs: ListMark,
    suffixes: ListMark,
    parameters: ListMark,
    qualifiers: ListMark,
    type_lists: ListMark,
    init_declarators: ListMark,
    enumerators: ListMark,
    bases: ListMark,
    bit_fields: ListMark,
    mem_initializers: ListMark,
    handlers: ListMark,
    template_parameters: ListMark,
    template_arguments: ListMark,
    attributes: ListMark,
}
impl Tree {
    /// Semantic equivalents use the same flat syntax storage without parsing text.
    pub(crate) fn implicit_name(
        &mut self,
        identifier: NameId,
        location: SourceOffset,
    ) -> Result<Name, super::ParseError> {
        let start = self.name_lists.begin();
        self.name_lists.push(NameComponent {
            location,
            kind: NameKind::Identifier(identifier),
            template_keyword: false,
            expanded: false,
            arguments: None,
        });
        let components = self.name_lists.finish(start)?;
        self.names.add(QualifiedName {
            location,
            global: false,
            typename_keyword: false,
            components,
            prefix_scope: None,
            prefix_state: PrefixState::Unqualified,
            prefix_template: TemplatePrefix::Ordinary,
            prefix_guesses: Guesses::default(),
            mentions_parameters: false,
            binding: None,
        })
    }
    pub(crate) fn implicit_declarator(
        &mut self,
        name: Name,
        location: SourceOffset,
    ) -> Result<DeclaratorId, super::ParseError> {
        self.declarators.add(Declarator {
            member_scope: None,
            attributes: List::default(),
            location,
            abstract_form: false,
            pointers: List::default(),
            pack: false,
            name: Some(name),
            nested: None,
            suffixes: List::default(),
        })
    }

    pub(crate) fn complete_body(
        &mut self,
        id: BodyId,
        body: FunctionBody,
    ) -> Result<(), super::ParseError> {
        if !matches!(self.bodies.get(id), Body::Deferred) {
            return Err(super::ParseError::Invariant("function body already parsed"));
        }
        self.bodies.replace(id, Body::Parsed(body));
        Ok(())
    }

    pub(crate) fn mark(&self) -> TreeMark {
        TreeMark {
            lambdas: self.lambdas.mark(),
            captures: self.captures.mark(),
            lambda_parameter_packs: self.lambda_parameter_packs.mark(),
            declarations: self.declarations.mark(),
            expressions: self.expressions.mark(),
            declarators: self.declarators.mark(),
            types: self.types.mark(),
            names: self.names.mark(),
            classes: self.classes.mark(),
            bodies: self.bodies.mark(),
            statements: self.statements.mark(),
            statement_readings: self.statement_readings.mark(),
            decl_lists: self.decl_lists.mark(),
            expr_lists: self.expr_lists.mark(),
            stmt_lists: self.stmt_lists.mark(),
            name_lists: self.name_lists.mark(),
            specs: self.specs.mark(),
            pointers: self.pointers.mark(),
            cvs: self.cvs.mark(),
            suffixes: self.suffixes.mark(),
            parameters: self.parameters.mark(),
            qualifiers: self.qualifiers.mark(),
            type_lists: self.type_lists.mark(),
            init_declarators: self.init_declarators.mark(),
            enumerators: self.enumerators.mark(),
            bases: self.bases.mark(),
            bit_fields: self.bit_fields.mark(),
            mem_initializers: self.mem_initializers.mark(),
            handlers: self.handlers.mark(),
            template_parameters: self.template_parameters.mark(),
            template_arguments: self.template_arguments.mark(),
            attributes: self.attributes.mark(),
        }
    }
    pub(crate) fn rewind(&mut self, mark: &TreeMark) {
        self.lambdas.rewind(mark.lambdas);
        self.captures.rewind(mark.captures);
        self.lambda_parameter_packs
            .rewind(mark.lambda_parameter_packs);
        self.declarations.rewind(mark.declarations);
        self.expressions.rewind(mark.expressions);
        self.declarators.rewind(mark.declarators);
        self.types.rewind(mark.types);
        self.names.rewind(mark.names);
        self.classes.rewind(mark.classes);
        self.bodies.rewind(mark.bodies);
        self.statements.rewind(mark.statements);
        self.statement_readings.rewind(mark.statement_readings);
        self.decl_lists.rewind(mark.decl_lists);
        self.expr_lists.rewind(mark.expr_lists);
        self.stmt_lists.rewind(mark.stmt_lists);
        self.name_lists.rewind(mark.name_lists);
        self.specs.rewind(mark.specs);
        self.pointers.rewind(mark.pointers);
        self.cvs.rewind(mark.cvs);
        self.suffixes.rewind(mark.suffixes);
        self.parameters.rewind(mark.parameters);
        self.qualifiers.rewind(mark.qualifiers);
        self.type_lists.rewind(mark.type_lists);
        self.init_declarators.rewind(mark.init_declarators);
        self.enumerators.rewind(mark.enumerators);
        self.bases.rewind(mark.bases);
        self.bit_fields.rewind(mark.bit_fields);
        self.mem_initializers.rewind(mark.mem_initializers);
        self.handlers.rewind(mark.handlers);
        self.template_parameters.rewind(mark.template_parameters);
        self.template_arguments.rewind(mark.template_arguments);
        self.attributes.rewind(mark.attributes);
    }
}
