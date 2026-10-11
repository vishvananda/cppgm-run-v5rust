template<class T, T V>
struct integral_constant {
  static constexpr T value = V;
};

template<class X>
struct iterator_traits;

template<class X>
using iterator_category_t = typename iterator_traits<X>::difference_type;

template<class Default, template<class> class Op, class Arg>
using detected_or_t = Default;

struct nat {};

template<class T, class U>
struct outer {
  struct inner : integral_constant<bool, __is_convertible(detected_or_t<nat, iterator_category_t, T>, U)> {};
};

typedef outer<int, int> outer_type;

int main() {
  return 0;
}
