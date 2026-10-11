template<bool, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<class A, class B>
struct is_same {
  static const bool value = false;
};

template<class T>
struct is_same<T, T> {
  static const bool value = true;
};

template<class Rep>
struct duration_like {
  typedef Rep rep;

  rep value;

  template<class Rep2, enable_if_t<is_same<Rep2, rep>::value, int> = 0>
  explicit duration_like(const Rep2 & r) : value(r) {}
};

int main() {
  duration_like<int> x(1);
  return x.value;
}
