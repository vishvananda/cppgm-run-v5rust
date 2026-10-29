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
    graph::{BindingId, BindingKind, EntityId, ScopeId},
    lookup::{Filter, LookupResult},
    procedural_calls::Candidates,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::{Keyword, SimpleToken},
    syntax::{ast, storage::List},
    types::FundamentalType,
};

/// Delayed ordinary declarations use their lexical point; instantiations can
/// see later global allocation declarations after definition-time viability.
#[derive(Clone, Copy, Default)]
pub(super) enum GlobalAllocationContext {
    #[default]
    Instantiation,
    Declaration,
}

#[derive(Clone, Copy)]
pub(super) struct Allocation {
    pub function: BindingId,
    pub target: TypeId,
    pub constructor: Option<(ConstructorId, ConstructorArguments)>,
    pub deallocator: Option<BindingId>,
    pub reserved: bool,
    pub array: Option<ArrayAllocation>,
    pub sized_array_delete: bool,
}

/// The first dimension stays a checked expression; only its listed prefix
/// has a fixed array type. Neither the tree nor a runtime-sized type is copied.
#[derive(Clone, Copy)]
pub(crate) struct ArrayAllocation {
    pub count: ast::ExprId,
    pub constant: Option<u64>,
    pub prefix: Option<TypeId>,
}

#[derive(Clone, Copy)]
pub(super) struct Deletion {
    pub function: BindingId,
    pub target: TypeId,
    pub incomplete: bool,
    pub sized_array_delete: bool,
}

#[derive(Clone, Copy)]
pub(crate) enum AllocationBuiltin {
    New,
    Delete,
}

pub(super) fn key(keyword: Keyword, array: bool) -> ValueKey {
    ValueKey::Special(SpecialName::Operator {
        token: SimpleToken::keyword(keyword),
        array,
    })
}

impl Analyzer<'_> {
    // analyzer_builtins.cpp::allocation_builtin selects the ordinary global
    // operator family, without class lookup or argument-dependent lookup.
    pub(super) fn allocation_builtin(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        operation: AllocationBuiltin,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let buffer = super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        for &argument in buffer.as_slice() {
            self.typed_expression(argument, scope)?;
        }
        let keyword = match operation {
            AllocationBuiltin::New => Keyword::New,
            AllocationBuiltin::Delete => Keyword::Delete,
        };
        // The frozen direct name-index query sees the global family at the
        // point of instantiation, including declarations after the pattern.
        let family: super::lookup::FunctionCandidates = self
            .graph
            .associated_functions_at(self.graph.global, key(keyword, false), None)
            .collect();
        let bindings = family.as_slice();
        let first = *bindings
            .first()
            .ok_or(Error::Invariant("missing global allocation family"))?;
        let facts = self.function_designation(first, bindings)?;
        let functions = facts
            .functions(&self.graph.expression_facts)
            .ok_or(Error::Invariant("allocation overload set"))?;
        let ast::ExprKind::Call { callee, .. } = self.tree.expressions.get(id).kind else {
            return Err(Error::Invariant("allocation builtin without call syntax"));
        };
        // Record the designation on the original callee. Parentheses share its
        // selected facts; no syntax is synthesized or rewritten for lowering.
        let mut named = callee;
        while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(named).kind {
            named = inner;
        }
        self.graph.expression_facts.set(named, facts);
        let mut parenthesized = callee;
        while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(parenthesized).kind {
            self.graph.expression_facts.share(parenthesized, named);
            parenthesized = inner;
        }
        let mut scratch = std::mem::take(&mut self.call_scratch);
        let selected = self.resolve_call(
            self.tree.expressions.get(id).location,
            Candidates::Functions(functions),
            buffer.as_slice(),
            None,
            &mut scratch,
        );
        let result = selected
            .and_then(|selected| self.finish_selected_call(id, facts, scope, selected, &scratch));
        scratch.clear();
        self.call_scratch = scratch;
        if result.is_ok() && named != callee {
            self.graph.expression_facts.note_cast_call_view(id);
        }
        result
    }

    fn initialize_allocated_class(
        &mut self,
        target: TypeId,
        initializer: ast::Initializer,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<Option<(ConstructorId, ConstructorArguments)>, Error> {
        let constructor = self.initialize_class_object(target, initializer, scope, location)?;
        if constructor.is_some() || !self.lowering() {
            return Ok(constructor);
        }
        let Some(source) = self.single_initializer(initializer) else {
            return Ok(None);
        };
        let facts = self.graph.expression_facts.get(source)?;
        let Some(selected) = facts.class_copy(&self.graph.expression_facts) else {
            return Ok(None);
        };
        if facts.category == ValueCategory::Prvalue {
            return Ok(None);
        }
        let class = self
            .graph
            .class_of(target)
            .ok_or(Error::Invariant("allocated copy without class"))?;
        // allocation_copy_ makes the selected glvalue copy callable even when
        // ordinary local initialization can transfer storage directly. Publish
        // that plan here, before lowering borrows the immutable semantic graph.
        let constructor = self.copy_constructor_plan(class, selected)?;
        self.demand_constructor_use(constructor)?;
        // initialize_object returns this constructor as the allocated object's
        // action. It is no longer a conversion constructing a temporary from
        // the source operand; its reference binding and copy selection remain.
        self.graph.expression_facts.set_copy_action(source, None);
        Ok(Some((constructor, ConstructorArguments::Single(source))))
    }

    pub(super) fn allocation_may_throw(
        &mut self,
        id: ast::ExprId,
        next: u16,
    ) -> Result<bool, Error> {
        let ast::ExprKind::New {
            placement,
            initializer,
            ..
        } = self.tree.expressions.get(id).kind
        else {
            return Err(Error::Invariant("allocation throw query without syntax"));
        };
        let use_ = self.graph.expression_facts.allocation(id)?;
        if self.function_may_throw(self.graph.binding(use_.function).entity)? {
            return Ok(true);
        }
        if let Some((constructor, _)) = use_.constructor {
            let plan = self.graph.constructors.get(constructor);
            let throws = if let Some(binding) = plan.binding {
                self.function_may_throw(self.graph.binding(binding).entity)?
            } else {
                !plan.nothrow
            };
            if throws {
                return Ok(true);
            }
        }
        if let Some(initializer) = initializer {
            match initializer.kind {
                ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                    self.throw_work.push((id, next));
                }
                ast::InitializerKind::Parenthesized(args) => self.throw_work.extend(
                    self.tree
                        .expr_lists
                        .get(args)
                        .iter()
                        .rev()
                        .map(|&id| (id, next)),
                ),
                _ => (),
            }
        }
        self.throw_work.extend(
            self.tree
                .expr_lists
                .get(placement.unwrap_or_default())
                .iter()
                .rev()
                .map(|&id| (id, next)),
        );
        Ok(false)
    }

    pub(super) fn deletion_may_throw(&mut self, id: ast::ExprId) -> Result<bool, Error> {
        let use_ = self.graph.expression_facts.deletion(id)?;
        if self.function_may_throw(self.graph.binding(use_.function).entity)? {
            return Ok(true);
        }
        if let Some(class) = self.graph.class_of(use_.target)
            && let Some(binding) = self.graph.destructor_binding(class)
        {
            return self.function_may_throw(self.graph.binding(binding).entity);
        }
        Ok(false)
    }

    pub(super) fn deletion_expression(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let node = self.tree.expressions.get(id);
        let ast::ExprKind::Delete {
            global,
            array,
            operand,
        } = node.kind
        else {
            return Err(Error::Invariant("deletion without delete syntax"));
        };
        let location = node.location;
        self.typed_expression(operand, scope)?;
        self.check_resolved_function(operand)?;
        let facts = self.convert_unique_class_operand(
            operand,
            super::conversion_functions::ConversionTarget::ObjectPointer,
            scope,
        )?;
        if self.graph.class_of(facts.ty).is_some() {
            return Err(Error::at(
                location,
                "a delete-expression's operand is not a pointer",
            ));
        }
        let pointer = self.decayed_type(facts.ty)?;
        let target = match self.graph.types.get(pointer).kind {
            TypeKind::Pointer(target)
                if !matches!(
                    self.graph.types.get(target).kind,
                    TypeKind::Function { .. } | TypeKind::Scalar(Scalar::Void)
                ) =>
            {
                target
            }
            _ => {
                return Err(Error::at(
                    location,
                    "a delete-expression's operand is not a pointer to an object",
                ));
            }
        };
        self.convert_operand_scalar(operand, pointer, false)?;
        let mut element = target;
        while array
            && let TypeKind::Array { element: inner, .. } = self.graph.types.get(element).kind
        {
            element = inner;
        }
        let class = self.graph.class_of(element);
        let incomplete = if let Some(class) = class {
            !self.complete_class(class)?
        } else {
            false
        };
        let class = class.filter(|_| !incomplete);
        if let Some(class) = class {
            self.check_class_destruction(element, scope, location)?;
            if !self.graph.destructor_trivial(class)?
                && let Some(binding) = self.destructor_callable(class)?
            {
                self.demand_function(binding, true)?;
            }
        }
        self.lookup_location = location;
        let (found, naming) = self.allocation_lookup((global, array), class, Keyword::Delete)?;
        if !found
            .binding
            .is_some_and(|binding| self.graph.binding(binding).kind == BindingKind::Function)
        {
            return Err(Error::at(
                location,
                "no deallocation function for the delete-expression",
            ));
        }
        let function = self
            .usual_deallocation(found.functions.as_slice())?
            .ok_or(Error::at(
                location,
                "no usual deallocation function for the delete-expression",
            ))?;
        self.check_function_not_deleted(self.graph.binding(function).entity, location)?;
        if let Some(naming) = naming
            && let Some(declaration) =
                self.selected_function_declaration(function, found.functions.as_slice())
        {
            self.check_named_member_access(declaration, naming, naming, scope, location)?;
        }
        self.demand_function(function, true)?;
        if !incomplete && self.unevaluated == 0 {
            self.request_object_layout(target, location);
        }
        let sized_array_delete = array && self.sized_array_deallocation(class)?;
        self.graph.expression_facts.set_deletion(
            id,
            Deletion {
                function,
                target,
                incomplete,
                sized_array_delete,
            },
        );
        let void = self.graph.types.scalar(Scalar::Void)?;
        self.new_expression_facts(void, ValueCategory::Prvalue)
    }

    pub(super) fn usual_deallocation(
        &mut self,
        functions: &[BindingId],
    ) -> Result<Option<BindingId>, Error> {
        let void = self.graph.types.scalar(Scalar::Void)?;
        let pointer = self.graph.types.pointer(void)?;
        let size = self
            .graph
            .types
            .scalar(Scalar::Basic(FundamentalType::UnsignedLongInt))?;
        // 3.7.4.2p2, 12.5p4: prefer the one-parameter function, then a
        // class member whose second parameter is std::size_t. Variadicness
        // is deliberately not a filter in the frozen usual-deallocation owner.
        for pass in 0..2 {
            for &binding in functions {
                let function = self.graph.entity(self.graph.binding(binding).entity);
                let ty = function
                    .ty
                    .ok_or(Error::Invariant("deallocation without type"))?;
                let TypeKind::Function { parameters, .. } = &self.graph.types.get(ty).kind else {
                    return Err(Error::Invariant("deallocation function type"));
                };
                if parameters.len() == pass + 1
                    && parameters[0] == pointer
                    && (pass == 0
                        || function.home.is_some_and(|home| {
                            self.graph.scope(home).kind == super::graph::ScopeKind::Class
                        }) && parameters[1] == size)
                {
                    return Ok(Some(binding));
                }
            }
        }
        Ok(None)
    }

    pub(super) fn allocation_expression(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let uses = self.dependent_uses.get();
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
        let count = self.allocation_bound(ty, location)?;
        let constant = count
            .map(|count| self.check_allocation_count(count, scope))
            .transpose()?
            .flatten();
        let (target, count) = if let Some(target) =
            self.allocated_placeholder(ty, initializer, count.is_some(), scope, location)?
        {
            (target, count)
        } else {
            self.allocation_type(ty, scope)?
        };
        if count.is_none() && matches!(self.graph.types.get(target).kind, TypeKind::Array { .. }) {
            return Err(Error::at(
                location,
                "array new-expressions are not supported here",
            ));
        }
        let mut element = target;
        while let TypeKind::Array { element: inner, .. } = self.graph.types.get(element).kind {
            element = inner;
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
        let class = self.graph.class_of(element);
        if let Some(class) = class
            && !self.complete_class(class)?
        {
            return Err(Error::at(
                location,
                "a new-expression allocates an incomplete class type",
            ));
        }
        self.check_abstract_object(element, location)?;
        if let (Some(count), Some(value)) = (count, constant) {
            let (bytes, _) = self.type_size(target, location)?;
            if bytes != 0 && value > i64::MAX.cast_unsigned() / bytes {
                return Err(Error::at(
                    self.tree.expressions.get(count).location,
                    "an array new-expression whose size exceeds PTRDIFF_MAX",
                ));
            }
        }
        let tree = &*self.tree;
        let argument_buffer = super::call_arguments::Arguments::new(
            tree.expr_lists.get(placement.unwrap_or_default()),
        );
        let args = argument_buffer.as_slice();
        for &arg in args {
            // Placement arguments are expressions, unlike a call's argument
            // list: analyzer_construction.cpp types each before resolution.
            self.typed_expression(arg, scope)?;
        }
        self.lookup_location = location;
        let bound = self.graph.lookup_limit.is_some() && self.dependent_uses.get() == uses;
        let size = self
            .graph
            .types
            .scalar(Scalar::Basic(FundamentalType::UnsignedLongInt))?;
        let (found, naming) =
            self.new_allocation_lookup((global, count.is_some()), class, args, size, bound)?;
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
            self.check_function_not_deleted(function, location)?;
            if let Some(naming) = naming
                && let Some(declaration) =
                    self.selected_function_declaration(chosen.binding, found.functions.as_slice())
            {
                self.check_named_member_access(declaration, naming, naming, scope, location)?;
            }
            self.apply_prefixed_arguments(chosen, args, scope, &scratch)?;
            Ok(chosen.binding)
        })();
        scratch.clear();
        self.call_scratch = scratch;
        let function = selected?;
        self.demand_specialization_body(function);
        self.demand_function(function, true)?;
        self.note_throwing_function(self.graph.binding(function).entity);
        let array = if let Some(count) = count {
            Some(self.initialize_array_allocation(
                count,
                constant,
                target,
                initializer,
                scope,
                location,
            )?)
        } else {
            None
        };
        let constructor = if let Some(array) = array {
            self.array_allocation_constructor(array, constant, element, scope, location)?
        } else if let Some(initializer) = initializer {
            if class.is_some() {
                self.initialize_allocated_class(target, initializer, scope, location)?
            } else {
                self.object_initializer(initializer, scope, target, location)?;
                None
            }
        } else if let Some(class) = class {
            let constructor = self.default_constructor(class, location)?;
            self.validate_constructor_use(constructor, class, scope, location)?;
            self.validate_const_default(target, constructor, location)?;
            self.demand_constructor_use(constructor)?;
            self.note_throwing_constructor(constructor);
            Some((constructor, ConstructorArguments::Values(List::default())))
        } else {
            if self.graph.types.get(target).cv.0 & 1 != 0 {
                return Err(Error::at(location, "a const object needs an initializer"));
            }
            None
        };
        let needs_deallocator = if let Some(array) = array {
            class.is_some() || array.prefix.is_some_and(|prefix| matches!(self.graph.types.get(prefix).kind, TypeKind::Array { bound, .. } if bound != 0))
        } else {
            class.is_some() || initializer.is_some()
        };
        let deallocator = if needs_deallocator {
            self.allocation_deallocator(
                (global, array.is_some()),
                class,
                function,
                placement.is_some_and(|args| !args.is_empty()),
                scope,
                location,
            )?
        } else {
            None
        };
        if constructor.is_none()
            && self.unevaluated == 0
            && let Some(class) = class
        {
            // The aggregate initializer consumes destruction properties for
            // completed subobjects. Preparing their callable identities does
            // not require access to the allocated complete object's destructor.
            self.prepare_destruction(class)?;
        }
        let reserved = self.reserved_placement(function)?;
        if self.unevaluated == 0 {
            self.request_object_layout(target, location);
        }
        let sized_array_delete = array.is_some() && self.sized_array_deallocation(class)?;
        self.graph.expression_facts.set_allocation(
            id,
            Allocation {
                function,
                target,
                constructor,
                deallocator,
                reserved,
                array,
                sized_array_delete,
            },
        );
        let pointer = self.graph.types.pointer(target)?;
        self.new_expression_facts(pointer, ValueCategory::Prvalue)
    }

    fn allocation_lookup(
        &mut self,
        (global, array): (bool, bool),
        class: Option<EntityId>,
        keyword: Keyword,
    ) -> Result<(LookupResult, Option<EntityId>), Error> {
        let key = key(keyword, array);
        if !global
            && let Some(class) = class
            && let Some(home) = self.graph.entity(class).context().scope
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
        let saved_limit = self.graph.lookup_limit;
        if matches!(
            self.global_allocation_context,
            GlobalAllocationContext::Instantiation
        ) {
            self.graph.lookup_limit = None;
        }
        let found = self.qualified_result(self.graph.global, key, Filter::Ordinary);
        self.graph.lookup_limit = saved_limit;
        Ok((found?, None))
    }

    fn new_allocation_lookup(
        &mut self,
        form: (bool, bool),
        class: Option<EntityId>,
        args: &[ast::ExprId],
        size: TypeId,
        bound: bool,
    ) -> Result<(LookupResult, Option<EntityId>), Error> {
        let location = self.lookup_location;
        let (found, naming) = self.allocation_lookup(form, class, Keyword::New)?;
        if !bound || naming.is_some() {
            return Ok((found, naming));
        }
        // A nondependent call must have a viable global candidate in the
        // definition context, before selection at the instantiation point.
        // Initializer dependence does not participate in this decision.
        let declared = self.qualified_result(
            self.graph.global,
            key(Keyword::New, form.1),
            Filter::Ordinary,
        )?;
        if !declared
            .binding
            .is_some_and(|binding| self.graph.binding(binding).kind == BindingKind::Function)
        {
            return Err(Error::at(
                location,
                "no allocation function for the new-expression",
            ));
        }
        let set = self.graph.overloads.add(declared.functions.as_slice())?;
        let mut scratch = std::mem::take(&mut self.call_scratch);
        let viable = (|| {
            self.rank_prefixed_call(
                Candidates::Functions(set),
                args,
                None,
                Some(ExprFacts::new(size, ValueCategory::Prvalue)),
                &mut scratch,
            )?;
            self.select_call(location, args.len() + 1, 0, &scratch)
        })();
        scratch.clear();
        self.call_scratch = scratch;
        viable?;
        Ok((found, naming))
    }

    fn sized_array_deallocation(&mut self, class: Option<EntityId>) -> Result<bool, Error> {
        let Some(home) = class.and_then(|class| self.graph.entity(class).context().scope) else {
            return Ok(false);
        };
        let found = match self.qualified_result(home, key(Keyword::Delete, true), Filter::Ordinary)
        {
            Ok(found) => found,
            Err(Error::Diagnostic {
                message: "ambiguous name",
                ..
            }) => return Ok(false),
            Err(error) => return Err(error),
        };
        let Some(function) = self.usual_deallocation(found.functions.as_slice())? else {
            return Ok(false);
        };
        let ty = self
            .graph
            .binding(function)
            .ty
            .ok_or(Error::Invariant("array deallocator type"))?;
        Ok(
            matches!(&self.graph.types.get(ty).kind, TypeKind::Function { parameters, .. } if parameters.len() == 2),
        )
    }

    fn allocation_deallocator(
        &mut self,
        form: (bool, bool),
        class: Option<EntityId>,
        allocation: BindingId,
        placement: bool,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<Option<BindingId>, Error> {
        let (found, naming) = self.allocation_lookup(form, class, Keyword::Delete)?;
        let selected = if placement {
            self.placement_deallocation(found.functions.as_slice(), allocation)?
        } else {
            self.usual_deallocation(found.functions.as_slice())?
        };
        if let Some(binding) = selected
            && class.is_some()
        {
            if let Some(naming) = naming
                && let Some(declaration) =
                    self.selected_function_declaration(binding, found.functions.as_slice())
            {
                self.check_named_member_access(declaration, naming, naming, scope, location)?;
            }
            self.demand_function(binding, true)?;
        }
        Ok(selected)
    }

    fn placement_deallocation(
        &mut self,
        functions: &[BindingId],
        allocation: BindingId,
    ) -> Result<Option<BindingId>, Error> {
        let wanted = self
            .graph
            .binding(allocation)
            .ty
            .ok_or(Error::Invariant("allocation signature"))?;
        let TypeKind::Function {
            parameters: wanted,
            variadic: wanted_variadic,
            ..
        } = &self.graph.types.get(wanted).kind
        else {
            return Err(Error::Invariant("allocation function type"));
        };
        if *wanted_variadic {
            return Ok(None);
        }
        for &candidate in functions {
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
            if *variadic
                || parameters.len() <= 1
                || parameters.len() != wanted.len()
                || parameters[1..] != wanted[1..]
            {
                continue;
            }
            if let TypeKind::Pointer(element) = self.graph.types.get(parameters[0]).kind
                && matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Scalar(Scalar::Void)
                )
                && self.graph.types.get(element).cv.0 == 0
            {
                return Ok(Some(candidate));
            }
        }
        Ok(None)
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
    fn builtin_calls_retain_selected_operators_without_rewriting_or_unrelated_demand() {
        let source = SourceBuffer::new(
            b"template<class T>void*operator new(unsigned long,T*){T::missing();}\
              void*operator new(unsigned long,void*)noexcept;\
              void operator delete(void*,void*)noexcept;struct Unused{int n;};\
              void f(void*p){((__builtin_operator_new))(4,p);((__builtin_operator_delete))(p,p);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"allocation-builtins").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let bodies = tree.bodies.mark();
        let declaration_count = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), declaration_count);
        assert_eq!(tree.bodies.mark(), bodies);
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[4])
            .kind
        else {
            panic!("function syntax");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("parsed function");
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("function compound");
        };
        for &statement in tree.stmt_lists.get(statements) {
            let ast::StmtKind::Expression(Some(id)) = tree.statements.get(statement).kind else {
                panic!("builtin call statement");
            };
            let ast::ExprKind::Call { callee, arguments } = tree.expressions.get(id).kind else {
                panic!("builtin call syntax");
            };
            let Some(super::super::function_resolution::CallTarget::Direct(binding)) = graph
                .expression_facts
                .get(id)
                .unwrap()
                .call(&graph.expression_facts)
            else {
                panic!("direct operator call");
            };
            let selected = graph.entity(graph.binding(binding).entity);
            assert!(selected.callable().builtin.is_none());
            assert_eq!(selected.home, Some(graph.global));
            assert_eq!(
                graph
                    .expression_facts
                    .get(callee)
                    .unwrap()
                    .function(&graph.expression_facts),
                Some(binding)
            );
            assert!(matches!(
                selected.callable().exceptions,
                super::super::exceptions::ExceptionState::Value(
                    super::super::exceptions::ExceptionSet::None
                )
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
        }
        let unused = cursor.intern_name(b"Unused").unwrap();
        let unused = graph
            .binding(graph.own(graph.global, unused, Filter::Elaborated).unwrap())
            .entity;
        assert!(matches!(
            graph.entity(unused).class().layout,
            super::super::layout::LayoutState::NotStarted
        ));
        let template = graph
            .functions_at(graph.global, key(Keyword::New, false))
            .find(|&binding| {
                graph
                    .entity(graph.binding(binding).entity)
                    .context()
                    .template_head
                    .is_some()
            })
            .unwrap();
        assert!(
            !graph
                .function_templates
                .body_demanded(graph.binding(template).entity)
        );
    }

    #[test]
    fn rejected_builtin_call_returns_overload_scratch_after_nested_arguments() {
        let source = SourceBuffer::new(
            b"void f(){__builtin_operator_delete(__builtin_operator_new(4),42);}".to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"allocation-failure").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        assert!(matches!(
            analyzer.declarations(declarations, global),
            Err(Error::UnconditionalDiagnostic {
                message: "no viable function for the call",
                ..
            })
        ));
        assert!(!analyzer.call_scratch.has_candidates());
        assert_eq!(analyzer.unevaluated, 0);
        assert!(analyzer.pretyped_callable.is_none());
    }

    #[test]
    fn allocated_reference_member_materializes_only_its_reached_temporary() {
        let source = SourceBuffer::new(
            b"struct T{~T();};struct A{const T&t;};struct Unused{};A*f(){return new A{T()};}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"allocated-reference").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        for name in [b"T".as_slice(), b"A"] {
            let name = cursor.intern_name(name).unwrap();
            let class = graph
                .binding(graph.own(graph.global, name, Filter::Elaborated).unwrap())
                .entity;
            assert!(graph.lower_class_layout(class).is_ok(), "{name:?}");
        }
        let name = cursor.intern_name(b"Unused").unwrap();
        let unused = graph
            .binding(graph.own(graph.global, name, Filter::Elaborated).unwrap())
            .entity;
        assert!(matches!(
            graph.entity(unused).class().layout,
            super::super::layout::LayoutState::NotStarted
        ));
    }

    #[test]
    fn deletion_retains_selected_functions_and_original_pointer_ownership() {
        let source = SourceBuffer::new(
            b"struct A{~A();static void operator delete(void*,unsigned long) noexcept;};\
              struct I;struct Unused{~Unused();};\
              void f(const A*p,I*q){delete ((p));::delete p;delete q;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"deletion-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let unused = cursor.intern_name(b"Unused").unwrap();
        let expressions = tree.expressions.mark();
        let declaration_count = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), declaration_count);
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[3])
            .kind
        else {
            panic!("function syntax");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("parsed function");
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("function compound");
        };
        let mut selected = Vec::new();
        for (index, &statement) in tree.stmt_lists.get(statements).iter().enumerate() {
            let ast::StmtKind::Expression(Some(id)) = tree.statements.get(statement).kind else {
                panic!("delete statement");
            };
            let use_ = graph.expression_facts.deletion(id).unwrap();
            assert_eq!(use_.incomplete, index == 2);
            let view = graph.lower_deletion(id).unwrap();
            assert_eq!(view.function, graph.binding(use_.function).entity);
            let count = graph
                .types
                .function_parameter_count(graph.binding(use_.function).ty.unwrap())
                .unwrap();
            assert_eq!(count, if index == 0 { 2 } else { 1 });
            let ast::ExprKind::Delete { operand, .. } = tree.expressions.get(id).kind else {
                panic!("delete operand");
            };
            let facts = graph.expression_facts.get(operand).unwrap();
            assert_eq!(facts.category, ValueCategory::Lvalue);
            assert!(!facts.conversion.unwrap().initialization);
            assert!(
                matches!(graph.types.get(facts.ty).kind, TypeKind::Pointer(target) if target == use_.target)
            );
            if index == 0 {
                assert_ne!(operand, graph.expression_facts.root(operand));
            }
            selected.push(view.function);
        }
        assert_ne!(selected[0], selected[1]);
        assert_eq!(selected[1], selected[2]);
        let unused = graph
            .binding(graph.own(graph.global, unused, Filter::Elaborated).unwrap())
            .entity;
        assert!(graph.entity(unused).class().implicit_destructor.is_none());
        assert!(
            graph.entity(unused).special().destruction_preparation
                == super::super::destruction::Preparation::Pending
        );
    }

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
