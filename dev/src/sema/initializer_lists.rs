// Translated from sema/analyzer_initializer_lists.cpp and analyzer_range_for.cpp.
// See NOTICE.
//! Declared list identities, conversion ranking and backing-array facts share original syntax.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExpressionOrigin, ValueCategory as V},
    graph::{Binding, BindingKind, DataMemberKind, EntityId, Graph, ScopeId, ScopeKind},
    lookup::Filter,
    types::{Cv, RefKind, TypeId, TypeKind},
};
use crate::{
    lex::identifiers::NameId,
    support::source::SourceOffset,
    syntax::{
        ast::{self, ExprKind},
        storage::List,
    },
};

#[derive(Clone, Copy)]
pub(super) enum RangeElement {
    Concrete(TypeId),
    Dependent(TypeId),
}

impl Analyzer<'_> {
    pub(super) fn initializer_list_constructor(
        &self,
        binding: super::graph::BindingId,
    ) -> Result<bool, Error> {
        let entity = self.graph.binding(binding).entity;
        let ty = self.graph.lower_entity_type(entity)?;
        let count = self.graph.types.function_parameter_count(ty)?;
        if count == 0
            || self
                .initializer_list_element(self.graph.types.function_parameter(ty, 0)?)
                .is_none()
        {
            return Ok(false);
        }
        Ok((1..count).all(|index| self.function_has_default(entity, index)))
    }

    pub(super) fn list_constructor_phase(
        &mut self,
        class: EntityId,
        list: ast::ExprId,
    ) -> Result<Option<super::graph::BindingId>, Error> {
        let mut scratch = std::mem::take(&mut self.call_scratch);
        let result = (|| {
            self.rank_call(
                super::procedural_calls::Candidates::ListConstructors(class),
                &[list],
                None,
                &mut scratch,
            )?;
            if !scratch.has_candidates() {
                return Ok(None);
            }
            self.unique_call(1, 0, &scratch)?
                .map(|chosen| Some(chosen.binding))
                .ok_or_else(|| {
                    Error::at(
                        self.tree.expressions.get(list).location,
                        "an ambiguous initializer-list constructor (13.3.1.7)",
                    )
                })
        })();
        scratch.clear();
        self.call_scratch = scratch;
        result
    }

    pub(super) fn deduce_list_elements(
        &mut self,
        pattern: TypeId,
        list: ast::ExprId,
        depth: u32,
        arguments: &mut [Option<TypeId>],
    ) -> Result<bool, Error> {
        self.check_stack()?;
        let Some(element) = self.initializer_list_element(pattern) else {
            return Ok(true);
        };
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(list).kind else {
            return Err(Error::Invariant("list deduction syntax"));
        };
        for index in 0..self.tree.expr_lists.get(elements).len() {
            let item = self.tree.expr_lists.get(elements)[index];
            if matches!(
                self.tree.expressions.get(item).kind,
                ExprKind::Braced { .. }
            ) {
                if !self.deduce_list_elements(element, item, depth, arguments)? {
                    return Ok(false);
                }
                continue;
            }
            let facts = self.graph.expression_facts.get(item)?;
            if facts.object.is_some()
                && (facts.functions.is_some()
                    || facts.function.is_some()
                    || facts.destructor.is_some())
            {
                return Ok(false);
            }
            if let Some(set) = facts.functions {
                self.deduce_overloaded_argument(element, set, depth, arguments)?;
            } else if !self.deduce_call_argument(
                element,
                facts.ty,
                facts.category,
                depth,
                arguments,
            )? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub(super) fn initializer_list_element(&self, mut ty: TypeId) -> Option<TypeId> {
        if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
            return None;
        };
        let instance = self.graph.class_templates.instance(class)?;
        let name = self.graph.entity(instance.primary).name?;
        let std = self.identifiers.find(b"std")?;
        (self.identifiers.bytes(name) == b"initializer_list"
            && instance.arguments.len() == 1
            && self.in_std_namespace(instance.primary, std))
        .then(|| instance.arguments[0])
    }

    pub(super) fn complete_builtin_list(&mut self, class: EntityId) -> Result<bool, Error> {
        let ty = self.graph.lower_entity_type(class)?;
        let Some(element) = self.initializer_list_element(ty) else {
            return Ok(false);
        };
        let parent = self
            .graph
            .entity(class)
            .home
            .ok_or(Error::Invariant("list home"))?;
        let scope = self.graph.add_scope(
            ScopeKind::Class,
            parent,
            None,
            self.graph.entity(class).name,
        )?;
        self.graph.scope_mut(scope).owner = Some(class);
        let entity = self.graph.entity_mut(class);
        entity.scope = Some(scope);
        entity.complete = true;
        entity.representation = crate::sema::graph::ClassRepresentation::BuiltinInitializerList;
        entity.defined = true;
        entity.aggregation = super::aggregate_initialization::Aggregation::Ineligible;
        let element = self.graph.types.qualify(element, Cv::CONST)?;
        let pointer = self.graph.types.pointer(element)?;
        let size = self.graph.types.scalar(super::types::Scalar::Basic(
            crate::types::FundamentalType::LongInt,
        ))?;
        for (spelling, ty) in [
            (b"<initializer_list __begin>".as_slice(), pointer),
            (b"<initializer_list __size>".as_slice(), size),
        ] {
            let name = self
                .identifiers
                .intern(spelling)
                .map_err(|_| Error::Limit("builtin list member"))?;
            let member = self.graph.add_entity(
                Some(ty),
                Some(name),
                crate::lex::vocabulary::Keyword::Int,
            )?;
            let entity = self.graph.entity_mut(member);
            entity.home = Some(scope);
            entity.data_member = Some(DataMemberKind::Instance);
            self.graph.entity_mut(class).members.push(member);
            self.graph.add_binding(
                scope,
                Some(scope),
                Binding {
                    kind: BindingKind::Variable,
                    name: Some(name),
                    entity: member,
                    ty: Some(ty),
                    key: crate::lex::vocabulary::Keyword::Int,
                },
            )?;
        }
        if self.semantic() {
            self.classify_class_copies(class)?;
            self.complete_destructor(class)?;
            self.complete_virtual_members(class, SourceOffset(0))?;
            self.complete_virtual_layout(class, SourceOffset(0))?;
        }
        Ok(true)
    }

    pub(super) fn initialize_list_object(
        &mut self,
        id: ast::ExprId,
        ty: TypeId,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        let element = self
            .initializer_list_element(ty)
            .ok_or(Error::Invariant("list object element"))?;
        let class = self
            .graph
            .class_of(ty)
            .ok_or(Error::Invariant("list object class"))?;
        if !self.complete_class(class)? {
            return Err(Error::at(
                self.tree.expressions.get(id).location,
                "std::initializer_list of an incomplete specialization",
            ));
        }
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("list object syntax"));
        };
        let mut next = 0;
        while next < self.tree.expr_lists.get(elements).len() {
            let item = self.tree.expr_lists.get(elements)[next];
            if !matches!(
                self.tree.expressions.get(item).kind,
                ExprKind::Braced { .. }
            ) {
                self.ensure_typed(item, scope)?;
                let subaggregate =
                    matches!(self.graph.types.get(element).kind, TypeKind::Array { .. })
                        || self
                            .graph
                            .class_of(element)
                            .is_some_and(|class| self.graph.aggregate_class(class));
                if subaggregate
                    && self.bare_type(self.graph.expression_facts.get(item)?.ty)?
                        != self.bare_type(element)?
                {
                    return Err(Error::at(
                        self.tree.expressions.get(item).location,
                        "an initializer_list element that does not convert to its element type",
                    ));
                }
            }
            self.aggregate_array_element(
                elements,
                &mut next,
                element,
                scope,
                self.tree.expressions.get(id).location,
            )?;
        }
        let element = self.graph.types.qualify(element, Cv::CONST)?;
        let array = self.graph.types.intern(
            TypeKind::Array {
                element,
                bound: u64::try_from(self.tree.expr_lists.get(elements).len())
                    .map_err(|_| Error::Limit("list array size"))?,
                operand: None,
            },
            Cv::default(),
        )?;
        let ty = self.bare_type(ty)?;
        if self.lowering() && self.unevaluated == 0 && self.template_depth == 0 {
            if !elements.is_empty() {
                self.request_object_layout(array, self.tree.expressions.get(id).location);
                // Its backing array owns destruction independently of the wrapper.
                self.request_temporary_destruction(array)?;
            }
            self.request_object_layout(ty, self.tree.expressions.get(id).location);
        }
        self.publish_list(id, ty, V::Prvalue)?;
        if self.graph.aggregate_class(class) {
            let mut facts = self.graph.expression_facts.get(id)?;
            facts.class_initialization = Some(
                super::expression_facts::ClassInitialization::Aggregate(elements),
            );
            self.graph.expression_facts.set(id, facts);
        }
        self.graph.expression_facts.set_initializer_array(id, array);
        Ok(ty)
    }

    fn declared_initializer_list(&mut self, location: SourceOffset) -> Result<EntityId, Error> {
        let std = self
            .identifiers
            .intern(b"std")
            .map_err(|_| Error::Limit("standard namespace name"))?;
        let list = self
            .identifiers
            .intern(b"initializer_list")
            .map_err(|_| Error::Limit("initializer-list name"))?;
        let space =
            self.qualified_declaration_result(self.graph.global, std.into(), Filter::Namespace)?;
        let scope = space.binding.and_then(|binding| {
            let bound = self.graph.binding(binding);
            matches!(
                bound.kind,
                BindingKind::Namespace | BindingKind::NamespaceAlias
            )
            .then(|| self.graph.entity(bound.entity).scope)
            .flatten()
        });
        if let Some(scope) = scope {
            let found =
                self.qualified_declaration_result(scope, list.into(), Filter::Elaborated)?;
            if let Some(primary) = found
                .binding
                .filter(|_| !found.ambiguous)
                .and_then(|binding| self.class_template_primary(binding))
                && self.graph.entity(primary).name == Some(list)
                && self.in_std_namespace(primary, std)
            {
                return Ok(primary);
            }
        }
        Err(Error::at(
            location,
            "std::initializer_list used without its declaration (8.5.4p2)",
        ))
    }

    pub(super) fn in_std_namespace(&self, primary: EntityId, std: NameId) -> bool {
        // A using-directive may find a template whose actual home is elsewhere.
        // Namespace scopes retain their names directly, without class owners.
        let mut home = self.graph.entity(primary).home;
        while let Some(scope) = home {
            let node = self.graph.scope(scope);
            if node.kind != ScopeKind::Namespace {
                return false;
            }
            if node.name == Some(std) {
                return node.parent == Some(self.graph.global);
            }
            if !node.is_inline {
                return false;
            }
            home = node.parent;
        }
        false
    }

    fn list_element_type(
        &mut self,
        item: ast::ExprId,
        scope: ScopeId,
        semantic: bool,
    ) -> Result<(TypeId, V), Error> {
        if semantic {
            let saved_initializing = self.initializing;
            let result = self.typed_expression(item, scope);
            self.initializing = saved_initializing;
            let mut facts = result?;
            if self.body_context.is_some() && facts.functions.is_none() {
                facts.origin = ExpressionOrigin::Implicit;
                self.graph.expression_facts.set(item, facts);
            }
            self.check_resolved_function(item)?;
            let facts = self.graph.expression_facts.get(item)?;
            return Ok((facts.ty, facts.category));
        }
        let ty = self.decltype_type(item, scope)?;
        let category = if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Reference {
                kind: RefKind::Lvalue,
                ..
            }
        ) || matches!(self.tree.expressions.get(item).kind, ExprKind::Id(_))
        {
            V::Lvalue
        } else {
            V::Prvalue
        };
        let ty = if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            element
        } else {
            ty
        };
        Ok((ty, category))
    }

    fn deduce_list_element(
        &mut self,
        placeholder: TypeId,
        ty: TypeId,
        category: V,
        element: &mut Option<TypeId>,
    ) -> Result<bool, Error> {
        let mut arguments = [None];
        if !self.deduce_call_argument(placeholder, ty, category, 0, &mut arguments)? {
            return Ok(false);
        }
        let Some(deduced) = arguments[0] else {
            return Ok(false);
        };
        let canonical = self.graph.types.canonical_function(deduced)?;
        if let Some(previous) = *element
            && self.graph.types.canonical_function(previous)? != canonical
        {
            return Ok(false);
        }
        *element = Some(deduced);
        Ok(true)
    }

    pub(super) fn deduce_list_object(
        &mut self,
        declared: TypeId,
        cv: Cv,
        elements: List<ast::ExprId>,
        scope: ScopeId,
        location: SourceOffset,
        known: &mut Option<TypeId>,
    ) -> Result<TypeId, Error> {
        let placeholder = self.graph.auto_type()?;
        let mut element = None;
        for index in 0..self.tree.expr_lists.get(elements).len() {
            let item = self.tree.expr_lists.get(elements)[index];
            if matches!(
                self.tree.expressions.get(item).kind,
                ExprKind::Braced { .. }
            ) {
                continue;
            }
            let (ty, category) =
                self.list_element_type(item, scope, self.semantic() && self.template_depth == 0)?;
            if self.template_depth != 0 && self.graph.dependent_type(ty) {
                return Ok(ty);
            }
            if !self.deduce_list_element(placeholder, ty, category, &mut element)? {
                return Err(Error::at(
                    self.tree.expressions.get(item).location,
                    "auto deduction from a braced-init-list's elements fails (7.1.6.4p6)",
                ));
            }
        }
        let element = element.ok_or_else(|| Error::at(location,
            "auto deduced from a braced-init-list without an element that deduces U (7.1.6.4p6)"))?;
        let primary = self.declared_initializer_list(location)?;
        let specialization = self.class_specialization(primary, vec![element])?;
        let replacing = self
            .graph
            .entity(specialization)
            .ty
            .ok_or(Error::Invariant("initializer-list specialization type"))?;
        let canonical = self.graph.types.canonical_function(replacing)?;
        if known.is_some_and(|known| known != canonical) {
            return Err(Error::at(
                location,
                "auto deduced as different types by one declaration's declarators (7.1.6.4p7)",
            ));
        }
        *known = Some(canonical);
        let ty = self.replace_auto(declared, replacing, cv)?;
        Ok(ty)
    }

    pub(super) fn deduce_list_range(
        &mut self,
        elements: List<ast::ExprId>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<RangeElement, Error> {
        if elements.is_empty() {
            return Err(Error::at(
                location,
                "a range-based for over an empty braced-init-list deduces no element type",
            ));
        }
        let placeholder = self.graph.auto_type()?;
        let mut element = None;
        let mut dependent = None;
        for index in 0..self.tree.expr_lists.get(elements).len() {
            let item = self.tree.expr_lists.get(elements)[index];
            if matches!(
                self.tree.expressions.get(item).kind,
                ExprKind::Braced { .. }
            ) {
                continue;
            }
            let (ty, category) = self.list_element_type(item, scope, false)?;
            if self.template_depth != 0 && self.graph.dependent_type(ty) {
                dependent = dependent.or(Some(ty));
                continue;
            }
            if !self.deduce_list_element(placeholder, ty, category, &mut element)? {
                return Err(Error::at(
                    self.tree.expressions.get(item).location,
                    "a braced-init-list range's elements deduce different types (7.1.6.4p6)",
                ));
            }
        }
        if let Some(dependent) = dependent {
            return Ok(RangeElement::Dependent(dependent));
        }
        let element = element.ok_or_else(|| {
            Error::at(
                location,
                "a braced-init-list range without an element that deduces its type (7.1.6.4p6)",
            )
        })?;
        self.declared_initializer_list(location)?;
        self.graph
            .types
            .qualify(element, Cv::CONST)
            .map(RangeElement::Concrete)
    }
}

impl Graph {
    pub(crate) fn lower_initializer_array(&self, id: ast::ExprId) -> Option<TypeId> {
        self.expression_facts.initializer_array(id)
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
    fn builtin_completion_and_backing_array_keep_original_syntax_and_demand() {
        let source = SourceBuffer::new(
            b"namespace std{template<class T>class initializer_list;}\
            std::initializer_list<int>*p;std::initializer_list<int>a={1,2};"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"list-completion").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let bodies = tree.bodies.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        for &id in &analyzer.tree.decl_lists.get(declarations)[..2].to_vec() {
            analyzer.declaration(id, global).unwrap();
        }
        let ast::DeclKind::Simple { declarators, .. } = analyzer
            .tree
            .declarations
            .get(analyzer.tree.decl_lists.get(declarations)[1])
            .kind
        else {
            panic!("pointer declaration")
        };
        let pointer = analyzer.tree.init_declarators.get(declarators)[0].declarator;
        let binding = analyzer.graph.declared_bindings[pointer.index()].unwrap();
        let TypeKind::Pointer(ty) = analyzer
            .graph
            .types
            .get(analyzer.graph.binding(binding).ty.unwrap())
            .kind
        else {
            panic!("pointer type")
        };
        let class = analyzer.graph.class_of(ty).unwrap();
        assert!(!analyzer.graph.entity(class).complete);
        let object = analyzer.tree.decl_lists.get(declarations)[2];
        analyzer.declaration(object, global).unwrap();
        assert!(
            analyzer.graph.entity(class).representation
                == crate::sema::graph::ClassRepresentation::BuiltinInitializerList
        );
        let members = analyzer.graph.entity(class).members.clone();
        assert_eq!(members.len(), 2);
        let ast::DeclKind::Simple { declarators, .. } = analyzer.tree.declarations.get(object).kind
        else {
            panic!("object declaration")
        };
        let initializer = analyzer.tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap();
        let ast::InitializerKind::Assign(list) = initializer.kind else {
            panic!("object list")
        };
        let array = analyzer
            .graph
            .expression_facts
            .initializer_array(list)
            .unwrap();
        let TypeKind::Array { element, bound, .. } = analyzer.graph.types.get(array).kind else {
            panic!("backing array")
        };
        assert_eq!(bound, 2);
        assert_eq!(analyzer.type_cv(element), Cv::CONST);
        let demands = analyzer.graph.root_demands.as_slice().len();
        assert!(analyzer.complete_class(class).unwrap());
        assert_eq!(analyzer.graph.entity(class).members, members);
        assert_eq!(analyzer.graph.root_demands.as_slice().len(), demands);
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert_eq!(analyzer.tree.bodies.mark(), bodies);
    }

    #[test]
    fn list_deduction_reuses_shells_without_definition_or_syntax_demand() {
        let source = SourceBuffer::new(
            b"namespace std{template<class T>class initializer_list;}\
            auto a={1,2};const auto&b={3,4};auto*c={5};"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"list-shells").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let bodies = tree.bodies.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer
            .declaration(analyzer.tree.decl_lists.get(declarations)[0], global)
            .unwrap();
        let mut shells = Vec::new();
        for index in 1..analyzer.tree.decl_lists.get(declarations).len() {
            let declaration = *analyzer
                .tree
                .declarations
                .get(analyzer.tree.decl_lists.get(declarations)[index]);
            let ast::DeclKind::Simple {
                specifiers,
                declarators,
                ..
            } = declaration.kind
            else {
                panic!("placeholder declaration");
            };
            let init = analyzer.tree.init_declarators.get(declarators)[0];
            let specs = analyzer.specifiers(specifiers, global, false).unwrap();
            let mut info = analyzer
                .placeholder_declarator(Some(init.declarator), specs, global)
                .unwrap();
            analyzer
                .deduce_declared_object(
                    &mut info,
                    specs,
                    init,
                    global,
                    declaration.location,
                    &mut None,
                )
                .unwrap();
            assert_eq!(
                analyzer.graph.declarator_types[init.declarator.index()],
                Some(info.ty)
            );
            let mut ty = info.ty;
            if let TypeKind::Reference { element, .. } | TypeKind::Pointer(element) =
                analyzer.graph.types.get(ty).kind
            {
                ty = element;
            }
            let TypeKind::Class(shell) = analyzer.graph.types.get(ty).kind else {
                panic!("deduced list specialization");
            };
            assert!(!analyzer.graph.entity(shell).complete);
            assert!(analyzer.graph.entity(shell).scope.is_none());
            let instance = analyzer.graph.class_templates.instance(shell).unwrap();
            assert!(!instance.dependent);
            assert_eq!(instance.arguments.len(), 1);
            shells.push(shell);
        }
        assert!(shells.iter().all(|&shell| shell == shells[0]));
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert_eq!(analyzer.tree.bodies.mark(), bodies);
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn missing_declaration_keeps_typed_elements_and_restores_initializer_owners() {
        let source =
            SourceBuffer::new(b"int next();int main(){auto values={next(),2};}".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"list-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        assert!(matches!(analyzer.declarations(declarations, global),
            Err(Error::Diagnostic { message, .. })
            if message == "std::initializer_list used without its declaration (8.5.4p2)"));
        let ast::DeclKind::Function { body, .. } = analyzer
            .tree
            .declarations
            .get(analyzer.tree.decl_lists.get(declarations)[1])
            .kind
        else {
            panic!("function definition");
        };
        let ast::Body::Parsed(body) = analyzer.tree.bodies.get(body) else {
            panic!("parsed function body");
        };
        let ast::StmtKind::Compound(statements) = analyzer.tree.statements.get(body.compound).kind
        else {
            panic!("compound body");
        };
        let ast::StmtKind::Declaration(declaration) = analyzer
            .tree
            .statements
            .get(analyzer.tree.stmt_lists.get(statements)[0])
            .kind
        else {
            panic!("local declaration");
        };
        let ast::DeclKind::Simple { declarators, .. } =
            analyzer.tree.declarations.get(declaration).kind
        else {
            panic!("simple local declaration");
        };
        let initializer = analyzer.tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap();
        let ast::InitializerKind::Assign(list) = initializer.kind else {
            panic!("list initializer");
        };
        let ExprKind::Braced { elements, .. } = analyzer.tree.expressions.get(list).kind else {
            panic!("braced syntax");
        };
        assert!(analyzer.graph.expression_facts.known(list).is_none());
        for &item in analyzer.tree.expr_lists.get(elements) {
            let facts = analyzer.graph.expression_facts.get(item).unwrap();
            assert_eq!(facts.origin, ExpressionOrigin::Implicit);
            assert!(facts.conversion.is_none());
        }
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert!(analyzer.initializing.is_none());
        assert!(analyzer.initializing_declarator.is_none());
        assert!(analyzer.deduced_initializer.is_none());
        assert!(analyzer.body_context.is_none());
    }
}
