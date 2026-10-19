// Translated from sema/analyzer_initialization.cpp::braced_argument/list_sequence. See NOTICE.
//! Lists have typed elements during ranking; their object facts are published only after selection.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{
        Conversion, ConversionRank, LvalueTransform, ReferenceSource, ValueConversion,
    },
    graph::{EntityId, ScopeId},
    procedural_calls::{ArgumentSequence, Candidates, UserConversion},
    types::{Cv, RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    syntax::{
        ast::{self, ExprKind},
        storage::List,
    },
};
impl Analyzer<'_> {
    pub(super) fn type_call_argument(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        self.check_stack()?;
        if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind {
            for syntax_index in 0..self.tree.expr_lists.get(elements).len() {
                let element = self.tree.expr_lists.get(elements)[syntax_index];
                self.type_call_argument(element, scope)?;
            }
        } else {
            self.typed_expression(id, scope)?;
        }
        Ok(())
    }
    pub(super) fn list_sequence(
        &mut self,
        id: ast::ExprId,
        parameter: TypeId,
    ) -> Result<Option<ArgumentSequence>, Error> {
        self.check_stack()?;
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("list sequence without list"));
        };
        let reference = match self.graph.types.get(parameter).kind {
            TypeKind::Reference { element, kind } => Some((element, kind)),
            _ => None,
        };
        let target = reference.map_or(parameter, |(element, _)| element);
        if reference.is_some_and(|(_, kind)| kind == RefKind::Lvalue)
            && self.type_cv(target) != Cv::CONST
        {
            return Ok(None);
        }
        let mut sequence = Conversion {
            target: parameter,
            result: target,
            first: LvalueTransform::None,
            second: ValueConversion::None,
            rank: ConversionRank::Exact,
            qualification: false,
            initialization: true,
            list_initialization: crate::sema::expression_facts::ListInitialization::Ordinary,
            temporary: false,
            reference_source: ReferenceSource::Other,
        };
        let mut constructor = None;
        let argument_buffer =
            super::call_arguments::Arguments::new(self.tree.expr_lists.get(elements));
        let values = argument_buffer.as_slice();
        if let Some(element) = self.initializer_list_element(parameter) {
            for &item in values {
                let step = if matches!(
                    self.tree.expressions.get(item).kind,
                    ExprKind::Braced { .. }
                ) {
                    self.list_sequence(item, element)?
                } else {
                    self.argument_sequence(item, element)?
                };
                let Some(step) = step else { return Ok(None) };
                let current = ArgumentSequence {
                    conversion: sequence,
                    application: Some(sequence),
                    function: None,
                    user_conversion: constructor,
                };
                if self.compare_argument_sequences(step, current)? == std::cmp::Ordering::Less {
                    sequence = step.conversion;
                    constructor = step.user_conversion;
                }
            }
            sequence.list_initialization =
                crate::sema::expression_facts::ListInitialization::InitializerList;
            sequence.target = parameter;
            sequence.result = target;
            sequence.temporary = reference.is_some();
            return Ok(Some(ArgumentSequence {
                conversion: sequence,
                application: Some(sequence),
                function: None,
                user_conversion: None,
            }));
        }
        match self.graph.types.get(target).kind {
            TypeKind::Scalar(Scalar::Void) => return Ok(None),
            TypeKind::Array { element, bound, .. } => {
                if bound == 0
                    || u64::try_from(values.len()).map_err(|_| Error::Limit("list size"))? > bound
                {
                    return Ok(None);
                }
                for &id in values {
                    let step =
                        if matches!(self.tree.expressions.get(id).kind, ExprKind::Braced { .. }) {
                            self.list_sequence(id, element)?
                        } else {
                            self.argument_sequence(id, element)?
                        };
                    let Some(step) = step else {
                        return Ok(None);
                    };
                    let current = ArgumentSequence {
                        conversion: sequence,
                        application: Some(sequence),
                        function: None,
                        user_conversion: constructor,
                    };
                    if self.compare_argument_sequences(step, current)? == std::cmp::Ordering::Less {
                        sequence = step.conversion;
                        constructor = step.user_conversion;
                    }
                }
                sequence.result = target;
            }
            TypeKind::Class(class) => {
                if !self.complete_class(class)? || self.graph.entity(class).scope.is_none() {
                    return Ok(None);
                }
                sequence.rank = ConversionRank::UserDefined;
                if self.graph.aggregate_class(class) {
                    let mut next = 0;
                    if !self.list_members_accept(elements, &mut next, class)?
                        || next != values.len()
                    {
                        return Ok(None);
                    }
                } else {
                    constructor =
                        if elements.is_empty() && self.class_has_default_constructor(class)? {
                            self.list_constructor_binding(elements, class)?
                        } else if let Some(binding) = self.list_constructor_phase(class, id)? {
                            Some(UserConversion::Constructor(binding))
                        } else {
                            self.list_constructor_binding(elements, class)?
                        };
                    if constructor.is_none() {
                        return Ok(None);
                    }
                }
                sequence.result = self.bare_type(target)?;
            }
            _ => {
                if values.len() > 1 {
                    return Ok(None);
                }
                if let Some(&single) = values.first() {
                    if matches!(
                        self.tree.expressions.get(single).kind,
                        ExprKind::Braced { .. }
                    ) {
                        return Ok(None);
                    }
                    let bare = self.bare_type(target)?;
                    let Some(step) = self.argument_sequence(single, bare)? else {
                        return Ok(None);
                    };
                    sequence = step.conversion;
                    constructor = step.user_conversion;
                }
                sequence.result = self.bare_type(target)?;
            }
        }
        sequence.target = parameter;
        sequence.temporary = reference.is_some();
        if reference.is_some() {
            sequence.result = target;
        }
        Ok(Some(ArgumentSequence {
            conversion: sequence,
            application: Some(sequence),
            function: None,
            user_conversion: constructor,
        }))
    }

    fn list_members_accept(
        &mut self,
        elements: List<ast::ExprId>,
        next: &mut usize,
        class: EntityId,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        for index in 0..self.graph.entity(class).members.len() {
            if *next == self.tree.expr_lists.get(elements).len() {
                break;
            }
            let member = self.graph.entity(class).members[index];
            if self.graph.unnamed_bit_field(member) {
                continue;
            }
            let ty = self.graph.lower_entity_type(member)?;
            if !self.list_element_accepts(elements, next, ty)? {
                return Ok(false);
            }
            if self.graph.entity(class).key == Keyword::Union {
                break;
            }
        }
        Ok(true)
    }
    fn list_elements_accept(
        &mut self,
        elements: List<ast::ExprId>,
        next: &mut usize,
        element: TypeId,
        bound: u64,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        for _ in 0..bound {
            if *next == self.tree.expr_lists.get(elements).len() {
                break;
            }
            let before = *next;
            if !self.list_element_accepts(elements, next, element)? {
                return Ok(false);
            }
            // An empty subaggregate takes no source element. Its identical
            // omitted array siblings cannot consume one either.
            if *next == before {
                break;
            }
        }
        Ok(true)
    }
    fn list_element_accepts(
        &mut self,
        elements: List<ast::ExprId>,
        next: &mut usize,
        target: TypeId,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        let id = *self
            .tree
            .expr_lists
            .get(elements)
            .get(*next)
            .ok_or(Error::Invariant("list ranking without element"))?;
        let array = match self.graph.types.get(target).kind {
            TypeKind::Array { element, bound, .. } => Some((element, bound)),
            _ => None,
        };
        if let ExprKind::Braced { elements: own, .. } = self.tree.expressions.get(id).kind {
            *next += 1;
            let Some((element, bound)) = array else {
                return Ok(self.list_sequence(id, target)?.is_some());
            };
            let argument_buffer =
                super::call_arguments::Arguments::new(self.tree.expr_lists.get(own));
            let values = argument_buffer.as_slice();
            if let [single] = values
                && self.string_literal(*single)
            {
                return Ok(true);
            }
            let mut inner = 0;
            return Ok(self.list_elements_accept(own, &mut inner, element, bound)?
                && inner == values.len());
        }
        if let Some((element, bound)) = array {
            if !self.string_literal(id) {
                return self.list_elements_accept(elements, next, element, bound);
            }
        } else if let TypeKind::Class(class) = self.graph.types.get(target).kind
            && self.graph.aggregate_class(class)
            && self.graph.class_of(self.graph.expression_facts.get(id)?.ty) != Some(class)
        {
            return self.list_members_accept(elements, next, class);
        }
        *next += 1;
        Ok(array.is_some() || self.argument_sequence(id, target)?.is_some())
    }
    fn list_constructor_binding(
        &mut self,
        elements: List<ast::ExprId>,
        class: EntityId,
    ) -> Result<Option<UserConversion>, Error> {
        if elements.is_empty() && self.class_has_default_constructor(class)? {
            return Ok(Some(self.declared_default_constructor(class)?.map_or(
                UserConversion::ImplicitConstructor(class),
                UserConversion::Constructor,
            )));
        }
        let mut scratch = std::mem::take(&mut self.call_scratch);
        let argument_buffer =
            super::call_arguments::Arguments::new(self.tree.expr_lists.get(elements));
        let result = self
            .rank_call(
                Candidates::Constructors {
                    class,
                    converting: false,
                    copies: matches!(argument_buffer.as_slice(), [id]
                        if !matches!(self.tree.expressions.get(*id).kind, ExprKind::Braced { .. })),
                },
                argument_buffer.as_slice(),
                None,
                &mut scratch,
            )
            .and_then(|()| {
                self.unique_call(self.tree.expr_lists.get(elements).len(), 0, &scratch)
                    .map(|chosen| chosen.map(|chosen| UserConversion::Constructor(chosen.binding)))
            });
        scratch.clear();
        self.call_scratch = scratch;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{construction::ConstructorState, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn list_ranking_completes_only_the_required_class_definition() {
        let source = SourceBuffer::new(
            b"template<class T>struct C{C(T);void unused(){T::missing();}};\
            C<int>*p;int x={3};"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"list-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let pointer = cursor.intern_name(b"p").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let binding = analyzer
            .graph
            .own(global, pointer, Filter::Ordinary)
            .unwrap();
        let TypeKind::Pointer(target) = analyzer
            .graph
            .types
            .get(analyzer.graph.binding(binding).ty.unwrap())
            .kind
        else {
            panic!("pointer type");
        };
        let class = analyzer.graph.class_of(target).unwrap();
        assert!(!analyzer.graph.entity(class).complete);
        let ast::DeclKind::Simple { declarators, .. } = analyzer
            .tree
            .declarations
            .get(analyzer.tree.decl_lists.get(declarations)[2])
            .kind
        else {
            panic!("variable syntax");
        };
        let ast::InitializerKind::Assign(list) = analyzer.tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap()
            .kind
        else {
            panic!("list syntax");
        };
        let demands = analyzer.graph.root_demands.as_slice().len();
        let selected = analyzer.list_sequence(list, target).unwrap().unwrap();
        assert!(analyzer.graph.entity(class).complete);
        assert!(matches!(
            selected.user_conversion,
            Some(UserConversion::Constructor(_))
        ));
        assert!(
            analyzer
                .graph
                .expression_facts
                .get(list)
                .unwrap()
                .constructor
                .is_none()
        );
        for &binding in &analyzer.graph.entity(class).declared_constructors {
            assert!(
                analyzer
                    .graph
                    .entity(analyzer.graph.binding(binding).entity)
                    .constructor
                    .is_none()
            );
        }
        let constructors = analyzer.graph.entity(class).declared_constructors.clone();
        let expressions = analyzer.tree.expressions.mark();
        analyzer.list_sequence(list, target).unwrap().unwrap();
        assert_eq!(
            analyzer.graph.entity(class).declared_constructors,
            constructors
        );
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert_eq!(analyzer.graph.root_demands.as_slice().len(), demands);
    }

    #[test]
    fn ranking_lists_preserves_original_facts_and_does_not_publish_construction() {
        let source = SourceBuffer::new(
            b"struct A{int a[2];};struct C{C(int);C(long);};struct D{int n=5;};int x={3};int y={};"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"list-ranking").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expression_count = tree.expressions.mark();
        let names = [b"A", b"C", b"D"].map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let class = |name| {
            analyzer
                .graph
                .binding(analyzer.graph.own(global, name, Filter::Prefix).unwrap())
                .entity
        };
        let [aggregate, constructed, implicit] = names.map(class);
        let lists: Vec<_> = analyzer.tree.decl_lists.get(declarations)[3..]
            .iter()
            .map(|&id| {
                let ast::DeclKind::Simple { declarators, .. } =
                    analyzer.tree.declarations.get(id).kind
                else {
                    panic!("variable");
                };
                let ast::InitializerKind::Assign(id) =
                    analyzer.tree.init_declarators.get(declarators)[0]
                        .initializer
                        .unwrap()
                        .kind
                else {
                    panic!("list initializer");
                };
                id
            })
            .collect();
        let original = analyzer.graph.expression_facts.get(lists[0]).unwrap();
        let ExprKind::Braced { elements, .. } = analyzer.tree.expressions.get(lists[0]).kind else {
            panic!("list");
        };
        let element = analyzer.tree.expr_lists.get(elements)[0];
        let element_facts = analyzer.graph.expression_facts.get(element).unwrap();
        let demands = analyzer.graph.root_demands.as_slice().len();
        let aggregate_ty = analyzer.graph.entity(aggregate).ty.unwrap();
        let sequence = analyzer
            .list_sequence(lists[0], aggregate_ty)
            .unwrap()
            .unwrap();
        assert_eq!(sequence.conversion.rank, ConversionRank::UserDefined);
        assert!(sequence.user_conversion.is_none());
        let class_ty = analyzer.graph.entity(constructed).ty.unwrap();
        let declared = analyzer.list_sequence(lists[0], class_ty).unwrap().unwrap();
        assert!(
            matches!(declared.user_conversion, Some(UserConversion::Constructor(binding)) if binding == analyzer.graph.entity(constructed).declared_constructors[0])
        );
        let implicit_ty = analyzer.graph.entity(implicit).ty.unwrap();
        let default = analyzer
            .list_sequence(lists[1], implicit_ty)
            .unwrap()
            .unwrap();
        assert!(
            matches!(default.user_conversion, Some(UserConversion::ImplicitConstructor(class)) if class == implicit)
        );
        let element_ty = analyzer.graph.types.qualify(class_ty, Cv::CONST).unwrap();
        let array = analyzer
            .graph
            .types
            .intern(
                TypeKind::Array {
                    element: element_ty,
                    bound: 2,
                    operand: None,
                },
                Cv::default(),
            )
            .unwrap();
        let reference = analyzer
            .graph
            .types
            .intern(
                TypeKind::Reference {
                    element: array,
                    kind: RefKind::Lvalue,
                },
                Cv::default(),
            )
            .unwrap();
        let array_step = analyzer
            .list_sequence(lists[0], reference)
            .unwrap()
            .unwrap();
        assert_eq!(array_step.conversion.rank, ConversionRank::UserDefined);
        assert_eq!(array_step.conversion.result, array);
        assert_eq!(array_step.conversion.target, reference);
        assert!(array_step.conversion.temporary);
        assert_eq!(array_step.user_conversion, declared.user_conversion);
        assert_eq!(analyzer.tree.expressions.mark(), expression_count);
        let after = analyzer.graph.expression_facts.get(lists[0]).unwrap();
        assert_eq!(after.ty, original.ty);
        assert_eq!(after.category, original.category);
        assert!(after.constructor.is_none());
        assert!(after.class_initialization.is_none());
        let after_element = analyzer.graph.expression_facts.get(element).unwrap();
        assert_eq!(after_element.ty, element_facts.ty);
        assert_eq!(
            after_element.conversion.unwrap().target,
            element_facts.conversion.unwrap().target
        );
        assert!(
            analyzer
                .graph
                .expression_facts
                .class_conversion(element)
                .is_none()
        );
        assert_eq!(analyzer.graph.root_demands.as_slice().len(), demands);
        for &binding in &analyzer.graph.entity(constructed).declared_constructors {
            assert!(
                analyzer
                    .graph
                    .entity(analyzer.graph.binding(binding).entity)
                    .constructor
                    .is_none()
            );
        }
        assert!(matches!(
            analyzer.graph.entity(implicit).default_construction,
            ConstructorState::NotStarted
        ));
        let reference = analyzer
            .graph
            .types
            .intern(
                TypeKind::Reference {
                    element: element_ty,
                    kind: RefKind::Lvalue,
                },
                Cv::default(),
            )
            .unwrap();
        let selected = analyzer
            .list_sequence(lists[0], reference)
            .unwrap()
            .unwrap();
        analyzer
            .initialize_ranked_list_argument(lists[0], selected, global)
            .unwrap();
        let initialized = analyzer.graph.expression_facts.get(lists[0]).unwrap();
        assert_eq!(initialized.ty, element_ty);
        let constructor = initialized.constructor.unwrap();
        assert_eq!(
            analyzer.graph.constructors.get(constructor).binding,
            Some(analyzer.graph.entity(constructed).declared_constructors[0]),
        );
        let losing = analyzer.graph.entity(constructed).declared_constructors[1];
        assert!(
            analyzer
                .graph
                .entity(analyzer.graph.binding(losing).entity)
                .constructor
                .is_none()
        );
        assert_eq!(tree.expressions.mark(), expression_count);
    }
}
