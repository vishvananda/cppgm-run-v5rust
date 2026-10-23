// Translated from sema/analyzer_builtins.cpp. See NOTICE.
//! Course builtin call forms, recognized by interned reserved identifiers.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::{Binding, BindingKind, ScopeId},
    lookup::LookupResult,
    types::{Cv, Scalar, TypeKind},
};
use crate::{
    lex::{PostTokenKind, identifiers::NameId, vocabulary::Keyword},
    syntax::{ast, storage::List},
    types::FundamentalType as F,
};
#[derive(Clone, Copy)]
pub(crate) enum BuiltinFunction {
    Abort,
    Expect,
    Memcpy,
    Memmove,
    Memcmp,
    Memset,
    Strlen,
    Unreachable,
    Invoke,
    Atomic(super::atomic_operations::AtomicBuiltinId),
    Floating(super::floating_intrinsics::FloatingOperation),
    LibraryNan(F),
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
    fn deferred_queries_reuse_builtin_identity_outside_source_visibility() {
        let source = SourceBuffer::new(b"int f()noexcept(noexcept(&__builtin_memcpy));void*(*p)(void*,const void*,unsigned long)=&__builtin_memcpy;int h(){return noexcept(f());}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"builtin-identity").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let key = analyzer.builtin_names.memcpy;
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(graph.functions_at(graph.global, key.into()).count(), 1);
    }

    #[test]
    fn invoke_signature_queries_keep_bodies_and_emission_undemanded() {
        let source = SourceBuffer::new(
            b"template<class T>T&& dv();\
            template<class T>struct R{typename T::missing member;};\
            struct F{template<class T>R<T> operator()(T){T::missing();}};\
            typedef decltype(__builtin_invoke(dv<F>(),1)) A;\
            typedef decltype(::__builtin_invoke(dv<F&>(),2)) B;"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"invoke-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let bodies = tree.bodies.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        assert_eq!(analyzer.tree.bodies.mark(), bodies);
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
        assert_eq!(analyzer.pretyped_callable, None);
        assert!(analyzer.body_context.is_none());
        assert_eq!(analyzer.unevaluated, 0);
        assert_eq!(analyzer.decltype_call, None);
        let key = analyzer.builtin_names.invoke;
        assert_eq!(analyzer.graph.functions_at(global, key.into()).count(), 1);
    }

    #[test]
    fn nested_invoke_failure_restores_pretyped_operand_context() {
        let source = SourceBuffer::new(
            b"int f(int);int g(){return __builtin_invoke(f,__builtin_invoke(0));}".to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"invoke-failure").unwrap());
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
        assert_eq!(analyzer.pretyped_callable, None);
    }
}
#[derive(Clone)]
pub(super) struct BuiltinNames {
    pub abort: NameId,
    pub expect: NameId,
    pub constant_p: NameId,
    pub invoke: NameId,
    pub memcpy: NameId,
    pub memmove: NameId,
    pub memcmp: NameId,
    pub memset: NameId,
    pub strlen: NameId,
    pub unreachable: NameId,
    pub atomic: super::atomic_operations::AtomicBuiltins,
    pub floating: Vec<(NameId, super::floating_intrinsics::FloatingOperation)>,
}
impl Analyzer<'_> {
    pub(super) fn ensure_builtin(
        &mut self,
        key: NameId,
    ) -> Result<Option<super::graph::BindingId>, Error> {
        let names = &self.builtin_names;
        let builtin = if key == names.abort {
            BuiltinFunction::Abort
        } else if key == names.expect {
            BuiltinFunction::Expect
        } else if key == names.memcpy {
            BuiltinFunction::Memcpy
        } else if key == names.memmove {
            BuiltinFunction::Memmove
        } else if key == names.memcmp {
            BuiltinFunction::Memcmp
        } else if key == names.memset {
            BuiltinFunction::Memset
        } else if key == names.strlen {
            BuiltinFunction::Strlen
        } else if key == names.unreachable {
            BuiltinFunction::Unreachable
        } else if key == names.invoke {
            BuiltinFunction::Invoke
        } else if let Some(id) = names.atomic.find(key) {
            BuiltinFunction::Atomic(id)
        } else if let Some((_, kind)) = names.floating.iter().find(|&&(name, _)| name == key) {
            BuiltinFunction::Floating(*kind)
        } else {
            return Ok(None);
        };
        // Registry identity is independent of a deferred query's source limit.
        // Inspect only this global name family, including an older builtin
        // behind an ordinary overload. A non-builtin entry blocks creation.
        let limit = self.graph.lookup_limit.take();
        let existing = self
            .graph
            .value_binding(self.graph.global, key.into())
            .map(|_| {
                self.graph
                    .functions_at(self.graph.global, key.into())
                    .find(|&binding| {
                        self.graph
                            .entity(self.graph.binding(binding).entity)
                            .builtin
                            .is_some()
                    })
            });
        self.graph.lookup_limit = limit;
        if let Some(existing) = existing {
            return Ok(existing);
        }
        let void = self.graph.types.scalar(Scalar::Void)?;
        let (result, parameters) = if matches!(
            builtin,
            BuiltinFunction::Abort | BuiltinFunction::Unreachable
        ) {
            (void, Vec::new())
        } else if matches!(
            builtin,
            BuiltinFunction::Floating(_) | BuiltinFunction::Atomic(_) | BuiltinFunction::Invoke
        ) {
            (self.graph.types.scalar(Scalar::Basic(F::Int))?, Vec::new())
        } else if matches!(builtin, BuiltinFunction::Expect) {
            let long = self.graph.types.scalar(Scalar::Basic(F::LongInt))?;
            (long, vec![long, long])
        } else {
            let size = self.graph.types.scalar(Scalar::Basic(F::UnsignedLongInt))?;
            if matches!(builtin, BuiltinFunction::Strlen) {
                let character = self.graph.types.scalar(Scalar::Basic(F::Char))?;
                let character = self.graph.types.qualify(character, Cv::CONST)?;
                let text = self
                    .graph
                    .types
                    .intern(TypeKind::Pointer(character), Cv::default())?;
                (size, vec![text])
            } else {
                let pointer = self
                    .graph
                    .types
                    .intern(TypeKind::Pointer(void), Cv::default())?;
                let constant = self.graph.types.qualify(void, Cv::CONST)?;
                let source = self
                    .graph
                    .types
                    .intern(TypeKind::Pointer(constant), Cv::default())?;
                let integer = self.graph.types.scalar(Scalar::Basic(F::Int))?;
                match builtin {
                    BuiltinFunction::Memcpy | BuiltinFunction::Memmove => {
                        (pointer, vec![pointer, source, size])
                    }
                    BuiltinFunction::Memcmp => (integer, vec![source, source, size]),
                    BuiltinFunction::Memset => (pointer, vec![pointer, integer, size]),
                    BuiltinFunction::Abort
                    | BuiltinFunction::Expect
                    | BuiltinFunction::Strlen
                    | BuiltinFunction::Unreachable
                    | BuiltinFunction::Invoke
                    | BuiltinFunction::LibraryNan(_)
                    | BuiltinFunction::Atomic(_)
                    | BuiltinFunction::Floating(_) => {
                        return Err(Error::Invariant("memory builtin signature"));
                    }
                }
            }
        };
        let ty = self.graph.types.intern(
            TypeKind::Function {
                result,
                parameters,
                variadic: matches!(
                    builtin,
                    BuiltinFunction::Floating(_)
                        | BuiltinFunction::Atomic(_)
                        | BuiltinFunction::Invoke
                ),
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        let entity = self.graph.add_entity(Some(ty), Some(key), Keyword::Int)?;
        self.graph.entity_mut(entity).builtin = Some(builtin);
        self.graph.entity_mut(entity).exceptions =
            super::exceptions::ExceptionState::Value(super::exceptions::ExceptionSet::None);
        self.graph.entity_mut(entity).function = Some(super::graph::FunctionProperties {
            no_inline: false,
            internal_linkage: false,
            language: super::linkage::Language::C,
            denotes: None,
            is_static: false,
            special: None,
            allocation: None,
        });
        Ok(Some(self.graph.add_binding(
            self.graph.global,
            None,
            Binding {
                kind: BindingKind::Function,
                name: Some(key),
                entity,
                ty: Some(ty),
                key: Keyword::Int,
            },
        )?))
    }
    pub(super) fn builtin_lookup(&mut self, key: NameId) -> Result<Option<LookupResult>, Error> {
        let Some(binding) = self.ensure_builtin(key)? else {
            return Ok(None);
        };
        // Like C++ builtin_lookup, this fallback returns the reserved name's
        // entire family, not a source-filtered singleton. Ordinary lookup
        // retains its own declaration-point visibility rules.
        let limit = self.graph.lookup_limit.take();
        let functions = self
            .graph
            .functions_at(self.graph.global, key.into())
            .collect();
        self.graph.lookup_limit = limit;
        Ok(Some(LookupResult {
            injected_template: false,
            binding: Some(binding),
            functions,
            qualifier: None,
            ambiguous: false,
        }))
    }
    pub(super) fn builtin_call(
        &mut self,
        id: ast::ExprId,
        name: ast::Name,
        arguments: List<ast::ExprId>,
        scope: ScopeId,
        found: &mut LookupResult,
    ) -> Result<Option<ExprFacts>, Error> {
        let node = self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        if let Some(binding) = found.binding
            && matches!(
                self.graph
                    .entity(self.graph.binding(binding).entity)
                    .builtin,
                Some(BuiltinFunction::Invoke)
            )
        {
            return self.builtin_invoke(id, arguments, scope).map(Some);
        }
        if let Some(binding) = found.binding
            && let Some(BuiltinFunction::Floating(kind)) = self
                .graph
                .entity(self.graph.binding(binding).entity)
                .builtin
        {
            return self.floating_builtin(id, arguments, kind, scope).map(Some);
        }
        if let Some(binding) = found.binding
            && let Some(BuiltinFunction::Atomic(builtin)) = self
                .graph
                .entity(self.graph.binding(binding).entity)
                .builtin
        {
            let form = builtin.form()?;
            return self.atomic_builtin(id, arguments, form, scope).map(Some);
        }
        if components.len() != 1 || found.binding.is_some() {
            return Ok(None);
        }
        let ast::NameKind::Identifier(key) = components[0].kind else {
            return Ok(None);
        };
        if let Some(builtin) = self.builtin_lookup(key)? {
            *found = builtin;
            if key == self.builtin_names.invoke {
                return self.builtin_invoke(id, arguments, scope).map(Some);
            }
            if let Some(binding) = found.binding
                && let Some(BuiltinFunction::Floating(operation)) = self
                    .graph
                    .entity(self.graph.binding(binding).entity)
                    .builtin
            {
                return self
                    .floating_builtin(id, arguments, operation, scope)
                    .map(Some);
            }
            if let Some(binding) = found.binding
                && let Some(BuiltinFunction::Atomic(builtin)) = self
                    .graph
                    .entity(self.graph.binding(binding).entity)
                    .builtin
            {
                let form = builtin.form()?;
                return self.atomic_builtin(id, arguments, form, scope).map(Some);
            }
            return Ok(None);
        }
        if key != self.builtin_names.constant_p {
            return Ok(None);
        }
        let arguments = self.tree.expr_lists.get(arguments);
        if arguments.len() != 1 {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "__builtin_constant_p takes one argument",
            ));
        }
        let operand = arguments[0];
        self.unevaluated += 1;
        let result = self.typed_expression(operand, scope);
        self.unevaluated -= 1;
        let facts = result?;
        self.check_resolved_function(operand)?;
        let address = self.is_address_expression(operand);
        let mut constant = if address {
            self.address_value(operand, scope)?.is_some_and(|value| {
                matches!(
                    value.root,
                    super::constant_addresses::Root::Null
                        | super::constant_addresses::Root::Literal(_)
                )
            })
        } else {
            self.constant_value(operand, scope, false)?.is_some()
        };
        // Floating, null and string-literal address constants are supported by this query.
        let mut inner = operand;
        while let ast::ExprKind::Paren(expr) = self.tree.expressions.get(inner).kind {
            inner = expr;
        }
        constant |= matches!(
            self.tree.expressions.get(inner).kind,
            ast::ExprKind::Keyword(Keyword::Nullptr)
                | ast::ExprKind::Literal(ast::Literal {
                    kind: PostTokenKind::String { .. }
                        | PostTokenKind::Literal {
                            ty: F::Float | F::Double | F::LongDouble,
                            ..
                        },
                    ..
                })
        );
        if !constant
            && !address
            && matches!(
                self.graph.types.get(facts.ty).kind,
                TypeKind::Scalar(Scalar::Basic(F::Float | F::Double | F::LongDouble))
            )
        {
            return Err(Error::Missing(
                "floating and address constant interpretation",
            ));
        }
        if matches!(
            self.graph.types.get(facts.ty).kind,
            TypeKind::Scalar(Scalar::Void)
        ) {
            constant = false;
        }
        let ty = self.graph.types.scalar(Scalar::Basic(F::Int))?;
        let mut facts = self.new_expression_facts(ty, V::Prvalue)?;
        facts.folded = Some(u64::from(constant));
        Ok(Some(facts))
    }

    fn builtin_invoke(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let arguments = super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let Some((&callable, rest)) = arguments.as_slice().split_first() else {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "__builtin_invoke without a callable operand",
            ));
        };
        let function = self.typed_expression(callable, scope)?;
        if matches!(
            self.graph.types.get(function.ty).kind,
            TypeKind::MemberPointer { .. }
        ) {
            return Err(Error::Missing("invoke of a pointer to member"));
        }
        // Rewrite only this fact-bearing instance. The retained template syntax
        // stays untouched, and lowering consumes the ordinary call's facts.
        let callee = self
            .tree
            .expressions
            .add(ast::Expression {
                location: self.tree.expressions.get(callable).location,
                kind: ast::ExprKind::Paren(callable),
            })
            .map_err(|_| Error::Limit("invoke syntax limit"))?;
        let start = self.tree.expr_lists.begin();
        for &argument in rest {
            self.tree.expr_lists.push(argument);
        }
        let arguments = self
            .tree
            .expr_lists
            .finish(start)
            .map_err(|_| Error::Limit("invoke argument limit"))?;
        let location = self.tree.expressions.get(id).location;
        self.tree.expressions.replace(
            id,
            ast::Expression {
                location,
                kind: ast::ExprKind::Call { callee, arguments },
            },
        );
        let saved = self.pretyped_callable.replace(callable);
        let result = self.procedural_call(id, callee, arguments, scope);
        self.pretyped_callable = saved;
        result
    }
}
