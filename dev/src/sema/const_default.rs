// Translated from analyzer_initialization.cpp::check_const_default. See NOTICE.
//! Successful const-default facts belong to the target class, not its selected
//! function: an inherited template can supply that function to several classes.
use super::{
    Error,
    analyzer::Analyzer,
    construction::{ConstructorId, DefaultFunction},
    graph::{EntityId, FunctionAvailability},
    special_members::SpecialDefinition,
    types::{TypeId, TypeKind},
};
use crate::{lex::vocabulary::Keyword, support::source::SourceOffset};

struct Frame {
    class: EntityId,
    next_base: usize,
    next_member: usize,
    members: usize,
    initialized: usize,
}
impl Analyzer<'_> {
    pub(super) fn validate_const_default(
        &mut self,
        target: TypeId,
        constructor: ConstructorId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.graph.types.get(target).cv.0 & 1 == 0 {
            return Ok(());
        }
        let class = self
            .graph
            .class_of(target)
            .ok_or(Error::Invariant("const default target without class"))?;
        self.check_const_default(
            class,
            self.graph
                .constructors
                .get(constructor)
                .declaration
                .map(|function| DefaultFunction {
                    function,
                    declared: self.graph.constructors.get(constructor).binding,
                }),
            location,
        )
    }

    pub(super) fn check_const_default(
        &mut self,
        class: EntityId,
        constructor: Option<DefaultFunction>,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let mut work = Vec::new();
        self.enter_const_default(&mut work, class, constructor)?;
        while let Some(frame) = work.last_mut() {
            if let Some(base) = self
                .graph
                .entity(frame.class)
                .class()
                .bases
                .get(frame.next_base)
            {
                frame.next_base += 1;
                let class = base.class;
                let function = self.implicit_constructor_function(class)?;
                self.enter_const_default(&mut work, class, function)?;
                continue;
            }
            let union = self.graph.entity(frame.class).key == Keyword::Union;
            if let Some(&member) = self
                .graph
                .entity(frame.class)
                .class()
                .members
                .get(frame.next_member)
            {
                frame.next_member += 1;
                let variable = self.graph.entity(member);
                let anonymous_object = variable
                    .ty
                    .and_then(|ty| self.graph.class_of(ty))
                    .and_then(|class| self.graph.anonymous_member_storage(class))
                    .is_some_and(|storage| storage == member);
                if variable.object().variable.is_none() && !anonymous_object
                    || variable.object().bit_field.is_some() && variable.name.is_none()
                {
                    continue;
                }
                frame.members += 1;
                if variable.initialization().member_initializer.is_some() {
                    frame.initialized += 1;
                    continue;
                }
                if union {
                    continue;
                }
                let mut object = variable
                    .ty
                    .ok_or(Error::Invariant("const default member without type"))?;
                while let TypeKind::Array { element, .. } = self.graph.types.get(object).kind {
                    object = element;
                }
                let TypeKind::Class(class) = self.graph.types.get(object).kind else {
                    return Err(Error::at(
                        location,
                        "a const object leaves a member uninitialized",
                    ));
                };
                let constructor = if self.graph.anonymous_member_storage(class).is_some() {
                    None
                } else {
                    self.implicit_constructor_function(class)?
                };
                self.enter_const_default(&mut work, class, constructor)?;
                continue;
            }
            if union && frame.members != 0 && frame.initialized != 1 {
                return Err(Error::at(
                    location,
                    "a const union object initializes no member",
                ));
            }
            self.graph.constructors.record_const_default(frame.class);
            work.pop();
        }
        Ok(())
    }

    fn enter_const_default(
        &mut self,
        work: &mut Vec<Frame>,
        class: EntityId,
        constructor: Option<DefaultFunction>,
    ) -> Result<(), Error> {
        if self.graph.constructors.const_default(class)
            || constructor.is_some_and(|slot| {
                let entity = self.graph.entity(slot.function);
                self.graph
                    .inherited_constructors
                    .source(slot.function)
                    .is_none()
                    && slot.declared.is_some()
                    && entity.availability != FunctionAvailability::Deleted
                    && !matches!(
                        entity.callable().special_definition,
                        Some(SpecialDefinition::Defaulted { first: true, .. })
                    )
            })
        {
            return Ok(());
        }
        self.check_stack()?;
        work.push(Frame {
            class,
            next_base: 0,
            next_member: 0,
            members: 0,
            initialized: 0,
        });
        Ok(())
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
    fn only_successful_class_checks_are_retained() {
        let source = SourceBuffer::new(
            b"struct Empty{};struct Incomplete{int n;};struct User{int n;User();};struct Anonymous{union{int n;};};".to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"const-default").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [b"Empty".as_slice(), b"Incomplete", b"User", b"Anonymous"]
            .map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let classes = names.map(|name| {
            analyzer
                .graph
                .binding(analyzer.graph.own(global, name, Filter::Prefix).unwrap())
                .entity
        });
        let constructors = classes.map(|class| {
            analyzer
                .default_constructor(class, SourceOffset(0))
                .unwrap();
            analyzer.graph.constructors.default_function(class).unwrap()
        });
        for class in classes {
            assert!(!analyzer.graph.constructors.const_default(class));
        }
        analyzer
            .check_const_default(classes[0], Some(constructors[0]), SourceOffset(0))
            .unwrap();
        assert!(analyzer.graph.constructors.const_default(classes[0]));
        analyzer
            .check_const_default(classes[0], None, SourceOffset(0))
            .unwrap();
        assert!(
            analyzer
                .check_const_default(classes[1], Some(constructors[1]), SourceOffset(0))
                .is_err()
        );
        assert!(!analyzer.graph.constructors.const_default(classes[1]));
        analyzer
            .check_const_default(classes[2], Some(constructors[2]), SourceOffset(0))
            .unwrap();
        assert!(!analyzer.graph.constructors.const_default(classes[2]));
        assert!(
            analyzer
                .check_const_default(classes[2], None, SourceOffset(0))
                .is_err()
        );
        assert!(!analyzer.graph.constructors.const_default(classes[2]));
        let error = analyzer
            .check_const_default(classes[3], Some(constructors[3]), SourceOffset(0))
            .unwrap_err();
        assert!(matches!(
            error,
            Error::Diagnostic {
                message: "a const union object initializes no member",
                ..
            }
        ));
        assert!(!analyzer.graph.constructors.const_default(classes[3]));
    }
}
