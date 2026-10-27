// Translated from sema/analyzer_builtins.cpp. See NOTICE.
//! Operand-checked hosted queries and hints retain their evaluation contract.
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
pub(crate) enum RuntimeIntrinsic {
    AddressOf,
    ObjectSize { minimum: bool },
    Prefetch,
    AssumeAligned,
    FltRounds,
    Overflow(super::overflow_intrinsics::OverflowOperation),
    VectorReduction(super::vector_builtins::Reduction),
    VectorLane(super::vector_builtins::LaneBuiltin),
}

pub(super) fn names(cursor: &mut Cursor<'_>) -> Result<Vec<(NameId, RuntimeIntrinsic)>, Error> {
    use super::overflow_intrinsics::OverflowOperation as O;
    use super::vector_builtins::Reduction as V;
    use RuntimeIntrinsic as R;
    [
        ("__builtin_addressof", R::AddressOf),
        ("__builtin_object_size", R::ObjectSize { minimum: false }),
        (
            "__builtin_dynamic_object_size",
            R::ObjectSize { minimum: false },
        ),
        ("__builtin_prefetch", R::Prefetch),
        ("__builtin_assume_aligned", R::AssumeAligned),
        ("__builtin_flt_rounds", R::FltRounds),
        ("__builtin_add_overflow", R::Overflow(O::Add)),
        ("__builtin_sub_overflow", R::Overflow(O::Subtract)),
        ("__builtin_mul_overflow", R::Overflow(O::Multiply)),
        ("__builtin_reduce_and", R::VectorReduction(V::And)),
        ("__builtin_reduce_or", R::VectorReduction(V::Or)),
        ("__builtin_reduce_xor", R::VectorReduction(V::Xor)),
        ("__builtin_reduce_add", R::VectorReduction(V::Add)),
        ("__builtin_reduce_mul", R::VectorReduction(V::Multiply)),
        ("__builtin_reduce_max", R::VectorReduction(V::Max)),
        ("__builtin_reduce_min", R::VectorReduction(V::Min)),
    ]
    .into_iter()
    .map(|(name, kind)| Ok((name.as_bytes(), kind)))
    .chain(
        crate::lex::vector_lanes::OPERATIONS
            .iter()
            .enumerate()
            .map(|(index, &(name, _))| {
                let operation = crate::lex::vector_lanes::OperationId::new(index)
                    .ok_or(Error::Invariant("vector operation vocabulary limit"))?;
                Ok((
                    name,
                    R::VectorLane(super::vector_builtins::LaneBuiltin::new(
                        operation,
                        super::vector_builtins::LaneSelector::Constant(0),
                    )),
                ))
            }),
    )
    .map(|entry| {
        let (name, kind) = entry?;
        let name = cursor
            .intern_name(name)
            .map_err(Error::identifier_interning)?;
        Ok((name, kind))
    })
    .collect()
}

impl Analyzer<'_> {
    pub(super) fn runtime_builtin(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        mut operation: RuntimeIntrinsic,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        use RuntimeIntrinsic as R;
        let buffer = super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let arguments = buffer.as_slice();
        let count = arguments.len();
        let location = self.tree.expressions.get(id).location;
        let size = self.graph.types.scalar(Scalar::Basic(F::UnsignedLongInt))?;
        let void = self.graph.types.scalar(Scalar::Void)?;
        let integer = self.graph.types.scalar(Scalar::Basic(F::Int))?;
        let result = match operation {
            R::AddressOf => {
                if count != 1 {
                    return Err(Error::UnconditionalDiagnostic {
                        location,
                        message: "__builtin_addressof takes one argument",
                    });
                }
                let operand = self.typed_expression(arguments[0], scope)?;
                self.check_resolved_function(arguments[0])?;
                if operand.category != ValueCategory::Lvalue {
                    return Err(Error::at(
                        location,
                        "__builtin_addressof of something that is no lvalue",
                    ));
                }
                self.graph
                    .types
                    .intern(TypeKind::Pointer(operand.ty), Cv::default())?
            }
            R::Overflow(kind) => return self.overflow_builtin(id, arguments, kind, scope),
            R::VectorReduction(kind) => return self.vector_reduction(id, arguments, kind, scope),
            R::VectorLane(builtin) => {
                return self.vector_lane_builtin(id, arguments, builtin.operation_id(), scope);
            }
            R::ObjectSize { .. } => {
                if count != 2 {
                    return Err(Error::at(
                        location,
                        "__builtin_object_size takes two arguments",
                    ));
                }
                self.unevaluated += 1;
                let pointer = self.typed_expression(arguments[0], scope);
                self.unevaluated -= 1;
                let pointer = pointer?;
                self.check_resolved_function(arguments[0])?;
                self.typed_expression(arguments[1], scope)?;
                self.check_resolved_function(arguments[1])?;
                if !self.exception_dependent_type(pointer.ty)
                    && !matches!(
                        self.graph.types.get(pointer.ty).kind,
                        TypeKind::Pointer(_)
                            | TypeKind::Array { .. }
                            | TypeKind::Scalar(Scalar::Nullptr)
                    )
                {
                    return Err(Error::at(
                        self.tree.expressions.get(arguments[0]).location,
                        "the first argument of __builtin_object_size is no pointer",
                    ));
                }
                let kind = self.constant_value(arguments[1], scope, false)?;
                if kind.is_some_and(|kind| kind.bits > 3) {
                    return Err(Error::at(
                        self.tree.expressions.get(arguments[1]).location,
                        "the second argument of __builtin_object_size is not from 0 to 3",
                    ));
                }
                if kind.is_none() && self.template_depth == 0 {
                    return Err(Error::at(
                        self.tree.expressions.get(arguments[1]).location,
                        "the second argument of __builtin_object_size is no integral constant expression",
                    ));
                }
                operation = R::ObjectSize {
                    minimum: kind.is_some_and(|kind| kind.bits & 2 != 0),
                };
                size
            }
            R::Prefetch | R::AssumeAligned => {
                let prefetch = operation == R::Prefetch;
                if count < if prefetch { 1 } else { 2 } || count > 3 {
                    return Err(Error::at(
                        location,
                        if prefetch {
                            "wrong number of arguments to __builtin_prefetch"
                        } else {
                            "wrong number of arguments to __builtin_assume_aligned"
                        },
                    ));
                }
                let constant_void = self.graph.types.qualify(void, Cv::CONST)?;
                let pointer = self
                    .graph
                    .types
                    .intern(TypeKind::Pointer(constant_void), Cv::default())?;
                for (index, &argument) in arguments.iter().enumerate() {
                    self.typed_expression(argument, scope)?;
                    self.check_resolved_function(argument)?;
                    let target = if index == 0 {
                        pointer
                    } else if prefetch {
                        let value = self.constant_value(argument, scope, false)?;
                        if value.is_none_or(|value| value.bits > if index == 1 { 1 } else { 3 }) {
                            return Err(Error::at(
                                self.tree.expressions.get(argument).location,
                                "__builtin_prefetch takes constant rw and locality operands",
                            ));
                        }
                        integer
                    } else {
                        size
                    };
                    self.convert_scalar(argument, target, false)?;
                }
                if prefetch {
                    void
                } else {
                    self.graph
                        .types
                        .intern(TypeKind::Pointer(void), Cv::default())?
                }
            }
            R::FltRounds => {
                if count != 0 {
                    return Err(Error::at(
                        location,
                        "__builtin_flt_rounds takes no arguments",
                    ));
                }
                integer
            }
        };
        self.graph
            .expression_facts
            .set_runtime_intrinsic(id, operation);
        self.new_expression_facts(result, ValueCategory::Prvalue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn object_size_keeps_template_bodies_and_storage_undemanded() {
        let source = SourceBuffer::new(
            b"struct I;template<class T>T* hidden(){T::missing();}\
            unsigned long f(){return __builtin_object_size(hidden<I>(),2);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"size-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let bodies = tree.bodies.mark();
        let hidden = cursor.intern_name(b"hidden").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        assert_eq!(analyzer.unevaluated, 0);
        assert_eq!(analyzer.tree.bodies.mark(), bodies);
        assert!(analyzer.graph.storage_layouts.is_empty());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        for binding in analyzer.graph.functions_at(global, hidden.into()) {
            let entity = analyzer.graph.binding(binding).entity;
            assert!(analyzer.graph.entity(entity).demands.as_slice().is_empty());
        }
    }

    #[test]
    fn invalid_unevaluated_operand_restores_context_before_returning_error() {
        let source = SourceBuffer::new(
            b"unsigned long f(){return __builtin_object_size(missing(),0);}".to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"size-error").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        assert!(matches!(
            analyzer.declarations(declarations, global),
            Err(Error::Diagnostic { .. })
        ));
        assert_eq!(analyzer.unevaluated, 0);
    }
}
