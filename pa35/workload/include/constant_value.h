#pragma once

#include <string>
#include <utility>
#include <vector>

#include "cpp_decl_model.h"
#include "recog_token.h"

namespace constant_eval {

struct ConstexprValue
{
  enum Kind
  {
    CV_INVALID,
    CV_INTEGRAL,
    CV_FLOATING,
    CV_NULLPTR,
    CV_POINTER,
    CV_FUNCTION,
    CV_MEMBER_POINTER,
    CV_AGGREGATE,
    CV_ARRAY
  };

  Kind kind = CV_INVALID;
  cpp_decl::TypePtr type;
  unsigned long long integral_value = 0;
  long double floating_value = 0.0L;
  std::string storage_identity;
  std::size_t pointer_offset = 0;
  std::vector<std::pair<std::string, ConstexprValue> > aggregate_members;
  std::vector<bool> aggregate_member_is_base;
  std::vector<ConstexprValue> array_elements;
};

ConstexprValue make_integral_value(long long value,
                                   const cpp_decl::TypePtr & type = cpp_decl::TypePtr());
ConstexprValue make_integral_bits_value(long long value,
                                        const cpp_decl::TypePtr & type = cpp_decl::TypePtr());
ConstexprValue make_integral_bits_value(unsigned long long value,
                                        const cpp_decl::TypePtr & type = cpp_decl::TypePtr());
ConstexprValue make_floating_value(long double value,
                                   const cpp_decl::TypePtr & type = cpp_decl::TypePtr());
ConstexprValue make_nullptr_value();
ConstexprValue make_pointer_value(const cpp_decl::TypePtr & type,
                                  const std::string & storage_identity,
                                  std::size_t pointer_offset = 0);
ConstexprValue make_function_value(const cpp_decl::TypePtr & type,
                                   const std::string & storage_identity);
ConstexprValue make_member_pointer_value(const cpp_decl::TypePtr & type,
                                         const std::string & member_identity,
                                         std::size_t member_offset = 0);
ConstexprValue make_aggregate_value(
    const cpp_decl::TypePtr & type,
    const std::vector<std::pair<std::string, ConstexprValue> > & members,
    const std::vector<bool> & member_is_base = std::vector<bool>());
ConstexprValue make_array_value(const cpp_decl::TypePtr & type,
                                const std::vector<ConstexprValue> & elements,
                                const std::string & storage_identity = std::string());
void assign_storage_identity(ConstexprValue & value,
                             const std::string & storage_identity,
                             std::size_t pointer_offset = 0);
bool aggregate_member_value(const ConstexprValue & aggregate,
                            const std::string & name,
                            ConstexprValue & out);
bool array_element_value(const ConstexprValue & array,
                         std::size_t index,
                         ConstexprValue & out);

bool constexpr_value_truthy(const ConstexprValue & value, bool & out);
bool constexpr_value_to_integral(const ConstexprValue & value, long long & out);
bool constexpr_value_to_unsigned_integral(const ConstexprValue & value,
                                          unsigned long long & out);
bool constexpr_value_cast(const ConstexprValue & value,
                          const cpp_decl::TypePtr & target,
                          ConstexprValue & out);
bool constexpr_value_apply_unary(ETokenType op,
                                 const ConstexprValue & operand,
                                 ConstexprValue & out);
bool constexpr_value_apply_binary(ETokenType op,
                                  const ConstexprValue & lhs,
                                  const ConstexprValue & rhs,
                                  ConstexprValue & out);

}  // namespace constant_eval
