// Translated from sema/constant_statements.cpp's scalar call path. See NOTICE.
//! Invoke selected functions over their analyzed bodies, never their source text.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, TemporaryValue as Value, nonconstant},
    constants::Constant,
    constexpr_frames::Frame,
    expression_facts::ValueCategory,
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

impl Analyzer<'_> {
    pub(super) fn check_constexpr_scalar_signature(
        &self,
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
        if let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind
            && matches!(
                self.graph.types.get(result).kind,
                TypeKind::Scalar(Scalar::Void)
            )
        {
            return Err(Error::at(
                location,
                "a constexpr function's return type is not a literal type",
            ));
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
        let kind = self
            .integral_kind(target)
            .ok_or(Error::Missing("nonintegral constexpr scalar value"))?;
        let value = if self.is_address_expression(id) {
            u64::from(self.evaluate_address(id, scope)?.root != Root::Null)
        } else {
            self.evaluate_constant(id, scope, false)?.bits
        };
        Ok(Value::Integer(Constant {
            bits: kind.convert(value),
            ty: self.bare_type(target)?,
        }))
    }
    pub(super) fn zero_constant_scalar(&mut self, target: TypeId) -> Result<Value, Error> {
        match self.graph.types.get(target).kind {
            TypeKind::Pointer(element) => Ok(Value::Pointer(Address::new(Root::Null, element))),
            TypeKind::Scalar(Scalar::Nullptr) => {
                Ok(Value::Pointer(Address::new(Root::Null, target)))
            }
            _ if self.integral_kind(target).is_some() => Ok(Value::Integer(Constant {
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
        let root = self.graph.expression_facts.root(id);
        let facts = self.graph.expression_facts.get(root)?;
        if facts.category == ValueCategory::Prvalue
            || facts
                .conversion
                .is_some_and(|conversion| conversion.temporary)
        {
            let value = self.constant_operand_value(id, element, scope)?;
            return self.constexpr_frames.allocate(
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
                bits: 0,
                ty: self.graph.expression_facts.get(id)?.ty,
            }),
            Returned::Value(Value::Pointer(_)) => {
                Err(nonconstant(location, "not an integral constant expression"))
            }
        }
    }
    pub(super) fn constant_call_result(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Returned, Error> {
        let ExprKind::Call { callee, arguments } = self.tree.expressions.get(id).kind else {
            return Err(Error::Missing("constexpr operator and literal invocation"));
        };
        let facts = self.graph.expression_facts.get(id)?;
        let location = self.tree.expressions.get(id).location;
        if facts.dispatch.is_virtual() {
            return Err(nonconstant(
                location,
                "a virtual function call in a constant expression",
            ));
        }
        let function = if let Some(CallTarget::Direct(binding)) = facts.call {
            self.graph.binding(binding).entity
        } else {
            let address = self.evaluate_address(callee, scope)?;
            let Root::Function(entity) = address.root else {
                return Err(nonconstant(
                    location,
                    "a call through a pointer that designates no function",
                ));
            };
            entity
        };
        // Optional evaluation of an ordinary reference/pointer-returning call
        // must fail softly before reaching the unported constexpr object path.
        if !self.graph.entity(function).constexpr {
            return Err(nonconstant(
                location,
                "a call of a function that is not constexpr",
            ));
        }
        if self
            .graph
            .entity(function)
            .function
            .is_some_and(|f| !f.is_static)
            && self
                .graph
                .entity(function)
                .home
                .is_some_and(|home| self.graph.scope(home).kind == super::graph::ScopeKind::Class)
        {
            return Err(Error::Missing("constexpr member object invocation"));
        }
        let callee = self.graph.expression_facts.root(callee);
        if let ExprKind::Member { base, .. } = self.tree.expressions.get(callee).kind {
            self.constant_member_object(base, scope)?;
        }
        self.invoke_constant_function(function, arguments, scope, location)
    }
    fn invoke_constant_function(
        &mut self,
        function: EntityId,
        arguments: crate::syntax::storage::List<ast::ExprId>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Returned, Error> {
        self.constant_step(location)?;
        let entity = self.graph.entity(function);
        if entity.availability == FunctionAvailability::Deleted {
            return Err(nonconstant(location, "a call of a deleted function"));
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
            result: returns,
            ref parameters,
            ..
        } = self.graph.types.get(ty).kind
        else {
            return Err(Error::Invariant("constant function without function type"));
        };
        let count = parameters.len();
        let written = self.tree.expr_lists.get(arguments).len();
        // Variadic extras are evaluated before the fixed parameters, as in C++.
        for index in count..written {
            self.discard_constant(self.tree.expr_lists.get(arguments)[index], scope)?;
        }
        let mut bound = Vec::with_capacity(count);
        for index in 0..count {
            let parameter = self.graph.types.function_parameter(ty, index)?;
            let argument = self
                .tree
                .expr_lists
                .get(arguments)
                .get(index)
                .copied()
                .or_else(|| self.graph.function_default(function, index))
                .ok_or_else(|| {
                    nonconstant(location, "a call without an argument for a parameter")
                })?;
            let value = match self.graph.types.get(parameter).kind {
                TypeKind::Reference { element, .. } => {
                    Returned::Place(self.bind_constant_reference(argument, element, scope)?)
                }
                _ => Returned::Value(self.constant_operand_value(argument, parameter, scope)?),
            };
            bound.push(value);
        }
        let region = self.constexpr_frames.region();
        let result = self.run_constant_function(function, returns, bound, location);
        self.constexpr_frames.release(region);
        if let Ok(Returned::Value(Value::Pointer(value))) = result {
            self.constexpr_frames.valid_pointer(value, location)?;
        }
        result
    }
    fn run_constant_function(
        &mut self,
        function: EntityId,
        returns: TypeId,
        bound: Vec<Returned>,
        location: SourceOffset,
    ) -> Result<Returned, Error> {
        let scope = self
            .graph
            .entity(function)
            .scope
            .ok_or(Error::Invariant("constant function without scope"))?;
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
        let mut frame = Frame::new(returns, scope);
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
                    self.constexpr_frames.allocate(ty, Some(value), location)?
                }
                Returned::Void => {
                    return Err(Error::Invariant("void argument to constant function"));
                }
            };
            if let Some(entity) = entity {
                frame.bind(entity, place);
            }
        }
        let saved = self
            .constant_evaluation
            .as_mut()
            .ok_or(Error::Invariant("call without evaluation"))?
            .suspend_temporaries();
        self.constexpr_frames.push(frame);
        let result = self
            .constant_compound(body.compound)
            .and_then(|flow| match flow {
                super::constexpr_statements::Flow::Return(value) => Ok(value),
                _ if matches!(
                    self.graph.types.get(returns).kind,
                    TypeKind::Scalar(Scalar::Void)
                ) =>
                {
                    Ok(Returned::Void)
                }
                _ => Err(nonconstant(
                    location,
                    "a constexpr function flows off its end",
                )),
            });
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
