template<class T, T V>
struct integral_constant {
  static inline constexpr const T value = V;
  typedef T value_type;
  typedef integral_constant type;
};

template<bool B>
using bool_constant = integral_constant<bool, B>;

template<class A, class B>
struct is_same : bool_constant<false> {};

template<class A>
struct is_same<A, A> : bool_constant<true> {};

static_assert(is_same<char, char>::value, "");

int main() {
  return 0;
}
