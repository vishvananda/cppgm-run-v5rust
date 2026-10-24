// Translated from syntax/ast_clone.cpp::SyntaxCloner. See NOTICE.
//! Copy demanded syntax into the unit's slabs. Every fact-bearing node gets
//! a fresh ID; source bytes, spellings and interned identifier IDs stay shared.
use super::{
    ParseError,
    ast::{
        AsmOperand, Attribute, Base, BitField, Body, BodyId, ClassId, Condition, DeclId, DeclKind,
        DeclaratorId, Enumerator, ExceptionDeclaration, ExprId, ExprKind, Handler, InitDeclarator,
        Initializer, InitializerKind, MemInitializer, Name, NameComponent, NameKind, Parameter,
        ParameterClause, Pointer, Qualifier, QualifierKind, Specifier, SpecifierKind,
        StatementReading, StmtId, StmtKind, Suffix, TemplateArgument, TemplateArgumentKind, Tree,
        TypeId,
    },
    storage::{List, Lists},
};
use crate::support::stack::StackBudget;
mod expansions;
use expansions::Expansion;
pub(crate) use expansions::PackExpander;

pub(crate) struct Cloner<'a> {
    tree: &'a mut Tree,
    budget: StackBudget,
    base: usize,
    retain_bodies: bool,
    expansion: Option<Box<Expansion<'a>>>,
}
impl<'a> Cloner<'a> {
    pub(crate) fn new(tree: &'a mut Tree, budget: StackBudget, base: usize) -> Self {
        Self {
            tree,
            budget,
            base,
            retain_bodies: false,
            expansion: None,
        }
    }
    pub(crate) fn with_expander(mut self, expander: &'a mut dyn PackExpander) -> Self {
        self.expansion = Some(Box::new(Expansion::new(expander)));
        self
    }
    pub(crate) fn class_definition(&mut self, id: ClassId) -> Result<ClassId, ParseError> {
        self.retain_bodies = true;
        self.class(id)
    }
    fn check_stack(&self) -> Result<(), ParseError> {
        let marker = 0u8;
        if self
            .budget
            .exceeded(self.base, std::ptr::from_ref(&marker).addr())
        {
            return Err(ParseError::Limit("constructs nested too deeply"));
        }
        Ok(())
    }
    fn list<T: Copy>(
        &mut self,
        list: List<T>,
        store: fn(&mut Tree) -> &mut Lists<T>,
        each: fn(&mut Self, T) -> Result<T, ParseError>,
    ) -> Result<List<T>, ParseError> {
        let count = store(self.tree).get(list).len();
        let start = store(self.tree).begin();
        for index in 0..count {
            let item = store(self.tree).get(list)[index];
            let item = each(self, item)?;
            store(self.tree).push(item);
        }
        store(self.tree).finish(start)
    }
    fn optional<T>(
        &mut self,
        value: Option<T>,
        each: fn(&mut Self, T) -> Result<T, ParseError>,
    ) -> Result<Option<T>, ParseError> {
        value.map(|value| each(self, value)).transpose()
    }
    fn expressions(&mut self, list: List<ExprId>) -> Result<List<ExprId>, ParseError> {
        if self.expanding() {
            return self.expanded_expressions(list);
        }
        self.list(list, |tree| &mut tree.expr_lists, Self::expression)
    }
    fn name(&mut self, id: Name) -> Result<Name, ParseError> {
        self.name_with_leading(id, true)
    }
    fn name_with_leading(&mut self, id: Name, leading: bool) -> Result<Name, ParseError> {
        self.check_stack()?;
        let mut node = *self.tree.names.get(id);
        let start = self.tree.name_lists.begin();
        for index in 0..self.tree.name_lists.get(node.components).len() {
            let mut component = self.tree.name_lists.get(node.components)[index];
            if leading
                && index == 0
                && !node.global
                && let NameKind::Identifier(name) = component.kind
            {
                component.kind = NameKind::Identifier(self.pack_name(name));
            }
            let component = self.component(component)?;
            self.tree.name_lists.push(component);
        }
        node.components = self.tree.name_lists.finish(start)?;
        self.tree.names.add(node)
    }
    fn component(&mut self, mut node: NameComponent) -> Result<NameComponent, ParseError> {
        node.kind = match node.kind {
            NameKind::Decltype(id) => NameKind::Decltype(self.expression(id)?),
            NameKind::DestructorDecltype(id) => NameKind::DestructorDecltype(self.expression(id)?),
            NameKind::Conversion(id) => NameKind::Conversion(self.ty(id)?),
            other => other,
        };
        if let Some(arguments) = node.arguments {
            node.expanded |= self.expanding() && self.tree.template_arguments.get(arguments).iter().any(|argument| {
                argument.pack || matches!(argument.kind, TemplateArgumentKind::Type(ty) if self.tree.types.get(ty).declarator.is_some_and(|d| self.tree.declarators.get(d).pack))
            });
            node.arguments = Some(self.copied_arguments(arguments)?);
        }
        Ok(node)
    }
    pub(crate) fn argument(
        &mut self,
        mut node: TemplateArgument,
    ) -> Result<TemplateArgument, ParseError> {
        node.kind = match node.kind {
            TemplateArgumentKind::Type(id) => TemplateArgumentKind::Type(self.ty(id)?),
            TemplateArgumentKind::Expression(id) => {
                TemplateArgumentKind::Expression(self.expression(id)?)
            }
        };
        Ok(node)
    }
    fn attribute(&mut self, mut node: Attribute) -> Result<Attribute, ParseError> {
        node.expression = self.optional(node.expression, Self::expression)?;
        node.ty = self.optional(node.ty, Self::ty)?;
        Ok(node)
    }
    fn attributes(&mut self, list: List<Attribute>) -> Result<List<Attribute>, ParseError> {
        self.list(list, |tree| &mut tree.attributes, Self::attribute)
    }
    fn specifier(&mut self, mut node: Specifier) -> Result<Specifier, ParseError> {
        node.kind = match node.kind {
            SpecifierKind::Named(id) => SpecifierKind::Named(self.name(id)?),
            SpecifierKind::Class(id) => SpecifierKind::Class(self.class(id)?),
            SpecifierKind::Enum(id) => SpecifierKind::Enum(self.declaration(id)?),
            SpecifierKind::Elaborated { key, name } => SpecifierKind::Elaborated {
                key,
                name: self.name(name)?,
            },
            SpecifierKind::Decltype(id) => SpecifierKind::Decltype(self.expression(id)?),
            SpecifierKind::ExplicitCondition(id) => {
                SpecifierKind::ExplicitCondition(self.expression(id)?)
            }
            SpecifierKind::Vector { extended, operand } => SpecifierKind::Vector {
                extended,
                operand: self.expression(operand)?,
            },
            other @ SpecifierKind::Keyword(_) => other,
        };
        Ok(node)
    }
    fn specifiers(&mut self, list: List<Specifier>) -> Result<List<Specifier>, ParseError> {
        self.list(list, |tree| &mut tree.specs, Self::specifier)
    }
    pub(crate) fn ty(&mut self, id: TypeId) -> Result<TypeId, ParseError> {
        self.check_stack()?;
        let mut node = *self.tree.types.get(id);
        (node.specifiers, node.declarator) =
            self.parameter_type(node.specifiers, node.declarator)?;
        self.tree.types.add(node)
    }
    // Retained template heads share syntax; each parameter formation owns
    // the type expressions whose semantic facts depend on its scope.
    pub(crate) fn parameter_type(
        &mut self,
        specifiers: List<Specifier>,
        declarator: Option<DeclaratorId>,
    ) -> Result<(List<Specifier>, Option<DeclaratorId>), ParseError> {
        Ok((
            self.specifiers(specifiers)?,
            self.optional(declarator, Self::declarator)?,
        ))
    }
    fn declarator(&mut self, id: DeclaratorId) -> Result<DeclaratorId, ParseError> {
        self.check_stack()?;
        let mut node = *self.tree.declarators.get(id);
        node.name = self.optional(node.name, Self::name)?;
        node.nested = self.optional(node.nested, Self::declarator)?;
        node.attributes = self.attributes(node.attributes)?;
        node.pointers = self.list(node.pointers, |tree| &mut tree.pointers, Self::pointer)?;
        node.suffixes = self.list(node.suffixes, |tree| &mut tree.suffixes, Self::suffix)?;
        self.tree.declarators.add(node)
    }
    fn pointer(&mut self, mut node: Pointer) -> Result<Pointer, ParseError> {
        node.class_name = self.optional(node.class_name, Self::name)?;
        Ok(node)
    }
    fn suffix(&mut self, node: Suffix) -> Result<Suffix, ParseError> {
        Ok(match node {
            Suffix::Array(id) => Suffix::Array(self.optional(id, Self::expression)?),
            Suffix::Function {
                parameters,
                qualifiers,
            } => Suffix::Function {
                parameters: self.parameters(parameters)?,
                qualifiers: self.list(qualifiers, |tree| &mut tree.qualifiers, Self::qualifier)?,
            },
        })
    }
    fn qualifier(&mut self, mut node: Qualifier) -> Result<Qualifier, ParseError> {
        node.kind = match node.kind {
            QualifierKind::Noexcept(id) => {
                QualifierKind::Noexcept(self.optional(id, Self::expression)?)
            }
            QualifierKind::Throw(ids) => QualifierKind::Throw(self.expanded_types(ids)?),
            QualifierKind::Trailing(id) => QualifierKind::Trailing(self.ty(id)?),
            other => other,
        };
        Ok(node)
    }
    pub(crate) fn parameters(
        &mut self,
        mut node: ParameterClause,
    ) -> Result<ParameterClause, ParseError> {
        if self.expanding() {
            node.parameters = self.expanded_parameters(node.parameters)?;
            return Ok(node);
        }
        node.parameters = self.list(
            node.parameters,
            |tree| &mut tree.parameters,
            Self::parameter,
        )?;
        Ok(node)
    }
    fn parameter(&mut self, mut node: Parameter) -> Result<Parameter, ParseError> {
        node.specifiers = self.specifiers(node.specifiers)?;
        node.declarator = self.optional(node.declarator, Self::declarator)?;
        node.default = self.optional(node.default, Self::initializer)?;
        Ok(node)
    }
    pub(crate) fn initializer(&mut self, mut node: Initializer) -> Result<Initializer, ParseError> {
        node.kind = match node.kind {
            InitializerKind::Assign(id) => InitializerKind::Assign(self.expression(id)?),
            InitializerKind::Braced(id) => InitializerKind::Braced(self.expression(id)?),
            InitializerKind::Parenthesized(ids) => {
                InitializerKind::Parenthesized(self.expressions(ids)?)
            }
            other => other,
        };
        Ok(node)
    }
    pub(crate) fn body(&mut self, id: BodyId) -> Result<BodyId, ParseError> {
        if self.retain_bodies {
            return Ok(id);
        }
        let Body::Parsed(mut node) = *self.tree.bodies.get(id) else {
            return Err(ParseError::Invariant("unparsed instantiation body"));
        };
        node.compound = self.statement(node.compound)?;
        node.initializers = self.expanded_initializers(node.initializers)?;
        if let Some(handlers) = node.handlers {
            node.handlers = Some(self.list(handlers, |tree| &mut tree.handlers, Self::handler)?);
        }
        self.tree.bodies.add(Body::Parsed(node))
    }
    fn mem_initializer(&mut self, mut node: MemInitializer) -> Result<MemInitializer, ParseError> {
        node.name = self.name(node.name)?;
        node.initializer = self.initializer(node.initializer)?;
        Ok(node)
    }
    fn handler(&mut self, mut node: Handler) -> Result<Handler, ParseError> {
        let mark = self.shadow_mark();
        if let ExceptionDeclaration::Typed {
            specifiers,
            declarator,
        } = node.exception
        {
            node.exception = ExceptionDeclaration::Typed {
                specifiers: self.specifiers(specifiers)?,
                declarator: self.optional(declarator, Self::declarator)?,
            };
            self.shadow_declarator(declarator);
        }
        node.body = self.statement(node.body)?;
        self.restore_shadows(mark);
        Ok(node)
    }
    pub(crate) fn expression(&mut self, id: ExprId) -> Result<ExprId, ParseError> {
        self.check_stack()?;
        let mut node = *self.tree.expressions.get(id);
        if matches!(node.kind, ExprKind::PackExpansion(_)) && self.collecting() {
            return Ok(id);
        }
        node.kind = match node.kind {
            ExprKind::Statement(body) => ExprKind::Statement(self.statement(body)?),
            ExprKind::Paren(id) => ExprKind::Paren(self.expression(id)?),
            ExprKind::PackExpansion(id) => ExprKind::PackExpansion(self.expression(id)?),
            ExprKind::Id(id) => ExprKind::Id(self.name(id)?),
            ExprKind::Unary { op, operand } => ExprKind::Unary {
                op,
                operand: self.expression(operand)?,
            },
            ExprKind::Postfix { op, operand } => ExprKind::Postfix {
                op,
                operand: self.expression(operand)?,
            },
            ExprKind::Binary {
                op,
                left,
                right,
                assignment,
            } => ExprKind::Binary {
                op,
                left: self.expression(left)?,
                right: self.expression(right)?,
                assignment,
            },
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => ExprKind::Conditional {
                condition: self.expression(condition)?,
                then_expr: self.expression(then_expr)?,
                else_expr: self.expression(else_expr)?,
            },
            ExprKind::Call { callee, arguments } => ExprKind::Call {
                callee: self.expression(callee)?,
                arguments: self.expressions(arguments)?,
            },
            ExprKind::Subscript { base, index } => ExprKind::Subscript {
                base: self.expression(base)?,
                index: self.expression(index)?,
            },
            ExprKind::Member { base, op, member } => ExprKind::Member {
                base: self.expression(base)?,
                op,
                member: self.name_with_leading(member, false)?,
            },
            ExprKind::Braced {
                elements,
                trailing_comma,
            } => ExprKind::Braced {
                elements: self.expressions(elements)?,
                trailing_comma,
            },
            ExprKind::FunctionalCast {
                specifiers,
                arguments,
                braced,
            } => ExprKind::FunctionalCast {
                specifiers: self.specifiers(specifiers)?,
                arguments: self.expressions(arguments)?,
                braced: self.optional(braced, Self::expression)?,
            },
            ExprKind::Cast {
                keyword,
                ty,
                operand,
            } => ExprKind::Cast {
                keyword,
                ty: self.ty(ty)?,
                operand: self.expression(operand)?,
            },
            ExprKind::Sizeof {
                ty,
                operand,
                parenthesized,
            } => ExprKind::Sizeof {
                ty: self.optional(ty, Self::ty)?,
                operand: self.optional(operand, Self::expression)?,
                parenthesized,
            },
            ExprKind::IntegerPack(operand) => ExprKind::IntegerPack(self.expression(operand)?),
            ExprKind::TypeTransform { transform, types } => ExprKind::TypeTransform {
                transform,
                types: self.expanded_types(types)?,
            },
            ExprKind::TypePredicate { predicate, types } => ExprKind::TypePredicate {
                predicate,
                types: self.expanded_types(types)?,
            },
            ExprKind::OffsetOf { ty, designator } => ExprKind::OffsetOf {
                ty: self.ty(ty)?,
                designator: self.expression(designator)?,
            },
            ExprKind::OffsetOfBase(ty) => ExprKind::OffsetOfBase(self.ty(ty)?),
            ExprKind::VaArg { operand, ty } => ExprKind::VaArg {
                ty: self.ty(ty)?,
                operand: self.expression(operand)?,
            },
            ExprKind::TypeTrait {
                keyword,
                ty,
                operand,
            } => ExprKind::TypeTrait {
                keyword,
                ty: self.optional(ty, Self::ty)?,
                operand: self.optional(operand, Self::expression)?,
            },
            ExprKind::New {
                global,
                placement,
                parenthesized_type,
                ty,
                initializer,
            } => ExprKind::New {
                global,
                placement: self.optional(placement, Self::expressions)?,
                parenthesized_type,
                ty: self.ty(ty)?,
                initializer: self.optional(initializer, Self::initializer)?,
            },
            ExprKind::Delete {
                global,
                array,
                operand,
            } => ExprKind::Delete {
                global,
                array,
                operand: self.expression(operand)?,
            },
            ExprKind::Throw(id) => ExprKind::Throw(self.optional(id, Self::expression)?),
            ExprKind::Lambda(id) => ExprKind::Lambda(self.lambda(id)?),
            other @ (ExprKind::Literal(_) | ExprKind::Keyword(_) | ExprKind::SizeofPack(_)) => {
                other
            }
        };
        self.tree.expressions.add(node)
    }
    fn lambda(&mut self, id: super::ast::LambdaId) -> Result<super::ast::LambdaId, ParseError> {
        let mut node = *self.tree.lambdas.get(id);
        node.captures = self.closure_captures(node.captures)?;
        let mark = self.shadow_mark();
        if let Some(head) = node.template_parameters {
            self.shadow_template_packs(head);
        }
        let pack_mark = self.parameter_packs().len();
        let pattern = node.parameters;
        node.parameters = self.optional(node.parameters, Self::parameters)?;
        if self.parameter_packs().len() > pack_mark {
            let packs = self.parameter_packs()[pack_mark..].to_vec();
            node.pattern_parameters = pattern;
            let start = self.tree.lambda_parameter_packs.begin();
            for pack in packs {
                self.tree.lambda_parameter_packs.push(pack);
            }
            node.parameter_packs = self.tree.lambda_parameter_packs.finish(start)?;
        }
        if let Some(parameters) = node.parameters {
            for index in 0..self.tree.parameters.get(parameters.parameters).len() {
                self.shadow_declarator(
                    self.tree.parameters.get(parameters.parameters)[index].declarator,
                );
            }
        }
        node.exception = self.optional(node.exception, Self::qualifier)?;
        node.trailing = self.optional(node.trailing, Self::ty)?;
        node.attributes = self.attributes(node.attributes)?;
        node.body = match node.body {
            super::ast::LambdaBody::Parsed(body) => {
                super::ast::LambdaBody::Parsed(self.statement(body)?)
            }
            super::ast::LambdaBody::Deferred => {
                return Err(ParseError::Invariant("unparsed lambda copy"));
            }
        };
        self.restore_shadows(mark);
        self.restore_parameter_packs(pack_mark);
        self.tree.lambdas.add(node)
    }
    fn condition(&mut self, node: Condition) -> Result<Condition, ParseError> {
        let node = match node {
            Condition::Expression(id) => Condition::Expression(self.expression(id)?),
            Condition::Declaration {
                specifiers,
                declarator,
                initializer,
            } => Condition::Declaration {
                specifiers: self.specifiers(specifiers)?,
                declarator: self.declarator(declarator)?,
                initializer: self.initializer(initializer)?,
            },
        };
        if let Condition::Declaration { declarator, .. } = node {
            self.shadow_declarator(Some(declarator));
        }
        Ok(node)
    }
    fn statement(&mut self, id: StmtId) -> Result<StmtId, ParseError> {
        self.check_stack()?;
        let mut node = *self.tree.statements.get(id);
        let mark = self.shadow_mark();
        let scoped = matches!(
            node.kind,
            StmtKind::Compound(_)
                | StmtKind::If { .. }
                | StmtKind::ConditionalLoop { .. }
                | StmtKind::For { .. }
                | StmtKind::RangeFor { .. }
        );
        node.kind = match node.kind {
            StmtKind::Ambiguous {
                name,
                component,
                readings,
            } => {
                let readings = self.copied_readings(readings)?;
                StmtKind::Ambiguous {
                    name: self.name(name)?,
                    component,
                    readings,
                }
            }
            StmtKind::Compound(ids) => {
                StmtKind::Compound(self.list(ids, |tree| &mut tree.stmt_lists, Self::statement)?)
            }
            StmtKind::Asm(id) => {
                let mut assembly = *self.tree.assembly.get(id);
                assembly.outputs = self.list(
                    assembly.outputs,
                    |tree| &mut tree.asm_operands,
                    Self::asm_operand,
                )?;
                assembly.inputs = self.list(
                    assembly.inputs,
                    |tree| &mut tree.asm_operands,
                    Self::asm_operand,
                )?;
                StmtKind::Asm(self.tree.assembly.add(assembly)?)
            }
            StmtKind::Expression(id) => StmtKind::Expression(self.optional(id, Self::expression)?),
            StmtKind::Return(id) => StmtKind::Return(self.optional(id, Self::expression)?),
            StmtKind::Throw(id) => StmtKind::Throw(self.optional(id, Self::expression)?),
            StmtKind::Declaration(id) => {
                let copy = self.declaration(id)?;
                self.shadow_declared(copy);
                StmtKind::Declaration(copy)
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => StmtKind::If {
                condition: self.condition(condition)?,
                then_branch: self.statement(then_branch)?,
                else_branch: self.optional(else_branch, Self::statement)?,
            },
            StmtKind::ConditionalLoop {
                keyword,
                condition,
                body,
            } => StmtKind::ConditionalLoop {
                keyword,
                condition: self.condition(condition)?,
                body: self.statement(body)?,
            },
            StmtKind::Do { body, condition } => StmtKind::Do {
                body: self.statement(body)?,
                condition: self.expression(condition)?,
            },
            StmtKind::For {
                declaration,
                init,
                condition,
                iteration,
                body,
            } => StmtKind::For {
                declaration: self.for_declaration(declaration)?,
                init: self.optional(init, Self::expression)?,
                condition: self.optional(condition, Self::condition)?,
                iteration: self.optional(iteration, Self::expression)?,
                body: self.statement(body)?,
            },
            StmtKind::RangeFor {
                specifiers,
                declarator,
                range,
                body,
            } => {
                let specifiers = self.specifiers(specifiers)?;
                let declarator = self.declarator(declarator)?;
                let range = self.expression(range)?;
                self.shadow_declarator(Some(declarator));
                StmtKind::RangeFor {
                    specifiers,
                    declarator,
                    range,
                    body: self.statement(body)?,
                }
            }
            StmtKind::Label { name, body } => StmtKind::Label {
                name,
                body: self.statement(body)?,
            },
            StmtKind::Case { value, body } => StmtKind::Case {
                value: self.expression(value)?,
                body: self.statement(body)?,
            },
            StmtKind::Default(id) => StmtKind::Default(self.statement(id)?),
            StmtKind::Try { body, handlers } => StmtKind::Try {
                body: self.statement(body)?,
                handlers: self.list(handlers, |tree| &mut tree.handlers, Self::handler)?,
            },
            other @ (StmtKind::Break | StmtKind::Continue | StmtKind::Goto(_)) => other,
        };
        if scoped {
            self.restore_shadows(mark);
        }
        self.tree.statements.add(node)
    }
    fn asm_operand(&mut self, mut operand: AsmOperand) -> Result<AsmOperand, ParseError> {
        operand.expression = self.expression(operand.expression)?;
        Ok(operand)
    }
    fn copied_readings(
        &mut self,
        list: List<StatementReading>,
    ) -> Result<List<StatementReading>, ParseError> {
        let mut readings = self.tree.statement_readings.get(list).to_vec();
        for node in readings.iter_mut().rev() {
            node.statement = self.statement(node.statement)?;
        }
        let start = self.tree.statement_readings.begin();
        for node in readings {
            self.tree.statement_readings.push(node);
        }
        self.tree.statement_readings.finish(start)
    }
    fn init_declarator(&mut self, mut node: InitDeclarator) -> Result<InitDeclarator, ParseError> {
        node.declarator = self.declarator(node.declarator)?;
        node.initializer = self.optional(node.initializer, Self::initializer)?;
        Ok(node)
    }
    fn enumerator(&mut self, mut node: Enumerator) -> Result<Enumerator, ParseError> {
        node.value = self.optional(node.value, Self::expression)?;
        Ok(node)
    }
    pub(crate) fn enum_values(
        &mut self,
        values: List<Enumerator>,
    ) -> Result<List<Enumerator>, ParseError> {
        self.list(values, |tree| &mut tree.enumerators, Self::enumerator)
    }
    fn base(&mut self, mut node: Base) -> Result<Base, ParseError> {
        node.name = self.name(node.name)?;
        Ok(node)
    }
    fn bit_field(&mut self, mut node: BitField) -> Result<BitField, ParseError> {
        node.declarator = self.optional(node.declarator, Self::declarator)?;
        node.width = self.expression(node.width)?;
        Ok(node)
    }
    fn class(&mut self, id: ClassId) -> Result<ClassId, ParseError> {
        self.check_stack()?;
        let mut node = *self.tree.classes.get(id);
        node.name = self.optional(node.name, Self::name)?;
        node.attributes = self.attributes(node.attributes)?;
        node.bases = self.expanded_bases(node.bases)?;
        node.members = self.list(node.members, |tree| &mut tree.decl_lists, Self::declaration)?;
        self.tree.classes.add(node)
    }
    fn declaration(&mut self, id: DeclId) -> Result<DeclId, ParseError> {
        self.check_stack()?;
        let mut node = *self.tree.declarations.get(id);
        let pack_mark = self.parameter_packs().len();
        let scoped_packs = matches!(
            node.kind,
            DeclKind::Simple { .. } | DeclKind::Function { .. } | DeclKind::SpecialMember { .. }
        );
        node.attributes = self.attributes(node.attributes)?;
        node.kind = match node.kind {
            DeclKind::Simple {
                specifiers,
                declarators,
            } => DeclKind::Simple {
                specifiers: self.specifiers(specifiers)?,
                declarators: self.list(
                    declarators,
                    |tree| &mut tree.init_declarators,
                    Self::init_declarator,
                )?,
            },
            DeclKind::Function {
                specifiers,
                declarator,
                body,
            } => DeclKind::Function {
                specifiers: self.specifiers(specifiers)?,
                declarator: self.declarator(declarator)?,
                body: self.body(body)?,
            },
            DeclKind::Class(specifier) => DeclKind::Class(self.specifier(specifier)?),
            DeclKind::SpecialMember {
                specifiers,
                name,
                declarator,
                initializer,
                body,
            } => DeclKind::SpecialMember {
                specifiers: self.specifiers(specifiers)?,
                name: self.name(name)?,
                declarator: self.declarator(declarator)?,
                initializer: self.optional(initializer, Self::initializer)?,
                body: self.optional(body, Self::body)?,
            },
            DeclKind::BitField {
                specifiers,
                declarators,
            } => DeclKind::BitField {
                specifiers: self.specifiers(specifiers)?,
                declarators: self.list(
                    declarators,
                    |tree| &mut tree.bit_fields,
                    Self::bit_field,
                )?,
            },
            DeclKind::Using {
                directive,
                typename_keyword,
                target,
            } => DeclKind::Using {
                directive,
                typename_keyword,
                target: self.name(target)?,
            },
            DeclKind::Alias { name, ty } => DeclKind::Alias {
                name,
                ty: self.ty(ty)?,
            },
            DeclKind::Enum {
                scope,
                name,
                key,
                underlying,
                enumerators,
                has_body,
                trailing_comma,
            } => DeclKind::Enum {
                scope,
                name: self.optional(name, Self::name)?,
                key,
                underlying: self.optional(underlying, Self::ty)?,
                enumerators: self.list(
                    enumerators,
                    |tree| &mut tree.enumerators,
                    Self::enumerator,
                )?,
                has_body,
                trailing_comma,
            },
            DeclKind::StaticAssert { condition, message } => DeclKind::StaticAssert {
                condition: self.expression(condition)?,
                message,
            },
            other @ (DeclKind::Empty | DeclKind::Access(_)) => other,
            // Template heads are immutable syntax. Each copied declaration
            // binds its own parameters and copies defaults only on demand;
            // member bodies remain retained until their specialization uses them.
            DeclKind::Template {
                parameters,
                declaration,
            } => {
                let mark = self.shadow_template_packs(parameters);
                let copy = self.declaration(declaration);
                self.restore_shadows(mark);
                DeclKind::Template {
                    parameters,
                    declaration: copy?,
                }
            }
            DeclKind::ExplicitInstantiation { .. }
            | DeclKind::Namespace { .. }
            | DeclKind::NamespaceAlias { .. }
            | DeclKind::Linkage { .. } => {
                return Err(ParseError::NotImplemented(
                    "nested template declaration copying",
                ));
            }
        };
        if scoped_packs {
            self.restore_parameter_packs(pack_mark);
        }
        self.tree.declarations.add(node)
    }
}
