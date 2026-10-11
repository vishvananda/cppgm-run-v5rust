#include <cmath>
using namespace std;

#include "constant_value.h"

namespace constant_eval {

using namespace cpp_decl;

namespace {

typedef __int128_t SignedIntegralValue;
typedef __uint128_t UnsignedIntegralValue;

bool signed_value_fits_long_long(SignedIntegralValue value)
{
  union SignedIntegralParts
  {
    SignedIntegralValue value;
    struct
    {
      unsigned long long lo;
      long long hi;
    } parts;
  } bits;
  bits.value = value;
  return (bits.parts.hi == 0 && static_cast<long long>(bits.parts.lo) >= 0) ||
         (bits.parts.hi == -1 && static_cast<long long>(bits.parts.lo) < 0);
}

bool unsigned_value_fits_ull(UnsignedIntegralValue value)
{
  union UnsignedIntegralParts
  {
    UnsignedIntegralValue value;
    struct
    {
      unsigned long long lo;
      unsigned long long hi;
    } parts;
  } bits;
  bits.value = value;
  return bits.parts.hi == 0;
}

bool is_enum_type(const TypePtr & type)
{
  if(!type) {
    return false;
  }
  TypePtr base = strip_top_level_cv(remove_reference_type(type));
  return base &&
         base->kind == Type::TK_NAMED &&
         base->named_key.compare(0, 5, "enum ") == 0;
}

bool type_is_volatile_object(const TypePtr & type)
{
  TypePtr base = type;
  if(!base) {
    return false;
  }
  if(base->kind == Type::TK_ARRAY) {
    return type_is_volatile_object(base->inner);
  }
  if(base->kind == Type::TK_CV) {
    return base->cv_volatile;
  }
  return false;
}

TypePtr promoted_integral_type(const TypePtr & type)
{
  if(!type) {
    return TypePtr();
  }
  TypePtr base = strip_top_level_cv(remove_reference_type(type));
  if(!base || base->kind != Type::TK_FUNDAMENTAL) {
    return TypePtr();
  }

  switch(base->fundamental) {
  case FT_BOOL:
  case FT_CHAR:
  case FT_SIGNED_CHAR:
  case FT_UNSIGNED_CHAR:
  case FT_SHORT_INT:
  case FT_UNSIGNED_SHORT_INT:
  case FT_WCHAR_T:
  case FT_CHAR16_T:
  case FT_CHAR32_T:
    return make_fundamental(FT_INT);
  default:
    return TypePtr();
  }
}

TypePtr promoted_integral_or_self(const TypePtr & type)
{
  TypePtr promoted = promoted_integral_type(type);
  if(promoted) {
    return promoted;
  }
  return type ? strip_top_level_cv(remove_reference_type(type)) : TypePtr();
}

int integral_conversion_rank(EFundamentalType type)
{
  switch(type) {
  case FT_BOOL: return 0;
  case FT_CHAR:
  case FT_SIGNED_CHAR:
  case FT_UNSIGNED_CHAR: return 1;
  case FT_SHORT_INT:
  case FT_UNSIGNED_SHORT_INT: return 2;
  case FT_INT:
  case FT_UNSIGNED_INT: return 3;
  case FT_LONG_INT:
  case FT_UNSIGNED_LONG_INT: return 4;
  case FT_LONG_LONG_INT:
  case FT_UNSIGNED_LONG_LONG_INT: return 5;
  case FT_INT128:
  case FT_UINT128: return 6;
  default: return -1;
  }
}

TypePtr corresponding_unsigned_type(const TypePtr & type)
{
  TypePtr base = type ? strip_top_level_cv(remove_reference_type(type)) : TypePtr();
  if(!base || base->kind != Type::TK_FUNDAMENTAL) {
    return TypePtr();
  }

  switch(base->fundamental) {
  case FT_BOOL: return make_fundamental(FT_BOOL);
  case FT_CHAR: return make_fundamental(FT_CHAR);
  case FT_CHAR16_T: return make_fundamental(FT_CHAR16_T);
  case FT_CHAR32_T: return make_fundamental(FT_CHAR32_T);
  case FT_WCHAR_T: return make_fundamental(FT_WCHAR_T);
  case FT_SIGNED_CHAR:
  case FT_UNSIGNED_CHAR: return make_fundamental(FT_UNSIGNED_CHAR);
  case FT_SHORT_INT:
  case FT_UNSIGNED_SHORT_INT: return make_fundamental(FT_UNSIGNED_SHORT_INT);
  case FT_INT:
  case FT_UNSIGNED_INT: return make_fundamental(FT_UNSIGNED_INT);
  case FT_LONG_INT:
  case FT_UNSIGNED_LONG_INT: return make_fundamental(FT_UNSIGNED_LONG_INT);
  case FT_LONG_LONG_INT:
  case FT_UNSIGNED_LONG_LONG_INT: return make_fundamental(FT_UNSIGNED_LONG_LONG_INT);
  case FT_INT128:
  case FT_UINT128: return make_fundamental(FT_UINT128);
  default: return TypePtr();
  }
}

TypePtr common_integral_type(const TypePtr & lhs, const TypePtr & rhs)
{
  TypePtr lhs_type = promoted_integral_or_self(lhs);
  TypePtr rhs_type = promoted_integral_or_self(rhs);
  if(type_equals(lhs_type, rhs_type)) {
    return lhs_type;
  }

  if(!lhs_type || !rhs_type ||
     lhs_type->kind != Type::TK_FUNDAMENTAL ||
     rhs_type->kind != Type::TK_FUNDAMENTAL) {
    return make_fundamental(FT_INT);
  }

  const bool lhs_unsigned = is_unsigned_integral_type(lhs_type);
  const bool rhs_unsigned = is_unsigned_integral_type(rhs_type);
  const int lhs_rank = integral_conversion_rank(lhs_type->fundamental);
  const int rhs_rank = integral_conversion_rank(rhs_type->fundamental);
  const size_t lhs_size = type_size(lhs_type);
  const size_t rhs_size = type_size(rhs_type);

  if(lhs_unsigned == rhs_unsigned) {
    return lhs_rank >= rhs_rank ? lhs_type : rhs_type;
  }

  const TypePtr & unsigned_type = lhs_unsigned ? lhs_type : rhs_type;
  const TypePtr & signed_type = lhs_unsigned ? rhs_type : lhs_type;
  const int unsigned_rank = lhs_unsigned ? lhs_rank : rhs_rank;
  const int signed_rank = lhs_unsigned ? rhs_rank : lhs_rank;
  const size_t unsigned_size = lhs_unsigned ? lhs_size : rhs_size;
  const size_t signed_size = lhs_unsigned ? rhs_size : lhs_size;

  if(unsigned_rank >= signed_rank) {
    return unsigned_type;
  }
  if(signed_size > unsigned_size) {
    return signed_type;
  }

  TypePtr unsigned_corresponding = corresponding_unsigned_type(signed_type);
  return unsigned_corresponding ? unsigned_corresponding : unsigned_type;
}

TypePtr common_arithmetic_type(const TypePtr & lhs, const TypePtr & rhs)
{
  TypePtr lhs_type = lhs ? strip_top_level_cv(remove_reference_type(lhs)) : TypePtr();
  TypePtr rhs_type = rhs ? strip_top_level_cv(remove_reference_type(rhs)) : TypePtr();
  if(is_floating_type(lhs_type) || is_floating_type(rhs_type)) {
    const EFundamentalType lhs_ft =
        lhs_type && lhs_type->kind == Type::TK_FUNDAMENTAL ? lhs_type->fundamental : FT_VOID;
    const EFundamentalType rhs_ft =
        rhs_type && rhs_type->kind == Type::TK_FUNDAMENTAL ? rhs_type->fundamental : FT_VOID;
    if(lhs_ft == FT_LONG_DOUBLE || rhs_ft == FT_LONG_DOUBLE) {
      return make_fundamental(FT_LONG_DOUBLE);
    }
    if(lhs_ft == FT_DOUBLE || rhs_ft == FT_DOUBLE) {
      return make_fundamental(FT_DOUBLE);
    }
    return make_fundamental(FT_FLOAT);
  }
  return common_integral_type(lhs_type, rhs_type);
}

bool pointee_qualification_conversion_allowed(const TypePtr & source,
                                              const TypePtr & target)
{
  if(!source || !target ||
     !type_equals(strip_top_level_cv(source), strip_top_level_cv(target))) {
    return false;
  }
  if(type_is_const_object(source) && !type_is_const_object(target)) {
    return false;
  }
  if(type_is_volatile_object(source) && !type_is_volatile_object(target)) {
    return false;
  }
  return true;
}

bool pointer_target_compatible(const TypePtr & source, const TypePtr & target)
{
  TypePtr source_base = strip_top_level_cv(remove_reference_type(source));
  TypePtr target_base = strip_top_level_cv(remove_reference_type(target));
  if(!source_base || !target_base) {
    return false;
  }
  if(type_equals(source_base, target_base)) {
    return true;
  }
  return source_base->kind == Type::TK_POINTER &&
         target_base->kind == Type::TK_POINTER &&
         source_base->inner &&
         target_base->inner &&
         pointee_qualification_conversion_allowed(source_base->inner, target_base->inner);
}

bool array_decay_target_compatible(const TypePtr & source, const TypePtr & target)
{
  TypePtr source_base = strip_top_level_cv(remove_reference_type(source));
  TypePtr target_base = strip_top_level_cv(remove_reference_type(target));
  return source_base &&
         target_base &&
         source_base->kind == Type::TK_ARRAY &&
         target_base->kind == Type::TK_POINTER &&
         source_base->inner &&
         target_base->inner &&
         pointee_qualification_conversion_allowed(source_base->inner, target_base->inner);
}

bool value_to_floating(const ConstexprValue & value, long double & out)
{
  if(value.kind == ConstexprValue::CV_FLOATING) {
    out = value.floating_value;
    return true;
  }
  if(value.kind == ConstexprValue::CV_INTEGRAL) {
    TypePtr base = value.type ? strip_top_level_cv(remove_reference_type(value.type)) : TypePtr();
    if(!base || is_enum_type(base) || base->kind != Type::TK_FUNDAMENTAL) {
      out = static_cast<long double>(value.integral_value);
      return true;
    }

    switch(base->fundamental) {
    case FT_BOOL:
      out = static_cast<long double>(static_cast<bool>(value.integral_value));
      return true;
    case FT_CHAR:
      out = static_cast<long double>(static_cast<char>(value.integral_value));
      return true;
    case FT_SIGNED_CHAR:
      out = static_cast<long double>(static_cast<signed char>(value.integral_value));
      return true;
    case FT_UNSIGNED_CHAR:
      out = static_cast<long double>(static_cast<unsigned char>(value.integral_value));
      return true;
    case FT_SHORT_INT:
      out = static_cast<long double>(static_cast<short>(value.integral_value));
      return true;
    case FT_UNSIGNED_SHORT_INT:
      out = static_cast<long double>(static_cast<unsigned short>(value.integral_value));
      return true;
    case FT_INT:
      out = static_cast<long double>(static_cast<int>(value.integral_value));
      return true;
    case FT_UNSIGNED_INT:
      out = static_cast<long double>(static_cast<unsigned int>(value.integral_value));
      return true;
    case FT_LONG_INT:
      out = static_cast<long double>(static_cast<long>(value.integral_value));
      return true;
    case FT_UNSIGNED_LONG_INT:
      out = static_cast<long double>(static_cast<unsigned long>(value.integral_value));
      return true;
    case FT_LONG_LONG_INT:
      out = static_cast<long double>(static_cast<long long>(value.integral_value));
      return true;
    case FT_UNSIGNED_LONG_LONG_INT:
      out = static_cast<long double>(static_cast<unsigned long long>(value.integral_value));
      return true;
    case FT_INT128:
      out = static_cast<long double>(static_cast<SignedIntegralValue>(value.integral_value));
      return true;
    case FT_UINT128:
      out = static_cast<long double>(static_cast<UnsignedIntegralValue>(value.integral_value));
      return true;
    case FT_WCHAR_T:
      out = static_cast<long double>(static_cast<wchar_t>(value.integral_value));
      return true;
    case FT_CHAR16_T:
      out = static_cast<long double>(static_cast<char16_t>(value.integral_value));
      return true;
    case FT_CHAR32_T:
      out = static_cast<long double>(static_cast<char32_t>(value.integral_value));
      return true;
    default:
      out = static_cast<long double>(value.integral_value);
      return true;
    }
  }
  if(value.kind == ConstexprValue::CV_NULLPTR) {
    out = 0.0L;
    return true;
  }
  return false;
}

bool cast_integral_to_target(SignedIntegralValue value,
                             const TypePtr & target,
                             ConstexprValue & out);

bool integral_value_to_signed_bits(const ConstexprValue & value,
                                   SignedIntegralValue & out)
{
  if(value.kind != ConstexprValue::CV_INTEGRAL) {
    return false;
  }

  TypePtr base = value.type ? strip_top_level_cv(remove_reference_type(value.type)) : TypePtr();
  if(!base || is_enum_type(base) || base->kind != Type::TK_FUNDAMENTAL) {
    out = value.integral_value;
    return true;
  }

  switch(base->fundamental) {
  case FT_BOOL: out = static_cast<bool>(value.integral_value); return true;
  case FT_CHAR: out = static_cast<char>(value.integral_value); return true;
  case FT_SIGNED_CHAR: out = static_cast<signed char>(value.integral_value); return true;
  case FT_UNSIGNED_CHAR: out = static_cast<unsigned char>(value.integral_value); return true;
  case FT_SHORT_INT: out = static_cast<short>(value.integral_value); return true;
  case FT_UNSIGNED_SHORT_INT: out = static_cast<unsigned short>(value.integral_value); return true;
  case FT_INT: out = static_cast<int>(value.integral_value); return true;
  case FT_UNSIGNED_INT: out = static_cast<unsigned int>(value.integral_value); return true;
  case FT_LONG_INT: out = static_cast<long>(value.integral_value); return true;
  case FT_UNSIGNED_LONG_INT: out = static_cast<unsigned long>(value.integral_value); return true;
  case FT_LONG_LONG_INT: out = static_cast<long long>(value.integral_value); return true;
  case FT_UNSIGNED_LONG_LONG_INT:
    out = static_cast<unsigned long long>(value.integral_value);
    return true;
  case FT_INT128: out = static_cast<SignedIntegralValue>(value.integral_value); return true;
  case FT_UINT128: out = static_cast<UnsignedIntegralValue>(value.integral_value); return true;
  case FT_WCHAR_T: out = static_cast<wchar_t>(value.integral_value); return true;
  case FT_CHAR16_T: out = static_cast<char16_t>(value.integral_value); return true;
  case FT_CHAR32_T: out = static_cast<char32_t>(value.integral_value); return true;
  default:
    out = value.integral_value;
    return true;
  }
}

bool integral_value_to_unsigned_bits(const ConstexprValue & value,
                                     UnsignedIntegralValue & out)
{
  if(value.kind != ConstexprValue::CV_INTEGRAL) {
    return false;
  }

  TypePtr base = value.type ? strip_top_level_cv(remove_reference_type(value.type)) : TypePtr();
  if(!base || is_enum_type(base) || base->kind != Type::TK_FUNDAMENTAL) {
    out = static_cast<UnsignedIntegralValue>(value.integral_value);
    return true;
  }

  switch(base->fundamental) {
  case FT_BOOL: out = static_cast<bool>(value.integral_value); return true;
  case FT_CHAR: out = static_cast<UnsignedIntegralValue>(static_cast<char>(value.integral_value)); return true;
  case FT_SIGNED_CHAR:
    out = static_cast<UnsignedIntegralValue>(static_cast<signed char>(value.integral_value));
    return true;
  case FT_UNSIGNED_CHAR: out = static_cast<unsigned char>(value.integral_value); return true;
  case FT_SHORT_INT:
    out = static_cast<UnsignedIntegralValue>(static_cast<short>(value.integral_value));
    return true;
  case FT_UNSIGNED_SHORT_INT: out = static_cast<unsigned short>(value.integral_value); return true;
  case FT_INT: out = static_cast<UnsignedIntegralValue>(static_cast<int>(value.integral_value)); return true;
  case FT_UNSIGNED_INT: out = static_cast<unsigned int>(value.integral_value); return true;
  case FT_LONG_INT:
    out = static_cast<UnsignedIntegralValue>(static_cast<long>(value.integral_value));
    return true;
  case FT_UNSIGNED_LONG_INT: out = static_cast<unsigned long>(value.integral_value); return true;
  case FT_LONG_LONG_INT:
    out = static_cast<UnsignedIntegralValue>(static_cast<long long>(value.integral_value));
    return true;
  case FT_UNSIGNED_LONG_LONG_INT:
    out = static_cast<unsigned long long>(value.integral_value);
    return true;
  case FT_INT128: out = static_cast<UnsignedIntegralValue>(value.integral_value); return true;
  case FT_UINT128: out = static_cast<UnsignedIntegralValue>(value.integral_value); return true;
  case FT_WCHAR_T:
    out = static_cast<UnsignedIntegralValue>(static_cast<wchar_t>(value.integral_value));
    return true;
  case FT_CHAR16_T: out = static_cast<char16_t>(value.integral_value); return true;
  case FT_CHAR32_T: out = static_cast<char32_t>(value.integral_value); return true;
  default:
    out = static_cast<UnsignedIntegralValue>(value.integral_value);
    return true;
  }
}

bool integral_value_to_signed(const ConstexprValue & value, long long & out)
{
  SignedIntegralValue wide;
  wide = 0;
  if(!integral_value_to_signed_bits(value, wide) || !signed_value_fits_long_long(wide)) {
    return false;
  }
  out = static_cast<long long>(wide);
  return true;
}

bool integral_value_to_unsigned(const ConstexprValue & value, unsigned long long & out)
{
  UnsignedIntegralValue wide;
  wide = 0;
  if(!integral_value_to_unsigned_bits(value, wide) || !unsigned_value_fits_ull(wide)) {
    return false;
  }
  out = static_cast<unsigned long long>(wide);
  return true;
}

bool cast_integral_bits_to_target(UnsignedIntegralValue bits,
                                  const TypePtr & target,
                                  ConstexprValue & out)
{
  return cast_integral_to_target(static_cast<SignedIntegralValue>(bits), target, out);
}

bool cast_signed_value_to_target(SignedIntegralValue value,
                                 const TypePtr & target,
                                 ConstexprValue & out)
{
  return cast_integral_to_target(value, target, out);
}

bool cast_unsigned_binary_result(ETokenType op,
                                 const TypePtr & target,
                                 UnsignedIntegralValue lhs_value,
                                 UnsignedIntegralValue rhs_value,
                                 ConstexprValue & out)
{
  switch(op) {
  case OP_PLUS:
  {
    UnsignedIntegralValue result;
    result = lhs_value + rhs_value;
    return cast_integral_bits_to_target(result, target, out);
  }
  case OP_MINUS:
  {
    UnsignedIntegralValue result;
    result = lhs_value - rhs_value;
    return cast_integral_bits_to_target(result, target, out);
  }
  case OP_STAR:
  {
    UnsignedIntegralValue result;
    result = lhs_value * rhs_value;
    return cast_integral_bits_to_target(result, target, out);
  }
  case OP_DIV:
    if(rhs_value == 0) {
      return false;
    }
  {
    UnsignedIntegralValue result;
    result = lhs_value / rhs_value;
    return cast_integral_bits_to_target(result, target, out);
  }
  case OP_MOD:
    if(rhs_value == 0) {
      return false;
    }
  {
    UnsignedIntegralValue result;
    result = lhs_value % rhs_value;
    return cast_integral_bits_to_target(result, target, out);
  }
  case OP_LT:
    out = make_integral_value(lhs_value < rhs_value, make_fundamental(FT_BOOL));
    return true;
  case OP_GT:
    out = make_integral_value(lhs_value > rhs_value, make_fundamental(FT_BOOL));
    return true;
  case OP_LE:
    out = make_integral_value(lhs_value <= rhs_value, make_fundamental(FT_BOOL));
    return true;
  case OP_GE:
    out = make_integral_value(lhs_value >= rhs_value, make_fundamental(FT_BOOL));
    return true;
  case OP_EQ:
    out = make_integral_value(lhs_value == rhs_value, make_fundamental(FT_BOOL));
    return true;
  case OP_NE:
    out = make_integral_value(lhs_value != rhs_value, make_fundamental(FT_BOOL));
    return true;
  case OP_AMP:
  {
    UnsignedIntegralValue result;
    result = lhs_value & rhs_value;
    return cast_integral_bits_to_target(result, target, out);
  }
  case OP_XOR:
  {
    UnsignedIntegralValue result;
    result = lhs_value ^ rhs_value;
    return cast_integral_bits_to_target(result, target, out);
  }
  case OP_BOR:
  {
    UnsignedIntegralValue result;
    result = lhs_value | rhs_value;
    return cast_integral_bits_to_target(result, target, out);
  }
  default:
    return false;
  }
}

bool cast_signed_binary_result(ETokenType op,
                               const TypePtr & target,
                               SignedIntegralValue lhs_value,
                               SignedIntegralValue rhs_value,
                               ConstexprValue & out)
{
  switch(op) {
  case OP_PLUS:
  {
    SignedIntegralValue result;
    result = lhs_value + rhs_value;
    return cast_signed_value_to_target(result, target, out);
  }
  case OP_MINUS:
  {
    SignedIntegralValue result;
    result = lhs_value - rhs_value;
    return cast_signed_value_to_target(result, target, out);
  }
  case OP_STAR:
  {
    SignedIntegralValue result;
    result = lhs_value * rhs_value;
    return cast_signed_value_to_target(result, target, out);
  }
  case OP_DIV:
    if(rhs_value == 0) {
      return false;
    }
  {
    SignedIntegralValue result;
    result = lhs_value / rhs_value;
    return cast_signed_value_to_target(result, target, out);
  }
  case OP_MOD:
    if(rhs_value == 0) {
      return false;
    }
  {
    SignedIntegralValue result;
    result = lhs_value % rhs_value;
    return cast_signed_value_to_target(result, target, out);
  }
  case OP_LT:
    out = make_integral_value(lhs_value < rhs_value, make_fundamental(FT_BOOL));
    return true;
  case OP_GT:
    out = make_integral_value(lhs_value > rhs_value, make_fundamental(FT_BOOL));
    return true;
  case OP_LE:
    out = make_integral_value(lhs_value <= rhs_value, make_fundamental(FT_BOOL));
    return true;
  case OP_GE:
    out = make_integral_value(lhs_value >= rhs_value, make_fundamental(FT_BOOL));
    return true;
  case OP_EQ:
    out = make_integral_value(lhs_value == rhs_value, make_fundamental(FT_BOOL));
    return true;
  case OP_NE:
    out = make_integral_value(lhs_value != rhs_value, make_fundamental(FT_BOOL));
    return true;
  case OP_AMP:
  {
    SignedIntegralValue result;
    result = lhs_value & rhs_value;
    return cast_signed_value_to_target(result, target, out);
  }
  case OP_XOR:
  {
    SignedIntegralValue result;
    result = lhs_value ^ rhs_value;
    return cast_signed_value_to_target(result, target, out);
  }
  case OP_BOR:
  {
    SignedIntegralValue result;
    result = lhs_value | rhs_value;
    return cast_signed_value_to_target(result, target, out);
  }
  default:
    return false;
  }
}

bool cast_integral_to_target(SignedIntegralValue value,
                             const TypePtr & target,
                             ConstexprValue & out)
{
  TypePtr base = strip_top_level_cv(remove_reference_type(target));
  if(!base) {
    return false;
  }
  if(is_enum_type(base)) {
    out = make_integral_value(value, base);
    return true;
  }
  if(base->kind != Type::TK_FUNDAMENTAL) {
    return false;
  }

  switch(base->fundamental) {
  case FT_BOOL: out = make_integral_value(value != 0, base); return true;
  case FT_CHAR: out = make_integral_value(static_cast<char>(value), base); return true;
  case FT_SIGNED_CHAR:
    out = make_integral_value(static_cast<signed char>(value), base);
    return true;
  case FT_UNSIGNED_CHAR:
    out = make_integral_value(static_cast<unsigned char>(value), base);
    return true;
  case FT_SHORT_INT:
    out = make_integral_value(static_cast<short>(value), base);
    return true;
  case FT_UNSIGNED_SHORT_INT:
    out = make_integral_value(static_cast<unsigned short>(value), base);
    return true;
  case FT_INT: out = make_integral_value(static_cast<int>(value), base); return true;
  case FT_UNSIGNED_INT:
    out = make_integral_value(static_cast<unsigned int>(value), base);
    return true;
  case FT_LONG_INT:
    out = make_integral_value(static_cast<long>(value), base);
    return true;
  case FT_UNSIGNED_LONG_INT:
    out = make_integral_value(static_cast<unsigned long>(value), base);
    return true;
  case FT_LONG_LONG_INT:
    out = make_integral_value(static_cast<long long>(value), base);
    return true;
  case FT_UNSIGNED_LONG_LONG_INT:
    out = make_integral_bits_value(static_cast<UnsignedIntegralValue>(value), base);
    return true;
  case FT_INT128:
    out = make_integral_bits_value(static_cast<SignedIntegralValue>(value), base);
    return true;
  case FT_UINT128:
    out = make_integral_bits_value(static_cast<UnsignedIntegralValue>(value), base);
    return true;
  case FT_WCHAR_T:
    out = make_integral_value(static_cast<wchar_t>(value), base);
    return true;
  case FT_CHAR16_T:
    out = make_integral_value(static_cast<char16_t>(value), base);
    return true;
  case FT_CHAR32_T:
    out = make_integral_value(static_cast<char32_t>(value), base);
    return true;
  case FT_FLOAT:
    out = make_floating_value(static_cast<float>(value), base);
    return true;
  case FT_DOUBLE:
    out = make_floating_value(static_cast<double>(value), base);
    return true;
  case FT_LONG_DOUBLE:
    out = make_floating_value(static_cast<long double>(value), base);
    return true;
  case FT_NULLPTR_T:
    if(value != 0) {
      return false;
    }
    out = make_nullptr_value();
    out.type = base;
    return true;
  default:
    return false;
  }
}

}  // namespace

ConstexprValue make_integral_value(long long value, const TypePtr & type)
{
  ConstexprValue out;
  out.kind = ConstexprValue::CV_INTEGRAL;
  out.type = type;
  out.integral_value = value;
  return out;
}

ConstexprValue make_integral_bits_value(SignedIntegralValue value, const TypePtr & type)
{
  ConstexprValue out;
  out.kind = ConstexprValue::CV_INTEGRAL;
  out.type = type;
  out.integral_value = value;
  return out;
}

ConstexprValue make_integral_bits_value(UnsignedIntegralValue value, const TypePtr & type)
{
  ConstexprValue out;
  out.kind = ConstexprValue::CV_INTEGRAL;
  out.type = type;
  out.integral_value = static_cast<SignedIntegralValue>(value);
  return out;
}

ConstexprValue make_floating_value(long double value, const TypePtr & type)
{
  ConstexprValue out;
  out.kind = ConstexprValue::CV_FLOATING;
  out.type = type;
  out.floating_value = value;
  return out;
}

ConstexprValue make_nullptr_value()
{
  ConstexprValue out;
  out.kind = ConstexprValue::CV_NULLPTR;
  out.type = make_fundamental(FT_NULLPTR_T);
  return out;
}

ConstexprValue make_pointer_value(const TypePtr & type,
                                  const string & storage_identity,
                                  size_t pointer_offset)
{
  ConstexprValue out;
  out.kind = ConstexprValue::CV_POINTER;
  out.type = type;
  out.storage_identity = storage_identity;
  out.pointer_offset = pointer_offset;
  return out;
}

ConstexprValue make_aggregate_value(
    const TypePtr & type,
    const vector<pair<string, ConstexprValue> > & members,
    const vector<bool> & member_is_base)
{
  ConstexprValue out;
  out.kind = ConstexprValue::CV_AGGREGATE;
  out.type = type;
  out.aggregate_members = members;
  out.aggregate_member_is_base = member_is_base;
  if(out.aggregate_member_is_base.size() < out.aggregate_members.size()) {
    out.aggregate_member_is_base.resize(out.aggregate_members.size(), false);
  }
  return out;
}

ConstexprValue make_array_value(const TypePtr & type,
                                const vector<ConstexprValue> & elements,
                                const string & storage_identity)
{
  ConstexprValue out;
  out.kind = ConstexprValue::CV_ARRAY;
  out.type = type;
  out.storage_identity = storage_identity;
  out.array_elements = elements;
  return out;
}

bool aggregate_member_value(const ConstexprValue & aggregate,
                            const string & name,
                            ConstexprValue & out)
{
  if(aggregate.kind != ConstexprValue::CV_AGGREGATE) {
    return false;
  }

  for(size_t i = 0; i < aggregate.aggregate_members.size(); ++i) {
    const bool is_base =
        i < aggregate.aggregate_member_is_base.size() && aggregate.aggregate_member_is_base[i];
    if(!is_base && aggregate.aggregate_members[i].first == name) {
      out = aggregate.aggregate_members[i].second;
      return true;
    }
  }

  for(size_t i = 0; i < aggregate.aggregate_members.size(); ++i) {
    const bool is_base =
        i < aggregate.aggregate_member_is_base.size() && aggregate.aggregate_member_is_base[i];
    if(is_base &&
       aggregate_member_value(aggregate.aggregate_members[i].second, name, out)) {
      return true;
    }
  }

  return false;
}

bool array_element_value(const ConstexprValue & array,
                         size_t index,
                         ConstexprValue & out)
{
  if(array.kind != ConstexprValue::CV_ARRAY || index >= array.array_elements.size()) {
    return false;
  }
  out = array.array_elements[index];
  return true;
}

bool constexpr_value_truthy(const ConstexprValue & value, bool & out)
{
  switch(value.kind) {
  case ConstexprValue::CV_INTEGRAL:
    out = value.integral_value != 0;
    return true;
  case ConstexprValue::CV_FLOATING:
    out = value.floating_value != 0.0L;
    return true;
  case ConstexprValue::CV_NULLPTR:
    out = false;
    return true;
  case ConstexprValue::CV_POINTER:
    out = true;
    return true;
  case ConstexprValue::CV_AGGREGATE:
  case ConstexprValue::CV_ARRAY:
  default:
    return false;
  }
}

bool constexpr_value_to_integral(const ConstexprValue & value, long long & out)
{
  switch(value.kind) {
  case ConstexprValue::CV_INTEGRAL:
    if(!signed_value_fits_long_long(value.integral_value)) {
      return false;
    }
    out = static_cast<long long>(value.integral_value);
    return true;
  case ConstexprValue::CV_FLOATING:
    out = static_cast<long long>(value.floating_value);
    return true;
  case ConstexprValue::CV_NULLPTR:
    out = 0;
    return true;
  case ConstexprValue::CV_POINTER:
  case ConstexprValue::CV_AGGREGATE:
  case ConstexprValue::CV_ARRAY:
  default:
    return false;
  }
}

bool constexpr_value_cast(const ConstexprValue & value,
                          const TypePtr & target,
                          ConstexprValue & out)
{
  TypePtr base = strip_top_level_cv(remove_reference_type(target));
  if(!base) {
    return false;
  }
  if(value.kind == ConstexprValue::CV_INVALID) {
    return false;
  }
  if(value.kind == ConstexprValue::CV_AGGREGATE) {
    TypePtr source = strip_top_level_cv(remove_reference_type(value.type));
    if(source && base && type_equals(source, base)) {
      out = value;
      out.type = target;
      return true;
    }
    return false;
  }
  if(value.kind == ConstexprValue::CV_ARRAY) {
    TypePtr source = strip_top_level_cv(remove_reference_type(value.type));
    if(source && base && type_equals(source, base)) {
      out = value;
      out.type = target;
      return true;
    }
    if(source && base &&
       source->kind == Type::TK_ARRAY &&
       base->kind == Type::TK_ARRAY &&
       source->inner &&
       base->inner &&
       type_equals(source->inner, base->inner) &&
       (!base->has_bound || base->bound == value.array_elements.size())) {
      out = value;
      out.type = target;
      return true;
    }
    if(source && base && array_decay_target_compatible(source, base) &&
       !value.storage_identity.empty()) {
      out = make_pointer_value(base, value.storage_identity, 0);
      out.type = target;
      return true;
    }
    return false;
  }
  if(value.kind == ConstexprValue::CV_POINTER) {
    if(is_bool_type(base)) {
      out = make_integral_value(1, base);
      return true;
    }
    if(pointer_target_compatible(value.type, base)) {
      out = value;
      out.type = target;
      return true;
    }
    return false;
  }
  if(value.kind == ConstexprValue::CV_NULLPTR && is_pointer_type(base)) {
    out = make_nullptr_value();
    out.type = base;
    return true;
  }

  if(value.kind == ConstexprValue::CV_FLOATING) {
    long double numeric = value.floating_value;
    if(base->kind == Type::TK_FUNDAMENTAL &&
       (base->fundamental == FT_FLOAT ||
        base->fundamental == FT_DOUBLE ||
        base->fundamental == FT_LONG_DOUBLE)) {
      switch(base->fundamental) {
      case FT_FLOAT: out = make_floating_value(static_cast<float>(numeric), base); return true;
      case FT_DOUBLE: out = make_floating_value(static_cast<double>(numeric), base); return true;
      case FT_LONG_DOUBLE: out = make_floating_value(numeric, base); return true;
      default: break;
      }
    }
    return cast_integral_to_target(static_cast<SignedIntegralValue>(numeric), base, out);
  }

  SignedIntegralValue integral;
  integral = 0;
  if(!integral_value_to_signed_bits(value, integral)) {
    return false;
  }
  return cast_integral_to_target(integral, base, out);
}

bool constexpr_value_apply_unary(ETokenType op,
                                 const ConstexprValue & operand,
                                 ConstexprValue & out)
{
  SignedIntegralValue integral;
  integral = 0;
  bool truthy = false;
  TypePtr promoted_type = promoted_integral_or_self(operand.type);
  switch(op) {
  case OP_PLUS:
    if(operand.kind == ConstexprValue::CV_INTEGRAL && promoted_type) {
      return constexpr_value_cast(operand, promoted_type, out);
    }
    out = operand;
    return out.kind != ConstexprValue::CV_INVALID;

  case OP_MINUS:
    if(operand.kind == ConstexprValue::CV_FLOATING) {
      out = make_floating_value(-operand.floating_value, operand.type);
      return true;
    }
    if(promoted_type && is_unsigned_integral_type(promoted_type)) {
      ConstexprValue promoted_operand;
      UnsignedIntegralValue unsigned_value;
      unsigned_value = 0;
      if(!constexpr_value_cast(operand, promoted_type, promoted_operand) ||
         !integral_value_to_unsigned_bits(promoted_operand, unsigned_value)) {
        return false;
      }
      UnsignedIntegralValue result;
      result = UnsignedIntegralValue(0) - unsigned_value;
      return cast_integral_bits_to_target(result, promoted_type, out);
    }
    if(!integral_value_to_signed_bits(operand, integral)) {
      return false;
    }
  {
    SignedIntegralValue result;
    result = -integral;
    return cast_signed_value_to_target(result,
                                       promoted_type ? promoted_type : operand.type,
                                       out);
  }

  case OP_LNOT:
    if(!constexpr_value_truthy(operand, truthy)) {
      return false;
    }
    out = make_integral_value(!truthy, make_fundamental(FT_BOOL));
    return true;

  case OP_COMPL:
    if(promoted_type && is_unsigned_integral_type(promoted_type)) {
      ConstexprValue promoted_operand;
      UnsignedIntegralValue unsigned_value;
      unsigned_value = 0;
      if(!constexpr_value_cast(operand, promoted_type, promoted_operand) ||
         !integral_value_to_unsigned_bits(promoted_operand, unsigned_value)) {
        return false;
      }
      UnsignedIntegralValue result;
      result = ~unsigned_value;
      return cast_integral_bits_to_target(result, promoted_type, out);
    }
    if(!integral_value_to_signed_bits(operand, integral)) {
      return false;
    }
  {
    SignedIntegralValue result;
    result = ~integral;
    return cast_signed_value_to_target(result,
                                       promoted_type ? promoted_type : operand.type,
                                       out);
  }

  default:
    return false;
  }
}

bool constexpr_value_apply_binary(ETokenType op,
                                  const ConstexprValue & lhs,
                                  const ConstexprValue & rhs,
                                  ConstexprValue & out)
{
  if(lhs.kind == ConstexprValue::CV_INVALID || rhs.kind == ConstexprValue::CV_INVALID) {
    return false;
  }

  if(op == OP_EQ || op == OP_NE) {
    if(lhs.kind == ConstexprValue::CV_NULLPTR || rhs.kind == ConstexprValue::CV_NULLPTR ||
       lhs.kind == ConstexprValue::CV_POINTER || rhs.kind == ConstexprValue::CV_POINTER) {
      bool eq = false;
      if(lhs.kind == ConstexprValue::CV_NULLPTR || rhs.kind == ConstexprValue::CV_NULLPTR) {
        eq = lhs.kind == rhs.kind;
      } else if(lhs.kind == ConstexprValue::CV_POINTER &&
                rhs.kind == ConstexprValue::CV_POINTER) {
        eq = lhs.storage_identity == rhs.storage_identity &&
             lhs.pointer_offset == rhs.pointer_offset;
      }
      out = make_integral_value(op == OP_EQ ? eq : !eq, make_fundamental(FT_BOOL));
      return true;
    }
  }

  if(lhs.kind == ConstexprValue::CV_FLOATING || rhs.kind == ConstexprValue::CV_FLOATING) {
    long double lhs_value = 0.0L;
    long double rhs_value = 0.0L;
    if(!value_to_floating(lhs, lhs_value) || !value_to_floating(rhs, rhs_value)) {
      return false;
    }

    switch(op) {
    case OP_PLUS:
      return constexpr_value_cast(make_floating_value(lhs_value + rhs_value,
                                                      common_arithmetic_type(lhs.type, rhs.type)),
                                  common_arithmetic_type(lhs.type, rhs.type),
                                  out);
    case OP_MINUS:
      return constexpr_value_cast(make_floating_value(lhs_value - rhs_value,
                                                      common_arithmetic_type(lhs.type, rhs.type)),
                                  common_arithmetic_type(lhs.type, rhs.type),
                                  out);
    case OP_STAR:
      return constexpr_value_cast(make_floating_value(lhs_value * rhs_value,
                                                      common_arithmetic_type(lhs.type, rhs.type)),
                                  common_arithmetic_type(lhs.type, rhs.type),
                                  out);
    case OP_DIV:
      if(rhs_value == 0.0L) {
        return false;
      }
      return constexpr_value_cast(make_floating_value(lhs_value / rhs_value,
                                                      common_arithmetic_type(lhs.type, rhs.type)),
                                  common_arithmetic_type(lhs.type, rhs.type),
                                  out);
    case OP_LT: out = make_integral_value(lhs_value < rhs_value, make_fundamental(FT_BOOL)); return true;
    case OP_GT: out = make_integral_value(lhs_value > rhs_value, make_fundamental(FT_BOOL)); return true;
    case OP_LE: out = make_integral_value(lhs_value <= rhs_value, make_fundamental(FT_BOOL)); return true;
    case OP_GE: out = make_integral_value(lhs_value >= rhs_value, make_fundamental(FT_BOOL)); return true;
    case OP_EQ: out = make_integral_value(lhs_value == rhs_value, make_fundamental(FT_BOOL)); return true;
    case OP_NE: out = make_integral_value(lhs_value != rhs_value, make_fundamental(FT_BOOL)); return true;
    default: return false;
    }
  }

  if(op == OP_LSHIFT || op == OP_RSHIFT) {
    TypePtr promoted_type = promoted_integral_or_self(lhs.type);
    TypePtr rhs_type = promoted_integral_or_self(rhs.type);
    ConstexprValue lhs_cast;
    ConstexprValue rhs_cast;
    if(!promoted_type || !rhs_type ||
       !constexpr_value_cast(lhs, promoted_type, lhs_cast) ||
       !constexpr_value_cast(rhs, rhs_type, rhs_cast)) {
      return false;
    }

    long long rhs_value = 0;
    if(!constexpr_value_to_integral(rhs_cast, rhs_value)) {
      return false;
    }

    if(is_unsigned_integral_type(promoted_type)) {
      UnsignedIntegralValue lhs_value;
      lhs_value = 0;
      if(!integral_value_to_unsigned_bits(lhs_cast, lhs_value)) {
        return false;
      }
      UnsignedIntegralValue result;
      result = op == OP_LSHIFT ? (lhs_value << rhs_value)
                               : (lhs_value >> rhs_value);
      return cast_integral_bits_to_target(result, promoted_type, out);
    }

    SignedIntegralValue lhs_value;
    lhs_value = 0;
    if(!integral_value_to_signed_bits(lhs_cast, lhs_value)) {
      return false;
    }
    SignedIntegralValue result;
    result = op == OP_LSHIFT ? (lhs_value << rhs_value)
                             : (lhs_value >> rhs_value);
    return cast_signed_value_to_target(result, promoted_type, out);
  }

  TypePtr result_type = common_arithmetic_type(lhs.type, rhs.type);
  ConstexprValue lhs_cast;
  ConstexprValue rhs_cast;
  if(!constexpr_value_cast(lhs, result_type, lhs_cast) ||
     !constexpr_value_cast(rhs, result_type, rhs_cast)) {
    return false;
  }

  if(is_unsigned_integral_type(result_type)) {
    UnsignedIntegralValue lhs_value;
    lhs_value = 0;
    UnsignedIntegralValue rhs_value;
    rhs_value = 0;
    if(!integral_value_to_unsigned_bits(lhs_cast, lhs_value) ||
       !integral_value_to_unsigned_bits(rhs_cast, rhs_value)) {
      return false;
    }
    return cast_unsigned_binary_result(op, result_type, lhs_value, rhs_value, out);
  }

  SignedIntegralValue lhs_value;
  lhs_value = 0;
  SignedIntegralValue rhs_value;
  rhs_value = 0;
  if(!integral_value_to_signed_bits(lhs_cast, lhs_value) ||
     !integral_value_to_signed_bits(rhs_cast, rhs_value)) {
    return false;
  }

  switch(op) {
  default:
    return cast_signed_binary_result(op, result_type, lhs_value, rhs_value, out);
  }
}

}  // namespace constant_eval
