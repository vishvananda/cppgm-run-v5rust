// Translated from lex/gnu_builtins.{h,cpp}. See NOTICE.
//! Typed predicates distinguish shape queries from class-completion queries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TypePredicate {
    IsClass,
    IsEnum,
    IsUnion,
    IsVoid,
    IsNullptr,
    IsIntegral,
    IsFloatingPoint,
    IsArray,
    IsBoundedArray,
    IsUnboundedArray,
    IsPointer,
    IsLvalueReference,
    IsRvalueReference,
    IsReference,
    IsMemberObjectPointer,
    IsMemberFunctionPointer,
    IsMemberPointer,
    IsFunction,
    IsArithmetic,
    IsFundamental,
    IsObject,
    IsScalar,
    IsCompound,
    IsConst,
    IsVolatile,
    IsSigned,
    IsUnsigned,
    IsReferenceable,
    IsScopedEnum,
    IsSame,
    ArrayRank,
    IsEmpty,
    IsPolymorphic,
    IsAbstract,
    IsFinal,
    IsAggregate,
    HasVirtualDestructor,
    IsBaseOf,
    IsTrivial,
    IsTriviallyCopyable,
    IsStandardLayout,
    IsPod,
    IsLiteralType,
    HasTrivialDestructor,
    IsDestructible,
    IsNothrowDestructible,
    IsTriviallyDestructible,
    HasTrivialConstructor,
    HasTrivialCopy,
    HasTrivialAssign,
    HasNothrowConstructor,
    HasNothrowCopy,
    HasNothrowAssign,
    IsAssignable,
    IsNothrowAssignable,
    IsTriviallyAssignable,
    IsConvertible,
    IsConvertibleTo,
    IsNothrowConvertible,
}

impl TypePredicate {
    const SPELLINGS: [(Self, &'static str); 59] = [
        (Self::IsClass, "__is_class"),
        (Self::IsEnum, "__is_enum"),
        (Self::IsUnion, "__is_union"),
        (Self::IsVoid, "__is_void"),
        (Self::IsNullptr, "__is_nullptr"),
        (Self::IsIntegral, "__is_integral"),
        (Self::IsFloatingPoint, "__is_floating_point"),
        (Self::IsArray, "__is_array"),
        (Self::IsBoundedArray, "__is_bounded_array"),
        (Self::IsUnboundedArray, "__is_unbounded_array"),
        (Self::IsPointer, "__is_pointer"),
        (Self::IsLvalueReference, "__is_lvalue_reference"),
        (Self::IsRvalueReference, "__is_rvalue_reference"),
        (Self::IsReference, "__is_reference"),
        (Self::IsMemberObjectPointer, "__is_member_object_pointer"),
        (
            Self::IsMemberFunctionPointer,
            "__is_member_function_pointer",
        ),
        (Self::IsMemberPointer, "__is_member_pointer"),
        (Self::IsFunction, "__is_function"),
        (Self::IsArithmetic, "__is_arithmetic"),
        (Self::IsFundamental, "__is_fundamental"),
        (Self::IsObject, "__is_object"),
        (Self::IsScalar, "__is_scalar"),
        (Self::IsCompound, "__is_compound"),
        (Self::IsConst, "__is_const"),
        (Self::IsVolatile, "__is_volatile"),
        (Self::IsSigned, "__is_signed"),
        (Self::IsUnsigned, "__is_unsigned"),
        (Self::IsReferenceable, "__is_referenceable"),
        (Self::IsScopedEnum, "__is_scoped_enum"),
        (Self::IsSame, "__is_same"),
        (Self::ArrayRank, "__array_rank"),
        (Self::IsEmpty, "__is_empty"),
        (Self::IsPolymorphic, "__is_polymorphic"),
        (Self::IsAbstract, "__is_abstract"),
        (Self::IsFinal, "__is_final"),
        (Self::IsAggregate, "__is_aggregate"),
        (Self::HasVirtualDestructor, "__has_virtual_destructor"),
        (Self::IsBaseOf, "__is_base_of"),
        (Self::IsTrivial, "__is_trivial"),
        (Self::IsTriviallyCopyable, "__is_trivially_copyable"),
        (Self::IsStandardLayout, "__is_standard_layout"),
        (Self::IsPod, "__is_pod"),
        (Self::IsLiteralType, "__is_literal_type"),
        (Self::HasTrivialDestructor, "__has_trivial_destructor"),
        (Self::IsDestructible, "__is_destructible"),
        (Self::IsNothrowDestructible, "__is_nothrow_destructible"),
        (Self::IsTriviallyDestructible, "__is_trivially_destructible"),
        (Self::HasTrivialConstructor, "__has_trivial_constructor"),
        (Self::HasTrivialCopy, "__has_trivial_copy"),
        (Self::HasTrivialAssign, "__has_trivial_assign"),
        (Self::HasNothrowConstructor, "__has_nothrow_constructor"),
        (Self::HasNothrowCopy, "__has_nothrow_copy"),
        (Self::HasNothrowAssign, "__has_nothrow_assign"),
        (Self::IsAssignable, "__is_assignable"),
        (Self::IsNothrowAssignable, "__is_nothrow_assignable"),
        (Self::IsTriviallyAssignable, "__is_trivially_assignable"),
        (Self::IsConvertible, "__is_convertible"),
        (Self::IsConvertibleTo, "__is_convertible_to"),
        (Self::IsNothrowConvertible, "__is_nothrow_convertible"),
    ];

    pub(crate) fn find(spelling: &[u8]) -> Option<Self> {
        Self::SPELLINGS
            .iter()
            .find_map(|&(kind, name)| (name.as_bytes() == spelling).then_some(kind))
    }

    pub(crate) fn spelling(self) -> &'static str {
        Self::SPELLINGS[self as usize].1
    }

    pub(crate) fn is_property(self) -> bool {
        self as usize >= Self::IsTrivial as usize && !self.is_expression()
    }

    pub(crate) fn is_expression(self) -> bool {
        self as usize >= Self::IsAssignable as usize
    }

    pub(crate) fn is_assignment(self) -> bool {
        matches!(
            self,
            Self::IsAssignable | Self::IsNothrowAssignable | Self::IsTriviallyAssignable
        )
    }

    pub(crate) fn arity(self) -> usize {
        if matches!(self, Self::IsSame | Self::IsBaseOf) || self.is_expression() {
            2
        } else {
            1
        }
    }
}
