// A concrete class-template specialization must retain each exact type-pack
// element while expanding a variadic base clause.  Namespace-qualified pack
// elements must not be looked up again from rendered type text.

template<bool B>
struct bool_constant
{
  static constexpr bool value = B;
};

namespace policy_impl
{
struct sign_policy
{
  static constexpr bool return_has_sign = true;
};

struct trailing_policy
{
  static constexpr bool report_trailing_zeros = false;
};

template<class... Policies>
struct policy_holder : Policies... {};
}

template<class SignPolicy>
int check_policy_holder()
{
  using namespace policy_impl;
  using holder_type = policy_holder<SignPolicy, trailing_policy>;
  using result = bool_constant<holder_type::return_has_sign &&
                               !holder_type::report_trailing_zeros>;
  return result::value ? 0 : 1;
}

int main()
{
  return check_policy_holder<policy_impl::sign_policy>();
}
