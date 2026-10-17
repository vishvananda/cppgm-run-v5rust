// Translated from constant_evaluator.cpp::record_constant_initializer and
// constant_interpreter.cpp::initialize/publish/root_storage. See NOTICE.
//! Declaration-owned aggregate values survive the evaluation that produced them.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, TemporaryValue as Value, nonconstant},
    constexpr_frames::Frozen,
    graph::{EntityId, ScopeId},
    types::TypeKind,
};
use crate::{support::source::SourceOffset, syntax::ast};
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct Values {
    complete: HashMap<EntityId, Frozen>,
    constructing: HashMap<EntityId, Address>,
}

impl Values {
    pub(super) fn get(&self, entity: EntityId) -> Option<&Frozen> {
        self.complete.get(&entity)
    }
    pub(super) fn constructing(&self, entity: EntityId) -> Option<Address> {
        self.constructing.get(&entity).copied()
    }
}
impl Analyzer<'_> {
    pub(super) fn record_aggregate_constant(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        scope: ScopeId,
        required: bool,
    ) -> Result<(), Error> {
        let location = initializer.map_or(SourceOffset(0), |i| i.location);
        let ty = self.graph.lower_entity_type(entity)?;
        if !matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Class(_) | TypeKind::Array { .. }
        ) || self.graph.aggregate_constants.get(entity).is_some()
        {
            return Ok(());
        }
        if required && !self.constant_literal_type(ty)? {
            return Err(Error::at(
                location,
                "a constexpr variable of a type that is not a literal type",
            ));
        }
        let result = self.in_constant_evaluation(|analyzer| {
            let region = analyzer.constexpr_frames.region();
            let result =
                analyzer.evaluate_aggregate_initializer(entity, initializer, scope, location);
            analyzer
                .graph
                .aggregate_constants
                .constructing
                .remove(&entity);
            analyzer.constexpr_frames.release(region);
            result
        });
        match result {
            Ok(value) => {
                self.graph
                    .aggregate_constants
                    .complete
                    .insert(entity, value);
            }
            Err(Error::NonConstant { location, message }) if required => {
                return Err(Error::at(location, message));
            }
            Err(Error::NonConstant { .. } | Error::Missing(_)) if !required => (),
            Err(error) => return Err(error),
        }
        Ok(())
    }
    fn evaluate_aggregate_initializer(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Frozen, Error> {
        if self
            .graph
            .aggregate_constants
            .constructing(entity)
            .is_some()
        {
            return Err(nonconstant(
                location,
                "a read of an object that is not initialized",
            ));
        }
        let ty = self.graph.lower_entity_type(entity)?;
        let place = self.allocate_constant_storage(ty, None, location)?;
        self.constexpr_frames.initialize_declaration(place)?;
        self.graph
            .aggregate_constants
            .constructing
            .insert(entity, place);
        let constructor = self.graph.lower_construction(entity);
        self.construct_constant_object(place, initializer, constructor, scope, location)?;
        let value = self.constexpr_frames.freeze(place, location, |value| {
            let Value::Pointer(mut pointer) = value else {
                return Ok(value);
            };
            if pointer.root == place.root {
                pointer.root = Root::Variable(entity);
            }
            if matches!(pointer.root, Root::Object(_) | Root::Ephemeral(_))
                || matches!(pointer.root, Root::Variable(root) if self.graph.lower_automatic(root)
                    || self.graph.lower_variable(root).is_ok_and(|v| v.thread_local))
            {
                return Err(nonconstant(
                    location,
                    "a constant designates an object without static storage duration",
                ));
            }
            Ok(Value::Pointer(pointer))
        })?;
        if !value.complete(|ty| {
            matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Class(_) | TypeKind::Array { .. }
            )
        }) {
            return Err(nonconstant(
                location,
                "a constexpr object leaves a subobject uninitialized",
            ));
        }
        Ok(value)
    }
    pub(super) fn published_scalar(
        &mut self,
        mut place: Address,
        location: SourceOffset,
    ) -> Result<Option<Value>, Error> {
        let Root::Variable(entity) = place.root else {
            return Ok(None);
        };
        self.instantiate_variable_value(entity)?;
        if !self.graph.entity(entity).constexpr {
            return Ok(None);
        }
        if self.type_cv(self.graph.lower_entity_type(entity)?).0 & 2 != 0 {
            return Err(nonconstant(location, "a read of a volatile object"));
        }
        place.designated = self.bare_type(place.designated)?;
        self.graph
            .aggregate_constants
            .get(entity)
            .map(|value| {
                value.read(
                    place,
                    self.constexpr_frames.designation(place.variant()),
                    location,
                )
            })
            .transpose()
    }
    pub(super) fn copy_constant_object(
        &mut self,
        mut target: Address,
        mut source: Address,
        location: SourceOffset,
    ) -> Result<(), Error> {
        target.designated = self.bare_type(target.designated)?;
        source.designated = self.bare_type(source.designated)?;
        if let Root::Variable(entity) = source.root {
            self.instantiate_variable_value(entity)?;
            if self.graph.entity(entity).constexpr
                && let Some(value) = self.graph.aggregate_constants.get(entity)
            {
                return self
                    .constexpr_frames
                    .copy_published(target, source, value, location);
            }
            return Err(nonconstant(
                location,
                "a variable is not usable in a constant expression",
            ));
        }
        self.constexpr_frames.copy(target, source, location)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::lookup::Filter,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn failed_static_probe_releases_partial_arguments_without_publishing_them() {
        let source = SourceBuffer::new(
            br"
            struct S {int n; constexpr S(int n):n(n){}};
            int runtime();
            constexpr S make(S s, int n) {return S(s.n+n);}
            constexpr S before(3);
            S dynamic=make(before,runtime());
            constexpr S after(7);
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"probe-rollback").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"before".as_slice(), b"dynamic", b"after"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let syntax = analyzer.tree.expressions.mark();
        analyzer.finish_static_initializers().unwrap();
        for (name, bits) in names.into_iter().zip([Some(3), None, Some(7)]) {
            let binding = analyzer.graph.own(scope, name, Filter::Ordinary).unwrap();
            let entity = analyzer.graph.binding(binding).entity;
            let value = analyzer.graph.aggregate_constants.get(entity);
            if let Some(bits) = bits {
                assert!(value.unwrap().values().any(|(_, _, _, value)|
                    matches!(value, Some(Value::Integer(value)) if value.bits == bits)));
            } else {
                assert!(value.is_none());
                assert!(analyzer.graph.lower_array_image(entity).is_none());
            }
        }
        assert_eq!(analyzer.tree.expressions.mark(), syntax);
        assert_eq!(analyzer.graph.aggregate_constants.complete.len(), 2);
        assert!(analyzer.graph.aggregate_constants.constructing.is_empty());
        assert!(analyzer.constant_evaluation.is_none());
        assert!(!analyzer.constexpr_frames.active());
    }

    #[test]
    fn completed_objects_survive_calls_and_copies_without_replaying_syntax() {
        let source = SourceBuffer::new(
            br"
            struct S {int n; constexpr S(int v):n(v){} };
            constexpr S increment(S s) {s.n+=1; return s;}
            constexpr S a(3);
            constexpr S b=a;
            constexpr S c=increment(a);
            constexpr S elements[2]={S(7),increment(S(8))};
            constexpr int n=increment(a).n+elements[1].n;
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"published-values").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"a", b"b", b"c"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        for (name, bits) in names.into_iter().zip([3, 3, 4]) {
            let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
            let value = analyzer
                .graph
                .aggregate_constants
                .get(analyzer.graph.binding(binding).entity)
                .unwrap();
            let values: Vec<_> = value
                .values()
                .filter_map(|(_, _, _, value)| match value {
                    Some(Value::Integer(value)) => Some(value.bits),
                    _ => None,
                })
                .collect();
            assert_eq!(values, [bits]);
        }
        let declaration = *analyzer.tree.decl_lists.get(declarations).last().unwrap();
        let ast::DeclKind::Simple { declarators, .. } =
            analyzer.tree.declarations.get(declaration).kind
        else {
            panic!("simple declaration");
        };
        let initializer = analyzer.tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap();
        let expr = analyzer.single_initializer(initializer).unwrap();
        let mark = analyzer.tree.expressions.mark();
        for _ in 0..2 {
            assert_eq!(
                analyzer
                    .constant_value(expr, global, true)
                    .unwrap()
                    .unwrap()
                    .bits,
                13
            );
            assert_eq!(analyzer.tree.expressions.mark(), mark);
            assert_eq!(analyzer.graph.aggregate_constants.complete.len(), 4);
            assert!(analyzer.graph.aggregate_constants.constructing.is_empty());
            assert!(analyzer.constant_evaluation.is_none());
            assert!(!analyzer.constexpr_frames.active());
        }
    }
}
