// Translated from sema/expression_facts.h and conversions.h. See NOTICE.
//! Dense facts on the original AST; conversions never overwrite source types.
use super::{Error, types::TypeId};
use crate::syntax::ast;

#[derive(Clone, Copy, Debug)]
pub(super) struct MemberAddressId(u32);
#[derive(Clone, Copy)]
pub(super) struct MemberAddress {
    pub naming: Option<super::graph::EntityId>,
    pub scope: super::graph::ScopeId,
    pub functions: super::function_resolution::OverloadId,
    // Sole ordinary declarations check at the name. Overload selection checks
    // at its use expression, which may surround the address with parentheses.
    pub location: Option<crate::support::source::SourceOffset>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ValueCategory {
    Lvalue,
    Prvalue,
    Xvalue,
}
impl ValueCategory {
    pub(super) fn spelling(self) -> &'static str {
        match self {
            Self::Lvalue => "lvalue",
            Self::Prvalue => "prvalue",
            Self::Xvalue => "xvalue",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LvalueTransform {
    None,
    Read,
    ArrayToPointer,
    FunctionToPointer,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ValueConversion {
    None,
    IntegralPromotion,
    FloatingPromotion,
    Arithmetic,
    Boolean,
    PointerBoolean,
    NullPointer,
    Pointer,
    DerivedToBase,
    MemberToDerived,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ConversionRank {
    Exact,
    Promotion,
    Conversion,
    UserDefined,
    Ellipsis,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum ReferenceSource {
    Other,
    FunctionLvalue,
    UnqualifiedObject,
}
impl ReferenceSource {
    pub(super) fn is_function_lvalue(self) -> bool {
        matches!(self, Self::FunctionLvalue)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ListInitialization {
    #[default]
    Ordinary,
    InitializerList,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Conversion {
    pub target: TypeId,
    pub result: TypeId,
    pub first: LvalueTransform,
    pub second: ValueConversion,
    pub rank: ConversionRank,
    pub qualification: bool,
    pub initialization: bool,
    pub list_initialization: ListInitialization,
    pub temporary: bool,
    pub reference_source: ReferenceSource,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum ObjectSource {
    Implicit,
    Explicit {
        expression: ast::ExprId,
        arrow: bool,
    },
}
#[derive(Clone, Copy, Debug)]
pub(super) struct ObjectArgument {
    pub source: ObjectSource,
    pub ty: TypeId,
    pub category: ValueCategory,
}
#[derive(Clone, Copy)]
pub(super) struct CallReceiver {
    pub object: ObjectArgument,
    pub path: Option<super::inheritance::BasePathId>,
    pub naming_path: Option<super::inheritance::BasePathId>,
    pub member_path: Option<super::inheritance::BasePathId>,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct BaseCast {
    pub path: super::inheritance::BasePathId,
    pub downcast: bool,
    pub project_zero: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CallForm {
    Ordinary,
    Operator,
    CallableObject,
}
#[derive(Clone, Copy, Debug, Default)]
pub(super) enum Dispatch {
    #[default]
    Direct,
    Virtual,
}
impl Dispatch {
    pub(super) fn is_virtual(self) -> bool {
        matches!(self, Self::Virtual)
    }
}
#[derive(Clone, Copy, Debug)]
pub(super) enum DestructorTarget {
    Scalar,
    Class(super::graph::BindingId),
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum GlvalueKind {
    Ordinary,
    BitField,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum ClassInitialization {
    Constructed(super::constructor_initialization::ConstructorArguments),
    Aggregate(crate::syntax::storage::List<ast::ExprId>),
    FromExpression(ast::ExprId),
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum QueryKind {
    Ordinary,
    Specialization,
}
impl QueryKind {
    pub(crate) fn is_specialization(self) -> bool {
        matches!(self, Self::Specialization)
    }
}
#[derive(Clone, Copy, Debug)]
pub(super) enum ExpressionDependence {
    Fixed,
    Dependent,
}
impl ExpressionDependence {
    pub(super) fn is_dependent(self) -> bool {
        matches!(self, Self::Dependent)
    }
    pub(super) fn note(&mut self, dependent: bool) {
        if dependent {
            *self = Self::Dependent;
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExpressionOrigin {
    Written,
    Implicit,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct ExprFacts {
    pub origin: ExpressionOrigin,
    pub dependence: ExpressionDependence,
    pub ty: TypeId,
    pub category: ValueCategory,
    pub glvalue: GlvalueKind,
    pub field_identity: Option<super::bit_fields::BitFieldId>,
    pub null_literal: bool,
    pub conversion: Option<Conversion>,
    pub commuted: bool,
    pub folded: Option<u64>,
    pub fold_style: FoldStyle,
    // Query constants retain their value without replacing the source query's
    // sizeof-expression presentation. Constant reads do not type its operand again.
    pub query_value: Option<u64>,
    pub query_kind: QueryKind,
    pub reference_cast: Option<TypeId>,
    pub functional_call: bool,
    pub intrinsic: Option<super::floating_intrinsics::FloatingIntrinsic>,
    pub functions: Option<super::function_resolution::OverloadId>,
    pub function: Option<super::graph::BindingId>,
    pub call_form: CallForm,
    pub dispatch: Dispatch,
    pub call: Option<super::function_resolution::CallTarget>,
    pub literal_call: Option<super::literal_calls::LiteralCall>,
    pub destructor: Option<DestructorTarget>,
    pub anonymous_member: Option<super::graph::BindingId>,
    // Explicit designation retains the selected entity independently of spelling.
    pub member: Option<super::graph::BindingId>,
    pub member_naming: Option<super::graph::EntityId>,
    pub object: Option<ObjectArgument>,
    pub base_path: Option<super::inheritance::BasePathId>,
    // The cast's route remains separate from a later conversion on its result.
    pub base_cast: Option<BaseCast>,
    pub member_path: Option<super::inheritance::BasePathId>,
    pub member_naming_path: Option<super::inheritance::BasePathId>,
    // Qualified address formation is distinct from parenthesized designations.
    pub member_address: Option<MemberAddressId>,
    pub constructor: Option<super::construction::ConstructorId>,
    pub class_initialization: Option<ClassInitialization>,
    pub class_copy: Option<super::copy_initialization::CopySelection>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FoldStyle {
    Literal,
    StaticMember,
    EnclosingLocal,
}
impl ExprFacts {
    pub(super) fn is_bit_field(self) -> bool {
        matches!(self.glvalue, GlvalueKind::BitField)
    }
    pub(super) fn set_bit_field(&mut self, value: bool) {
        self.glvalue = if value {
            GlvalueKind::BitField
        } else {
            GlvalueKind::Ordinary
        };
    }

    pub(super) fn new(ty: TypeId, category: ValueCategory) -> Self {
        Self {
            origin: ExpressionOrigin::Written,
            dependence: ExpressionDependence::Fixed,
            ty,
            category,
            glvalue: GlvalueKind::Ordinary,
            field_identity: None,
            null_literal: false,
            conversion: None,
            commuted: false,
            folded: None,
            fold_style: FoldStyle::Literal,
            query_value: None,
            query_kind: QueryKind::Ordinary,
            reference_cast: None,
            functional_call: false,
            intrinsic: None,
            functions: None,
            function: None,
            call_form: CallForm::Ordinary,
            dispatch: Dispatch::Direct,
            call: None,
            literal_call: None,
            destructor: None,
            anonymous_member: None,
            member: None,
            member_naming: None,
            object: None,
            base_path: None,
            base_cast: None,
            member_path: None,
            member_naming_path: None,
            member_address: None,
            constructor: None,
            class_initialization: None,
            class_copy: None,
        }
    }
}
#[derive(Clone, Copy)]
struct ClassConversion {
    constructor: super::construction::ConstructorId,
    arguments: super::constructor_initialization::ConstructorArguments,
    outer: Option<Conversion>,
}
#[derive(Clone, Copy)]
enum CallView {
    Cast,
    MemberData,
}

#[derive(Default)]
pub(super) struct ExpressionFacts {
    // Successful, class-specific designated-list shapes; source operands have
    // stable types before ranking. No candidate publishes conversions here.
    designated_lists: std::collections::HashMap<
        (ast::ExprId, super::graph::EntityId),
        super::aggregate_initialization::AggregateInitialization,
    >,
    // Only explicit union designations need this sparse fact. Normalized list
    // identities are unique per source list and completed candidate class.
    union_lists: std::collections::HashMap<
        crate::syntax::storage::List<ast::ExprId>,
        super::graph::EntityId,
    >,
    // C++ builtin rewrites leave selected callee shells untyped. Keep full
    // Rust designation facts and retain their sparse presentation provenance
    // independently of the typed facts consumed by lowering.
    call_views: std::collections::HashMap<ast::ExprId, CallView>,
    // Atomic calls are sparse; keep the dense fact record at its existing size.
    constant_evaluation_queries: std::collections::HashSet<ast::ExprId>,
    stack_intrinsics:
        std::collections::HashMap<ast::ExprId, super::stack_intrinsics::StackIntrinsic>,
    source_locations:
        std::collections::HashMap<ast::ExprId, super::source_locations::SourceLocation>,
    runtime_intrinsics:
        std::collections::HashMap<ast::ExprId, super::runtime_intrinsics::RuntimeIntrinsic>,
    integer_intrinsics:
        std::collections::HashMap<ast::ExprId, super::integer_intrinsics::BitOperation>,
    atomic_intrinsics:
        std::collections::HashMap<ast::ExprId, super::atomic_operations::AtomicIntrinsic>,
    // Allocated only when a floating builtin publishes a value. Its compact
    // handle is indexed directly without enlarging every expression record.
    floating_values: Vec<Option<super::constant_floats::FloatingId>>,
    call_object_effects: std::collections::HashMap<ast::ExprId, super::call_tables::ObjectEffect>,
    initializer_arrays: std::collections::HashMap<ast::ExprId, TypeId>,
    runtime_casts: std::collections::HashMap<ast::ExprId, super::dynamic_casts::RuntimeCast>,
    type_queries: std::collections::HashMap<ast::ExprId, super::type_queries::TypeQuery>,
    type_identities: std::collections::HashSet<ast::ExprId>,
    records: Vec<Option<ExprFacts>>,
    // Qualified addresses keep their original lookup independently of the
    // specialization set. The dense use fact carries only a compact identity.
    member_addresses: Vec<MemberAddress>,
    // Only nonelided class copies own a lowering action. Selection remains
    // on the source facts, independently of its existing operand conversion.
    copy_actions: std::collections::HashMap<ast::ExprId, super::copy_initialization::CopyAction>,
    // Parentheses share facts, as in the C++ owner. Chains are flattened at publication.
    aliases: Vec<Option<ast::ExprId>>,
    // Only differing-class glvalue conditionals need a base-subobject route.
    conditional_paths:
        std::collections::HashMap<ast::ExprId, [Option<super::inheritance::BasePathId>; 2]>,
    // Converting class-subobject initializations are sparse use facts. Keep
    // them separate from the source's original type and argument conversion.
    class_conversions: std::collections::HashMap<ast::ExprId, ClassConversion>,
    function_conversions:
        std::collections::HashMap<ast::ExprId, super::conversion_functions::FunctionConversion>,
    allocations: std::collections::HashMap<ast::ExprId, super::allocation::Allocation>,
    deletions: std::collections::HashMap<ast::ExprId, super::allocation::Deletion>,
    // The member access owns a bounded selected-call chain over its original base.
    arrow_calls: std::collections::HashMap<ast::ExprId, Vec<super::arrow_calls::ArrowStep>>,
    // A callable object's designation remains independent of operator()'s
    // implicit object conversion, including when the object is itself a member.
    callable_receivers: std::collections::HashMap<ast::ExprId, CallReceiver>,
}
impl ExpressionFacts {
    pub(super) fn union_list_member(
        &self,
        elements: crate::syntax::storage::List<ast::ExprId>,
    ) -> Option<super::graph::EntityId> {
        self.union_lists.get(&elements).copied()
    }
    pub(super) fn set_union_list_member(
        &mut self,
        elements: crate::syntax::storage::List<ast::ExprId>,
        member: super::graph::EntityId,
    ) {
        self.union_lists.insert(elements, member);
    }

    pub(super) fn designated_list(
        &self,
        id: ast::ExprId,
        class: super::graph::EntityId,
    ) -> Option<super::aggregate_initialization::AggregateInitialization> {
        self.designated_lists.get(&(id, class)).copied()
    }
    pub(super) fn set_designated_list(
        &mut self,
        id: ast::ExprId,
        class: super::graph::EntityId,
        plan: super::aggregate_initialization::AggregateInitialization,
    ) {
        self.designated_lists.insert((id, class), plan);
    }

    pub(super) fn is_intrinsic(&self, id: ast::ExprId, facts: ExprFacts) -> bool {
        self.source_location(id).is_some()
            || self.stack_intrinsic(id).is_some()
            || self.runtime_intrinsic(id).is_some()
            || self.constant_evaluation_query(id)
            || facts.intrinsic.is_some()
            || self.integer_intrinsic(id).is_some()
            || self.atomic_intrinsic(id).is_some()
    }

    pub(super) fn source_location(
        &self,
        id: ast::ExprId,
    ) -> Option<super::source_locations::SourceLocation> {
        self.source_locations.get(&self.root(id)).copied()
    }
    pub(super) fn set_source_location(
        &mut self,
        id: ast::ExprId,
        kind: super::source_locations::SourceLocation,
    ) {
        self.source_locations.insert(self.root(id), kind);
    }
    pub(super) fn constant_evaluation_query(&self, id: ast::ExprId) -> bool {
        self.constant_evaluation_queries.contains(&self.root(id))
    }
    pub(super) fn note_constant_evaluation_query(&mut self, id: ast::ExprId) {
        self.constant_evaluation_queries.insert(self.root(id));
    }
    pub(super) fn stack_intrinsic(
        &self,
        id: ast::ExprId,
    ) -> Option<super::stack_intrinsics::StackIntrinsic> {
        self.stack_intrinsics.get(&self.root(id)).copied()
    }
    pub(super) fn set_stack_intrinsic(
        &mut self,
        id: ast::ExprId,
        kind: super::stack_intrinsics::StackIntrinsic,
    ) {
        self.stack_intrinsics.insert(self.root(id), kind);
    }
    pub(super) fn integer_intrinsic(
        &self,
        id: ast::ExprId,
    ) -> Option<super::integer_intrinsics::BitOperation> {
        self.integer_intrinsics.get(&self.root(id)).copied()
    }
    pub(super) fn runtime_intrinsic(
        &self,
        id: ast::ExprId,
    ) -> Option<super::runtime_intrinsics::RuntimeIntrinsic> {
        self.runtime_intrinsics.get(&self.root(id)).copied()
    }
    pub(super) fn member_data_call_view(&self, id: ast::ExprId) -> bool {
        matches!(
            self.call_views.get(&self.root(id)),
            Some(CallView::MemberData)
        )
    }
    pub(super) fn note_member_data_call_view(&mut self, id: ast::ExprId) {
        self.call_views.insert(self.root(id), CallView::MemberData);
    }
    pub(super) fn cast_call_view(&self, id: ast::ExprId) -> bool {
        matches!(self.call_views.get(&self.root(id)), Some(CallView::Cast))
    }
    pub(super) fn note_cast_call_view(&mut self, id: ast::ExprId) {
        self.call_views.insert(self.root(id), CallView::Cast);
    }
    pub(super) fn set_runtime_intrinsic(
        &mut self,
        id: ast::ExprId,
        operation: super::runtime_intrinsics::RuntimeIntrinsic,
    ) {
        self.runtime_intrinsics.insert(self.root(id), operation);
    }
    pub(super) fn set_integer_intrinsic(
        &mut self,
        id: ast::ExprId,
        operation: super::integer_intrinsics::BitOperation,
    ) {
        self.integer_intrinsics.insert(self.root(id), operation);
    }
    pub(super) fn atomic_intrinsic(
        &self,
        id: ast::ExprId,
    ) -> Option<super::atomic_operations::AtomicIntrinsic> {
        self.atomic_intrinsics.get(&self.root(id)).copied()
    }
    pub(super) fn set_atomic_intrinsic(
        &mut self,
        id: ast::ExprId,
        value: super::atomic_operations::AtomicIntrinsic,
    ) {
        let id = self.root(id);
        self.atomic_intrinsics.insert(id, value);
    }

    pub(super) fn floating_value(
        &self,
        id: ast::ExprId,
    ) -> Result<super::constant_floats::FloatingId, Error> {
        self.floating_values
            .get(self.root(id).index())
            .copied()
            .flatten()
            .ok_or(Error::Invariant("floating builtin without constant value"))
    }
    pub(super) fn set_floating_value(
        &mut self,
        id: ast::ExprId,
        value: super::constant_floats::FloatingId,
    ) {
        let id = self.root(id);
        super::record(&mut self.floating_values, id.index(), value);
    }
    pub(super) fn call_object_effect(
        &self,
        id: ast::ExprId,
    ) -> Option<super::call_tables::ObjectEffect> {
        self.call_object_effects.get(&self.root(id)).copied()
    }
    pub(super) fn set_call_object_effect(
        &mut self,
        id: ast::ExprId,
        effect: super::call_tables::ObjectEffect,
    ) {
        self.call_object_effects.insert(self.root(id), effect);
    }
    pub(super) fn initializer_array(&self, id: ast::ExprId) -> Option<TypeId> {
        self.initializer_arrays.get(&self.root(id)).copied()
    }
    pub(super) fn set_initializer_array(&mut self, id: ast::ExprId, array: TypeId) {
        self.initializer_arrays.insert(self.root(id), array);
    }

    pub(super) fn type_query(&self, id: ast::ExprId) -> Option<super::type_queries::TypeQuery> {
        self.type_queries.get(&self.root(id)).copied()
    }
    pub(super) fn set_type_query(
        &mut self,
        id: ast::ExprId,
        query: super::type_queries::TypeQuery,
    ) {
        self.type_queries.insert(self.root(id), query);
    }
    pub(super) fn type_identity(&self, id: ast::ExprId) -> bool {
        self.type_identities.contains(&self.root(id))
    }
    pub(super) fn set_type_identity(&mut self, id: ast::ExprId) {
        self.type_identities.insert(self.root(id));
    }
    pub(super) fn runtime_cast(
        &self,
        id: ast::ExprId,
    ) -> Option<super::dynamic_casts::RuntimeCast> {
        self.runtime_casts.get(&self.root(id)).copied()
    }
    pub(super) fn set_runtime_cast(
        &mut self,
        id: ast::ExprId,
        cast: super::dynamic_casts::RuntimeCast,
    ) {
        self.runtime_casts.insert(self.root(id), cast);
    }
    pub(super) fn add_member_address(
        &mut self,
        address: MemberAddress,
    ) -> Result<MemberAddressId, Error> {
        let id = u32::try_from(self.member_addresses.len())
            .map_err(|_| Error::Limit("member address limit"))?;
        self.member_addresses.push(address);
        Ok(MemberAddressId(id))
    }
    pub(super) fn member_address(&self, id: MemberAddressId) -> MemberAddress {
        self.member_addresses[id.0 as usize]
    }
    pub(super) fn callable_receiver(&self, id: ast::ExprId) -> Option<CallReceiver> {
        self.callable_receivers.get(&self.root(id)).copied()
    }
    pub(super) fn set_callable_receiver(&mut self, id: ast::ExprId, receiver: CallReceiver) {
        self.callable_receivers.insert(self.root(id), receiver);
    }
    pub(super) fn function_conversion(
        &self,
        id: ast::ExprId,
    ) -> Option<super::conversion_functions::FunctionConversion> {
        self.function_conversions.get(&self.root(id)).copied()
    }
    pub(super) fn set_function_conversion(
        &mut self,
        id: ast::ExprId,
        conversion: super::conversion_functions::FunctionConversion,
    ) {
        self.function_conversions.insert(self.root(id), conversion);
    }
    pub(super) fn deletion(&self, id: ast::ExprId) -> Result<super::allocation::Deletion, Error> {
        self.deletions
            .get(&self.root(id))
            .copied()
            .ok_or(Error::Invariant("deletion without selected use"))
    }
    pub(super) fn set_deletion(&mut self, id: ast::ExprId, deletion: super::allocation::Deletion) {
        self.deletions.insert(self.root(id), deletion);
    }
    pub(super) fn arrow_calls(&self, base: ast::ExprId) -> &[super::arrow_calls::ArrowStep] {
        self.arrow_calls
            .get(&self.root(base))
            .map_or(&[], Vec::as_slice)
    }
    pub(super) fn set_arrow_calls(
        &mut self,
        base: ast::ExprId,
        calls: Vec<super::arrow_calls::ArrowStep>,
    ) {
        self.arrow_calls.insert(self.root(base), calls);
    }
    pub(super) fn allocation(
        &self,
        id: ast::ExprId,
    ) -> Result<super::allocation::Allocation, Error> {
        self.allocations
            .get(&self.root(id))
            .copied()
            .ok_or(Error::Invariant("allocation without selected use"))
    }
    pub(super) fn set_allocation(
        &mut self,
        id: ast::ExprId,
        allocation: super::allocation::Allocation,
    ) {
        self.allocations.insert(self.root(id), allocation);
    }
    pub(super) fn copy_action(
        &self,
        id: ast::ExprId,
    ) -> Option<super::copy_initialization::CopyAction> {
        self.copy_actions.get(&self.root(id)).copied()
    }
    pub(super) fn set_copy_action(
        &mut self,
        id: ast::ExprId,
        action: Option<super::copy_initialization::CopyAction>,
    ) {
        let id = self.root(id);
        if let Some(action) = action {
            self.copy_actions.insert(id, action);
        } else {
            self.copy_actions.remove(&id);
        }
    }
    pub(super) fn conditional_paths(
        &self,
        id: ast::ExprId,
    ) -> [Option<super::inheritance::BasePathId>; 2] {
        self.conditional_paths
            .get(&self.root(id))
            .copied()
            .unwrap_or([None; 2])
    }
    pub(super) fn set_conditional_paths(
        &mut self,
        id: ast::ExprId,
        paths: [Option<super::inheritance::BasePathId>; 2],
    ) {
        self.conditional_paths.insert(self.root(id), paths);
    }
    pub(super) fn class_conversion(
        &self,
        id: ast::ExprId,
    ) -> Option<(
        super::construction::ConstructorId,
        super::constructor_initialization::ConstructorArguments,
    )> {
        self.class_conversions
            .get(&self.root(id))
            .map(|record| (record.constructor, record.arguments))
    }
    pub(super) fn set_class_conversion(
        &mut self,
        id: ast::ExprId,
        selected: (
            super::construction::ConstructorId,
            super::constructor_initialization::ConstructorArguments,
        ),
    ) {
        self.class_conversions.insert(
            self.root(id),
            ClassConversion {
                constructor: selected.0,
                arguments: selected.1,
                outer: None,
            },
        );
    }
    pub(super) fn user_conversion(&self, id: ast::ExprId) -> Option<Conversion> {
        self.class_conversions
            .get(&self.root(id))
            .and_then(|record| record.outer)
    }
    pub(super) fn set_user_conversion(
        &mut self,
        id: ast::ExprId,
        constructor: super::construction::ConstructorId,
        outer: Conversion,
    ) {
        self.class_conversions.insert(
            self.root(id),
            ClassConversion {
                constructor,
                arguments: super::constructor_initialization::ConstructorArguments::Single(id),
                outer: Some(outer),
            },
        );
    }
    pub(super) fn root(&self, id: ast::ExprId) -> ast::ExprId {
        self.aliases
            .get(id.index())
            .copied()
            .flatten()
            .unwrap_or(id)
    }
    pub(super) fn get(&self, id: ast::ExprId) -> Result<ExprFacts, Error> {
        self.known(id)
            .ok_or(Error::Invariant("expression without semantic facts"))
    }
    pub(super) fn known(&self, id: ast::ExprId) -> Option<ExprFacts> {
        self.records.get(self.root(id).index()).copied().flatten()
    }
    pub(super) fn set(&mut self, id: ast::ExprId, facts: ExprFacts) {
        let id = self.root(id);
        self.call_object_effects.remove(&id);
        super::record(&mut self.records, id.index(), facts);
    }
    pub(super) fn share(&mut self, id: ast::ExprId, inner: ast::ExprId) {
        let root = self.root(inner);
        super::record(&mut self.aliases, id.index(), root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        lex::vocabulary::Keyword, support::source::SourceOffset, syntax::storage::Store,
        types::FundamentalType,
    };
    #[test]
    fn parentheses_share_one_conversion_record_without_recursive_alias_lookup() {
        let mut types = super::super::types::Types::default();
        let int = types
            .scalar(super::super::types::Scalar::Basic(FundamentalType::Int))
            .unwrap();
        let long = types
            .scalar(super::super::types::Scalar::Basic(FundamentalType::LongInt))
            .unwrap();
        let mut syntax = Store::default();
        let leaf = syntax
            .add(ast::Expression {
                location: SourceOffset(0),
                kind: ast::ExprKind::Keyword(Keyword::True),
            })
            .unwrap();
        let mut facts = ExpressionFacts::default();
        facts.set(leaf, ExprFacts::new(int, ValueCategory::Lvalue));
        let mut outer = leaf;
        for _ in 0..4096 {
            let parent = syntax
                .add(ast::Expression {
                    location: SourceOffset(0),
                    kind: ast::ExprKind::Paren(outer),
                })
                .unwrap();
            facts.share(parent, outer);
            assert_eq!(facts.root(parent), leaf);
            outer = parent;
        }
        let mut updated = facts.get(outer).unwrap();
        updated.conversion = Some(Conversion {
            target: long,
            result: long,
            first: LvalueTransform::Read,
            second: ValueConversion::Arithmetic,
            rank: ConversionRank::Conversion,
            qualification: false,
            initialization: true,
            list_initialization: crate::sema::expression_facts::ListInitialization::Ordinary,
            temporary: false,
            reference_source: ReferenceSource::Other,
        });
        facts.set(outer, updated);
        assert_eq!(facts.get(leaf).unwrap().ty, int);
        assert_eq!(facts.get(leaf).unwrap().category, ValueCategory::Lvalue);
        let conversion = facts.get(leaf).unwrap().conversion.unwrap();
        assert_eq!(conversion.target, long);
        assert_eq!(conversion.rank, ConversionRank::Conversion);
        assert!(!conversion.qualification);
        assert_eq!(
            facts.records.iter().filter(|fact| fact.is_some()).count(),
            1
        );
    }
}
