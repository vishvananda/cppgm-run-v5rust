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
    lex::{identifiers::NameId, vocabulary::Keyword},
    syntax::{ast, storage::List},
    types::FundamentalType as F,
};
#[derive(Clone, Copy)]
pub(crate) enum BuiltinFunction {
    ConstantP,
    IsConstantEvaluated,
    MakeComplex,
    Abort,
    Expect,
    Memcpy,
    Memmove,
    Memcmp,
    Memset,
    Strlen,
    Unreachable,
    Trap,
    Invoke,
    Allocation(super::allocation::AllocationBuiltin),
    Runtime(super::runtime_intrinsics::RuntimeIntrinsic),
    SourceLocation(super::source_locations::SourceLocation),
    Stack(super::stack_intrinsics::StackIntrinsic),
    Atomic(super::atomic_operations::AtomicBuiltinId),
    Floating(super::floating_intrinsics::FloatingOperation),
    Integer(super::integer_intrinsics::IntegerBuiltin),
    LibraryNan(F),
    Library,
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
    fn resumed_analysis_reuses_unit_vocabulary_and_rechecks_changed_locality() {
        use crate::sema::lookup::Filter;
        let source = SourceBuffer::new(b"struct S {};".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"resumed").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"S").unwrap();
        let analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let vocabulary = std::rc::Rc::clone(&analyzer.builtin_names);
        let mut graph = analyzer.analyze(declarations).unwrap();
        let entity = graph
            .binding(graph.own(graph.global, name, Filter::Ordinary).unwrap())
            .entity;
        let ty = graph.lower_entity_type(entity).unwrap();
        assert!(!graph.lower_type_unit_local(ty).unwrap());
        assert!(!graph.lower_type_unit_local(ty).unwrap());
        let mut analyzer =
            Analyzer::with_graph(tree, cursor, StackBudget::default(), Some(&mut graph)).unwrap();
        assert!(std::rc::Rc::ptr_eq(&vocabulary, &analyzer.builtin_names));
        // Model publication of a new declaration context during a semantic
        // demand. An old read snapshot must not mask that changed dependency.
        analyzer.graph.entity_mut(entity).home = None;
        assert!(analyzer.graph.lower_type_unit_local(ty).unwrap());
        let graph = analyzer.into_graph();
        assert!(graph.lower_type_unit_local(ty).unwrap());
    }

    #[test]
    fn builtin_template_rejection_leaves_call_operands_undemanded() {
        for name in [
            "__builtin_constant_p",
            "__builtin_addressof",
            "__builtin_object_size",
            "__builtin_dynamic_object_size",
            "__builtin_prefetch",
            "__builtin_assume_aligned",
            "__builtin_flt_rounds",
            "__builtin_add_overflow",
            "__builtin_sub_overflow",
            "__builtin_mul_overflow",
            "__builtin_trap",
            "__builtin_operator_new",
            "__builtin_operator_delete",
        ] {
            let source = SourceBuffer::new(
                format!("namespace N{{using ::{name};}}void f(){{((N::{name}<int>))(missing());}}")
                    .into_bytes(),
            )
            .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"builtin-template").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let global = analyzer.graph.global;
            assert!(matches!(
                analyzer.declarations(declarations, global),
                Err(Error::Diagnostic {
                    message: "template-id does not name a function template",
                    ..
                })
            ));
            assert!(analyzer.graph.root_demands.as_slice().is_empty());
            assert!(!analyzer.call_scratch.has_candidates());
            assert!(analyzer.pretyped_callable.is_none());
            assert_eq!(analyzer.unevaluated, 0);
        }
    }

    #[test]
    fn deferred_queries_reuse_builtin_identity_outside_source_visibility() {
        for (name, source) in [
            (b"__builtin_memcpy".as_slice(), b"int f()noexcept(noexcept(&__builtin_memcpy));void*(*p)(void*,const void*,unsigned long)=&__builtin_memcpy;int h(){return noexcept(f());}".as_slice()),
            (b"__builtin_sqrt", b"int f()noexcept(noexcept(&__builtin_sqrt));double(*p)(double)=&__builtin_sqrt;int h(){return noexcept(f());}"),
        ] {
            let source = SourceBuffer::new(source.to_vec()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"builtin-identity").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let key = cursor.intern_name(name).unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let graph = analyzer.analyze(declarations).unwrap();
            assert_eq!(graph.functions_at(graph.global, key.into()).count(), 1);
        }
    }

    #[test]
    fn using_first_demand_reuses_the_global_builtin_entity() {
        let source = SourceBuffer::new(
            b"namespace N{using ::__builtin_sqrt;using ::__builtin_memcpy;}\
            double f(double x){return N::__builtin_sqrt(x)+::__builtin_sqrt(x);}\
            void*g(void*d,const void*s){return N::__builtin_memcpy(d,s,4);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"builtin-using").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let sqrt = cursor.intern_name(b"__builtin_sqrt").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let memcpy = analyzer.builtin_names.memcpy;
        let graph = analyzer.analyze(declarations).unwrap();
        for key in [sqrt, memcpy] {
            assert_eq!(graph.functions_at(graph.global, key.into()).count(), 1);
        }
    }

    #[test]
    fn variadic_library_signature_uses_declared_alias_outside_query_visibility() {
        let source = SourceBuffer::new(b"typedef int __builtin_va_list;".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"builtin-va-alias").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let key = cursor.intern_name(b"__builtin_vsnprintf").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        // An absent alias must not publish or cache an incomplete signature.
        assert!(analyzer.ensure_builtin(key).unwrap().is_none());
        analyzer.declarations(declarations, global).unwrap();
        analyzer.graph.lookup_limit = Some(0);
        let binding = analyzer.ensure_builtin(key).unwrap().unwrap();
        assert_eq!(analyzer.graph.lookup_limit, Some(0));
        let ty = analyzer.graph.binding(binding).ty.unwrap();
        let TypeKind::Function { parameters, .. } = &analyzer.graph.types.get(ty).kind else {
            panic!("library builtin is no function");
        };
        assert_eq!(parameters.len(), 4);
        assert_eq!(
            analyzer.graph.types.get(parameters[3]).kind,
            TypeKind::Scalar(Scalar::Basic(F::Int))
        );
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
    pub is_constant_evaluated: NameId,
    pub make_complex: NameId,
    pub abort: NameId,
    pub expect: NameId,
    pub constant_p: NameId,
    pub invoke: NameId,
    pub operator_new: NameId,
    pub operator_delete: NameId,
    pub memcpy: NameId,
    pub memmove: NameId,
    pub memcmp: NameId,
    pub memset: NameId,
    pub strlen: NameId,
    pub unreachable: NameId,
    pub trap: NameId,
    pub source_locations: Vec<(NameId, super::source_locations::SourceLocation)>,
    pub runtime: Vec<(NameId, super::runtime_intrinsics::RuntimeIntrinsic)>,
    pub stack: super::stack_intrinsics::StackNames,
    pub atomic: super::atomic_operations::AtomicBuiltins,
    pub integer: Vec<(NameId, super::integer_intrinsics::IntegerBuiltin)>,
    pub floating: Vec<(NameId, super::floating_intrinsics::FloatingOperation)>,
}
impl BuiltinNames {
    pub(super) fn new(cursor: &mut crate::syntax::cursor::Cursor<'_>) -> Result<Self, Error> {
        Ok(Self {
            make_complex: cursor
                .intern_name(b"__builtin_complex")
                .map_err(Error::identifier_interning)?,
            is_constant_evaluated: cursor
                .intern_name(b"__builtin_is_constant_evaluated")
                .map_err(Error::identifier_interning)?,
            abort: cursor
                .intern_name(b"__builtin_abort")
                .map_err(Error::identifier_interning)?,
            expect: cursor
                .intern_name(b"__builtin_expect")
                .map_err(Error::identifier_interning)?,
            constant_p: cursor
                .intern_name(b"__builtin_constant_p")
                .map_err(Error::identifier_interning)?,
            invoke: cursor
                .intern_name(b"__builtin_invoke")
                .map_err(Error::identifier_interning)?,
            operator_new: cursor
                .intern_name(b"__builtin_operator_new")
                .map_err(Error::identifier_interning)?,
            operator_delete: cursor
                .intern_name(b"__builtin_operator_delete")
                .map_err(Error::identifier_interning)?,
            memcpy: cursor
                .intern_name(b"__builtin_memcpy")
                .map_err(Error::identifier_interning)?,
            memmove: cursor
                .intern_name(b"__builtin_memmove")
                .map_err(Error::identifier_interning)?,
            memcmp: cursor
                .intern_name(b"__builtin_memcmp")
                .map_err(Error::identifier_interning)?,
            memset: cursor
                .intern_name(b"__builtin_memset")
                .map_err(Error::identifier_interning)?,
            strlen: cursor
                .intern_name(b"__builtin_strlen")
                .map_err(Error::identifier_interning)?,
            unreachable: cursor
                .intern_name(b"__builtin_unreachable")
                .map_err(Error::identifier_interning)?,
            trap: cursor
                .intern_name(b"__builtin_trap")
                .map_err(Error::identifier_interning)?,
            runtime: super::runtime_intrinsics::names(cursor)?,
            source_locations: super::source_locations::names(cursor)?,
            stack: super::stack_intrinsics::StackNames::new(cursor)?,
            floating: super::floating_intrinsics::names(cursor)?,
            integer: super::integer_intrinsics::names(cursor)?,
            atomic: super::atomic_operations::names(cursor)?,
        })
    }
}
impl Analyzer<'_> {
    // analyzer_calls.cpp::template_callee checks the template-id before
    // dispatching any builtin or typing the call's operands.
    pub(super) fn builtin_template_callee(
        &mut self,
        id: ast::ExprId,
        component: ast::NameComponent,
        found: &mut LookupResult,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let ast::ExprKind::Call { callee, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("builtin template callee without call"));
        };
        let mut named = callee;
        while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(named).kind {
            named = inner;
        }
        let component = ast::NameComponent {
            location: self.tree.expressions.get(named).location,
            ..component
        };
        if !found.functions.iter().any(|&binding| {
            self.graph
                .entity(self.graph.binding(binding).entity)
                .context()
                .template_head
                .is_some()
        }) {
            self.explicit_function_arguments(
                component
                    .arguments
                    .ok_or(Error::Invariant("missing explicit builtin arguments"))?,
                scope,
                component.location,
                None,
            )?;
            return Err(Error::at(
                component.location,
                "template-id does not name a function template",
            ));
        }
        self.explicit_function_specializations(component, found, scope, true)
    }

    // A call-expanded builtin has a canonical lookup identity but no
    // addressable function definition, including through a using declaration.
    pub(super) fn check_intrinsic_designation(
        &self,
        binding: super::graph::BindingId,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        if matches!(
            self.graph
                .entity(self.graph.binding(binding).entity)
                .callable()
                .builtin,
            Some(
                BuiltinFunction::Integer(_)
                    | BuiltinFunction::SourceLocation(_)
                    | BuiltinFunction::Runtime(_)
                    | BuiltinFunction::Floating(_)
                    | BuiltinFunction::Atomic(_)
                    | BuiltinFunction::Stack(_)
                    | BuiltinFunction::Invoke
                    | BuiltinFunction::Allocation(_)
                    | BuiltinFunction::MakeComplex
                    | BuiltinFunction::IsConstantEvaluated
                    | BuiltinFunction::ConstantP
            )
        ) {
            return self.reject_substitution(
                location,
                "a builtin expanded where it is called named other than in a call",
                false,
            );
        }
        Ok(())
    }

    pub(super) fn ensure_builtin(
        &mut self,
        key: NameId,
    ) -> Result<Option<super::graph::BindingId>, Error> {
        let names = &self.builtin_names;
        let builtin = if key == names.constant_p {
            BuiltinFunction::ConstantP
        } else if key == names.make_complex {
            BuiltinFunction::MakeComplex
        } else if key == names.is_constant_evaluated {
            BuiltinFunction::IsConstantEvaluated
        } else if key == names.abort {
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
        } else if key == names.trap {
            BuiltinFunction::Trap
        } else if key == names.invoke {
            BuiltinFunction::Invoke
        } else if key == names.operator_new {
            BuiltinFunction::Allocation(super::allocation::AllocationBuiltin::New)
        } else if key == names.operator_delete {
            BuiltinFunction::Allocation(super::allocation::AllocationBuiltin::Delete)
        } else if let Some((_, operation)) = names
            .source_locations
            .iter()
            .find(|&&(name, _)| name == key)
        {
            BuiltinFunction::SourceLocation(*operation)
        } else if let Some(operation) = names.stack.find(key) {
            BuiltinFunction::Stack(operation)
        } else if let Some((_, operation)) = names.runtime.iter().find(|&&(name, _)| name == key) {
            BuiltinFunction::Runtime(*operation)
        } else if let Some(id) = names.atomic.find(key) {
            BuiltinFunction::Atomic(id)
        } else if let Some((_, kind)) = names.integer.iter().find(|&&(name, _)| name == key) {
            BuiltinFunction::Integer(*kind)
        } else if let Some((_, kind)) = names.floating.iter().find(|&&(name, _)| name == key) {
            BuiltinFunction::Floating(*kind)
        } else if crate::lex::library_builtins::lookup(self.identifiers.bytes(key)).is_some() {
            BuiltinFunction::Library
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
                            .callable()
                            .builtin
                            .is_some()
                    })
            });
        self.graph.lookup_limit = limit;
        if let Some(existing) = existing {
            return Ok(existing);
        }
        let void = self.graph.types.scalar(Scalar::Void)?;
        let (result, parameters) = if matches!(builtin, BuiltinFunction::Library) {
            let Some(signature) = self.library_builtin_signature(key)? else {
                return Ok(None);
            };
            signature
        } else if matches!(
            builtin,
            BuiltinFunction::Abort | BuiltinFunction::Unreachable | BuiltinFunction::Trap
        ) {
            (void, Vec::new())
        } else if matches!(
            builtin,
            BuiltinFunction::Integer(_)
                | BuiltinFunction::SourceLocation(_)
                | BuiltinFunction::Runtime(_)
                | BuiltinFunction::Floating(_)
                | BuiltinFunction::Atomic(_)
                | BuiltinFunction::Stack(_)
                | BuiltinFunction::Invoke
                | BuiltinFunction::Allocation(_)
                | BuiltinFunction::MakeComplex
                | BuiltinFunction::IsConstantEvaluated
                | BuiltinFunction::ConstantP
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
                    | BuiltinFunction::MakeComplex
                    | BuiltinFunction::IsConstantEvaluated
                    | BuiltinFunction::ConstantP
                    | BuiltinFunction::Expect
                    | BuiltinFunction::Strlen
                    | BuiltinFunction::Unreachable
                    | BuiltinFunction::Trap
                    | BuiltinFunction::SourceLocation(_)
                    | BuiltinFunction::Runtime(_)
                    | BuiltinFunction::Invoke
                    | BuiltinFunction::Allocation(_)
                    | BuiltinFunction::LibraryNan(_)
                    | BuiltinFunction::Library
                    | BuiltinFunction::Stack(_)
                    | BuiltinFunction::Atomic(_)
                    | BuiltinFunction::Integer(_)
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
                    BuiltinFunction::Integer(_)
                        | BuiltinFunction::SourceLocation(_)
                        | BuiltinFunction::Runtime(_)
                        | BuiltinFunction::Floating(_)
                        | BuiltinFunction::Stack(_)
                        | BuiltinFunction::Atomic(_)
                        | BuiltinFunction::Invoke
                        | BuiltinFunction::Allocation(_)
                        | BuiltinFunction::MakeComplex
                        | BuiltinFunction::IsConstantEvaluated
                        | BuiltinFunction::ConstantP
                ),
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        let entity = self.graph.add_entity(Some(ty), Some(key), Keyword::Int)?;
        self.graph.entity_mut(entity).callable_mut().builtin = Some(builtin);
        self.graph.entity_mut(entity).callable_mut().exceptions =
            super::exceptions::ExceptionState::Value(super::exceptions::ExceptionSet::None);
        self.graph.entity_mut(entity).callable_mut().function =
            Some(super::graph::FunctionProperties {
                effects: super::graph::FunctionEffect::Default,
                no_inline: false,
                returns: super::graph::FunctionReturn::MayReturn,
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
            dependent_prefix: None,
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
        let last = components.last().copied();
        let single = if components.len() == 1 {
            Some(components[0])
        } else {
            None
        };
        if found.binding.is_none()
            && let Some(ast::NameComponent {
                kind: ast::NameKind::Identifier(key),
                ..
            }) = single
            && let Some(builtin) = self.builtin_lookup(key)?
        {
            *found = builtin;
        }
        if let Some(binding) = found.binding {
            if self
                .graph
                .entity(self.graph.binding(binding).entity)
                .callable()
                .builtin
                .is_some()
                && let Some(component) = last.filter(|last| last.arguments.is_some())
            {
                // template_callee precedes intrinsic dispatch in the C++ owner.
                // A registry placeholder is no template; diagnose its explicit
                // arguments before visiting the call's operands.
                self.builtin_template_callee(id, component, found, scope)?;
                return Ok(None);
            }
            return match self
                .graph
                .entity(self.graph.binding(binding).entity)
                .callable()
                .builtin
            {
                Some(BuiltinFunction::ConstantP) => {
                    self.constant_p_call(id, arguments, scope).map(Some)
                }
                Some(BuiltinFunction::MakeComplex) => {
                    self.make_complex_builtin(id, arguments, scope).map(Some)
                }
                Some(BuiltinFunction::IsConstantEvaluated) => {
                    if !self.tree.expr_lists.get(arguments).is_empty() {
                        return Err(Error::at(
                            self.tree.expressions.get(id).location,
                            "__builtin_is_constant_evaluated takes no arguments",
                        ));
                    }
                    let ty = self.graph.types.scalar(Scalar::Bool)?;
                    let facts = self.new_expression_facts(ty, V::Prvalue)?;
                    self.graph
                        .expression_facts
                        .note_constant_evaluation_query(id);
                    Ok(Some(facts))
                }
                Some(BuiltinFunction::SourceLocation(operation)) => self
                    .source_location_builtin(id, arguments, operation)
                    .map(Some),
                Some(BuiltinFunction::Invoke) => {
                    self.builtin_invoke(id, arguments, scope).map(Some)
                }
                Some(BuiltinFunction::Allocation(operation)) => self
                    .allocation_builtin(id, arguments, operation, scope)
                    .map(Some),
                Some(BuiltinFunction::Integer(operation)) => self
                    .integer_builtin(id, arguments, operation, scope)
                    .map(Some),
                Some(BuiltinFunction::Floating(operation)) => self
                    .floating_builtin(id, arguments, operation, scope)
                    .map(Some),
                Some(BuiltinFunction::Atomic(builtin)) => self
                    .atomic_builtin(id, arguments, builtin.form()?, scope)
                    .map(Some),
                Some(BuiltinFunction::Stack(operation)) => self
                    .stack_builtin(id, arguments, operation, scope)
                    .map(Some),
                Some(BuiltinFunction::Runtime(operation)) => self
                    .runtime_builtin(id, arguments, operation, scope)
                    .map(Some),
                _ => Ok(None),
            };
        }
        Ok(None)
    }

    fn constant_p_call(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let arguments = self.tree.expr_lists.get(arguments);
        if arguments.len() != 1 {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "__builtin_constant_p takes one argument",
            ));
        }
        let operand = arguments[0];
        self.unevaluated += 1;
        let result = self.typed_expression(operand, scope).and_then(|facts| {
            self.check_resolved_function(operand)?;
            Ok(facts)
        });
        self.unevaluated -= 1;
        let facts = result?;
        // C++ first asks for an integral constant, then starts a separate
        // interpreter demand for floating/null/literal pointer results.
        let mut constant = self.constant_value(operand, scope, false)?.is_some();
        let scalar_fallback = matches!(
            self.graph.types.get(facts.ty).kind,
            TypeKind::Pointer(_) | TypeKind::Array { .. } | TypeKind::Scalar(Scalar::Nullptr)
        ) || matches!(self.graph.types.get(facts.ty).kind,
                TypeKind::Scalar(Scalar::Basic(f)) if f.floating_rank() != 0);
        if !constant && self.semantic() && self.template_depth == 0 && scalar_fallback {
            use super::constant_addresses::{Root, TemporaryValue};
            constant = match self
                .with_fresh_constant_query(|analyzer| analyzer.scalar_value(operand, scope))
            {
                Ok(value @ TemporaryValue::Floating(_)) => !value.indeterminate(),
                Ok(TemporaryValue::Pointer(value)) => {
                    matches!(value.root, Root::Null | Root::Literal(_))
                }
                Ok(_) | Err(Error::NonConstant { .. }) => false,
                Err(error) => return Err(error),
            };
        }
        let ty = self.graph.types.scalar(Scalar::Basic(F::Int))?;
        let mut facts = self.new_expression_facts(ty, V::Prvalue)?;
        facts.set_folded(Some(u64::from(constant)));
        Ok(facts)
    }

    pub(super) fn builtin_invoke(
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
        if let TypeKind::MemberPointer { owner, element } = self.graph.types.get(function.ty).kind {
            return self.invoke_member(id, callable, rest, owner, element, scope);
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
        self.invoke_call(id, callable, None, scope)
    }

    pub(super) fn invoke_call(
        &mut self,
        id: ast::ExprId,
        callable: ast::ExprId,
        object: Option<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let ast::ExprKind::Call { callee, arguments } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("invoke rewrite without call syntax"));
        };
        let saved_callable = self.pretyped_callable.replace(callable);
        let saved_object = std::mem::replace(&mut self.pretyped_object, object);
        let result = self.procedural_call(id, callee, arguments, scope);
        self.pretyped_callable = saved_callable;
        self.pretyped_object = saved_object;
        result
    }
}
