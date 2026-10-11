template <class T, T V>
struct integral_constant {
  static constexpr T value = V;
};

template <class T>
struct is_trivially_copyable : integral_constant<bool, __is_trivially_copyable(T)> {};

template <class T>
struct A {
  static_assert(is_trivially_copyable<T>::value, "trivial");
};

A<long long> a;

int main() { return 0; }
