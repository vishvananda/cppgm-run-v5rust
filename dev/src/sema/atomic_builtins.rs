// Translated from sema/analyzer_builtins.cpp::atomic_builtin. See NOTICE.
//! Atomic operands are checked once and publish conversions and operation identity.
use super::{
    Error,
    analyzer::Analyzer,
    atomic_operations::{AtomicForm, AtomicOperand as Role, AtomicOperation as Op, AtomicResult},
    expression_facts::{ExprFacts, ValueCategory},
    graph::ScopeId,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{
    syntax::{ast, storage::List},
    types::FundamentalType as F,
};

impl Analyzer<'_> {
    pub(super) fn atomic_builtin(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        form: AtomicForm,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let buffer = super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let arguments = buffer.as_slice();
        let location = self.tree.expressions.get(id).location;
        let needed = form.roles.len();
        if if form.trailing {
            arguments.len() < needed
        } else {
            arguments.len() != needed
        } {
            return Err(Error::at(
                location,
                "wrong number of arguments to an atomic builtin",
            ));
        }
        let kind = form.intrinsic.operation;
        let generic = kind.generic() || (form.intrinsic.c11 && !matches!(kind, Op::Update(..)));
        let mut object = None;
        let mut size = 0;
        for (index, &argument) in arguments.iter().enumerate() {
            let operand = self.typed_expression(argument, scope)?;
            self.check_resolved_function(argument)?;
            let Some(&role) = form.roles.get(index) else {
                continue;
            };
            if matches!(role, Role::Pointer | Role::Reference) {
                let pointer = match self.graph.types.get(operand.ty).kind {
                    TypeKind::Array { element, .. } => self.graph.types.pointer(element)?,
                    _ => self.bare_type(operand.ty)?,
                };
                let element = match self.graph.types.get(pointer).kind {
                    TypeKind::Pointer(element) => Some(element),
                    _ => None,
                };
                let constant = matches!(form.result, AtomicResult::Constant);
                if element.is_none() && !constant {
                    return Err(Error::at(
                        self.tree.expressions.get(argument).location,
                        "an atomic builtin's operand is no pointer",
                    ));
                }
                let destination = match role {
                    Role::Pointer => !kind.loads(),
                    _ => matches!(
                        (kind, index),
                        (Op::Load(true) | Op::CompareExchange(_), 1) | (Op::Exchange(true), 2)
                    ),
                };
                if !constant
                    && destination
                    && element.is_some_and(|ty| self.graph.types.get(ty).cv.0 & Cv::CONST.0 != 0)
                {
                    return Err(Error::at(
                        self.tree.expressions.get(argument).location,
                        "an atomic builtin writing through a pointer to const",
                    ));
                }
                if matches!(role, Role::Pointer) && !kind.byte() {
                    let element = element.ok_or(Error::Invariant("atomic object pointer"))?;
                    if form.intrinsic.c11 && self.type_cv(element).0 & Cv::ATOMIC.0 == 0 {
                        return Err(Error::at(
                            self.tree.expressions.get(argument).location,
                            "a __c11_atomic builtin on an object of no atomic type",
                        ));
                    }
                    let ty = self.bare_type(element)?;
                    if form.intrinsic.c11
                        && matches!(kind, Op::Update(..))
                        && let TypeKind::Pointer(pointee) = self.graph.types.get(ty).kind
                        && (matches!(
                            self.graph.types.get(pointee).kind,
                            TypeKind::Scalar(Scalar::Void) | TypeKind::Function { .. }
                        ) || !matches!(
                            kind,
                            Op::Update(
                                super::atomic_operations::AtomicArithmetic::Add
                                    | super::atomic_operations::AtomicArithmetic::Sub,
                                _
                            )
                        ))
                    {
                        return Err(Error::at(
                            self.tree.expressions.get(argument).location,
                            "__c11_atomic arithmetic on a pointer other than an object pointer's addition or subtraction",
                        ));
                    }
                    size = self.atomic_object_size(ty, generic, location)?;
                    object = Some(ty);
                }
                if matches!(role, Role::Reference) && generic && object.is_some() {
                    let ty = element.ok_or(Error::Invariant("generic atomic value pointer"))?;
                    if self
                        .type_size(ty, self.tree.expressions.get(argument).location)?
                        .0
                        != size
                    {
                        return Err(Error::at(
                            self.tree.expressions.get(argument).location,
                            "a generic atomic builtin's operand of another size than the object",
                        ));
                    }
                }
                self.convert_operand_scalar(argument, pointer, false)?;
                continue;
            }
            let target = match role {
                Role::Bool => Some(self.graph.types.scalar(Scalar::Bool)?),
                Role::Size => Some(self.graph.types.scalar(Scalar::Basic(F::UnsignedLongInt))?),
                Role::Order => Some(self.graph.types.scalar(Scalar::Basic(F::Int))?),
                _ if matches!(kind, Op::Update(..))
                    && object.is_some_and(|ty| {
                        matches!(self.graph.types.get(ty).kind, TypeKind::Pointer(_))
                    }) =>
                {
                    Some(self.graph.types.scalar(Scalar::Basic(F::LongInt))?)
                }
                _ => object,
            };
            if let Some(target) = target {
                self.convert_scalar(argument, target, false)?;
            }
        }
        if matches!(form.result, AtomicResult::Constant) {
            let constant = self
                .with_fresh_constant_query(|analyzer| {
                    analyzer.evaluate_constant(arguments[0], scope, false)
                })
                .map_err(|_| {
                    Error::at(
                        location,
                        "__atomic_always_lock_free of a size that is no constant",
                    )
                })?;
            let boolean = self.graph.types.scalar(Scalar::Bool)?;
            let mut facts = self.new_expression_facts(boolean, ValueCategory::Prvalue)?;
            facts.set_folded(Some(u64::from(if form.intrinsic.c11 {
                (1..=8).contains(&constant.bits)
            } else {
                matches!(constant.bits, 1 | 2 | 4 | 8)
            })));
            return Ok(facts);
        }
        let result = match form.result {
            AtomicResult::Object => {
                object.ok_or(Error::Invariant("atomic result without object"))?
            }
            AtomicResult::Bool => self.graph.types.scalar(Scalar::Bool)?,
            _ => self.graph.types.scalar(Scalar::Void)?,
        };
        let facts = self.new_expression_facts(result, ValueCategory::Prvalue)?;
        self.graph
            .expression_facts
            .set_atomic_intrinsic(id, form.intrinsic);
        Ok(facts)
    }

    fn atomic_object_size(
        &mut self,
        ty: TypeId,
        generic: bool,
        location: crate::support::source::SourceOffset,
    ) -> Result<u64, Error> {
        let size = self.type_size(ty, location)?.0;
        if !matches!(size, 1 | 2 | 4 | 8)
            || (!generic
                && !self.is_integral_operand(ty)
                && !matches!(self.graph.types.get(ty).kind, TypeKind::Pointer(_)))
        {
            return Err(Error::at(
                location,
                if generic {
                    "a generic atomic builtin on an object of no 1, 2, 4 or 8 bytes (libatomic's calls are not supported)"
                } else {
                    "an atomic builtin on an object of no integral or pointer type of 1, 2, 4 or 8 bytes"
                },
            ));
        }
        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{
            SyntaxUnit,
            ast::{Body, DeclKind, ExprKind, StmtKind},
        },
    };
    #[test]
    fn c11_weakness_is_a_typed_fact_without_rewriting_source_arguments() {
        let source = SourceBuffer::new(
            b"void f(_Atomic(int)*p,int*e){__c11_atomic_init(p,1);__c11_atomic_compare_exchange_strong(p,e,2,5,2);__c11_atomic_compare_exchange_weak(p,e,3,5,0);}".to_vec(),
        ).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"c11-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let syntax_mark = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), syntax_mark);
        let DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[0])
            .kind
        else {
            panic!("expected function");
        };
        let Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("expected parsed body");
        };
        let StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("expected body");
        };
        for (&statement, (arity, weak)) in tree.stmt_lists.get(statements).iter().zip([
            (2, None),
            (5, Some(false)),
            (5, Some(true)),
        ]) {
            let StmtKind::Expression(Some(id)) = tree.statements.get(statement).kind else {
                panic!("expected expression");
            };
            let ExprKind::Call { arguments, .. } = tree.expressions.get(id).kind else {
                panic!("expected call");
            };
            assert_eq!(tree.expr_lists.get(arguments).len(), arity);
            let intrinsic = graph.expression_facts.atomic_intrinsic(id).unwrap();
            assert!(intrinsic.c11);
            assert_eq!(intrinsic.weak, weak);
        }
        assert_eq!(tree.stmt_lists.get(statements).len(), 3);
    }
    #[test]
    fn runtime_atomics_in_required_constants_report_language_rejection() {
        for input in [
            "int n;static_assert(__atomic_load_n(&n,2),\"constant\");",
            "int n;constexpr int f(){return __atomic_load_n(&n,2);}constexpr int x=f();",
        ] {
            let source = SourceBuffer::new(input.as_bytes().to_vec()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"atomic-constant").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            assert!(matches!(
                analyzer.analyze(declarations),
                Err(Error::UnconditionalDiagnostic { .. })
            ));
        }
    }
    #[test]
    fn atomic_calls_share_registry_and_keep_source_ownership_and_conversions() {
        let source = SourceBuffer::new(b"unsigned f(volatile unsigned*p,unsigned v){__atomic_fetch_add(p,v,5);return ::__atomic_fetch_add(p,v,2);}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"atomic-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let syntax_mark = tree.expressions.mark();
        let key = cursor.intern_name(b"__atomic_fetch_add").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), syntax_mark);
        assert!(std::mem::size_of::<ExprFacts>() <= 256);
        assert!(std::mem::size_of::<super::super::builtin_calls::BuiltinFunction>() <= 4);
        let registry: Vec<_> = graph.functions_at(graph.global, key.into()).collect();
        assert_eq!(registry.len(), 1);
        assert!(
            graph
                .root_demands
                .as_slice()
                .iter()
                .all(|demand| !matches!(demand, super::super::function_demands::Demand::Function { binding, .. } if graph.binding(*binding).entity == graph.binding(registry[0]).entity))
        );
        let DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[0])
            .kind
        else {
            panic!("expected function");
        };
        let Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("expected parsed body");
        };
        let StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("expected compound body");
        };
        for &statement in tree.stmt_lists.get(statements) {
            let (StmtKind::Expression(Some(id)) | StmtKind::Return(Some(id))) =
                tree.statements.get(statement).kind
            else {
                panic!("expected atomic call");
            };
            let intrinsic = graph.expression_facts.atomic_intrinsic(id).unwrap();
            assert_eq!(
                intrinsic.operation,
                Op::Update(super::super::atomic_operations::AtomicArithmetic::Add, true)
            );
            assert!(intrinsic.implied.is_none());
            assert!(
                graph
                    .expression_facts
                    .get(id)
                    .unwrap()
                    .call(&graph.expression_facts)
                    .is_none()
            );
            let ExprKind::Call { callee, arguments } = tree.expressions.get(id).kind else {
                panic!("expected call syntax");
            };
            assert!(graph.expression_facts.get(callee).is_err());
            let arguments = tree.expr_lists.get(arguments);
            assert_eq!(arguments.len(), 3);
            for &argument in arguments {
                assert!(
                    graph
                        .expression_facts
                        .get(argument)
                        .unwrap()
                        .conversion
                        .is_some()
                );
            }
            let address = graph.expression_facts.get(arguments[0]).unwrap();
            let TypeKind::Pointer(element) = graph.types.get(address.ty).kind else {
                panic!("expected pointer type");
            };
            assert_eq!(graph.types.get(element).cv, Cv(2));
        }
    }
}
