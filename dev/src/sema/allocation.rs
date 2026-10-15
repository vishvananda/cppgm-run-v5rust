// Translated from sema/analyzer_construction.cpp::allocation/new_deallocation.
// See NOTICE.
//! Allocation uses retain selected functions and original initialization actions.
use super::{
    Error,
    analyzer::Analyzer,
    construction::ConstructorId,
    constructor_initialization::ConstructorArguments,
    expression_facts::{ExprFacts, ValueCategory},
    function_names::{SpecialName, ValueKey},
    graph::{BindingId, BindingKind, EntityId, FunctionAvailability, ScopeId},
    lookup::{Filter, LookupResult},
    procedural_calls::Candidates,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::{Keyword, SimpleToken},
    syntax::{ast, storage::List},
    types::FundamentalType,
};

#[derive(Clone, Copy)]
pub(super) struct Allocation {
    pub function: BindingId,
    pub target: TypeId,
    pub constructor: Option<(ConstructorId, ConstructorArguments)>,
    pub deallocator: Option<BindingId>,
    pub reserved: bool,
}

fn key(keyword: Keyword) -> ValueKey {
    ValueKey::Special(SpecialName::Operator {
        token: SimpleToken::keyword(keyword),
        array: false,
    })
}

impl Analyzer<'_> {
    pub(super) fn allocation_expression(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let node = self.tree.expressions.get(id);
        let ast::ExprKind::New {
            global,
            placement,
            ty,
            initializer,
            ..
        } = node.kind
        else {
            return Err(Error::Invariant("allocation without new syntax"));
        };
        let location = node.location;
        let (target, count) = self.allocation_type(ty, scope)?;
        if count.is_some() || matches!(self.graph.types.get(target).kind, TypeKind::Array { .. }) {
            return Err(Error::Missing("array allocation expressions"));
        }
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Function { .. } | TypeKind::Reference { .. } | TypeKind::Scalar(Scalar::Void)
        ) {
            return Err(Error::at(
                location,
                "a new-expression allocates an object type",
            ));
        }
        let class = self.graph.class_of(target);
        if class.is_some_and(|class| !self.graph.entity(class).complete) {
            return Err(Error::at(
                location,
                "a new-expression allocates an incomplete class type",
            ));
        }
        let tree = self.tree;
        let args = tree.expr_lists.get(placement.unwrap_or_default());
        for &arg in args {
            self.type_call_argument(arg, scope)?;
        }
        self.lookup_location = location;
        let (found, naming) = self.allocation_lookup(global, class, Keyword::New)?;
        if !found
            .binding
            .is_some_and(|binding| self.graph.binding(binding).kind == BindingKind::Function)
        {
            return Err(Error::at(
                location,
                "no allocation function for the new-expression",
            ));
        }
        let set = self.graph.overloads.add(found.functions.as_slice())?;
        let size = self
            .graph
            .types
            .scalar(Scalar::Basic(FundamentalType::UnsignedLongInt))?;
        let mut scratch = std::mem::take(&mut self.call_scratch);
        let selected = (|| {
            self.rank_prefixed_call(
                Candidates::Functions(set),
                args,
                None,
                Some(ExprFacts::new(size, ValueCategory::Prvalue)),
                &mut scratch,
            )?;
            let chosen = self.select_call(location, args.len() + 1, 0, &scratch)?;
            let function = self.graph.binding(chosen.binding).entity;
            if self.graph.entity(function).availability == FunctionAvailability::Deleted {
                return Err(Error::at(location, "use of a deleted function"));
            }
            if let Some(naming) = naming {
                self.check_named_member_access(chosen.binding, naming, naming, scope, location)?;
            }
            self.apply_prefixed_arguments(chosen, args, scope, &scratch)?;
            Ok(chosen.binding)
        })();
        scratch.clear();
        self.call_scratch = scratch;
        let function = selected?;
        self.record_argument_base_paths(args, scope)?;
        self.check_class_argument_slice(function, args, 1, scope)?;
        self.demand_function(function, true);
        let constructor = if let Some(initializer) = initializer {
            if class.is_some() {
                self.initialize_class_object(target, initializer, scope, location)?
            } else {
                self.object_initializer(initializer, scope, target, location)?;
                None
            }
        } else if let Some(class) = class {
            let constructor = self.default_constructor(class, location)?;
            self.validate_constructor_use(constructor, scope, location)?;
            self.validate_const_default(target, constructor, location)?;
            self.demand_constructor_use(constructor);
            Some((constructor, ConstructorArguments::Values(List::default())))
        } else {
            if self.graph.types.get(target).cv.0 & 1 != 0 {
                return Err(Error::at(location, "a const object needs an initializer"));
            }
            None
        };
        let deallocator = if class.is_some() || initializer.is_some() {
            self.allocation_deallocator(
                global,
                class,
                function,
                placement.is_some_and(|args| !args.is_empty()),
                scope,
                location,
            )?
        } else {
            None
        };
        let reserved = self.reserved_placement(function)?;
        if self.unevaluated == 0 {
            self.request_object_layout(target, location);
        }
        self.graph.expression_facts.set_allocation(
            id,
            Allocation {
                function,
                target,
                constructor,
                deallocator,
                reserved,
            },
        );
        let pointer = self.graph.types.pointer(target)?;
        self.new_expression_facts(pointer, ValueCategory::Prvalue)
    }

    fn allocation_lookup(
        &mut self,
        global: bool,
        class: Option<EntityId>,
        keyword: Keyword,
    ) -> Result<(LookupResult, Option<EntityId>), Error> {
        let key = key(keyword);
        if !global
            && let Some(class) = class
            && let Some(home) = self.graph.entity(class).scope
        {
            let found = self
                .qualified_result(home, key, Filter::Ordinary)
                .map_err(|error| match error {
                    Error::Diagnostic {
                        message: "ambiguous name",
                        ..
                    } => Error::at(
                        self.lookup_location,
                        if keyword == Keyword::New {
                            "ambiguous class allocation function name"
                        } else {
                            "ambiguous class deallocation function name"
                        },
                    ),
                    other => other,
                })?;
            if found.binding.is_some() {
                return Ok((found, Some(class)));
            }
        }
        Ok((
            self.qualified_result(self.graph.global, key, Filter::Ordinary)?,
            None,
        ))
    }

    fn allocation_deallocator(
        &mut self,
        global: bool,
        class: Option<EntityId>,
        allocation: BindingId,
        placement: bool,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<Option<BindingId>, Error> {
        let (found, naming) = self.allocation_lookup(global, class, Keyword::Delete)?;
        let wanted = self
            .graph
            .binding(allocation)
            .ty
            .ok_or(Error::Invariant("allocation signature"))?;
        let void = self.graph.types.scalar(Scalar::Void)?;
        let pointer = self.graph.types.pointer(void)?;
        let size = self
            .graph
            .types
            .scalar(Scalar::Basic(FundamentalType::UnsignedLongInt))?;
        let mut selected = None;
        for pass in 0..if placement { 1 } else { 2 } {
            for &candidate in found.functions.as_slice() {
                let ty = self
                    .graph
                    .binding(candidate)
                    .ty
                    .ok_or(Error::Invariant("deallocation signature"))?;
                let TypeKind::Function {
                    parameters,
                    variadic,
                    ..
                } = &self.graph.types.get(ty).kind
                else {
                    return Err(Error::Invariant("deallocation function type"));
                };
                let same = !variadic
                    && parameters.first() == Some(&pointer)
                    && if placement {
                        let TypeKind::Function {
                            parameters: wanted,
                            variadic,
                            ..
                        } = &self.graph.types.get(wanted).kind
                        else {
                            return Err(Error::Invariant("allocation function type"));
                        };
                        !variadic
                            && parameters.len() > 1
                            && parameters.len() == wanted.len()
                            && parameters[1..] == wanted[1..]
                    } else {
                        parameters.len() == pass + 1
                            && (pass == 0 || naming.is_some() && parameters[1] == size)
                    };
                if same {
                    selected = Some(candidate);
                    break;
                }
            }
            if selected.is_some() {
                break;
            }
        }
        if let Some(binding) = selected
            && class.is_some()
        {
            if let Some(naming) = naming {
                self.check_named_member_access(binding, naming, naming, scope, location)?;
            }
            self.demand_function(binding, true);
        }
        Ok(selected)
    }

    fn reserved_placement(&self, binding: BindingId) -> Result<bool, Error> {
        let entity = self.graph.entity(self.graph.binding(binding).entity);
        let ty = entity
            .ty
            .ok_or(Error::Invariant("allocation without type"))?;
        let TypeKind::Function { parameters, .. } = &self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("allocation without function type"));
        };
        Ok(entity.home == Some(self.graph.global)
            && parameters.len() == 2
            && matches!(self.graph.types.get(parameters[1]).kind, TypeKind::Pointer(element)
                if matches!(self.graph.types.get(element).kind, TypeKind::Scalar(Scalar::Void)) && self.graph.types.get(element).cv.0 == 0))
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
    fn allocation_uses_original_operands_and_demands_its_storage_layout() {
        let source = SourceBuffer::new(
            b"void* operator new(unsigned long,void*) noexcept;\
            void* operator new(unsigned long,void*,long)=delete;\
            struct A{int n;};struct B{int n;B(int);};int storage;\
            A* a=::new(&storage) A{3};B* b=::new(&storage) B(4);"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"allocation-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        for (index, constructed) in [(5, false), (6, true)] {
            let ast::DeclKind::Simple { declarators, .. } = tree
                .declarations
                .get(tree.decl_lists.get(declarations)[index])
                .kind
            else {
                panic!("pointer declaration");
            };
            let ast::InitializerKind::Assign(id) = tree.init_declarators.get(declarators)[0]
                .initializer
                .unwrap()
                .kind
            else {
                panic!("new initializer");
            };
            let use_ = graph.expression_facts.allocation(id).unwrap();
            assert!(use_.reserved);
            assert!(use_.deallocator.is_none());
            assert_eq!(use_.constructor.is_some(), constructed);
            assert_eq!(
                graph
                    .types
                    .function_parameter_count(graph.binding(use_.function).ty.unwrap())
                    .unwrap(),
                2
            );
            let class = graph.class_of(use_.target).unwrap();
            assert_eq!(graph.lower_class_layout(class).unwrap(), (4, 2));
            let ast::ExprKind::New {
                placement: Some(args),
                ..
            } = tree.expressions.get(id).kind
            else {
                panic!("placement operands");
            };
            let argument = tree.expr_lists.get(args)[0];
            let original = graph.expression_facts.get(argument).unwrap();
            assert_ne!(original.ty, original.conversion.unwrap().target);
            assert_eq!(original.category, ValueCategory::Prvalue);
        }
    }
}
