// N3485 14.3.2, 15.4p1: resolving a specification can instantiate a
// class that declares nested templates. Each constant query uses its own
// typed facts; definition-time operands remain dependent. expect exit 0
template<int N> struct flag { static const bool value = N != 0; };
template<class T> struct metadata {
  template<int N> struct nested {
    typedef flag<N> named;
    typedef flag<+N> unary;
    typedef flag<+1> fixed_unary;
    typedef flag<int(1)> fixed_cast;
    typedef flag<(true ? 1 : 2L)> fixed_conditional;
    typedef flag<(1 + 2)> fixed_binary;
    typedef flag<int(N)> cast;
    typedef flag<(N > 0 ? N : -N)> conditional;
    typedef flag<(N + 1)> binary;
  };
  static const bool value = true;
};
void target() noexcept(metadata<int>::value);
int main() {
  static_assert(noexcept(target()), "specification");
  static_assert(metadata<int>::nested<2>::named::value, "named");
  static_assert(metadata<int>::nested<2>::unary::value, "unary");
  static_assert(metadata<int>::nested<2>::cast::value, "cast");
  static_assert(metadata<int>::nested<2>::conditional::value, "conditional");
  static_assert(metadata<int>::nested<2>::binary::value, "binary");
  static_assert(metadata<int>::nested<2>::fixed_unary::value, "fixed unary");
  static_assert(metadata<int>::nested<2>::fixed_cast::value, "fixed cast");
  static_assert(metadata<int>::nested<2>::fixed_conditional::value, "fixed conditional");
  static_assert(metadata<int>::nested<2>::fixed_binary::value, "fixed binary");
  return 0;
}
