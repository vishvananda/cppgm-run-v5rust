// Translated from analyzer_expressions.cpp and analyzer_member_pointers.cpp. See NOTICE.
//! Qualified member addresses, typed object routes and bound-call qualification.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory as V},
    graph::{BindingId, DataMemberKind, ScopeId},
    types::{Cv, TypeKind},
};
use crate::{lex::pp_lexer::Punctuator as P, syntax::ast};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ObjectExtent {
    Complete,
    Base,
}
#[derive(Clone, Copy)]
pub(crate) struct NullSubobject {
    pub ty: super::types::TypeId,
    pub offset: u64,
    pub extent: ObjectExtent,
}

impl Analyzer<'_> {
    pub(super) fn check_member_pointer_call(
        &self,
        id: ast::ExprId,
        callee: ast::ExprId,
        arguments: crate::syntax::storage::List<ast::ExprId>,
    ) -> Result<(), Error> {
        let facts = self.graph.expression_facts.get(callee)?;
        let object = facts
            .object
            .ok_or(Error::Invariant("bound member object"))?;
        let TypeKind::Function {
            cv,
            reference,
            ref parameters,
            variadic,
            ..
        } = self.graph.types.get(facts.ty).kind
        else {
            return Err(Error::Invariant("bound member signature"));
        };
        let lvalue = object.category == V::Lvalue;
        let count = self.tree.expr_lists.get(arguments).len();
        let failure = if self.type_cv(object.ty).0 & !cv.0 & 3 != 0 {
            Some("a call through a pointer to member function drops the object's cv-qualifiers")
        } else if matches!(reference, Some(super::types::RefKind::Lvalue)) && !lvalue
            || matches!(reference, Some(super::types::RefKind::Rvalue)) && lvalue
        {
            Some("the object's value category does not suit the member function's ref-qualifier")
        } else if count < parameters.len() || count > parameters.len() && !variadic {
            Some("wrong number of arguments in a call through a pointer to member function")
        } else {
            None
        };
        if let Some(message) = failure {
            return Err(Error::at(self.tree.expressions.get(id).location, message));
        }
        Ok(())
    }
    pub(super) fn member_pointer_cast(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
        source: ExprFacts,
        goal: super::types::TypeId,
        scope: ScopeId,
        c_style: bool,
    ) -> Result<Option<ExprFacts>, Error> {
        use super::expression_facts::{
            BaseCast, Conversion, ConversionRank, ListInitialization, LvalueTransform,
            ReferenceSource, ValueConversion,
        };
        let location = self.tree.expressions.get(id).location;
        let from = self.decayed_type(source.ty)?;
        let mut conversion = self.standard_conversion(source, goal, true)?;
        let mut cast = None;
        if conversion.is_none()
            && c_style
            && let (TypeKind::MemberPointer { element, .. }, TypeKind::MemberPointer { owner, .. }) = (
                self.graph.types.get(from).kind.clone(),
                self.graph.types.get(goal).kind.clone(),
            )
        {
            let kept = self.graph.types.member_pointer(owner, element)?;
            conversion = self.standard_conversion(source, kept, true)?;
            if let Some(conversion) = &mut conversion {
                conversion.target = goal;
                conversion.result = goal;
            }
        }
        if conversion.is_none() {
            let TypeKind::MemberPointer {
                owner: derived,
                element: a,
            } = self.graph.types.get(from).kind
            else {
                return Err(Error::at(
                    location,
                    "invalid conversion to a pointer to member",
                ));
            };
            let TypeKind::MemberPointer {
                owner: base,
                element: b,
            } = self.graph.types.get(goal).kind
            else {
                return Err(Error::Invariant("member cast target"));
            };
            if self.bare_type(a)? != self.bare_type(b)?
                || !c_style && self.type_cv(a).0 & !self.type_cv(b).0 != 0
            {
                return if c_style {
                    Ok(None)
                } else {
                    Err(Error::at(
                        location,
                        "invalid conversion to a pointer to member",
                    ))
                };
            }
            let derived = self
                .graph
                .class_of(derived)
                .ok_or(Error::Invariant("member cast class"))?;
            let base = self
                .graph
                .class_of(base)
                .ok_or(Error::Invariant("member cast base"))?;
            if derived == base || self.graph.is_base_of(base, derived) {
                let path = self
                    .checked_member_pointer_path(derived, base, scope, location, !c_style, true)?;
                cast = Some(BaseCast {
                    path,
                    downcast: true,
                    project_zero: false,
                });
            } else {
                return if c_style {
                    Ok(None)
                } else {
                    Err(Error::at(
                        location,
                        "invalid conversion to a pointer to member",
                    ))
                };
            }
            conversion = Some(Conversion {
                target: goal,
                result: goal,
                first: LvalueTransform::None,
                second: ValueConversion::None,
                rank: ConversionRank::Conversion,
                qualification: false,
                initialization: false,
                list_initialization: ListInitialization::Ordinary,
                temporary: false,
                reference_source: ReferenceSource::Other,
            });
        }
        let mut conversion = conversion.ok_or(Error::Invariant("member cast sequence"))?;
        conversion.initialization = false;
        let mut facts = source;
        facts.base_path =
            self.conversion_base_path(source, conversion, scope, location, !c_style)?;
        self.record_conversion(&mut facts, conversion);
        self.graph.expression_facts.set(operand, facts);
        let mut result = self.new_expression_facts(goal, V::Prvalue)?;
        result.reference_cast = Some(goal);
        result.base_cast = cast;
        Ok(Some(result))
    }
    pub(super) fn qualified_data_address(
        &mut self,
        operand: ast::ExprId,
        binding: BindingId,
        naming: super::graph::EntityId,
        scope: ScopeId,
    ) -> Result<Option<ExprFacts>, Error> {
        let entity = self.graph.binding(binding).entity;
        if self.graph.entity(entity).data_member != Some(DataMemberKind::Instance) {
            return Ok(None);
        }
        let location = self.tree.expressions.get(operand).location;
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("member without type"))?;
        if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
            return Err(Error::at(location, "pointer to member of reference type"));
        }
        self.check_named_member_access(binding, naming, naming, scope, location)?;
        let holder = self.graph.member_holder(entity, None)?;
        let owner = self
            .graph
            .entity(holder)
            .ty
            .ok_or(Error::Invariant("data member without class"))?;
        if self.lowering() {
            self.demand_storage_layout(holder, location)?;
        }
        let mut inner = self.new_expression_facts(ty, V::Lvalue)?;
        inner.member = Some(binding);
        self.graph.expression_facts.set(operand, inner);
        super::record(&mut self.graph.expression_types, operand.index(), ty);
        let pointer = self.graph.types.member_pointer(owner, ty)?;
        let mut facts = self.new_expression_facts(pointer, V::Prvalue)?;
        facts.member = Some(binding);
        Ok(Some(facts))
    }

    pub(super) fn member_pointer_access(
        &mut self,
        id: ast::ExprId,
        op: P,
        left: ast::ExprId,
        right: ast::ExprId,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let location = self.tree.expressions.get(id).location;
        self.check_resolved_function(left)?;
        let lf = self.graph.expression_facts.get(left)?;
        let rf = self.graph.expression_facts.get(right)?;
        let pointer = self.decayed_type(rf.ty)?;
        let TypeKind::MemberPointer { owner, element } = self.graph.types.get(pointer).kind else {
            return Err(Error::at(
                location,
                "right operand of .* or ->* is not a pointer to member",
            ));
        };
        self.convert_operand_scalar(right, pointer, false)?;
        let (object, category) = if op == P::ArrowStar {
            let pointer = self.decayed_type(lf.ty)?;
            let TypeKind::Pointer(object) = self.graph.types.get(pointer).kind else {
                return Err(Error::at(location, "left operand of ->* is not a pointer"));
            };
            self.convert_operand_scalar(left, pointer, false)?;
            (object, V::Lvalue)
        } else {
            (lf.ty, lf.category)
        };
        let owner = self
            .graph
            .class_of(owner)
            .ok_or(Error::Invariant("member pointer owner"))?;
        let Some(class) = self.graph.class_of(object) else {
            return Err(Error::at(
                location,
                "object and pointer to member classes do not match",
            ));
        };
        if class != owner
            && !self.derived_class_conversion(
                self.graph
                    .entity(owner)
                    .ty
                    .ok_or(Error::Invariant("class type"))?,
                object,
            )?
        {
            return Err(Error::at(
                location,
                "object and pointer to member classes do not match",
            ));
        }
        let path = if class == owner {
            None
        } else {
            Some(
                self.checked_base_path(class, owner, scope, location, true, false)
                    .map_err(|error| match error {
                        Error::Diagnostic {
                            message: "conversion to an ambiguous base class",
                            ..
                        } => Error::at(
                            location,
                            "a pointer to member of an ambiguous base class of the object",
                        ),
                        other => other,
                    })?,
            )
        };
        if matches!(
            self.graph.types.get(element).kind,
            TypeKind::Function { .. }
        ) {
            let mut facts = self.new_expression_facts(element, V::Prvalue)?;
            facts.object = Some(super::expression_facts::ObjectArgument {
                source: super::expression_facts::ObjectSource::Explicit {
                    expression: left,
                    arrow: op == P::ArrowStar,
                },
                ty: object,
                category,
            });
            facts.member_path = path;
            if self.lowering() {
                self.demand_storage_layout(owner, location)?;
            }
            return Ok(facts);
        }
        let cv = Cv(self.graph.types.get(element).cv.0 | self.type_cv(object).0);
        let ty = self.graph.types.qualify(element, cv)?;
        let mut facts = self.new_expression_facts(
            ty,
            if category == V::Lvalue {
                V::Lvalue
            } else {
                V::Xvalue
            },
        )?;
        facts.member_path = path;
        Ok(facts)
    }
}

impl super::graph::Graph {
    pub(crate) fn lower_holds_data_member_pointer(
        &self,
        mut ty: super::types::TypeId,
    ) -> Result<bool, Error> {
        while let TypeKind::Array { element, .. } = self.types.get(ty).kind {
            ty = element;
        }
        Ok(match self.types.get(ty).kind {
            TypeKind::MemberPointer { element, .. } => {
                !matches!(self.types.get(element).kind, TypeKind::Function { .. })
            }
            TypeKind::Class(class) => match self.entity(class).layout {
                super::layout::LayoutState::Value {
                    contains_data_member_pointer,
                    ..
                } => contains_data_member_pointer,
                _ => return Err(Error::Invariant("null member query without class layout")),
            },
            _ => false,
        })
    }
    /// Only the subobjects whose zero image contains the data-member null sentinel.
    pub(crate) fn lower_null_subobjects(
        &self,
        class: super::graph::EntityId,
        extent: ObjectExtent,
    ) -> impl Iterator<Item = Result<NullSubobject, Error>> + '_ {
        let bases = self.entity(class).bases.iter().filter_map(|edge| {
            (|| -> Result<_, Error> {
                if edge.is_virtual {
                    return Ok(None);
                }
                let ty = self.lower_entity_type(edge.class)?;
                if !self.lower_holds_data_member_pointer(ty)? {
                    return Ok(None);
                }
                let offset = edge
                    .offset
                    .ok_or(Error::Invariant("null base without layout"))?;
                Ok(Some(NullSubobject {
                    ty,
                    offset,
                    extent: ObjectExtent::Base,
                }))
            })()
            .transpose()
        });
        let count = if self.entity(class).key == crate::lex::vocabulary::Keyword::Union {
            1
        } else {
            self.entity(class).members.len()
        };
        let members = self
            .entity(class)
            .members
            .iter()
            .take(count)
            .filter_map(|&member| {
                (|| -> Result<_, Error> {
                    let ty = self.lower_entity_type(member)?;
                    if self.entity(member).bit_field.is_some()
                        || !self.lower_holds_data_member_pointer(ty)?
                    {
                        return Ok(None);
                    }
                    Ok(Some(NullSubobject {
                        ty,
                        offset: self.lower_member_offset(member)?,
                        extent: ObjectExtent::Complete,
                    }))
                })()
                .transpose()
            });
        let shared = self
            .lower_virtual_bases(class)
            .iter()
            .filter_map(move |&(base, offset)| {
                (|| -> Result<_, Error> {
                    if extent != ObjectExtent::Complete {
                        return Ok(None);
                    }
                    let ty = self.lower_entity_type(base)?;
                    Ok(self
                        .lower_holds_data_member_pointer(ty)?
                        .then_some(NullSubobject {
                            ty,
                            offset,
                            extent: ObjectExtent::Base,
                        }))
                })()
                .transpose()
            });
        bases.chain(members).chain(shared)
    }

    pub(crate) fn lower_bound_member_pointer(&self, id: ast::ExprId) -> Result<bool, Error> {
        let facts = self.expression_facts.get(id)?;
        Ok(facts.object.is_some()
            && facts.function.is_none()
            && facts.functions.is_none()
            && facts.destructor.is_none()
            && matches!(self.types.get(facts.ty).kind, TypeKind::Function { .. }))
    }

    pub(crate) fn lower_member_function_address(
        &self,
        id: ast::ExprId,
    ) -> Result<Option<super::graph::EntityId>, Error> {
        let facts = self.expression_facts.get(id)?;
        Ok(if self.lower_member_pointer_type(facts.ty).is_some() {
            facts.function.map(|binding| self.binding(binding).entity)
        } else {
            None
        })
    }
    pub(crate) fn lower_member_pointer_conversion(
        &self,
        id: ast::ExprId,
    ) -> Result<Option<super::base_projections::BaseProjection>, Error> {
        let facts = self.expression_facts.get(id)?;
        if !facts
            .conversion
            .is_some_and(|c| c.second == super::expression_facts::ValueConversion::MemberToDerived)
        {
            return Ok(None);
        }
        let path = facts
            .base_path
            .ok_or(Error::Invariant("member conversion without route"))?;
        self.lower_path_projection(path).map(Some)
    }

    pub(crate) fn lower_member_pointer_cast(
        &self,
        id: ast::ExprId,
    ) -> Result<Option<super::base_projections::BaseProjection>, Error> {
        let facts = self.expression_facts.get(id)?;
        if self.lower_member_pointer_type(facts.ty).is_none() {
            return Ok(None);
        }
        facts
            .base_cast
            .map(|cast| self.lower_path_projection(cast.path)?.negated())
            .transpose()
    }
    pub(crate) fn lower_data_member_address(&self, id: ast::ExprId) -> Result<Option<u64>, Error> {
        let facts = self.expression_facts.get(id)?;
        // Only the prvalue address formation carries an address constant.
        // An implicit member read can itself have member-pointer type and
        // a selected member, but its value must still be loaded from storage.
        if facts.category != V::Prvalue
            || self.lower_member_pointer_type(facts.ty).is_none()
            || facts.function.is_some()
        {
            return Ok(None);
        }
        facts
            .member
            .map(|binding| {
                let entity = self.binding(binding).entity;
                Ok(self
                    .lower_member_offset(entity)?
                    .wrapping_add(self.lower_anonymous_member_offset(entity, None)?))
            })
            .transpose()
    }

    pub(crate) fn lower_member_pointer_projection(
        &self,
        id: ast::ExprId,
    ) -> Result<Option<super::base_projections::BaseProjection>, Error> {
        self.expression_facts
            .get(id)?
            .member_path
            .map(|path| self.lower_path_projection(path))
            .transpose()
    }

    pub(crate) fn lower_member_pointer_union(
        &self,
        object: super::types::TypeId,
        arrow: bool,
    ) -> bool {
        let object = if arrow {
            if let TypeKind::Pointer(element) = self.types.get(object).kind {
                element
            } else {
                object
            }
        } else {
            object
        };
        self.class_of(object)
            .is_some_and(|class| self.entity(class).key == crate::lex::vocabulary::Keyword::Union)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{Analyzer, layout::LayoutState, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn member_layouts_follow_evaluated_query_uses_without_retyping() {
        let source = SourceBuffer::new(
            br"namespace std { struct type_info {}; }
            struct Result { virtual int get(); };
            struct Holder { int pad; Result value; };
            struct Prefix { int pad; }; struct Base { Result value; };
            struct Derived : Prefix, Base {};
            struct Receiver { int pad; };
            struct Unused { Result value; };
            struct UnusedDerived : Prefix, Base {};
            struct UnusedReceiver { int pad; };
            struct Dormant { Result value; };
            void query(Holder* p, Derived* d, Receiver* r, Result& (Receiver::*f)(),
                       Unused* u, UnusedDerived* v, UnusedReceiver* w,
                       Result& (UnusedReceiver::*g)()) {
                typeid((typeid(p->*(&Holder::value)), p->*(&Holder::value)));
                typeid(d->*static_cast<Result Derived::*>(&Base::value));
                typeid((r->*f)());
                sizeof(u->*(&Unused::value));
                typeid((v->*static_cast<Result UnusedDerived::*>(&Base::value), 0));
                typeid(((w->*g)(), 0));
            }
            inline void dormant(Dormant* p) { typeid(p->*(&Dormant::value)); }"
                .to_vec(),
        )
        .unwrap();
        let input = Preprocessor::with_name(&source, b"member-query-layouts").unwrap();
        let mut unit = SyntaxUnit::new(input);
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let count = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), count);
        for (spelling, evaluated) in [
            (b"Holder".as_slice(), true),
            (b"Derived", true),
            (b"Receiver", true),
            (b"Unused", false),
            (b"UnusedDerived", false),
            (b"UnusedReceiver", false),
            (b"Dormant", false),
        ] {
            let name = cursor.intern_name(spelling).unwrap();
            let binding = graph.own(graph.global, name, Filter::Prefix).unwrap();
            assert_eq!(
                matches!(
                    graph.entity(graph.binding(binding).entity).layout,
                    LayoutState::NotStarted
                ),
                !evaluated,
            );
        }
    }
}
