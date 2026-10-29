// Translated from analyzer_calls.cpp::call_operator/surrogate_call. See NOTICE.
//! Callable objects prefer a selected member, then an unambiguous visible surrogate.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ConversionRank, ExprFacts, ObjectArgument, ObjectSource, ValueConversion},
    graph::{BindingId, BindingKind, EntityId, ScopeId},
    lookup::Filter,
    procedural_calls::{ArgumentSequence, Candidates},
    specifiers::Explicitness,
    types::TypeKind,
};
use crate::{lex::pp_lexer::Punctuator, syntax::ast};
use std::cmp::Ordering;

#[derive(Default)]
pub(super) struct SurrogateScratch {
    best: Vec<Option<ArgumentSequence>>,
    current: Vec<Option<ArgumentSequence>>,
}
impl SurrogateScratch {
    pub(super) fn clear(&mut self) {
        self.best.clear();
        self.current.clear();
    }
}

impl Analyzer<'_> {
    pub(super) fn callable_object_call(
        &mut self,
        id: ast::ExprId,
        facts: ExprFacts,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let ast::ExprKind::Call { callee, arguments } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("callable object without call syntax"));
        };
        let class = self
            .graph
            .class_of(facts.ty)
            .ok_or(Error::Invariant("callable class"))?;
        let location = self.tree.expressions.get(id).location;
        self.note_closure_invocation(id, callee, class)?;
        if !self.graph.entity(class).complete {
            return Err(Error::at(
                location,
                "call of an object of incomplete class type",
            ));
        }
        let home = self
            .graph
            .entity(class)
            .context()
            .scope
            .ok_or(Error::Invariant("call class without scope"))?;
        self.lookup_location = location;
        let found = self
            .qualified_result(
                home,
                super::operator_candidates::operator_key(Punctuator::LeftParen)?,
                Filter::Ordinary,
            )
            .map_err(|error| match error {
                Error::Diagnostic {
                    message: "ambiguous name",
                    ..
                } => Error::at(location, "ambiguous member name"),
                other => other,
            })?;
        let binding = found
            .binding
            .filter(|&binding| self.graph.binding(binding).kind == BindingKind::Function);
        let mut scratch = std::mem::take(&mut self.call_scratch);
        let result = (|| {
            if let Some(binding) = binding {
                let object = ObjectArgument {
                    source: ObjectSource::Explicit {
                        expression: callee,
                        arrow: false,
                    },
                    ty: facts.ty,
                    category: facts.category,
                };
                let mut designation =
                    self.function_designation(binding, found.functions.as_slice())?;
                designation.set_object(Some(object), &mut self.graph.expression_facts)?;
                designation.set_member_naming(Some(class), &mut self.graph.expression_facts)?;
                designation.set_call_form(super::expression_facts::CallForm::CallableObject);
                let set = designation
                    .functions(&self.graph.expression_facts)
                    .ok_or(Error::Invariant("call operator candidates"))?;
                let argument_buffer =
                    super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
                self.rank_call(
                    Candidates::Functions(set),
                    argument_buffer.as_slice(),
                    Some(object),
                    &mut scratch,
                )?;
                let selected =
                    self.unique_call(self.tree.expr_lists.get(arguments).len(), 1, &scratch)?;
                if selected.is_none()
                    && let Some(binding) =
                        self.surrogate_call(id, class, &mut scratch.surrogates)?
                {
                    let produced = self.convert_class_operand(callee, binding, scope)?;
                    return self.indirect_call(id, callee, arguments, produced.ty, scope);
                }
                // The first resolution only decides whether to consider a
                // surrogate. The ordinary member-call resolution then runs
                // independently, retaining its own argument sequences.
                let selected = self.resolve_call(
                    location,
                    Candidates::Functions(set),
                    argument_buffer.as_slice(),
                    Some(object),
                    &mut scratch,
                )?;
                self.finish_selected_call(id, designation, scope, selected, &scratch)
            } else if let Some(binding) = self.surrogate_call(id, class, &mut scratch.surrogates)? {
                let produced = self.convert_class_operand(callee, binding, scope)?;
                self.indirect_call(id, callee, arguments, produced.ty, scope)
            } else {
                Err(Error::at(
                    location,
                    "called object is not a function or pointer to function",
                ))
            }
        })();
        scratch.clear();
        self.call_scratch = scratch;
        result
    }

    fn surrogate_call(
        &mut self,
        id: ast::ExprId,
        class: EntityId,
        scratch: &mut SurrogateScratch,
    ) -> Result<Option<BindingId>, Error> {
        let ast::ExprKind::Call { arguments, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("surrogate without call syntax"));
        };
        let argument_buffer =
            super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        let args = argument_buffer.as_slice();
        let visible = self.visible_conversion_functions(class)?;
        let count = visible.as_slice(&self.graph).len();
        let mut best = None;
        let mut ambiguous = false;
        for index in 0..count {
            let binding = visible.as_slice(&self.graph)[index];
            let entity = self.graph.entity(self.graph.binding(binding).entity);
            if entity.callable().explicitness == Explicitness::Explicit
                || entity.context().template_head.is_some()
            {
                continue;
            }
            let signature = entity
                .ty
                .ok_or(Error::Invariant("surrogate conversion signature"))?;
            let mut result = self.call_result_view(signature)?.ty;
            if let TypeKind::Pointer(element) = self.graph.types.get(result).kind {
                result = element;
            }
            let TypeKind::Function {
                ref parameters,
                variadic,
                ..
            } = self.graph.types.get(result).kind
            else {
                continue;
            };
            let parameters = parameters.len();
            if args.len() != parameters && !(variadic && args.len() > parameters) {
                continue;
            }
            scratch.current.clear();
            scratch.current.resize(args.len(), None);
            let mut viable = true;
            for (index, &argument) in args.iter().take(parameters).enumerate() {
                let target = self.graph.types.function_parameter(result, index)?;
                let Some(sequence) = self.argument_sequence(argument, target)? else {
                    viable = false;
                    break;
                };
                scratch.current[index] = Some(sequence);
            }
            if !viable {
                continue;
            }
            let mut better = best.is_none();
            let mut worse = false;
            if best.is_some() {
                // The C++ walk compares only this candidate's fixed parameters;
                // padded entries are default standard identities, not ellipses.
                for index in 0..parameters {
                    let sequence = scratch.current[index]
                        .ok_or(Error::Invariant("surrogate argument sequence"))?;
                    let order = if let Some(previous) = scratch.best[index] {
                        self.compare_argument_sequences(sequence, previous)?
                    } else {
                        compare_with_default(sequence)
                    };
                    better |= order == Ordering::Greater;
                    worse |= order == Ordering::Less;
                }
            }
            if better && !worse {
                best = Some(binding);
                std::mem::swap(&mut scratch.best, &mut scratch.current);
                ambiguous = false;
            } else if !worse || better {
                ambiguous = true;
            }
        }
        Ok(best.filter(|_| !ambiguous))
    }
}

fn compare_with_default(sequence: ArgumentSequence) -> Ordering {
    let conversion = sequence.conversion;
    if conversion.rank == ConversionRank::UserDefined
        || conversion.second != ValueConversion::None
        || conversion.qualification
    {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{function_demands::Demand, function_resolution::CallTarget},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn static_callable_receiver_storage_follows_the_emitted_caller() {
        let source = SourceBuffer::new(
            b"struct F{static int operator()(int);};int use(){return F()(7);}\
              struct U{static int operator()();};inline int unused(){return U()();}\
              typedef decltype(U()()) Q;"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"static-receiver").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = super::super::template_bodies::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        for (name, needed) in [(b"F".as_slice(), true), (b"U".as_slice(), false)] {
            let name = cursor.intern_name(name).unwrap();
            let class = graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity;
            assert_eq!(graph.lower_class_layout(class).is_ok(), needed);
        }
    }

    #[test]
    fn callable_member_keeps_its_designation_and_owns_a_separate_call_receiver() {
        use super::super::{expression_facts::CallForm, template_bodies::analyze_unit};
        use crate::syntax::ast::{Body, ExprKind, StmtKind};
        let source = SourceBuffer::new(
            b"struct F0{int operator()(int x)const{return x;}};struct F:F0{};\
              struct B{F const& f;};struct A:B{int run(int x)const{return ((B::f))(x);}};\
              int use(A const&a){return a.run(1);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"call-receiver").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let a = cursor.intern_name(b"A").unwrap();
        let run = cursor.intern_name(b"run").unwrap();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let binding = graph.own(graph.global, a, Filter::Ordinary).unwrap();
        let class = graph.binding(binding).entity;
        let binding = graph
            .own(
                graph.entity(class).context().scope.unwrap(),
                run,
                Filter::Ordinary,
            )
            .unwrap();
        let definition = graph
            .lower_function(graph.binding(binding).entity)
            .unwrap()
            .definition
            .unwrap();
        let ast::DeclKind::Function { body, .. } = tree.declarations.get(definition).kind else {
            panic!("member definition")
        };
        let Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("parsed member body")
        };
        let StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
            panic!("compound")
        };
        let StmtKind::Return(Some(call)) = tree.statements.get(tree.stmt_lists.get(items)[0]).kind
        else {
            panic!("return")
        };
        let ExprKind::Call { callee, .. } = tree.expressions.get(call).kind else {
            panic!("call")
        };
        let object = graph.expression_facts.get(callee).unwrap();
        assert!(matches!(
            object.object(&graph.expression_facts).unwrap().source,
            ObjectSource::Implicit
        ));
        assert!(object.member(&graph.expression_facts).is_some());
        assert!(object.function(&graph.expression_facts).is_none());
        assert!(object.conversion.is_none());
        let receiver = graph.expression_facts.callable_receiver(call).unwrap();
        assert!(
            matches!(receiver.object.source, ObjectSource::Explicit { expression, arrow: false } if expression == callee)
        );
        assert!(receiver.path.is_some());
        assert_ne!(receiver.path, object.member_path(&graph.expression_facts));
        let call_facts = graph.expression_facts.get(call).unwrap();
        assert_eq!(call_facts.call_form(), CallForm::CallableObject);
        assert!(
            call_facts
                .conversion
                .is_none_or(|conversion| conversion.target == call_facts.ty)
        );
        assert!(matches!(
            call_facts.call(&graph.expression_facts),
            Some(CallTarget::Direct(_))
        ));
    }

    #[test]
    fn indirect_result_boundaries_follow_evaluation_and_body_demand() {
        for (body, needed) in [
            ("void f(S&s){typedef decltype(s(1)) Result;}", false),
            ("void f(S&s){A a=s(1);}", true),
        ] {
            let source = SourceBuffer::new(
                format!(
                    "struct A{{int n;}};typedef A(*P)(int);struct S{{operator P();}};\
                 inline void unused(S&s){{A a=s(1);}}{body}"
                )
                .into_bytes(),
            )
            .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"indirect-demand").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let graph = analyzer.analyze(declarations).unwrap();
            let name = cursor.intern_name(b"A").unwrap();
            let class = graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity;
            assert_eq!(
                matches!(
                    graph.entity(class).class().layout,
                    super::super::layout::LayoutState::Value { .. }
                ),
                needed
            );
            assert_eq!(
                graph
                    .entity(class)
                    .special()
                    .implicit_copies
                    .and_then(|copies| copies.passing)
                    .is_some(),
                needed
            );
        }
    }

    #[test]
    fn surrogate_preserves_original_receiver_and_demands_only_the_selected_conversion() {
        let source = SourceBuffer::new(
            b"typedef int(*P)(int,int);typedef int(*Q)(long,long);\
              struct B{operator P();operator Q();};\
              struct S:B{int operator()(int);};struct Unused{operator P();};\
              int f(S&s){return (((s)))(1,2);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"surrogate-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let nodes = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(expressions, tree.expressions.mark());
        assert_eq!(nodes, tree.declarations.mark());
        let mut class = |name: &[u8]| {
            let name = cursor.intern_name(name).unwrap();
            graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity
        };
        let base = class(b"B");
        let source_class = class(b"S");
        let unused = class(b"Unused");
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(*tree.decl_lists.get(declarations).last().unwrap())
            .kind
        else {
            panic!("function");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("parsed body")
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("body")
        };
        let ast::StmtKind::Return(Some(call)) =
            tree.statements.get(tree.stmt_lists.get(statements)[0]).kind
        else {
            panic!("return")
        };
        let ast::ExprKind::Call { callee, arguments } = tree.expressions.get(call).kind else {
            panic!("call")
        };
        let facts = graph.expression_facts.get(callee).unwrap();
        assert_eq!(graph.class_of(facts.ty), Some(source_class));
        assert!(facts.object(&graph.expression_facts).is_none());
        assert!(facts.functions(&graph.expression_facts).is_none());
        assert!(graph.expression_scopes[graph.expression_facts.root(callee).index()].is_some());
        let selected = graph.expression_facts.function_conversion(callee).unwrap();
        assert_eq!(
            selected.binding,
            graph.entity(base).class().declared_conversions[0]
        );
        assert!(selected.path.is_some());
        assert_ne!(callee, graph.expression_facts.root(callee));
        assert_eq!(
            selected.binding,
            graph
                .expression_facts
                .function_conversion(graph.expression_facts.root(callee))
                .unwrap()
                .binding
        );
        assert!(matches!(
            graph
                .expression_facts
                .get(call)
                .unwrap()
                .call(&graph.expression_facts),
            Some(CallTarget::Indirect(_, _))
        ));
        for &argument in tree.expr_lists.get(arguments) {
            assert!(
                graph
                    .expression_facts
                    .get(argument)
                    .unwrap()
                    .conversion
                    .is_some()
            );
        }
        let demanded =
            |binding| {
                graph.root_demands.as_slice().iter().any(
                    |d| matches!(d, Demand::Function { binding: found, .. } if *found == binding),
                )
            };
        assert!(demanded(selected.binding));
        assert!(!demanded(
            graph.entity(base).class().declared_conversions[1]
        ));
        assert!(!demanded(
            graph.entity(unused).class().declared_conversions[0]
        ));
        assert!(matches!(
            graph.entity(unused).class().layout,
            super::super::layout::LayoutState::NotStarted
        ));
    }
}
