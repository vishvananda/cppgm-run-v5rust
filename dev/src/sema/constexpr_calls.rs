// Translated from sema/constant_statements.cpp's scalar call path. See NOTICE.
//! Invoke selected functions over their analyzed bodies, never their source text.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, TemporaryValue as Value, nonconstant},
    constants::Constant,
    constexpr_frames::Frame,
    expression_facts::{CallForm, ValueCategory},
    function_resolution::CallTarget,
    graph::{EntityId, FunctionAvailability, ScopeId},
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind},
};

#[derive(Clone, Copy)]
pub(super) enum Returned {
    Value(Value),
    Place(Address),
    Void,
}

/// Written arguments keep their analyzed conversions; forwarded arguments
/// already designate parameter/subobject storage and must not replay syntax.
#[derive(Clone, Copy)]
pub(super) enum ConstantArguments<'a> {
    Written(&'a [ast::ExprId]),
    Bound(&'a [Address]),
    Values(&'a [Value]),
}
impl ConstantArguments<'_> {
    pub(super) fn first_place(
        self,
        analyzer: &mut Analyzer<'_>,
        parameter: TypeId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        match self {
            Self::Values(_) => Err(Error::Invariant(
                "scalar literal arguments used for object copy",
            )),
            Self::Bound(values) => values
                .first()
                .copied()
                .ok_or(Error::Invariant("copy without source")),
            Self::Written(values) => {
                let &expr = values
                    .first()
                    .ok_or(Error::Invariant("copy without source"))?;
                let TypeKind::Reference { element, .. } = analyzer.graph.types.get(parameter).kind
                else {
                    return Err(Error::Invariant("copy source parameter without reference"));
                };
                analyzer.bind_constant_reference(expr, element, scope)
            }
        }
    }
}

impl Analyzer<'_> {
    pub(super) fn check_constexpr_scalar_signature(
        &mut self,
        entity: EntityId,
        ty: TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let function = self.graph.entity(entity);
        if !self.semantic()
            || !function.constexpr
            || function.availability == FunctionAvailability::Deleted
            || function.template_head.is_some()
            || self.graph.function_templates.primary(entity).is_some()
            || function.home.is_some_and(|home| {
                self.templated_class_scope(home) || self.graph.within_class_instance(home)
            })
        {
            return Ok(());
        }
        let constructor = function
            .member_function
            .is_some_and(|key| key.kind == super::graph::MemberKind::Constructor);
        if let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind
            && !constructor
            && !self.constant_literal_type(result)?
            && !(self.graph.cxx14
                && matches!(
                    self.graph.types.get(result).kind,
                    TypeKind::Scalar(super::types::Scalar::Void)
                ))
        {
            return Err(Error::at(
                location,
                "a constexpr function's return type is not a literal type",
            ));
        }
        if !constructor {
            let entity = self.graph.entity(entity);
            if entity.function.is_some_and(|function| !function.is_static)
                && let Some(home) = entity
                    .home
                    .filter(|&home| self.graph.scope(home).kind == super::graph::ScopeKind::Class)
                    .and_then(|home| self.graph.scope(home).owner)
                && !self.constant_literal_type(self.graph.lower_entity_type(home)?)?
            {
                return Err(Error::at(
                    location,
                    "a constexpr member function of a class that is not a literal type",
                ));
            }
        }
        if let TypeKind::Function { ref parameters, .. } = self.graph.types.get(ty).kind {
            let count = parameters.len();
            for index in 0..count {
                let parameter = self.graph.types.function_parameter(ty, index)?;
                if !self.constant_literal_type(parameter)? {
                    return Err(Error::at(
                        location,
                        "a constexpr function's parameter type is not a literal type",
                    ));
                }
            }
        }
        Ok(())
    }
    pub(super) fn constant_step(&mut self, location: SourceOffset) -> Result<(), Error> {
        self.check_stack()?;
        self.constexpr_frames.step(location)
    }
    pub(super) fn constant_operand_value(
        &mut self,
        mut id: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
    ) -> Result<Value, Error> {
        if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind {
            if let Some(&single) = self.tree.expr_lists.get(elements).first() {
                id = single;
            } else {
                return self.zero_constant_scalar(target);
            }
        }
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Scalar(Scalar::Bool)
        ) && self
            .graph
            .expression_facts
            .get(id)
            .is_ok_and(|facts| self.complex_part_type(facts.ty).is_some())
        {
            let truth = self.constant_complex_truth(id, scope)?;
            return Ok(Value::Integer(
                self.integral_value(target, u64::from(truth)),
            ));
        }
        if let TypeKind::Pointer(element) = self.graph.types.get(target).kind {
            let mut value = self.address_operand(id, scope)?;
            value.designated = element;
            return Ok(Value::Pointer(value));
        }
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Scalar(Scalar::Nullptr)
        ) {
            return Ok(Value::Pointer(self.evaluate_address(id, scope)?));
        }
        if matches!(
            self.graph.types.get(target).kind,
            TypeKind::MemberPointer { .. }
        ) {
            return self
                .member_pointer_operand(id, target, scope)
                .map(Value::MemberPointer);
        }
        if self.float_format(target).is_some()
            || self.floating_expression(id)
            || self.member_pointer_expression(id)
        {
            let value = self.scalar_value(id, scope)?;
            return self.convert_constant_scalar(
                value,
                target,
                self.tree.expressions.get(id).location,
            );
        }
        let value = if self.is_address_expression(id) {
            let truth = self.evaluate_address(id, scope)?.root != Root::Null;
            self.integral_value(target, u64::from(truth))
        } else {
            self.evaluate_constant(id, scope, false)?
        };
        let target = self.bare_type(target)?;
        Ok(Value::Integer(
            self.convert_integer_constant(value, target)?,
        ))
    }
    pub(super) fn zero_constant_scalar(&mut self, target: TypeId) -> Result<Value, Error> {
        if self.float_format(target).is_some() {
            return Ok(Value::Floating(super::constant_floats::Floating {
                ty: self.bare_type(target)?,
                value: crate::types::float_value::BinaryFloat::zero(),
            }));
        }
        match self.graph.types.get(target).kind {
            TypeKind::MemberPointer { .. } => Ok(Value::MemberPointer(
                super::symbolic_members::MemberPointer::null(self.bare_type(target)?),
            )),
            TypeKind::Pointer(element) => Ok(Value::Pointer(Address::new(Root::Null, element))),
            TypeKind::Scalar(Scalar::Nullptr) => {
                Ok(Value::Pointer(Address::new(Root::Null, target)))
            }
            _ if self.integral_kind(target).is_some() => Ok(Value::Integer(Constant {
                high: 0,
                bits: 0,
                ty: self.bare_type(target)?,
            })),
            _ => Err(Error::Missing("constexpr aggregate initialization")),
        }
    }
    pub(super) fn bind_constant_reference(
        &mut self,
        id: ast::ExprId,
        element: TypeId,
        scope: ScopeId,
    ) -> Result<Address, Error> {
        self.check_deferred_constant(id)?;
        let root = self.graph.expression_facts.root(id);
        let facts = self.graph.expression_facts.get(root)?;
        let location = self.tree.expressions.get(id).location;
        if let Some(step) = self.graph.expression_facts.function_conversion(id) {
            let returned = self.constant_conversion_result(id, scope)?;
            let mut place = match returned {
                Returned::Place(place) if !step.after.temporary => place,
                Returned::Place(place) if self.graph.class_of(element).is_some() => place,
                _ => {
                    let value = self.returned_scalar(returned, location)?;
                    let value = self.convert_constant_scalar(value, element, location)?;
                    self.allocate_constant_object(element, Some(value), location)?
                }
            };
            place = self.constant_project_path(place, step.after_path, location)?;
            place.designated = element;
            return Ok(place);
        }
        if let Some((constructor, arguments)) = self.graph.expression_facts.class_conversion(id)
            && self.graph.class_of(element) == Some(self.graph.constructors.get(constructor).class)
        {
            let location = self.tree.expressions.get(id).location;
            let class = self.graph.constructors.get(constructor).class;
            let ty = self.graph.lower_entity_type(class)?;
            let place = self.allocate_constant_object(ty, None, location)?;
            let arguments = self.constant_constructor_arguments(arguments);
            self.run_constant_constructor(
                place,
                constructor,
                arguments.as_slice(),
                scope,
                location,
            )?;
            return Ok(place);
        }
        if self.graph.class_of(element).is_some() || self.complex_part_type(element).is_some() {
            let place = self.constant_object_place(id, scope)?;
            let mut place = self.constant_project_path(
                place,
                facts
                    .class_copy
                    .map(|copy| copy.binding)
                    .or(facts.conversion)
                    .filter(|conversion| {
                        conversion.second == super::expression_facts::ValueConversion::DerivedToBase
                    })
                    .and(facts.base_path),
                self.tree.expressions.get(id).location,
            )?;
            place.designated = element;
            return Ok(place);
        }
        if facts.category == ValueCategory::Prvalue
            || facts
                .conversion
                .is_some_and(|conversion| conversion.temporary)
        {
            if matches!(self.graph.types.get(facts.ty).kind, TypeKind::Array { .. })
                && facts.category == ValueCategory::Prvalue
            {
                // bind() obtains the array temporary's value, then its scalar
                // conversion fails. Preserve construction failures first.
                self.constant_object_place(id, scope)?;
                return Err(nonconstant(
                    location,
                    "a conversion outside constant evaluation",
                ));
            }
            let value = self.constant_operand_value(id, element, scope)?;
            return self.allocate_constant_object(
                element,
                Some(value),
                self.tree.expressions.get(id).location,
            );
        }
        let mut place = self.constant_place(id, scope)?;
        place.designated = element;
        Ok(place)
    }
    pub(super) fn interpret_constant_call(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Constant, Error> {
        let location = self.tree.expressions.get(id).location;
        match self.constant_call_result(id, scope)? {
            Returned::Value(Value::Integer(value)) => Ok(value),
            Returned::Place(place) => self.read_constant_integer(place, location),
            Returned::Void => Ok(Constant {
                high: 0,
                bits: 0,
                ty: self.graph.expression_facts.get(id)?.ty,
            }),
            Returned::Value(Value::Pointer(_) | Value::Floating(_) | Value::MemberPointer(_)) => {
                Err(nonconstant(location, "not an integral constant expression"))
            }
        }
    }
    pub(super) fn constant_call_result(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Returned, Error> {
        self.constant_call_into(id, scope, None)
    }
    pub(super) fn constant_call_into(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        destination: Option<Address>,
    ) -> Result<Returned, Error> {
        let id = self.graph.expression_facts.root(id);
        if let Some(kind) = self.graph.expression_facts.source_location(id) {
            return self.constant_source_location(id, kind);
        }
        if let Some(super::runtime_intrinsics::RuntimeIntrinsic::Overflow(operation)) =
            self.graph.expression_facts.runtime_intrinsic(id)
        {
            return self
                .constant_overflow_intrinsic(id, operation, scope)
                .map(|value| Returned::Value(Value::Integer(value)));
        }
        if self.graph.expression_facts.atomic_intrinsic(id).is_some()
            || self.graph.expression_facts.runtime_intrinsic(id).is_some()
        {
            let ast::ExprKind::Call { arguments, .. } = self.tree.expressions.get(id).kind else {
                return Err(Error::Invariant("runtime constant query without call"));
            };
            // The frozen intrinsic evaluator attempts its last operand, then
            // returns false for the non-floating runtime operation without
            // recording a failure. Required initialization owns that fallback.
            if let Some(&last) = self.tree.expr_lists.get(arguments).last() {
                let facts = self.graph.expression_facts.get(last)?;
                let target = facts.conversion.map_or(facts.ty, |step| step.target);
                self.constant_operand_value(last, target, scope)?;
            }
            return Err(Error::NonConstant {
                failure: super::ConstantFailure::Unreported,
                location: self.tree.expressions.get(id).location,
                message: "not an integral constant expression",
            });
        }
        if self.graph.expression_facts.get(id)?.literal_call.is_some() {
            return self.constant_literal_call(id, scope, destination);
        }
        if self.graph.expression_facts.get(id)?.call_form == CallForm::Operator
            && !matches!(self.tree.expressions.get(id).kind, ExprKind::Call { .. })
        {
            return self.constant_operator_into(id, scope, destination);
        }
        let ExprKind::Call { callee, arguments } = self.tree.expressions.get(id).kind else {
            return Err(Error::Missing("constexpr operator and literal invocation"));
        };
        let facts = self.graph.expression_facts.get(id)?;
        let location = self.tree.expressions.get(id).location;
        if matches!(
            facts.destructor,
            Some(super::expression_facts::DestructorTarget::Scalar)
        ) {
            return Err(nonconstant(
                location,
                "a pseudo-destructor call in a constant expression",
            ));
        }
        if facts.dispatch.is_virtual() {
            return Err(nonconstant(
                location,
                "a virtual function call in a constant expression",
            ));
        }
        let bound = if self.graph.lower_bound_member_pointer(callee)? {
            let (pointer, object) = self.constant_member_pointer_object(callee, scope)?;
            let function = pointer
                .member
                .ok_or(Error::Invariant("bound null member constant"))?;
            if self.graph.lower_virtual_slot(function).is_some() {
                return Err(nonconstant(
                    location,
                    "a virtual function call in a constant expression",
                ));
            }
            Some((function, object))
        } else {
            None
        };
        let function = if let Some((function, _)) = bound {
            function
        } else if let Some(CallTarget::Direct(binding)) = facts.call {
            self.graph.binding(binding).entity
        } else {
            let address = self.address_operand(callee, scope)?;
            let Root::Function(entity) = address.root else {
                return Err(nonconstant(
                    location,
                    "a call through a pointer that designates no function",
                ));
            };
            entity
        };
        let receiver = self
            .graph
            .expression_facts
            .callable_receiver(id)
            .or_else(|| {
                let facts = self.graph.expression_facts.get(callee).ok()?;
                Some(super::expression_facts::CallReceiver {
                    object: facts.object?,
                    path: facts.base_path,
                    naming_path: facts.member_naming_path,
                    member_path: facts.member_path,
                })
            })
            .filter(|_| {
                !self
                    .graph
                    .entity(function)
                    .function
                    .is_some_and(|f| f.is_static)
            });
        let object = if let Some((_, object)) = bound {
            Some(object)
        } else if let Some(receiver) = receiver {
            let place = self.constant_receiver_place(receiver.object.source, scope, location)?;
            Some(
                self.constant_project_path(place, receiver.naming_path, location)
                    .and_then(|place| {
                        self.constant_project_path(
                            place,
                            receiver.member_path.or(receiver.path),
                            location,
                        )
                    })?,
            )
        } else {
            let callee = self.graph.expression_facts.root(callee);
            if let ExprKind::Member { .. } = self.tree.expressions.get(callee).kind {
                self.constant_member_object(callee, scope)?;
            }
            None
        };
        // The frozen interpreter evaluates the receiver, then treats a
        // trivial destructor as an inert call, without requiring constexpr.
        if let Some(super::expression_facts::DestructorTarget::Class(binding)) = facts.destructor {
            let class = self
                .graph
                .entity(self.graph.binding(binding).entity)
                .member_function
                .ok_or(Error::Invariant("constant destructor without class"))?
                .class;
            if self.graph.destructor_trivial(class)? {
                return Ok(Returned::Void);
            }
        }
        let arguments = super::call_arguments::Arguments::new(self.tree.expr_lists.get(arguments));
        self.invoke_constant_function_into(
            function,
            (ConstantArguments::Written(arguments.as_slice()), false),
            (object, destination),
            scope,
            location,
        )
    }
    fn constant_operator_into(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
        destination: Option<Address>,
    ) -> Result<Returned, Error> {
        let facts = self.graph.expression_facts.get(id)?;
        let location = self.tree.expressions.get(id).location;
        let Some(CallTarget::Direct(binding)) = facts.call else {
            return Err(Error::Invariant("operator without selected function"));
        };
        let function = self.graph.binding(binding).entity;
        let (operands, count, postfix) = match self.tree.expressions.get(id).kind {
            ExprKind::Binary { left, right, .. } => ([left, right], 2, false),
            ExprKind::Subscript { base, index } => ([base, index], 2, false),
            ExprKind::Unary { operand, .. } => ([operand; 2], 1, false),
            ExprKind::Postfix { operand, .. } => ([operand; 2], 1, true),
            _ => return Err(Error::Invariant("operator call without operands")),
        };
        let member = self.member_function_owner(binding).is_some();
        let object = if member {
            let receiver = operands[0];
            let place = self.constant_object_place(receiver, scope)?;
            let path = self.graph.expression_facts.get(receiver)?.base_path;
            Some(self.constant_project_path(place, path, location)?)
        } else {
            None
        };
        self.invoke_constant_function_into(
            function,
            (
                ConstantArguments::Written(&operands[usize::from(member)..count]),
                postfix,
            ),
            (object, destination),
            scope,
            location,
        )
    }
    pub(super) fn invoke_constant_function(
        &mut self,
        function: EntityId,
        arguments: &[ast::ExprId],
        object: Option<Address>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Returned, Error> {
        self.invoke_constant_function_into(
            function,
            (ConstantArguments::Written(arguments), false),
            (object, None),
            scope,
            location,
        )
    }
    pub(super) fn invoke_constant_function_into(
        &mut self,
        function: EntityId,
        (arguments, postfix): (ConstantArguments<'_>, bool),
        (object, destination): (Option<Address>, Option<Address>),
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Returned, Error> {
        self.constant_step(location)?;
        let entity = self.graph.entity(function);
        if entity.availability == FunctionAvailability::Deleted {
            return Err(nonconstant(location, "a call of a deleted function"));
        }
        if entity.assignment.is_some()
            && let Some(object) = object
        {
            let ty = self.graph.lower_entity_type(function)?;
            let parameter = self.graph.types.function_parameter(ty, 0)?;
            let source = arguments.first_place(self, parameter, scope)?;
            self.constant_assign_subobjects(function, object, source, scope, location)?;
            return Ok(Returned::Place(object));
        }
        if !entity.constexpr {
            return Err(nonconstant(
                location,
                "a call of a function that is not constexpr",
            ));
        }
        if self.constexpr_frames.depth() >= 512 {
            return Err(nonconstant(location, "constexpr calls nested too deeply"));
        }
        if !self.constant_function_definition(function)? {
            return Err(nonconstant(
                location,
                "a constexpr function called before its definition",
            ));
        }
        let ty = self.graph.lower_entity_type(function)?;
        let TypeKind::Function {
            result: returns, ..
        } = self.graph.types.get(ty).kind
        else {
            return Err(Error::Invariant("constant function without function type"));
        };
        let destination = if self.graph.class_of(returns).is_some()
            || self.complex_part_type(returns).is_some()
        {
            Some(if let Some(destination) = destination {
                destination
            } else {
                self.allocate_constant_object(returns, None, location)?
            })
        } else {
            None
        };
        let bound =
            self.bind_constant_inputs(function, ty, (arguments, postfix), scope, location)?;
        let mut parameters = Vec::new();
        for (index, &value) in bound.iter().enumerate() {
            if self
                .graph
                .class_of(self.graph.types.function_parameter(ty, index)?)
                .is_some()
                && let Returned::Place(place) = value
            {
                parameters.push(place);
            }
        }
        let region = self.constexpr_frames.region();
        let result =
            self.run_constant_function(function, (returns, destination), bound, object, location);
        for parameter in parameters {
            self.constexpr_frames.end_lifetime(parameter);
        }
        self.constexpr_frames.release(region);
        if let Ok(Returned::Value(Value::Pointer(value))) = result {
            self.constexpr_frames.valid_pointer(value, location)?;
        }
        result
    }
    fn constant_assign_subobjects(
        &mut self,
        function: EntityId,
        target: Address,
        source: Address,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let plan = self
            .graph
            .lower_assignment(function)
            .ok_or(Error::Invariant("constant assignment without plan"))?;
        if plan.deleted {
            return Err(nonconstant(location, "a call of a deleted function"));
        }
        let count = plan.actions.len();
        for index in 0..count {
            let action = self
                .graph
                .lower_assignment(function)
                .ok_or(Error::Invariant("lost constant assignment plan"))?
                .actions[index];
            if action
                .member
                .is_some_and(|member| self.graph.unnamed_bit_field(member))
            {
                continue;
            }
            let to = self.constant_assigned_subobject(target, action, location)?;
            let from = self.constant_assigned_subobject(source, action, location)?;
            if let Some(child) = action.function.filter(|&child| {
                !self
                    .graph
                    .lower_assignment(child)
                    .is_some_and(|plan| plan.trivial)
            }) {
                self.invoke_constant_function_into(
                    child,
                    (ConstantArguments::Bound(&[from]), false),
                    (Some(to), None),
                    scope,
                    location,
                )?;
            } else {
                self.copy_constant_object(to, from, location)?;
            }
        }
        Ok(())
    }
    fn constant_assigned_subobject(
        &mut self,
        place: Address,
        action: super::assignment::Action,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        if let Some(member) = action.member {
            return self.constant_member_slot(place, member, location);
        }
        let (class, index) = action
            .base
            .ok_or(Error::Invariant("assignment action without subobject"))?;
        self.class_layout(class, location)?;
        let offset = self.graph.entity(class).bases[index as usize]
            .offset
            .ok_or(Error::Invariant("constant assignment base without offset"))?;
        Ok(Address {
            offset: place.offset.wrapping_add(super::integral::signed(offset)),
            designated: self.bare_type(action.ty)?,
            ..place
        })
    }
    pub(super) fn bind_constant_inputs(
        &mut self,
        function: EntityId,
        ty: TypeId,
        (arguments, postfix): (ConstantArguments<'_>, bool),
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Vec<Returned>, Error> {
        let TypeKind::Function { ref parameters, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("parameter binding without function type"));
        };
        let count = parameters.len();
        // Variadic extras are evaluated before the fixed parameters, as in C++.
        if let ConstantArguments::Written(arguments) = arguments {
            for &argument in arguments.iter().skip(count) {
                let facts = self.graph.expression_facts.get(argument)?;
                let ty = facts.ty;
                if self.graph.class_of(ty).is_some() {
                    let place = self.allocate_constant_object(ty, None, location)?;
                    self.initialize_constant_class_from(argument, place, scope)?;
                } else {
                    let target = facts.conversion.map_or(ty, |conversion| conversion.result);
                    let target = self.decayed_type(target)?;
                    self.constant_operand_value(argument, target, scope)?;
                }
            }
        }
        let mut bound = Vec::with_capacity(count);
        for index in 0..count {
            let parameter = self.graph.types.function_parameter(ty, index)?;
            if let ConstantArguments::Bound(places) = arguments
                && let Some(&source) = places.get(index)
            {
                let value = match self.graph.types.get(parameter).kind {
                    TypeKind::Reference { .. } => Returned::Place(source),
                    TypeKind::Class(_) | TypeKind::Complex(_) => {
                        let place = self.allocate_constant_object(parameter, None, location)?;
                        self.copy_constant_object(place, source, location)?;
                        Returned::Place(place)
                    }
                    _ => {
                        let value = self.read_constant_scalar(source, location)?;
                        Returned::Value(self.convert_constant_scalar(value, parameter, location)?)
                    }
                };
                bound.push(value);
                continue;
            }
            if let ConstantArguments::Values(values) = arguments
                && let Some(&value) = values.get(index)
            {
                bound.push(Returned::Value(
                    self.convert_constant_scalar(value, parameter, location)?,
                ));
                continue;
            }
            let written = match arguments {
                ConstantArguments::Written(values) => values,
                ConstantArguments::Bound(_) | ConstantArguments::Values(_) => &[],
            };
            if postfix && index == written.len() {
                bound.push(Returned::Value(self.zero_constant_scalar(parameter)?));
                continue;
            }
            let argument = if let Some(&argument) = written.get(index) {
                Some(argument)
            } else {
                self.constant_default_argument(function, index)?
            }
            .ok_or_else(|| nonconstant(location, "a call without an argument for a parameter"))?;
            let value =
                self.with_source_site((index >= written.len()).then_some(location), |analyzer| {
                    match analyzer.graph.types.get(parameter).kind {
                        TypeKind::Reference { element, .. } => Ok(Returned::Place(
                            analyzer.bind_constant_reference(argument, element, scope)?,
                        )),
                        TypeKind::Class(_) | TypeKind::Complex(_) => {
                            let place =
                                analyzer.allocate_constant_object(parameter, None, location)?;
                            analyzer.initialize_constant_class_from(argument, place, scope)?;
                            Ok(Returned::Place(place))
                        }
                        _ => Ok(Returned::Value(
                            analyzer.constant_operand_value(argument, parameter, scope)?,
                        )),
                    }
                })?;
            bound.push(value);
        }
        Ok(bound)
    }
    fn run_constant_function(
        &mut self,
        function: EntityId,
        (returns, destination): (TypeId, Option<Address>),
        bound: Vec<Returned>,
        object: Option<Address>,
        location: SourceOffset,
    ) -> Result<Returned, Error> {
        let body = self
            .graph
            .lower_function_body(function)
            .ok_or_else(|| nonconstant(location, "a constexpr function without a body"))?;
        let ast::Body::Parsed(body) = *self.tree.bodies.get(body) else {
            return Err(Error::Invariant("unparsed constant function"));
        };
        if body.handlers.is_some() {
            return Err(nonconstant(
                location,
                "a function-try-block in a constant expression",
            ));
        }
        let mut frame = self.constant_function_frame(function, returns, bound, object, location)?;
        frame.destination = destination;
        self.with_constant_frame(frame, |analyzer| {
            analyzer
                .constant_compound(body.compound)
                .and_then(|flow| match flow {
                    super::constexpr_statements::Flow::Return(value) => Ok(value),
                    _ if matches!(
                        analyzer.graph.types.get(returns).kind,
                        TypeKind::Scalar(Scalar::Void)
                    ) =>
                    {
                        Ok(Returned::Void)
                    }
                    _ => Err(nonconstant(
                        location,
                        "a constexpr function flows off its end",
                    )),
                })
        })
    }
    pub(super) fn constant_function_frame(
        &mut self,
        function: EntityId,
        returns: TypeId,
        bound: Vec<Returned>,
        object: Option<Address>,
        location: SourceOffset,
    ) -> Result<Frame, Error> {
        let scope = self
            .graph
            .entity(function)
            .scope
            .ok_or(Error::Invariant("constant function without scope"))?;
        let mut frame = Frame::new(returns, scope);
        frame.source_function = Some(super::predefined_names::SourceFunction::Function(function));
        frame.object = object;
        for (index, value) in bound.into_iter().enumerate() {
            let entity = self
                .graph
                .entity(function)
                .parameter_entities
                .get(index)
                .copied();
            let place = match value {
                Returned::Place(place) => place,
                Returned::Value(value) => {
                    let ty =
                        entity
                            .map(|entity| self.graph.lower_entity_type(entity))
                            .transpose()?
                            .unwrap_or(self.graph.types.function_parameter(
                                self.graph.lower_entity_type(function)?,
                                index,
                            )?);
                    self.allocate_constant_object(ty, Some(value), location)?
                }
                Returned::Void => {
                    return Err(Error::Invariant("void argument to constant function"));
                }
            };
            if let Some(entity) = entity {
                frame.bind(entity, place);
            }
            frame.parameters.push(place);
        }
        Ok(frame)
    }
    pub(super) fn with_constant_frame<T>(
        &mut self,
        frame: Frame,
        execute: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let saved = self
            .constant_evaluation
            .as_mut()
            .ok_or(Error::Invariant("call without evaluation"))?
            .suspend_temporaries();
        self.constexpr_frames.push(frame);
        let result = execute(self);
        self.constexpr_frames.pop();
        self.constant_evaluation
            .as_mut()
            .ok_or(Error::Invariant("call lost evaluation"))?
            .resume_temporaries(saved);
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{analyzer::Analyzer, constants::ConstantState, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{SyntaxUnit, ast},
    };

    #[test]
    fn operators_reuse_syntax_and_release_each_invocation_region() {
        let source = SourceBuffer::new(
            br"
            struct S {
                int n;
                constexpr S(int x): n(x) {}
                constexpr S operator++(int zero) const { return S(n+zero); }
                constexpr const int& operator[](int) const { return n; }
                friend constexpr S operator+(S a, S b) { return S(a.n+b.n); }
            };
            constexpr int f(int n) { S s(n); S old=s++; return (old+2)[0]*10+s[0]; }
            constexpr int a=f(3);
            constexpr int b=f(7);
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"constexpr-operators").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"a".as_slice(), b"b"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        let expressions = analyzer.tree.expressions.mark();
        analyzer.declarations(declarations, global).unwrap();
        for (name, bits) in names.into_iter().zip([53, 97]) {
            let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
            assert!(
                matches!(analyzer.graph.entity(analyzer.graph.binding(binding).entity).constant,
                ConstantState::Value(value) if value.bits == bits)
            );
        }
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert!(!analyzer.constexpr_frames.active());
        assert!(analyzer.constant_evaluation.is_none());
    }

    #[test]
    fn scalar_calls_reuse_body_facts_without_publishing_invocation_values() {
        let source = SourceBuffer::new(
            b"template<class T>constexpr T next(T n){return n+1;}\
            constexpr int sum(int n){int local=n;return n?local+sum(n-1):0;}\
            constexpr int a=next(2);constexpr int b=next(4);\
            constexpr int c=sum(3);constexpr int d=sum(4);            template<class T>struct S{static constexpr int f(int n){return n+sizeof(T);}static const int value=f(1);};constexpr int e=S<int>::value;"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"constexpr-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"a".as_slice(), b"b", b"c", b"d", b"e", b"sum"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        for (&name, bits) in names.iter().zip([3, 5, 6, 10, 5]) {
            let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
            assert!(
                matches!(analyzer.graph.entity(analyzer.graph.binding(binding).entity).constant, ConstantState::Value(value) if value.bits == bits)
            );
        }
        let binding = analyzer
            .graph
            .own(global, names[5], Filter::Ordinary)
            .unwrap();
        let function = analyzer.graph.binding(binding).entity;
        let body_id = analyzer.graph.lower_function_body(function).unwrap();
        let ast::Body::Parsed(body) = *analyzer.tree.bodies.get(body_id) else {
            panic!("parsed body");
        };
        let ast::StmtKind::Compound(statements) = analyzer.tree.statements.get(body.compound).kind
        else {
            panic!("compound body");
        };
        let returned = analyzer.tree.stmt_lists.get(statements)[1];
        let ast::StmtKind::Return(Some(value)) = analyzer.tree.statements.get(returned).kind else {
            panic!("return");
        };
        assert!(analyzer.graph.constant_fact(value).is_none());
        let mut instances = Vec::new();
        for index in [2, 3] {
            let declaration = analyzer.tree.decl_lists.get(declarations)[index];
            let ast::DeclKind::Simple { declarators, .. } =
                analyzer.tree.declarations.get(declaration).kind
            else {
                panic!("simple declaration");
            };
            let initializer = analyzer.tree.init_declarators.get(declarators)[0]
                .initializer
                .unwrap();
            let expression = analyzer.single_initializer(initializer).unwrap();
            let Some(super::CallTarget::Direct(binding)) = analyzer
                .graph
                .expression_facts
                .get(expression)
                .unwrap()
                .call
            else {
                panic!("direct call");
            };
            instances.push(analyzer.graph.binding(binding).entity);
        }
        assert_eq!(instances[0], instances[1]);
        let instance = instances[0];
        assert_eq!(analyzer.graph.entity(instance).parameter_entities.len(), 1);
        let instance_body = analyzer.graph.lower_function_body(instance);
        let expressions = analyzer.tree.expressions.mark();
        let scopes = analyzer.graph.entity(function).scope;
        assert!(analyzer.constant_function_definition(function).unwrap());
        assert!(analyzer.constant_function_definition(instance).unwrap());
        assert_eq!(analyzer.graph.lower_function_body(instance), instance_body);
        assert_eq!(analyzer.graph.lower_function_body(function), Some(body_id));
        assert_eq!(analyzer.graph.entity(function).scope, scopes);
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert!(!analyzer.constexpr_frames.active());
        assert!(analyzer.constant_evaluation.is_none());
    }
}
