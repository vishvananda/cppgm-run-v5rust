// Translated from sema/analyzer_rtti.cpp. See NOTICE.
//! Query facts retain canonical target types; deferred uses preserve operand typing once.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory},
    graph::{BindingId, BindingKind, EntityId, Graph, ScopeId},
    lookup::Filter,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{pp_lexer::Punctuator, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind},
};

#[derive(Clone, Copy)]
pub(crate) struct TypeQuery {
    pub target: TypeId,
    pub dynamic: bool,
}
#[derive(Clone, Copy)]
pub(super) enum DeferredUse {
    Function(BindingId, bool),
    Query(ast::ExprId),
    RuntimeCast(ast::ExprId),
    Layout(EntityId, SourceOffset),
    Constructor(super::construction::ConstructorId),
    Local(ast::ExprId, BindingId, ScopeId),
    This(SourceOffset),
    Variable(EntityId),
    Error(SourceOffset, &'static str),
}
#[derive(Default)]
pub(super) struct QueryContext {
    pub deferring: u32,
    pub uses: Vec<DeferredUse>,
    layouts: std::collections::HashSet<TypeId>,
    requests: std::collections::HashSet<(Option<EntityId>, TypeId)>,
}
impl Graph {
    pub(crate) fn lower_class_complete(&self, class: EntityId) -> bool {
        self.entity(class).complete
    }
    pub(crate) fn lower_rtti_type(&self, ty: TypeId) -> Result<TypeId, Error> {
        self.rtti_types
            .get(&ty)
            .copied()
            .ok_or(Error::Invariant("RTTI type without prepared identity"))
    }
    pub(crate) fn lower_type_query(&self, id: ast::ExprId) -> Option<TypeQuery> {
        self.expression_facts.type_query(id)
    }
    pub(crate) fn lower_type_identity(&self, id: ast::ExprId) -> bool {
        self.expression_facts.type_identity(id)
    }
}
impl Analyzer<'_> {
    pub(super) fn constant_typeid_place(
        &self,
        id: ast::ExprId,
    ) -> Result<super::constant_addresses::Address, Error> {
        use super::constant_addresses::{Address, Root, nonconstant};
        let query = self
            .graph
            .expression_facts
            .type_query(id)
            .ok_or(Error::Invariant("missing constant type query"))?;
        if query.dynamic {
            return Err(nonconstant(
                self.tree.expressions.get(id).location,
                "a glvalue outside constant evaluation",
            ));
        }
        Ok(Address::new(
            Root::TypeInformation(query.target),
            self.graph.expression_facts.get(id)?.ty,
        ))
    }
    pub(super) fn deferring_type_query(&self) -> bool {
        self.unevaluated != 0 && self.unevaluated == self.type_queries.deferring
    }
    fn type_info_class(&mut self, location: SourceOffset) -> Result<EntityId, Error> {
        if let Some(class) = self.graph.analysis.type_info {
            return Ok(class);
        }
        let std = self
            .identifiers
            .intern(b"std")
            .map_err(|_| Error::Limit("standard namespace name"))?;
        let info = self
            .identifiers
            .intern(b"type_info")
            .map_err(|_| Error::Limit("type information name"))?;
        let space =
            self.qualified_declaration_result(self.graph.global, std.into(), Filter::Namespace)?;
        let scope = space.binding.and_then(|binding| {
            let bound = self.graph.binding(binding);
            (bound.kind == BindingKind::Namespace)
                .then(|| self.graph.entity(bound.entity).scope)
                .flatten()
        });
        if let Some(scope) = scope {
            let found =
                self.qualified_declaration_result(scope, info.into(), Filter::Elaborated)?;
            if let Some(binding) = found.binding.filter(|_| !found.ambiguous)
                && self.graph.binding(binding).kind == BindingKind::Type
                && let Some(class) = self
                    .graph
                    .binding(binding)
                    .ty
                    .and_then(|ty| self.graph.class_of(ty))
            {
                self.graph.analysis.type_info = Some(class);
                return Ok(class);
            }
        }
        Err(Error::at(
            location,
            "typeid without a declaration of std::type_info (5.2.8p6)",
        ))
    }
    pub(super) fn prepare_type_information(&mut self, ty: TypeId) -> Result<(), Error> {
        self.check_stack()?;
        if self.graph.rtti_types.contains_key(&ty) {
            return Ok(());
        }
        let bare = self.bare_type(ty)?;
        self.graph.rtti_types.insert(ty, bare);
        self.graph.rtti_types.insert(bare, bare);
        match self.graph.types.get(bare).kind {
            TypeKind::Pointer(element) => self.prepare_type_information(element)?,
            TypeKind::MemberPointer { owner, element } => {
                self.prepare_type_information(element)?;
                self.prepare_type_information(owner)?;
            }
            _ => (),
        }
        Ok(())
    }
    pub(super) fn prepare_rtti_layouts(
        &mut self,
        ty: TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_stack()?;
        if !self.type_queries.layouts.insert(ty) {
            return Ok(());
        }
        match self.graph.types.get(ty).kind {
            TypeKind::Class(class) if self.graph.entity(class).complete => {
                self.class_layout(class, location)?;
            }
            TypeKind::Pointer(element) => self.prepare_rtti_layouts(element, location)?,
            TypeKind::MemberPointer { owner, element } => {
                self.prepare_rtti_layouts(element, location)?;
                self.prepare_rtti_layouts(owner, location)?;
            }
            _ => (),
        }
        Ok(())
    }
    pub(super) fn typeid_expression(
        &mut self,
        id: ast::ExprId,
        ty: Option<ast::TypeId>,
        operand: Option<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let location = self.tree.expressions.get(id).location;
        let info = self.type_info_class(location)?;
        let (queried, dynamic) = if let Some(ty) = ty {
            let mut queried = self.type_syntax(ty, scope)?;
            if let TypeKind::Reference { element, .. } = self.graph.types.get(queried).kind {
                queried = element;
            }
            if let Some(class) = self.graph.class_of(queried)
                && !self.complete_class(class)?
            {
                return Err(Error::at(location, "typeid of an incomplete class type"));
            }
            (queried, false)
        } else {
            self.typeid_operand(
                location,
                operand.ok_or(Error::Invariant("typeid without operand"))?,
                scope,
            )?
        };
        let target = self.bare_type(queried)?;
        self.graph
            .expression_facts
            .set_type_query(id, TypeQuery { target, dynamic });
        self.demand_type_query(id)?;
        let ty = self
            .graph
            .types
            .qualify(self.graph.lower_entity_type(info)?, Cv::CONST)?;
        self.new_expression_facts(ty, ValueCategory::Lvalue)
    }
    fn demand_type_query(&mut self, id: ast::ExprId) -> Result<(), Error> {
        let TypeQuery { target, dynamic } = self
            .graph
            .expression_facts
            .type_query(id)
            .ok_or(Error::Invariant("missing query demand"))?;
        let location = self.tree.expressions.get(id).location;
        if self.deferring_type_query() {
            self.type_queries.uses.push(DeferredUse::Query(id));
        }
        if !dynamic && self.unevaluated == 0 {
            self.prepare_type_information(target)?;
            let owner = self.body_context.as_ref().and_then(|body| body.function);
            if self.type_queries.requests.insert((owner, target)) {
                self.graph.storage_layouts.push((
                    owner,
                    super::layout::StorageDemand::TypeInformation(target),
                    location,
                ));
            }
        }
        if self.unevaluated == 0
            && dynamic
            && let Some(class) = self.graph.class_of(target)
        {
            self.graph
                .class_information_uses
                .push((self.body_context.as_ref().and_then(|b| b.function), class));
        }
        Ok(())
    }
    fn typeid_operand(
        &mut self,
        location: SourceOffset,
        operand: ast::ExprId,
        scope: ScopeId,
    ) -> Result<(TypeId, bool), Error> {
        let first = self.type_queries.uses.len();
        let saved = self.type_queries.deferring;
        self.unevaluated += 1;
        self.type_queries.deferring = self.unevaluated;
        let result = self.typed_expression(operand, scope);
        self.type_queries.deferring = saved;
        self.unevaluated -= 1;
        let result = (|| {
            let mut facts = result?;
            let class = self.graph.class_of(facts.ty);
            if let Some(class) = class
                && !self.complete_class(class)?
            {
                return Err(Error::at(
                    self.tree.expressions.get(operand).location,
                    "typeid of an expression of incomplete class type",
                ));
            }
            let dynamic = class.is_some_and(|class| self.graph.lower_polymorphic(class))
                && facts.category != ValueCategory::Prvalue;
            if dynamic {
                self.replay_query_uses(first)?;
            }
            // Resolve a sole function without odr-using it. Non-static member
            // designations and unresolved overload sets are not object operands.
            if let Some(set) = facts.functions {
                let bindings = self.graph.overloads.get(set);
                if let [binding] = bindings
                    && self
                        .graph
                        .entity(self.graph.binding(*binding).entity)
                        .template_head
                        .is_none()
                {
                    facts.function = Some(
                        self.graph
                            .function_origin(self.graph.binding(*binding).entity)?,
                    );
                    facts.functions = None;
                    self.graph.expression_facts.set(operand, facts);
                }
            }
            let static_member = facts.function.is_some_and(|binding| {
                self.graph
                    .entity(self.graph.binding(binding).entity)
                    .function
                    .is_some_and(|props| props.is_static)
            });
            if facts.functions.is_some()
                || facts.object.is_some() && facts.function.is_some() && !static_member
            {
                return Err(Error::at(
                    location,
                    "typeid of a function name that is not an object",
                ));
            }
            Ok((facts.ty, dynamic))
        })();
        self.type_queries.uses.truncate(first);
        result
    }
    fn replay_query_uses(&mut self, first: usize) -> Result<(), Error> {
        let count = self.type_queries.uses.len();
        for index in first..count {
            match self.type_queries.uses[index] {
                DeferredUse::Function(binding, call) => {
                    self.demand_specialization_body(binding);
                    self.demand_function(binding, call)?;
                }
                DeferredUse::Local(id, binding, scope) => {
                    let mut facts = self.graph.expression_facts.get(id)?;
                    self.local_name(id, &mut facts, binding, scope)?;
                    self.graph.expression_facts.set(id, facts);
                }
                DeferredUse::Query(id) => self.demand_type_query(id)?,
                DeferredUse::RuntimeCast(id) => self.demand_runtime_cast(id)?,
                DeferredUse::Layout(entity, location) => {
                    self.demand_storage_layout(entity, location)?;
                }
                DeferredUse::Constructor(id) => {
                    self.demand_constructor_use(id)?;
                    let class = self.graph.constructors.get(id).class;
                    self.request_object_layout(
                        self.graph.lower_entity_type(class)?,
                        SourceOffset(0),
                    );
                }
                DeferredUse::This(location) => self.capture_this(location)?,
                DeferredUse::Variable(entity) => self.use_instance_variable(entity)?,
                DeferredUse::Error(location, message) => self.evaluated_error(location, message)?,
            }
        }
        Ok(())
    }
    pub(super) fn evaluated_error(
        &mut self,
        location: SourceOffset,
        message: &'static str,
    ) -> Result<(), Error> {
        if self.unevaluated == 0 {
            return Err(Error::at(location, message));
        }
        if self.deferring_type_query() {
            self.type_queries
                .uses
                .push(DeferredUse::Error(location, message));
        }
        Ok(())
    }
    pub(super) fn type_identity(
        &mut self,
        id: ast::ExprId,
        op: Punctuator,
        operands: &[ast::ExprId],
        selected: BindingId,
    ) -> Result<Option<ExprFacts>, Error> {
        if !matches!(op, Punctuator::Equal | Punctuator::NotEqual)
            || operands.len() != 2
            || !operands.iter().all(|&id| {
                matches!(
                    self.tree
                        .expressions
                        .get(self.graph.expression_facts.root(id))
                        .kind,
                    ExprKind::TypeTrait {
                        keyword: Keyword::Typeid,
                        ..
                    }
                )
            })
        {
            return Ok(None);
        }
        let Some(info) = self.graph.analysis.type_info else {
            return Ok(None);
        };
        if self
            .member_function_owner(selected)
            .and_then(|ty| self.graph.class_of(ty))
            != Some(info)
        {
            return Ok(None);
        }
        self.graph.expression_facts.set_type_identity(id);
        let ty = self.graph.types.scalar(Scalar::Bool)?;
        Ok(Some(self.new_expression_facts(ty, ValueCategory::Prvalue)?))
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
    fn positive_type_info_identity_survives_resumed_lookup_context() {
        let source = SourceBuffer::new(b"namespace std{class type_info{};}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"type-info-resume").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        analyzer
            .declarations(declarations, analyzer.graph.global)
            .unwrap();
        let identity = analyzer.type_info_class(SourceOffset(0)).unwrap();
        let mut graph = analyzer.finish_analysis(false).unwrap();
        // A later demand can use a declaration context preceding std's
        // declaration. The reference keeps its successful identity lookup.
        graph.lookup_limit = Some(0);
        let mut resumed =
            Analyzer::with_graph(tree, cursor, StackBudget::default(), Some(graph)).unwrap();
        assert_eq!(resumed.type_info_class(SourceOffset(0)).unwrap(), identity);
        assert_eq!(resumed.graph.lookup_limit, Some(0));
    }

    #[test]
    fn failed_operands_restore_deferred_use_context_without_reparsing() {
        for text in [
            "namespace std{class type_info;}struct P;const std::type_info&f(P&p){return typeid(p);}",
            "namespace std{class type_info;}const std::type_info&f(){return typeid(missing());}",
        ] {
            let source = SourceBuffer::new(text.as_bytes().to_vec()).unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"query-rollback").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let count = tree.expressions.mark();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            analyzer.set_lowering();
            let marker = 0_u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let global = analyzer.graph.global;
            assert!(analyzer.declarations(declarations, global).is_err());
            assert_eq!(analyzer.unevaluated, 0);
            assert_eq!(analyzer.type_queries.deferring, 0);
            assert!(analyzer.type_queries.uses.is_empty());
            assert_eq!(analyzer.tree.expressions.mark(), count);
        }
    }
}
