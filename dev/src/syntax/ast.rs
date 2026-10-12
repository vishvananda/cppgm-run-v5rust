//! Source-faithful syntax, owned once in typed flat stores. See NOTICE.
use super::scopes::{Binding, ScopeId};
use super::storage::{Id, List, ListMark, Lists, Store, Text};
use crate::lex::{
    PostTokenKind,
    identifiers::NameId,
    vocabulary::{Keyword, SimpleToken},
};
use crate::support::source::SourceOffset;

pub(super) type DeclId = Id<Declaration>;
pub(super) type ExprId = Id<Expression>;
pub(super) type DeclaratorId = Id<Declarator>;
pub(super) type TypeId = Id<TypeSyntax>;
pub(super) type Name = Id<QualifiedName>;
pub(super) type BodyId = Id<Body>;
pub(super) type ClassId = Id<ClassSyntax>;
pub(super) type StmtId = Id<Statement>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AttributeKind {
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
pub(super) enum AttributeForm {
    Standard,
    Gnu,
    Alignment,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AttributeSite {
    Entity,
    Type,
    Class,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Attribute {
    pub kind: AttributeKind,
    pub location: SourceOffset,
    pub form: AttributeForm,
    pub site: AttributeSite,
    pub continued: bool,
    pub expression: Option<ExprId>,
    pub ty: Option<TypeId>,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct QualifiedName {
    pub global: bool,
    pub typename_keyword: bool,
    pub components: List<NameComponent>,
    pub prefix_scope: Option<ScopeId>,
    pub prefix_state: PrefixState,
    pub mentions_parameters: bool,
    pub binding: Option<Binding>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PrefixState {
    Unqualified,
    Known,
    Unknown,
    Dependent,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct NameComponent {
    #[expect(
        dead_code,
        reason = "Name component locations are retained for later qualified lookup diagnostics"
    )]
    pub location: SourceOffset,
    pub kind: NameKind,
    pub template_keyword: bool,
    pub arguments: Option<List<TemplateArgument>>,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum NameKind {
    Identifier(NameId),
    Operator { op: Operator, array: bool },
    LiteralOperator(NameId),
    Conversion(TypeId),
    Destructor(NameId),
    DestructorDecltype(ExprId),
    Decltype(ExprId),
}
#[derive(Clone, Copy, Debug)]
pub(super) enum TemplateArgumentKind {
    Type(TypeId),
    Expression(ExprId),
}
#[derive(Clone, Copy, Debug)]
pub(super) struct TemplateArgument {
    pub kind: TemplateArgumentKind,
    pub pack: bool,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct TemplateParameter {
    #[expect(
        dead_code,
        reason = "Template parameter location is retained for later semantic diagnostics"
    )]
    pub location: SourceOffset,
    pub pack: bool,
    pub kind: TemplateParameterKind,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct TemplateParameterClause {
    #[expect(
        dead_code,
        reason = "Template head location is retained for later semantic diagnostics"
    )]
    pub location: SourceOffset,
    pub parameters: List<TemplateParameter>,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum TemplateParameterKind {
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
pub(super) struct Operator {
    pub token: SimpleToken,
    pub alternative: bool,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Literal {
    pub kind: PostTokenKind,
    pub spelling: Text,
    pub token_form: bool,
    #[expect(
        dead_code,
        reason = "Phase-7 decoded bytes retained for semantic literal analysis"
    )]
    pub data: Text,
    #[expect(
        dead_code,
        reason = "Literal suffix identity is consumed by the later semantic literal owner"
    )]
    pub suffix: Text,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Specifier {
    Keyword(Keyword),
    Named(Name),
    Enum(DeclId),
    Class(ClassId),
    Elaborated { key: Keyword, name: Name },
    Decltype(ExprId),
    Vector { extended: bool, operand: ExprId },
}
#[derive(Clone, Copy, Debug)]
pub(super) struct TypeSyntax {
    #[expect(
        dead_code,
        reason = "Source location retained for later semantic diagnostics on this typed syntax node"
    )]
    pub location: SourceOffset,
    pub specifiers: List<Specifier>,
    pub declarator: Option<DeclaratorId>,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Pointer {
    pub op: Operator,
    pub class_name: Option<Name>,
    pub cv: List<Keyword>,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Declarator {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Retained attributes are consumed by later semantic analysis"
        )
    )]
    pub attributes: List<Attribute>,
    #[expect(
        dead_code,
        reason = "Source location retained for later semantic diagnostics on this typed syntax node"
    )]
    pub location: SourceOffset,
    pub abstract_form: bool,
    pub pointers: List<Pointer>,
    pub pack: bool,
    pub name: Option<Name>,
    pub nested: Option<DeclaratorId>,
    pub suffixes: List<Suffix>,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Suffix {
    Array(Option<ExprId>),
    Function {
        parameters: ParameterClause,
        qualifiers: List<Qualifier>,
    },
}
#[derive(Clone, Copy, Debug)]
pub(super) struct ParameterClause {
    pub parameters: List<Parameter>,
    pub ellipsis: bool,
    pub ellipsis_without_comma: bool,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Parameter {
    pub specifiers: List<Specifier>,
    pub declarator: Option<DeclaratorId>,
    pub default: Option<Initializer>,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Qualifier {
    Cv(Keyword),
    Ref(Operator),
    Noexcept(Option<ExprId>),
    Throw(List<TypeId>),
    Trailing(TypeId),
    Virt(NameId),
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Initializer {
    Assign(ExprId),
    Braced(ExprId),
    Parenthesized(List<ExprId>),
    Default,
    Delete,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct InitDeclarator {
    pub declarator: DeclaratorId,
    pub initializer: Option<Initializer>,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Enumerator {
    pub name: NameId,
    pub value: Option<ExprId>,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Body {
    Deferred,
    Parsed(FunctionBody),
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Base {
    #[expect(
        dead_code,
        reason = "Base source location is retained for later access and inheritance diagnostics"
    )]
    pub location: SourceOffset,
    pub name: Name,
    pub access: Option<Keyword>,
    pub is_virtual: bool,
    pub virtual_first: bool,
    pub pack: bool,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct ClassSyntax {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Retained attributes are consumed by later semantic analysis"
        )
    )]
    pub attributes: List<Attribute>,
    #[expect(
        dead_code,
        reason = "Class key source location is retained for later declaration diagnostics"
    )]
    pub location: SourceOffset,
    pub key: Keyword,
    pub name: Option<Name>,
    pub scope: ScopeId,
    pub final_specifier: bool,
    pub bases: List<Base>,
    pub members: List<DeclId>,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct BitField {
    pub declarator: Option<DeclaratorId>,
    pub width: ExprId,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct MemInitializer {
    #[expect(
        dead_code,
        reason = "Initializer source location is retained for later constructor analysis"
    )]
    pub location: SourceOffset,
    pub name: Name,
    pub initializer: Initializer,
    pub pack: bool,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct FunctionBody {
    pub initializers: List<MemInitializer>,
    pub compound: StmtId,
    pub handlers: Option<List<Handler>>,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum ExceptionDeclaration {
    Ellipsis,
    Typed {
        specifiers: List<Specifier>,
        declarator: Option<DeclaratorId>,
    },
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Handler {
    pub exception: ExceptionDeclaration,
    pub body: StmtId,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Declaration {
    pub attributes: List<Attribute>,
    #[expect(
        dead_code,
        reason = "Source location retained for later semantic diagnostics on this typed syntax node"
    )]
    pub location: SourceOffset,
    pub kind: DeclKind,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum DeclKind {
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
        #[expect(
            dead_code,
            reason = "Linkage brace form is preserved for later source rendering"
        )]
        braced: bool,
        declarations: List<DeclId>,
    },
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Expression {
    pub location: SourceOffset,
    pub kind: ExprKind,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum ExprKind {
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
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Statement {
    #[expect(
        dead_code,
        reason = "Source location retained for later semantic diagnostics on this typed syntax node"
    )]
    pub location: SourceOffset,
    pub kind: StmtKind,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Condition {
    Expression(ExprId),
    Declaration {
        specifiers: List<Specifier>,
        declarator: DeclaratorId,
        initializer: Initializer,
    },
}
#[derive(Clone, Copy, Debug)]
pub(super) enum StmtKind {
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

#[derive(Default)]
pub(super) struct Tree {
    pub declarations: Store<Declaration>,
    pub expressions: Store<Expression>,
    pub declarators: Store<Declarator>,
    pub types: Store<TypeSyntax>,
    pub names: Store<QualifiedName>,
    pub classes: Store<ClassSyntax>,
    pub bodies: Store<Body>,
    pub statements: Store<Statement>,
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
pub(super) struct TreeMark {
    declarations: usize,
    expressions: usize,
    declarators: usize,
    types: usize,
    names: usize,
    classes: usize,
    bodies: usize,
    statements: usize,
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
    pub(super) fn complete_body(
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

    pub(super) fn mark(&self) -> TreeMark {
        TreeMark {
            declarations: self.declarations.mark(),
            expressions: self.expressions.mark(),
            declarators: self.declarators.mark(),
            types: self.types.mark(),
            names: self.names.mark(),
            classes: self.classes.mark(),
            bodies: self.bodies.mark(),
            statements: self.statements.mark(),
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
    pub(super) fn rewind(&mut self, mark: &TreeMark) {
        self.declarations.rewind(mark.declarations);
        self.expressions.rewind(mark.expressions);
        self.declarators.rewind(mark.declarators);
        self.types.rewind(mark.types);
        self.names.rewind(mark.names);
        self.classes.rewind(mark.classes);
        self.bodies.rewind(mark.bodies);
        self.statements.rewind(mark.statements);
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
