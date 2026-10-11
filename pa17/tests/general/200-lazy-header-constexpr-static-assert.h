typedef unsigned long size_t;

template<class T, T v>
struct integral_constant {
  static constexpr T value = v;
  using value_type = T;
  using type = integral_constant<T, v>;
  constexpr operator value_type() const noexcept { return value; }
};

template<bool v>
using bool_constant = integral_constant<bool, v>;

using true_type = bool_constant<true>;
using false_type = bool_constant<false>;

template<class T>
struct type_identity {
  using type = T;
};

template<class T, size_t = sizeof(T)>
constexpr true_type complete_or_fallback(type_identity<T>)
{
  return {};
}

template<class TypeIdentity, class NestedType = typename TypeIdentity::type>
constexpr false_type complete_or_fallback(TypeIdentity)
{
  return {};
}

template<class T>
struct needs_complete {
  static_assert(complete_or_fallback(type_identity<T>{}), "complete");
};
