// Translated from analyzer.cpp::using_declaration and
// analyzer_construction.cpp's inheriting constructor family. See NOTICE.
//! Sparse unit-owned inheritance identities. Signatures are published at class
//! completion with structural plans; parameter storage belongs to reached calls.
use super::{
    Error,
    analyzer::Analyzer,
    construction::ConstructorId,
    function_names::ValueKey,
    graph::{
        Binding, BindingId, BindingKind, EntityId, FunctionProperties, MemberKey, MemberKind,
        ScopeId, ScopeKind,
    },
    types::{Cv, Scalar, TypeKind},
};
use crate::support::hash::HashMap;
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset, syntax::ast};

fn constructor_properties() -> FunctionProperties {
    FunctionProperties {
        effects: super::graph::FunctionEffect::Default,
        no_inline: false,
        returns: super::graph::FunctionReturn::MayReturn,
        internal_linkage: false,
        language: super::linkage::Language::Cpp,
        denotes: None,
        is_static: false,
        special: None,
        allocation: None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct InstanceKey {
    class: EntityId,
    source: EntityId,
    count: usize,
}
#[derive(Clone, Copy)]
enum Instance {
    Ready(BindingId),
}
#[derive(Clone, Copy)]
struct Inheritance {
    source: BindingId,
    root: EntityId,
    full: bool,
}
#[derive(Default)]
pub(super) struct InheritedConstructors {
    bases: HashMap<EntityId, EntityId>,
    sources: HashMap<EntityId, Inheritance>,
    // Keep the demand-only cache off the large by-value semantic graph.
    specializations: Option<Box<HashMap<InstanceKey, Instance>>>,
}
impl InheritedConstructors {
    fn instance(&self, key: InstanceKey) -> Option<Instance> {
        self.specializations.as_deref()?.get(&key).copied()
    }
    fn set_instance(&mut self, key: InstanceKey, value: Instance) {
        self.specializations
            .get_or_insert_with(Box::default)
            .insert(key, value);
    }
    pub(super) fn source(&self, function: EntityId) -> Option<BindingId> {
        self.sources.get(&function).map(|record| record.source)
    }
    pub(super) fn root(&self, function: EntityId) -> Option<EntityId> {
        self.sources.get(&function).map(|record| record.root)
    }
    pub(super) fn full_root(&self, function: EntityId) -> Option<EntityId> {
        self.sources
            .get(&function)
            .filter(|record| record.full)
            .map(|record| record.root)
    }
}
impl Analyzer<'_> {
    /// Patch only already planned inherited entries after concrete member typing.
    pub(super) fn typed_inheriting_member(&mut self, member: EntityId) {
        let Some(mut class) = self
            .graph
            .entity(member)
            .home
            .and_then(|home| self.graph.scope(home).owner)
        else {
            return;
        };
        while let Some(storage) = self.graph.anonymous_member_storage(class) {
            let Some(owner) = self
                .graph
                .entity(storage)
                .home
                .and_then(|home| self.graph.scope(home).owner)
            else {
                break;
            };
            class = owner;
        }
        if !self.graph.inherited_constructors.bases.contains_key(&class) {
            return;
        }
        if !matches!(
            self.graph.entity(class).class().literal,
            super::literal_types::LiteralState::InProgress
        ) {
            self.graph.entity_mut(class).class_mut().literal =
                super::literal_types::LiteralState::NotStarted;
        }
        let Some(constructor) = self.graph.entity(member).initialization().constructor else {
            return;
        };
        let action = super::construction::MemberConstruction {
            member,
            constructor,
            arguments: self
                .graph
                .entity(member)
                .initialization()
                .constructor_arguments,
        };
        let mut work: Vec<_> = self
            .graph
            .entity(class)
            .class()
            .declared_constructors
            .iter()
            .filter_map(|&binding| {
                let function = self.graph.binding(binding).entity;
                (self.graph.entity(function).home == self.graph.entity(class).context().scope
                    && self.graph.inherited_constructors.source(function).is_some())
                .then(|| self.graph.entity(function).initialization().constructor)
                .flatten()
            })
            .collect();
        while let Some(id) = work.pop() {
            for child in self.graph.constructors.get(id).subobjects.iter().flatten() {
                if self
                    .graph
                    .constructors
                    .get(child.constructor)
                    .declaration
                    .is_none()
                {
                    work.push(child.constructor);
                }
            }
            self.graph.constructors.patch_default_member(id, action);
        }
    }
    fn record_inherited_constructor(
        &mut self,
        function: EntityId,
        source: BindingId,
    ) -> Result<(), Error> {
        // C++ record_inherited_constructor: retain the original declaring
        // constructor even through shortened/default-argument forwarding forms.
        let parent = self.graph.binding(source).entity;
        let root = self
            .graph
            .inherited_constructors
            .sources
            .get(&parent)
            .map_or(parent, |record| record.root);
        let own = self.graph.lower_entity_type(function)?;
        let declared = self.graph.lower_entity_type(root)?;
        let (
            TypeKind::Function {
                parameters,
                variadic,
                ..
            },
            TypeKind::Function {
                parameters: root_parameters,
                variadic: root_variadic,
                ..
            },
        ) = (
            &self.graph.types.get(own).kind,
            &self.graph.types.get(declared).kind,
        )
        else {
            return Err(Error::Invariant("inherited constructor signature"));
        };
        let full = parameters.len() == root_parameters.len() && variadic == root_variadic;
        self.graph
            .inherited_constructors
            .sources
            .insert(function, Inheritance { source, root, full });
        Ok(())
    }
    pub(super) fn note_inherited_constructors(
        &mut self,
        scope: ScopeId,
        prefix: ScopeId,
        key: ValueKey,
        target: ast::Name,
    ) -> Result<bool, Error> {
        if self.graph.scope(prefix).kind != ScopeKind::Class {
            return Ok(false);
        }
        let base = self
            .graph
            .scope(prefix)
            .owner
            .ok_or(Error::Invariant("base without class"))?;
        let components = self
            .tree
            .name_lists
            .get(self.tree.names.get(target).components);
        let names_owner = self.graph.scope(scope).kind == ScopeKind::Class
            && components.len() >= 2
            && matches!(components[components.len() - 2].kind, ast::NameKind::Identifier(name) if Some(name) == key.identifier());
        if key.identifier() != self.graph.entity(base).name && !names_owner {
            return Ok(false);
        }
        if self.graph.scope(scope).kind == ScopeKind::Class {
            let class = self
                .graph
                .scope(scope)
                .owner
                .ok_or(Error::Invariant("using without class"))?;
            self.graph.inherited_constructors.bases.insert(class, base);
        }
        Ok(true)
    }
    pub(super) fn declare_inherited_constructors(&mut self, class: EntityId) -> Result<(), Error> {
        let Some(&base) = self.graph.inherited_constructors.bases.get(&class) else {
            return Ok(());
        };
        let owner = self
            .graph
            .entity(class)
            .context()
            .scope
            .ok_or(Error::Invariant("inheriting class without scope"))?;
        let least = usize::from(self.graph.entity(class).class().user_constructor.is_none());
        let inherited = self
            .graph
            .entity(base)
            .class()
            .declared_constructors
            .clone();
        for source in inherited.into_iter().rev() {
            let entity = self.graph.binding(source).entity;
            if self.graph.entity(entity).context().template_head.is_some() {
                // The base template retains its own canonical identity. Only a
                // selected specialization creates a derived forwarding entry.
                let binding = self
                    .graph
                    .add_binding(owner, None, self.graph.binding(source))?;
                self.graph.inherit_binding_access(binding, source);
                self.graph
                    .entity_mut(class)
                    .class_mut()
                    .declared_constructors
                    .push(binding);
                continue;
            }
            let ty = self
                .graph
                .binding(source)
                .ty
                .ok_or(Error::Invariant("inherited constructor without type"))?;
            let count = self.graph.types.function_parameter_count(ty)?;
            let mut first = count;
            while first > 0
                && self
                    .graph
                    .entity(entity)
                    .callable()
                    .defaults
                    .get(first - 1)
                    .is_some_and(Option::is_some)
            {
                first -= 1;
            }
            for count in (first.max(least)..=count).rev() {
                if count == 1 {
                    let parameter = self.graph.types.function_parameter(ty, 0)?;
                    if matches!(self.graph.types.get(parameter).kind, TypeKind::Reference {element, ..} if self.graph.class_of(element) == Some(base))
                    {
                        break;
                    }
                }
                let void = self.graph.types.scalar(Scalar::Void)?;
                let parameters = (0..count)
                    .map(|i| self.graph.types.function_parameter(ty, i))
                    .collect::<Result<Vec<_>, _>>()?;
                let signature = self.graph.types.intern(
                    TypeKind::Function {
                        result: void,
                        parameters,
                        variadic: false,
                        cv: Cv::default(),
                        reference: None,
                    },
                    Cv::default(),
                )?;
                if self.graph.constructor(class, signature).is_some() {
                    continue;
                }
                let explicitness = self.graph.entity(entity).callable().explicitness;
                let function = self.graph.add_entity(
                    Some(signature),
                    self.graph.entity(class).name,
                    Keyword::Int,
                )?;
                let seed = !self.function_may_throw(entity)?;
                let availability = self.graph.entity(entity).availability;
                let mut node = self.graph.entity_mut(function);
                node.defined = true;
                node.inline = super::specifiers::InlinePreference::Inline;
                node.callable_mut().definition_emission =
                    super::function_demands::DefinitionEmission::Demanded;
                node.callable_mut().explicitness = explicitness;
                node.availability = availability;
                node.callable_mut().exceptions =
                    super::exceptions::ExceptionState::Value(if seed {
                        super::exceptions::ExceptionSet::None
                    } else {
                        super::exceptions::ExceptionSet::All
                    });
                node.callable_mut().function = Some(constructor_properties());
                let binding = self.graph.add_binding(
                    owner,
                    None,
                    Binding {
                        kind: BindingKind::Function,
                        name: None,
                        entity: function,
                        ty: Some(signature),
                        key: Keyword::Int,
                    },
                )?;
                self.graph.inherit_binding_access(binding, source);
                self.graph.record_member_function(
                    MemberKey {
                        class,
                        kind: MemberKind::Constructor,
                        signature,
                        shape: None,
                    },
                    binding,
                )?;
                self.record_inherited_constructor(function, source)?;
                if availability != super::graph::FunctionAvailability::Deleted {
                    self.planned_inheriting_constructor(binding, class, SourceOffset(0))?;
                }
            }
        }
        Ok(())
    }
    pub(super) fn inherited_constructor_specialization(
        &mut self,
        class: EntityId,
        source: BindingId,
        arguments: usize,
        location: SourceOffset,
    ) -> Result<BindingId, Error> {
        let function = self.graph.binding(source).entity;
        let Some(member) = self.graph.entity(function).callable().member_function else {
            return Ok(source);
        };
        if member.class == class || self.graph.function_templates.primary(function).is_none() {
            return Ok(source);
        }
        self.check_stack()?;
        let base = *self
            .graph
            .inherited_constructors
            .bases
            .get(&class)
            .ok_or(Error::Invariant("inherited template without named base"))?;
        let forwarded = if member.class == base {
            source
        } else {
            self.inherited_constructor_specialization(base, source, arguments, location)?
        };
        let ty = self
            .graph
            .lower_entity_type(self.graph.binding(forwarded).entity)?;
        let count = arguments.min(self.graph.types.function_parameter_count(ty)?);
        let key = InstanceKey {
            class,
            source: function,
            count,
        };
        if let Some(Instance::Ready(binding)) = self.graph.inherited_constructors.instance(key) {
            return Ok(binding);
        }
        let TypeKind::Function { variadic, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("inherited template without function type"));
        };
        let parameters = (0..count)
            .map(|index| self.graph.types.function_parameter(ty, index))
            .collect::<Result<Vec<_>, _>>()?;
        let void = self.graph.types.scalar(Scalar::Void)?;
        let signature = self.graph.types.intern(
            TypeKind::Function {
                result: void,
                parameters,
                variadic,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        let owner = self
            .graph
            .entity(class)
            .context()
            .scope
            .ok_or(Error::Invariant("inherited template without owner"))?;
        let source_function = self.graph.binding(forwarded).entity;
        let explicitness = self.graph.entity(source_function).callable().explicitness;
        let inherited =
            self.graph
                .add_entity(Some(signature), self.graph.entity(class).name, Keyword::Int)?;
        let seed = !self.function_may_throw(source_function)?;
        let availability = self.graph.entity(source_function).availability;
        let mut node = self.graph.entity_mut(inherited);
        node.home = Some(owner);
        node.defined = true;
        node.inline = super::specifiers::InlinePreference::Inline;
        node.callable_mut().definition_emission =
            super::function_demands::DefinitionEmission::Demanded;
        node.callable_mut().function = Some(constructor_properties());
        node.availability = availability;
        node.callable_mut().explicitness = explicitness;
        node.callable_mut().exceptions = super::exceptions::ExceptionState::Value(if seed {
            super::exceptions::ExceptionSet::None
        } else {
            super::exceptions::ExceptionSet::All
        });
        node.callable_mut().member_function = Some(MemberKey {
            class,
            kind: MemberKind::Constructor,
            signature,
            shape: None,
        });
        let binding = self.graph.bind_implicit_constructor(inherited)?;
        self.graph.inherit_binding_access(binding, forwarded);
        self.record_inherited_constructor(inherited, forwarded)?;
        // Source publishes a positive identity before planning. Re-entry sees
        // this same function even while its entries are still being filled.
        self.graph
            .inherited_constructors
            .set_instance(key, Instance::Ready(binding));
        if availability != super::graph::FunctionAvailability::Deleted {
            self.planned_inheriting_constructor(binding, class, location)?;
        }
        Ok(binding)
    }
    fn planned_inheriting_constructor(
        &mut self,
        binding: BindingId,
        class: EntityId,
        location: SourceOffset,
    ) -> Result<ConstructorId, Error> {
        let owner = self.graph.analysis.substitution.current().owner;
        let (id, failed) = self.with_substitution_soft(owner, |analyzer| {
            let id = analyzer.inherited_structural_plan(binding, class, location)?;
            let failed = analyzer
                .graph
                .analysis
                .substitution
                .current()
                .failure
                .is_some();
            Ok((id, failed))
        })?;
        let function = self.graph.binding(binding).entity;
        if failed || self.graph.constructors.get(id).deleted {
            self.graph.constructors.mark_default_deleted(id);
            self.graph.entity_mut(function).availability =
                super::graph::FunctionAvailability::Deleted;
        }
        self.graph
            .entity_mut(function)
            .callable_mut()
            .constructor_exceptions =
            super::construction::ConstructorExceptionSource::InheritedPlan {
                constructor: id,
                pending: true,
            };
        Ok(id)
    }
    pub(super) fn inherited_constructor_plan(
        &mut self,
        binding: BindingId,
    ) -> Result<ConstructorId, Error> {
        let function = self.graph.binding(binding).entity;
        if let Some(id) = self.graph.entity(function).initialization().constructor {
            return Ok(id);
        }
        let class = self
            .graph
            .entity(function)
            .callable()
            .member_function
            .ok_or(Error::Invariant("inheriting constructor without class"))?
            .class;
        if self.graph.entity(function).availability == super::graph::FunctionAvailability::Deleted {
            return self.constructor_declaration_plan(binding, class);
        }
        self.planned_inheriting_constructor(binding, class, SourceOffset(0))
    }
    pub(super) fn check_inherited_constructor_access(
        &mut self,
        binding: BindingId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let function = self.graph.binding(binding).entity;
        let Some(source) = self.graph.inherited_constructors.source(function) else {
            return Ok(());
        };
        let class = self
            .graph
            .entity(function)
            .callable()
            .member_function
            .ok_or(Error::Invariant("inheriting constructor without owner"))?
            .class;
        let scope = self
            .graph
            .entity(class)
            .context()
            .scope
            .ok_or(Error::Invariant("inheriting class without scope"))?;
        let Some(&base) = self.graph.inherited_constructors.bases.get(&class) else {
            return Ok(());
        };
        self.check_constructor_function_access(
            self.graph.binding(source).entity,
            base,
            scope,
            location,
            Some(class),
        )
    }
}

impl super::graph::Graph {
    pub(crate) fn lower_positional_parameters(&self, function: EntityId) -> bool {
        self.inherited_constructors.source(function).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::Instance;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{Analyzer, construction::ConstructorState, lookup::Filter, types::TypeKind},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };
    #[test]
    fn inheritance_keeps_signatures_sources_and_demanded_storage_separate() {
        let source = SourceBuffer::new(b"struct B{B(int,int=3);B(const B&);};typedef B Alias;struct D:Alias{using Alias::Alias;int n=7;};struct E:D{using D::D;};struct Unused:B{using B::B;};void f(){E e(1);}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"inherited-identities").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(expressions, tree.expressions.mark());
        let mut class = |name: &[u8]| {
            let name = cursor.intern_name(name).unwrap();
            graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity
        };
        let base = class(b"B");
        let derived = class(b"D");
        let end = class(b"E");
        let unused = class(b"Unused");
        for owner in [derived, end, unused] {
            let declarations = &graph.entity(owner).class().declared_constructors;
            assert_eq!(declarations.len(), 2); // The base copy constructor is excluded.
            let mut counts: Vec<_> = declarations
                .iter()
                .map(|&id| {
                    graph
                        .types
                        .function_parameter_count(graph.binding(id).ty.unwrap())
                        .unwrap()
                })
                .collect();
            counts.sort_unstable();
            assert_eq!(counts, [1, 2]);
            for &binding in declarations {
                let function = graph.binding(binding).entity;
                let record = graph.inherited_constructors.sources[&function];
                assert_eq!(
                    graph
                        .entity(record.root)
                        .callable()
                        .member_function
                        .unwrap()
                        .class,
                    base
                );
                let count = graph
                    .types
                    .function_parameter_count(graph.binding(binding).ty.unwrap())
                    .unwrap();
                assert_eq!(record.full, count == 2);
                assert_eq!(
                    graph
                        .lower_function_in_mode(function, true)
                        .unwrap()
                        .inherited_constructor,
                    (count == 2).then_some(record.root)
                );
                assert!(
                    graph
                        .lower_function(function)
                        .unwrap()
                        .inherited_constructor
                        .is_none()
                );
            }
            assert!(matches!(
                graph.entity(owner).special().default_construction,
                ConstructorState::NotStarted
            ));
        }
        let one_argument = |owner| {
            graph
                .entity(owner)
                .class()
                .declared_constructors
                .iter()
                .copied()
                .find(|&binding| {
                    graph
                        .types
                        .function_parameter_count(graph.binding(binding).ty.unwrap())
                        .unwrap()
                        == 1
                })
                .unwrap()
        };
        let binding = one_argument(end);
        let function = graph.binding(binding).entity;
        let plan = graph
            .constructors
            .get(graph.entity(function).initialization().constructor.unwrap());
        assert_eq!(plan.class, end);
        assert!(plan.bases[0].forwards);
        let source = graph.inherited_constructors.source(function).unwrap();
        assert_eq!(source, one_argument(derived));
        let middle = graph.constructors.get(plan.bases[0].constructor.unwrap());
        assert_eq!(middle.class, derived);
        let crate::syntax::ast::InitializerKind::Assign(expression) =
            middle.members[0].1.unwrap().kind
        else {
            panic!("member initializer shape changed");
        };
        let crate::syntax::ast::InitializerKind::Assign(original_expression) = graph
            .entity(graph.entity(derived).class().members[0])
            .initialization()
            .member_initializer
            .unwrap()
            .kind
        else {
            panic!("original initializer shape changed");
        };
        assert_eq!(expression, original_expression);
        let original = graph
            .inherited_constructors
            .source(graph.binding(source).entity)
            .unwrap();
        assert_eq!(
            graph
                .entity(graph.binding(original).entity)
                .callable()
                .member_function
                .unwrap()
                .class,
            base
        );
        assert!(graph.entity(function).context().scope.is_none());
        assert!(
            graph
                .entity(function)
                .callable()
                .parameter_entities
                .is_empty()
        );
        assert!(matches!(
            graph
                .types
                .get(
                    graph
                        .types
                        .function_parameter(graph.entity(function).ty.unwrap(), 0)
                        .unwrap()
                )
                .kind,
            TypeKind::Scalar(_)
        ));
        assert!(
            graph
                .entity(unused)
                .class()
                .declared_constructors
                .iter()
                .all(|&binding| graph
                    .entity(graph.binding(binding).entity)
                    .context()
                    .scope
                    .is_none())
        );
        assert!(graph.lower_class_layout(unused).is_err());
    }

    #[test]
    fn inherited_exception_demand_consumes_pending_and_retains_seed_on_error() {
        for (operand, fails) in [("false", false), ("missing", true)] {
            let text = format!(
                "struct B{{B(int)noexcept;}};struct M{{M()noexcept({operand})=default;}};struct D:B{{using B::B;M m;}};"
            );
            let source = SourceBuffer::new(text.into_bytes()).unwrap();
            let mut unit = SyntaxUnit::new(
                Preprocessor::with_name(&source, b"inherited-pending-plan").unwrap(),
            );
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let name = cursor.intern_name(b"D").unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let global = analyzer.graph.global;
            analyzer.declarations(declarations, global).unwrap();
            let class = analyzer
                .graph
                .binding(analyzer.graph.own(global, name, Filter::Prefix).unwrap())
                .entity;
            let binding = analyzer.graph.entity(class).class().declared_constructors[0];
            let function = analyzer.graph.binding(binding).entity;
            let id = analyzer
                .graph
                .entity(function)
                .initialization()
                .constructor
                .unwrap();
            assert!(analyzer.graph.function_nothrow_value(function));
            assert!(matches!(
                analyzer
                    .graph
                    .entity(function)
                    .callable()
                    .constructor_exceptions,
                crate::sema::construction::ConstructorExceptionSource::InheritedPlan {
                    pending: true,
                    ..
                }
            ));
            assert!(analyzer.graph.entity(function).context().scope.is_none());
            assert!(analyzer.graph.root_demands.as_slice().is_empty());
            let result = analyzer.function_may_throw(function);
            if fails {
                assert!(result.is_err());
            } else {
                assert!(result.unwrap());
            }
            assert!(matches!(
                analyzer
                    .graph
                    .entity(function)
                    .callable()
                    .constructor_exceptions,
                crate::sema::construction::ConstructorExceptionSource::InheritedPlan {
                    pending: false,
                    ..
                }
            ));
            assert_eq!(analyzer.graph.function_nothrow_value(function), fails);
            assert_eq!(analyzer.function_may_throw(function).unwrap(), !fails);
            assert_eq!(
                analyzer.graph.entity(function).initialization().constructor,
                Some(id)
            );
            assert!(analyzer.graph.entity(function).context().scope.is_none());
            assert!(analyzer.graph.root_demands.as_slice().is_empty());
        }
    }

    #[test]
    fn inherited_instance_keys_retain_class_source_and_forwarded_count() {
        let source = SourceBuffer::new(
            br#"
            struct A { int n; template<class T>constexpr A(T x,int y=5):n(x+y){} };
            struct B:A { using A::A; };
            struct C:A { using A::A; };
            constexpr B b1(3),b2(4),b3(5,6);
            constexpr C c1(7);
            static_assert(b1.n==8 && b2.n==9 && b3.n==11 && c1.n==12, "values");
            int main(){return 0;}
        "#
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"inherited-keys").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let entries: Vec<_> = analyzer
            .graph
            .inherited_constructors
            .specializations
            .as_ref()
            .unwrap()
            .iter()
            .map(|(&key, &value)| (key, value))
            .collect();
        assert_eq!(entries.len(), 3);
        assert_eq!(
            entries
                .iter()
                .map(|(key, _)| key.source)
                .collect::<crate::support::hash::HashSet<_>>()
                .len(),
            1
        );
        assert_eq!(
            entries
                .iter()
                .map(|(key, _)| key.class)
                .collect::<crate::support::hash::HashSet<_>>()
                .len(),
            2
        );
        let mark = analyzer.tree.expressions.mark();
        for (key, value) in entries {
            let Instance::Ready(binding) = value;
            let source = analyzer.graph.function_origin(key.source).unwrap();
            assert_eq!(
                analyzer
                    .inherited_constructor_specialization(
                        key.class,
                        source,
                        key.count,
                        crate::support::source::SourceOffset(0)
                    )
                    .unwrap(),
                binding
            );
            let entity = analyzer
                .graph
                .entity(analyzer.graph.binding(binding).entity);
            assert!(entity.callable().parameter_entities.is_empty());
            assert!(entity.context().scope.is_none());
            assert!(entity.initialization().constructor.is_some());
            let record = analyzer.graph.inherited_constructors.sources
                [&analyzer.graph.binding(binding).entity];
            assert_eq!(record.root, key.source);
            assert_eq!(record.full, key.count == 2);
        }
        assert_eq!(analyzer.tree.expressions.mark(), mark);
        assert_eq!(analyzer.constexpr_frames.depth(), 0);
    }
}
