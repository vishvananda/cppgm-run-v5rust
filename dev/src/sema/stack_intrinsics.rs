// Translated from sema/analyzer_builtins.cpp and analyzer_traits.cpp. See NOTICE.
//! Stack storage and variadic operations keep typed facts on their parsed expression.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory},
    graph::ScopeId,
    types::{Cv, Scalar, TypeKind},
};
use crate::{
    lex::identifiers::NameId,
    syntax::{ast, cursor::Cursor, storage::List},
    types::FundamentalType as F,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StackIntrinsic {
    Alloca,
    VaStart,
    VaEnd,
    VaCopy,
}
#[derive(Clone)]
pub(super) struct StackNames {
    operations: [(NameId, StackIntrinsic); 4],
    tag: NameId,
}
impl StackNames {
    pub(super) fn new(cursor: &mut Cursor<'_>) -> Result<Self, Error> {
        let mut name = |text: &[u8]| {
            cursor
                .intern_name(text)
                .map_err(Error::identifier_interning)
        };
        Ok(Self {
            operations: [
                (name(b"__builtin_alloca")?, StackIntrinsic::Alloca),
                (name(b"__builtin_va_start")?, StackIntrinsic::VaStart),
                (name(b"__builtin_va_end")?, StackIntrinsic::VaEnd),
                (name(b"__builtin_va_copy")?, StackIntrinsic::VaCopy),
            ],
            tag: name(b"__va_list_tag")?,
        })
    }
    pub(super) fn find(&self, key: NameId) -> Option<StackIntrinsic> {
        self.operations
            .iter()
            .find_map(|&(name, operation)| (key == name).then_some(operation))
    }
}
impl Analyzer<'_> {
    fn check_va_list(&self, operand: ast::ExprId, ty: super::types::TypeId) -> Result<(), Error> {
        let tag = match self.graph.types.get(ty).kind {
            TypeKind::Array { element, .. } | TypeKind::Pointer(element) => element,
            _ => ty,
        };
        if self.graph.class_of(tag).is_some_and(|entity| {
            self.graph.entity(entity).name == Some(self.builtin_names.stack.tag)
        }) {
            Ok(())
        } else {
            Err(Error::at(
                self.tree.expressions.get(operand).location,
                "a variadic argument builtin's operand is no __builtin_va_list",
            ))
        }
    }
    pub(super) fn stack_builtin(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        operation: StackIntrinsic,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let arguments = super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let arguments = arguments.as_slice();
        let location = self.tree.expressions.get(id).location;
        let alloca = operation == StackIntrinsic::Alloca;
        let count = if matches!(operation, StackIntrinsic::Alloca | StackIntrinsic::VaEnd) {
            1
        } else {
            2
        };
        if arguments.len() != count {
            return Err(Error::at(
                location,
                if alloca {
                    "__builtin_alloca takes one argument"
                } else {
                    "wrong number of arguments to a variadic argument builtin"
                },
            ));
        }
        if operation == StackIntrinsic::VaStart
            && !self
                .body_context
                .as_ref()
                .and_then(|body| body.function)
                .is_some_and(|function| {
                    self.graph.entity(function).ty.is_some_and(|ty| {
                        matches!(
                            self.graph.types.get(ty).kind,
                            TypeKind::Function { variadic: true, .. }
                        )
                    })
                })
        {
            return Err(Error::at(
                location,
                "__builtin_va_start outside a variadic function",
            ));
        }
        let void = self.graph.types.scalar(Scalar::Void)?;
        for (index, &argument) in arguments.iter().enumerate() {
            let facts = self.typed_expression(argument, scope)?;
            self.check_resolved_function(argument)?;
            if alloca {
                let size = self.graph.types.scalar(Scalar::Basic(F::UnsignedLongInt))?;
                self.convert_scalar(argument, size, false)?;
            } else if operation != StackIntrinsic::VaStart || index != 1 {
                self.check_va_list(argument, facts.ty)?;
            }
        }
        let ty = if alloca {
            self.graph
                .types
                .intern(TypeKind::Pointer(void), Cv::default())?
        } else {
            void
        };
        let facts = self.new_expression_facts(ty, ValueCategory::Prvalue)?;
        self.graph
            .expression_facts
            .set_stack_intrinsic(id, operation);
        Ok(facts)
    }
    pub(super) fn va_arg_expression(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        ty: ast::TypeId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let list = self.typed_expression(operand, scope)?;
        self.check_resolved_function(operand)?;
        let ty = self.type_syntax(ty, scope)?;
        self.check_va_list(operand, list.ty)?;
        let ty = self.bare_type(ty)?;
        if self.template_depth == 0
            && !matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Scalar(
                    Scalar::Bool
                        | Scalar::SignedChar
                        | Scalar::UnsignedChar
                        | Scalar::Short
                        | Scalar::UnsignedShort
                        | Scalar::Nullptr
                        | Scalar::Basic(_)
                ) | TypeKind::Enum(_)
                    | TypeKind::Pointer(_)
                    | TypeKind::MemberPointer { .. }
            )
        {
            if let Some(class) = self.graph.class_of(ty) {
                if !self.complete_class(class)? || !self.class_trivially_copyable(class)? {
                    return Err(Error::at(
                        self.tree.expressions.get(id).location,
                        "__builtin_va_arg of a type that is no scalar or trivially copyable class type",
                    ));
                }
                self.prepare_class_parameter(class)?;
                self.demand_storage_layout(class, self.tree.expressions.get(id).location)?;
            } else {
                return Err(Error::at(
                    self.tree.expressions.get(id).location,
                    "__builtin_va_arg of a type that is no scalar or trivially copyable class type",
                ));
            }
        }
        self.new_expression_facts(ty, ValueCategory::Prvalue)
    }
}
