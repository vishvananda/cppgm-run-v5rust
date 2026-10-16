// Translated from sema/analyzer_construction.cpp::allocation and
// analyzer_initialization.cpp::array_new_list. See NOTICE.
//! Array storage retains the promoted bound and the deduced, checked list prefix.
use super::{
    Error,
    allocation::ArrayAllocation,
    analyzer::Analyzer,
    construction::ConstructorId,
    constructor_initialization::ConstructorArguments,
    graph::ScopeId,
    types::{TypeId, TypeKind},
};
use crate::syntax::{ast, storage::List};

impl Analyzer<'_> {
    pub(super) fn check_allocation_count(
        &mut self,
        count: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Option<u64>, Error> {
        self.typed_expression(count, scope)?;
        let facts = self.convert_unique_class_operand(
            count,
            super::conversion_functions::ConversionTarget::Integral,
            scope,
        )?;
        let ty = self.decayed_type(facts.ty)?;
        if !self.is_integral_operand(ty) {
            return Err(Error::at(
                self.tree.expressions.get(count).location,
                "an array bound of non-integral type",
            ));
        }
        let promoted = self.promoted_type(ty)?;
        self.convert_operand_scalar(count, promoted, false)?;
        let value = self.constant_value(count, scope, false)?;
        if let Some(value) = value {
            if self.integral_kind(value.ty).is_some_and(|kind| kind.signed)
                && value.bits.cast_signed() < 0
            {
                return Err(Error::at(
                    self.tree.expressions.get(count).location,
                    "a negative array new-expression count",
                ));
            }
            return Ok(Some(value.bits));
        }
        Ok(None)
    }

    pub(super) fn initialize_array_allocation(
        &mut self,
        count: ast::ExprId,
        constant: Option<u64>,
        target: TypeId,
        initializer: Option<ast::Initializer>,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<ArrayAllocation, Error> {
        let mut element = target;
        while let TypeKind::Array { element: inner, .. } = self.graph.types.get(element).kind {
            element = inner;
        }
        let mut prefix = None;
        if let Some(initializer) = initializer {
            match initializer.kind {
                ast::InitializerKind::Braced(list) => {
                    let unknown = self.graph.types.intern(
                        TypeKind::Array {
                            element: target,
                            bound: 0,
                            operand: None,
                        },
                        super::types::Cv::default(),
                    )?;
                    let ty = self.list_initialize(list, unknown, scope)?;
                    let TypeKind::Array { bound, .. } = self.graph.types.get(ty).kind else {
                        return Err(Error::Invariant("array new list without array type"));
                    };
                    if constant.is_some_and(|count| count < bound) {
                        return Err(Error::at(
                            self.tree.expressions.get(list).location,
                            "more initializers than the array new-expression's elements",
                        ));
                    }
                    prefix = Some(ty);
                }
                ast::InitializerKind::Parenthesized(args) if args.is_empty() => (),
                _ => {
                    return Err(Error::at(
                        location,
                        "an array new-expression with this initializer is not supported",
                    ));
                }
            }
        } else if self.graph.class_of(element).is_none() && self.type_cv(element).0 & 1 != 0 {
            return Err(Error::at(location, "a const object needs an initializer"));
        }
        // Validation uses any integral constant expression. The frozen O0
        // producer folds its size only for a literal or an already folded use.
        let mut root = count;
        while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(root).kind {
            root = inner;
        }
        let immediate = matches!(
            self.tree.expressions.get(root).kind,
            ast::ExprKind::Literal(_)
        ) || self.graph.expression_facts.get(root)?.folded.is_some();
        Ok(ArrayAllocation {
            count,
            constant: constant.filter(|_| immediate),
            prefix,
        })
    }

    pub(super) fn array_allocation_constructor(
        &mut self,
        array: ArrayAllocation,
        constant: Option<u64>,
        element: TypeId,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<Option<(ConstructorId, ConstructorArguments)>, Error> {
        let Some(class) = self.graph.class_of(element) else {
            return Ok(None);
        };
        let listed = match array.prefix.map(|ty| &self.graph.types.get(ty).kind) {
            Some(TypeKind::Array { bound, .. }) => *bound,
            _ => 0,
        };
        let mut trailing = constant.is_none_or(|count| count > listed)
            || constant == Some(0) && array.prefix.is_none();
        if constant == Some(0) && array.prefix.is_some() {
            trailing = if self.graph.entity(class).user_constructor.is_none() {
                true
            } else {
                match self.default_constructor_binding(class, location) {
                    Ok(binding) => {
                        self.graph
                            .entity(self.graph.binding(binding).entity)
                            .availability
                            != super::graph::FunctionAvailability::Deleted
                    }
                    Err(Error::Diagnostic {
                        message: "no default constructor for the class",
                        ..
                    }) => false,
                    Err(error) => return Err(error),
                }
            };
        }
        let constructor = if trailing {
            let constructor = self.default_constructor(class, location)?;
            self.validate_constructor_use(constructor, scope, location)?;
            self.demand_constructor_use(constructor);
            Some((constructor, ConstructorArguments::Values(List::default())))
        } else {
            None
        };
        self.class_parameter_destruction(element, scope, location)?;
        Ok(constructor)
    }

    pub(super) fn array_length_may_throw(&mut self, id: ast::ExprId) -> Result<bool, Error> {
        let use_ = self.graph.expression_facts.allocation(id)?;
        let Some(array) = use_.array else {
            return Ok(false);
        };
        if !use_.reserved && !self.function_may_throw(self.graph.binding(use_.function).entity)? {
            return Ok(false);
        }
        let listed = match array.prefix.map(|ty| &self.graph.types.get(ty).kind) {
            Some(TypeKind::Array { bound, .. }) => *bound,
            _ => 0,
        };
        let size = self
            .type_size(use_.target, self.tree.expressions.get(id).location)?
            .0;
        let mut element = use_.target;
        while let TypeKind::Array { element: inner, .. } = self.graph.types.get(element).kind {
            element = inner;
        }
        let cookie = if let Some(class) = self.graph.class_of(element) {
            let signature = self
                .graph
                .binding(use_.function)
                .ty
                .ok_or(Error::Invariant("array allocation type"))?;
            let variadic = matches!(
                self.graph.types.get(signature).kind,
                TypeKind::Function { variadic: true, .. }
            );
            if (!use_.reserved || variadic)
                && (use_.sized_array_delete || !self.graph.destructor_trivial(class)?)
            {
                8_u64.max(
                    self.type_size(element, self.tree.expressions.get(id).location)?
                        .1,
                )
            } else {
                0
            }
        } else {
            0
        };
        let largest = i64::MAX
            .cast_unsigned()
            .wrapping_sub(cookie)
            .checked_div(size)
            .unwrap_or(i64::MAX.cast_unsigned());
        if let Some(value) = array.constant {
            return Ok(value > largest || value < listed);
        }
        let facts = self.graph.expression_facts.get(array.count)?;
        let ty = facts
            .conversion
            .map_or(facts.ty, |conversion| conversion.result);
        let kind = self
            .integral_kind(ty)
            .ok_or(Error::Invariant("array bound without integral type"))?;
        Ok(listed != 0 || kind.signed || kind.bits >= 64 || ((1_u64 << kind.bits) - 1) > largest)
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
    fn full_class_prefix_keeps_cookie_facts_without_demanding_a_deleted_default() {
        let source = SourceBuffer::new(
            b"struct Unused{};struct A{A(int);A()=delete;~A();\
              static void operator delete[](void*,unsigned long)noexcept;};\
              A*f(){return new A[2]{1,2};}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"class-array-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let declaration_count = tree.declarations.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        assert_eq!(tree.declarations.mark(), declaration_count);
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[2])
            .kind
        else {
            panic!("function");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("body");
        };
        let ast::StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
            panic!("compound");
        };
        let ast::StmtKind::Return(Some(id)) =
            tree.statements.get(tree.stmt_lists.get(items)[0]).kind
        else {
            panic!("return");
        };
        let allocation = graph.lower_allocation(id).unwrap();
        assert!(allocation.constructor.is_none());
        assert!(allocation.sized_array_delete);
        assert!(allocation.deallocator.is_some());
        let array = allocation.array.unwrap();
        assert_eq!(array.constant, Some(2));
        assert!(matches!(graph.lower_type(array.prefix.unwrap()).unwrap(),
            super::super::lowering::TypeView::Array { element, bound: 2 } if element == allocation.target));
        let unused = cursor.intern_name(b"Unused").unwrap();
        let unused = graph
            .binding(
                graph
                    .own(graph.global, unused, super::super::lookup::Filter::Prefix)
                    .unwrap(),
            )
            .entity;
        assert!(matches!(
            graph.entity(unused).layout,
            super::super::layout::LayoutState::NotStarted
        ));
    }

    #[test]
    fn runtime_array_keeps_original_bound_and_only_interns_its_list_prefix() {
        let source = SourceBuffer::new(
            b"struct Unused{};typedef int Row[2];Row*f(unsigned n){return new Row[n]{1,2,3};}"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"array-storage").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), expressions);
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[2])
            .kind
        else {
            panic!("function");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("parsed body");
        };
        let ast::StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
            panic!("compound");
        };
        let ast::StmtKind::Return(Some(id)) =
            tree.statements.get(tree.stmt_lists.get(items)[0]).kind
        else {
            panic!("return");
        };
        let use_ = graph.expression_facts.allocation(id).unwrap();
        let array = use_.array.unwrap();
        assert!(array.constant.is_none());
        let original = graph.expression_facts.get(array.count).unwrap();
        assert_eq!(
            original.category,
            super::super::expression_facts::ValueCategory::Lvalue
        );
        assert_eq!(original.ty, original.conversion.unwrap().target);
        let prefix = array.prefix.unwrap();
        assert!(
            matches!(graph.types.get(prefix).kind, TypeKind::Array{element,bound:2, operand: None } if element==use_.target)
        );
        assert!(matches!(
            graph.types.get(use_.target).kind,
            TypeKind::Array { bound: 2, .. }
        ));
        assert!(use_.constructor.is_none());
        let unused = cursor.intern_name(b"Unused").unwrap();
        let unused = graph
            .binding(
                graph
                    .own(graph.global, unused, super::super::lookup::Filter::Prefix)
                    .unwrap(),
            )
            .entity;
        assert!(matches!(
            graph.entity(unused).layout,
            super::super::layout::LayoutState::NotStarted
        ));
    }
}
