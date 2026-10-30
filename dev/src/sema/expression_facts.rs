// Translated from sema/expression_facts.h and conversions.h. See NOTICE.
//! Dense facts on the original AST; conversions never overwrite source types.
use super::{Error, types::TypeId};
use crate::syntax::ast;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum LvalueTransform {
    None,
    Read,
    ArrayToPointer,
    FunctionToPointer,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum ValueConversion {
    None,
    IntegralPromotion,
    FloatingPromotion,
    Arithmetic,
    Complex,
    Boolean,
    PointerBoolean,
    NullPointer,
    Pointer,
    DerivedToBase,
    MemberToDerived,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum ConversionRank {
    Exact,
    Promotion,
    Conversion,
    ComplexReal,
    UserDefined,
    Ellipsis,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum ListInitialization {
    #[default]
    Ordinary,
    InitializerList,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ObjectSource {
    Implicit,
    Explicit {
        expression: ast::ExprId,
        arrow: bool,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum DestructorTarget {
    Scalar,
    Class(super::graph::BindingId),
}
#[derive(Clone, Copy, Debug)]
pub enum GlvalueKind {
    Ordinary,
    BitField,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClassInitialization {
    Constructed(super::constructor_initialization::ConstructorArguments),
    Aggregate(crate::syntax::storage::List<ast::ExprId>),
    FlexibleAggregate(crate::syntax::storage::List<ast::ExprId>),
    FromExpression(ast::ExprId),
}
#[derive(Clone, Copy, Debug)]
pub enum QueryKind {
    Ordinary,
    Specialization,
}
impl QueryKind {
    #[inline]
    pub fn is_specialization(self) -> bool {
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
pub enum ExpressionOrigin {
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
    pub conversion: Option<Conversion>,
    flags: u16,
    details: Option<super::expression_details::DetailsId>,
    folded_value: u64,
}
const _: () = assert!(std::mem::size_of::<ExprFacts>() <= 40);
const _: () = assert!(std::mem::size_of::<Option<ExprFacts>>() <= 40);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FoldStyle {
    Literal,
    StaticMember,
    EnclosingLocal,
}
impl ExprFacts {
    pub(super) fn display_details(
        self,
        store: &ExpressionFacts,
    ) -> super::expression_details::Details {
        *store.details.get(self.details)
    }
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
            conversion: None,
            flags: 0,
            details: None,
            folded_value: 0,
        }
    }
    pub(super) fn folded(self) -> Option<u64> {
        (self.flags & 1 != 0).then_some(self.folded_value)
    }
    pub(super) fn set_folded(&mut self, value: Option<u64>) {
        self.flags = (self.flags & !1) | u16::from(value.is_some());
        self.folded_value = value.unwrap_or(0);
    }

    pub(super) fn null_literal(self) -> bool {
        self.flags & (1 << 1) != 0
    }
    pub(super) fn set_null_literal(&mut self, value: bool) {
        self.flags = (self.flags & !(1 << 1)) | (u16::from(value) << 1);
    }
    pub(super) fn commuted(self) -> bool {
        self.flags & (1 << 2) != 0
    }
    pub(super) fn set_commuted(&mut self, value: bool) {
        self.flags = (self.flags & !(1 << 2)) | (u16::from(value) << 2);
    }
    pub(super) fn fold_style(self) -> FoldStyle {
        match (self.flags >> 3) & 3 {
            1 => FoldStyle::StaticMember,
            2 => FoldStyle::EnclosingLocal,
            _ => FoldStyle::Literal,
        }
    }
    pub(super) fn set_fold_style(&mut self, value: FoldStyle) {
        self.flags = (self.flags & !(3 << 3))
            | ((match value {
                FoldStyle::Literal => 0,
                FoldStyle::StaticMember => 1,
                FoldStyle::EnclosingLocal => 2,
            }) << 3);
    }
    pub(super) fn query_kind(self) -> QueryKind {
        match (self.flags >> 5) & 1 {
            1 => QueryKind::Specialization,
            _ => QueryKind::Ordinary,
        }
    }
    pub(super) fn set_query_kind(&mut self, value: QueryKind) {
        self.flags = (self.flags & !(1 << 5))
            | ((match value {
                QueryKind::Ordinary => 0,
                QueryKind::Specialization => 1,
            }) << 5);
    }
    pub(super) fn functional_call(self) -> bool {
        self.flags & (1 << 6) != 0
    }
    pub(super) fn set_functional_call(&mut self, value: bool) {
        self.flags = (self.flags & !(1 << 6)) | (u16::from(value) << 6);
    }
    pub(super) fn call_form(self) -> CallForm {
        match (self.flags >> 7) & 3 {
            1 => CallForm::Operator,
            2 => CallForm::CallableObject,
            _ => CallForm::Ordinary,
        }
    }
    pub(super) fn set_call_form(&mut self, value: CallForm) {
        self.flags = (self.flags & !(3 << 7))
            | ((match value {
                CallForm::Ordinary => 0,
                CallForm::Operator => 1,
                CallForm::CallableObject => 2,
            }) << 7);
    }
    pub(super) fn dispatch(self) -> Dispatch {
        match (self.flags >> 9) & 1 {
            1 => Dispatch::Virtual,
            _ => Dispatch::Direct,
        }
    }
    pub(super) fn set_dispatch(&mut self, value: Dispatch) {
        self.flags = (self.flags & !(1 << 9))
            | ((match value {
                Dispatch::Direct => 0,
                Dispatch::Virtual => 1,
            }) << 9);
    }
    pub(super) fn field_identity(
        self,
        store: &ExpressionFacts,
    ) -> Option<super::bit_fields::BitFieldId> {
        store.details.get(self.details).field_identity
    }
    pub(super) fn set_field_identity(
        &mut self,
        value: Option<super::bit_fields::BitFieldId>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.field_identity = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn query_value(self, store: &ExpressionFacts) -> Option<u64> {
        store.details.get(self.details).query_value
    }
    pub(super) fn set_query_value(
        &mut self,
        value: Option<u64>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.query_value = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn reference_cast(self, store: &ExpressionFacts) -> Option<TypeId> {
        store.details.get(self.details).reference_cast
    }
    pub(super) fn set_reference_cast(
        &mut self,
        value: Option<TypeId>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.reference_cast = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn intrinsic(
        self,
        store: &ExpressionFacts,
    ) -> Option<super::floating_intrinsics::FloatingIntrinsic> {
        store.details.get(self.details).intrinsic
    }
    pub(super) fn set_intrinsic(
        &mut self,
        value: Option<super::floating_intrinsics::FloatingIntrinsic>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.intrinsic = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn functions(
        self,
        store: &ExpressionFacts,
    ) -> Option<super::function_resolution::OverloadId> {
        store.details.get(self.details).functions
    }
    pub(super) fn set_functions(
        &mut self,
        value: Option<super::function_resolution::OverloadId>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.functions = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn function(self, store: &ExpressionFacts) -> Option<super::graph::BindingId> {
        store.details.get(self.details).function
    }
    pub(super) fn set_function(
        &mut self,
        value: Option<super::graph::BindingId>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.function = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn call(
        self,
        store: &ExpressionFacts,
    ) -> Option<super::function_resolution::CallTarget> {
        store.details.get(self.details).call
    }
    pub(super) fn set_call(
        &mut self,
        value: Option<super::function_resolution::CallTarget>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.call = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn literal_call(
        self,
        store: &ExpressionFacts,
    ) -> Option<super::literal_calls::LiteralCall> {
        store.details.get(self.details).literal_call
    }
    pub(super) fn set_literal_call(
        &mut self,
        value: Option<super::literal_calls::LiteralCall>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.literal_call = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn destructor(self, store: &ExpressionFacts) -> Option<DestructorTarget> {
        store.details.get(self.details).destructor
    }
    pub(super) fn set_destructor(
        &mut self,
        value: Option<DestructorTarget>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.destructor = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn anonymous_member(
        self,
        store: &ExpressionFacts,
    ) -> Option<super::graph::BindingId> {
        store.details.get(self.details).anonymous_member
    }
    pub(super) fn set_anonymous_member(
        &mut self,
        value: Option<super::graph::BindingId>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.anonymous_member = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn member(self, store: &ExpressionFacts) -> Option<super::graph::BindingId> {
        store.details.get(self.details).member
    }
    pub(super) fn set_member(
        &mut self,
        value: Option<super::graph::BindingId>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.member = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn member_naming(self, store: &ExpressionFacts) -> Option<super::graph::EntityId> {
        store.details.get(self.details).member_naming
    }
    pub(super) fn set_member_naming(
        &mut self,
        value: Option<super::graph::EntityId>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.member_naming = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn object(self, store: &ExpressionFacts) -> Option<ObjectArgument> {
        store.details.get(self.details).object
    }
    pub(super) fn set_object(
        &mut self,
        value: Option<ObjectArgument>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.object = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn base_path(
        self,
        store: &ExpressionFacts,
    ) -> Option<super::inheritance::BasePathId> {
        store.details.get(self.details).base_path
    }
    pub(super) fn set_base_path(
        &mut self,
        value: Option<super::inheritance::BasePathId>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.base_path = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn base_cast(self, store: &ExpressionFacts) -> Option<BaseCast> {
        store.details.get(self.details).base_cast
    }
    pub(super) fn set_base_cast(
        &mut self,
        value: Option<BaseCast>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.base_cast = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn member_path(
        self,
        store: &ExpressionFacts,
    ) -> Option<super::inheritance::BasePathId> {
        store.details.get(self.details).member_path
    }
    pub(super) fn set_member_path(
        &mut self,
        value: Option<super::inheritance::BasePathId>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.member_path = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn member_naming_path(
        self,
        store: &ExpressionFacts,
    ) -> Option<super::inheritance::BasePathId> {
        store.details.get(self.details).member_naming_path
    }
    pub(super) fn set_member_naming_path(
        &mut self,
        value: Option<super::inheritance::BasePathId>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.member_naming_path = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn member_address(self, store: &ExpressionFacts) -> Option<MemberAddressId> {
        store.details.get(self.details).member_address
    }
    pub(super) fn set_member_address(
        &mut self,
        value: Option<MemberAddressId>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.member_address = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn constructor(
        self,
        store: &ExpressionFacts,
    ) -> Option<super::construction::ConstructorId> {
        store.details.get(self.details).constructor
    }
    pub(super) fn set_constructor(
        &mut self,
        value: Option<super::construction::ConstructorId>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.constructor = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn class_initialization(
        self,
        store: &ExpressionFacts,
    ) -> Option<ClassInitialization> {
        store.details.get(self.details).class_initialization
    }
    pub(super) fn set_class_initialization(
        &mut self,
        value: Option<ClassInitialization>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.class_initialization = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
    pub(super) fn class_copy(
        self,
        store: &ExpressionFacts,
    ) -> Option<super::copy_initialization::CopySelection> {
        store.details.get(self.details).class_copy
    }
    pub(super) fn set_class_copy(
        &mut self,
        value: Option<super::copy_initialization::CopySelection>,
        store: &mut ExpressionFacts,
    ) -> Result<(), Error> {
        let mut data = *store.details.get(self.details);
        data.class_copy = value;
        self.details = store.details.intern(data)?;
        Ok(())
    }
}
#[derive(Clone, Copy)]
struct ClassConversion {
    constructor: super::construction::ConstructorId,
    arguments: super::constructor_initialization::ConstructorArguments,
    outer: Option<Conversion>,
    parameter_copy: Option<super::copy_initialization::InternalCopy>,
}
#[derive(Clone, Copy)]
enum CallView {
    Cast,
    MemberData,
}

#[derive(Default)]
pub(super) struct ExpressionFacts {
    details: super::expression_details::DetailsStore,
    // Successful, class-specific designated-list shapes; source operands have
    // stable types before ranking. No candidate publishes conversions here.
    designated_lists: crate::support::hash::HashMap<
        (ast::ExprId, super::graph::EntityId),
        super::aggregate_initialization::AggregateInitialization,
    >,
    // Only explicit union designations need this sparse fact. Normalized list
    // identities are unique per source list and completed candidate class.
    union_lists: crate::support::hash::HashMap<
        crate::syntax::storage::List<ast::ExprId>,
        super::graph::EntityId,
    >,
    // C++ builtin rewrites leave selected callee shells untyped. Keep full
    // Rust designation facts and retain their sparse presentation provenance
    // independently of the typed facts consumed by lowering.
    call_views: crate::support::hash::HashMap<ast::ExprId, CallView>,
    // Atomic calls are sparse; keep the dense fact record at its existing size.
    complex_constructions: crate::support::hash::HashSet<ast::ExprId>,
    constant_evaluation_queries: crate::support::hash::HashSet<ast::ExprId>,
    stack_intrinsics:
        crate::support::hash::HashMap<ast::ExprId, super::stack_intrinsics::StackIntrinsic>,
    source_locations:
        crate::support::hash::HashMap<ast::ExprId, super::source_locations::SourceLocation>,
    runtime_intrinsics:
        crate::support::hash::HashMap<ast::ExprId, super::runtime_intrinsics::RuntimeIntrinsic>,
    integer_intrinsics:
        crate::support::hash::HashMap<ast::ExprId, super::integer_intrinsics::BitOperation>,
    atomic_intrinsics:
        crate::support::hash::HashMap<ast::ExprId, super::atomic_operations::AtomicIntrinsic>,
    // Allocated only when a floating builtin publishes a value. Its compact
    // handle is indexed directly without enlarging every expression record.
    floating_values: Vec<Option<super::constant_floats::FloatingId>>,
    call_object_effects:
        crate::support::hash::HashMap<ast::ExprId, super::call_tables::ObjectEffect>,
    initializer_arrays: crate::support::hash::HashMap<ast::ExprId, TypeId>,
    runtime_casts: crate::support::hash::HashMap<ast::ExprId, super::dynamic_casts::RuntimeCast>,
    type_queries: crate::support::hash::HashMap<ast::ExprId, super::type_queries::TypeQuery>,
    type_identities: crate::support::hash::HashSet<ast::ExprId>,
    records: Vec<Option<ExprFacts>>,
    // Qualified addresses keep their original lookup independently of the
    // specialization set. The dense use fact carries only a compact identity.
    member_addresses: Vec<MemberAddress>,
    // The source function list survives resolving its separate overload flag.
    function_declarations:
        crate::support::hash::HashMap<ast::ExprId, super::function_resolution::OverloadId>,
    // Only nonelided class copies own a lowering action. Selection remains
    // on the source facts, independently of its existing operand conversion.
    copy_actions:
        crate::support::hash::HashMap<ast::ExprId, super::copy_initialization::CopyAction>,
    // Parentheses share facts, as in the C++ owner. Chains are flattened at publication.
    aliases: Vec<Option<ast::ExprId>>,
    // Only differing-class glvalue conditionals need a base-subobject route.
    conditional_paths:
        crate::support::hash::HashMap<ast::ExprId, [Option<super::inheritance::BasePathId>; 2]>,
    // Converting class-subobject initializations are sparse use facts. Keep
    // them separate from the source's original type and argument conversion.
    class_conversions: crate::support::hash::HashMap<ast::ExprId, ClassConversion>,
    function_conversions:
        crate::support::hash::HashMap<ast::ExprId, super::conversion_functions::FunctionConversion>,
    allocations: crate::support::hash::HashMap<ast::ExprId, super::allocation::Allocation>,
    deletions: crate::support::hash::HashMap<ast::ExprId, super::allocation::Deletion>,
    // The member access owns a bounded selected-call chain over its original base.
    arrow_calls: crate::support::hash::HashMap<ast::ExprId, Vec<super::arrow_calls::ArrowStep>>,
    // A callable object's designation remains independent of operator()'s
    // implicit object conversion, including when the object is itself a member.
    callable_receivers: crate::support::hash::HashMap<ast::ExprId, CallReceiver>,
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
            || self.complex_construction(id)
            || facts.intrinsic(self).is_some()
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
    pub(super) fn complex_construction(&self, id: ast::ExprId) -> bool {
        self.complex_constructions.contains(&self.root(id))
    }
    pub(super) fn note_complex_construction(&mut self, id: ast::ExprId) {
        self.complex_constructions.insert(self.root(id));
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
    pub(super) fn function_declarations(
        &self,
        id: ast::ExprId,
    ) -> Option<super::function_resolution::OverloadId> {
        self.function_declarations.get(&self.root(id)).copied()
    }
    pub(super) fn set_function_declarations(
        &mut self,
        id: ast::ExprId,
        functions: super::function_resolution::OverloadId,
    ) {
        self.function_declarations.insert(self.root(id), functions);
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
    pub(super) fn user_conversion(&self, id: ast::ExprId) -> Option<Conversion> {
        self.class_conversions
            .get(&self.root(id))
            .and_then(|record| record.outer)
    }
    // C++ facts->conversion is the outer conversion. Constructor conversions
    // retain their parameter step separately in Rust's scalar facts.
    pub(super) fn outer_conversion(&self, id: ast::ExprId) -> Result<Option<Conversion>, Error> {
        Ok(self.user_conversion(id).or(self.get(id)?.conversion))
    }

    pub(super) fn set_user_conversion(
        &mut self,
        id: ast::ExprId,
        constructor: super::construction::ConstructorId,
        outer: Conversion,
        parameter_copy: Option<super::copy_initialization::InternalCopy>,
    ) {
        self.class_conversions.insert(
            self.root(id),
            ClassConversion {
                constructor,
                arguments: super::constructor_initialization::ConstructorArguments::Single(id),
                outer: Some(outer),
                parameter_copy,
            },
        );
    }
    pub(super) fn parameter_copy(
        &self,
        id: ast::ExprId,
    ) -> Option<super::copy_initialization::InternalCopy> {
        self.class_conversions
            .get(&self.root(id))
            .and_then(|record| record.parameter_copy)
    }
    pub(super) fn set_parameter_copy_action(
        &mut self,
        id: ast::ExprId,
        action: super::copy_initialization::CopyAction,
    ) {
        let root = self.root(id);
        if let Some(copy) = self
            .class_conversions
            .get_mut(&root)
            .and_then(|record| record.parameter_copy.as_mut())
        {
            copy.action = Some(action);
        }
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
    fn compact_facts_keep_copies_independent_and_share_equal_details() {
        let mut types = super::super::types::Types::default();
        let int = types
            .scalar(super::super::types::Scalar::Basic(FundamentalType::Int))
            .unwrap();
        let mut store = ExpressionFacts::default();
        let mut first = ExprFacts::new(int, ValueCategory::Prvalue);
        first.set_reference_cast(Some(int), &mut store).unwrap();
        first.set_folded(Some(u64::MAX));
        first.set_query_kind(QueryKind::Specialization);
        first.set_call_form(CallForm::CallableObject);
        let mut second = first;
        second.set_reference_cast(None, &mut store).unwrap();
        second.set_query_value(Some(0), &mut store).unwrap();
        second.set_folded(None);
        assert_eq!(first.reference_cast(&store), Some(int));
        assert_eq!(second.reference_cast(&store), None);
        assert_eq!(first.query_value(&store), None);
        assert_eq!(second.query_value(&store), Some(0));
        assert_eq!(first.folded(), Some(u64::MAX));
        assert_eq!(second.folded(), None);
        assert!(second.query_kind().is_specialization());
        assert_eq!(second.call_form(), CallForm::CallableObject);
        let mut same = ExprFacts::new(int, ValueCategory::Lvalue);
        same.set_reference_cast(Some(int), &mut store).unwrap();
        assert_eq!(same.details, first.details);
    }
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
