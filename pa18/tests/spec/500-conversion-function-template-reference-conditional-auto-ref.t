// VALIDATION: compile-pass
// N3485 focus: 5.16 [expr.cond], 7.1.6.4 [dcl.spec.auto],
// 12.3.2 [class.conv.fct], 14.8.2.3 [temp.deduct.conv]

template<class T>
struct static_storage {
  static constexpr T value = T();
};

template<class T>
constexpr T static_storage<T>::value;

template<class T>
constexpr const T& static_const_var() {
  return static_storage<T>::value;
}

struct unique_tag {};
struct callable_tag {};

template<class T>
struct deduce_unique {
  constexpr deduce_unique() {}

  template<class F>
  constexpr operator const F&() const {
    return static_const_var<F>();
  }
};

template<class F>
struct wrapper {
  int value;
  constexpr wrapper() : value(11) {}
};

template<class T>
struct factor {
  constexpr factor() {}

  template<class F>
  constexpr const wrapper<F>& operator=(const F&) const {
    return static_const_var<wrapper<F> >();
  }
};

static constexpr auto& selected =
    true ? deduce_unique<unique_tag>() : factor<unique_tag>() = callable_tag();

int main() {
  return selected.value == 11 ? 0 : 1;
}
