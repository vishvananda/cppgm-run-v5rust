template<bool, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<class From, class To>
struct is_convertible {
  static const bool value = false;
};

template<class T>
struct is_convertible<const T&, T> {
  static const bool value = true;
};

template<class Rep>
struct duration_like {
  typedef Rep rep;

  rep value;

  template<class Rep2, enable_if_t<is_convertible<const Rep2&, rep>::value, int> = 0>
  explicit duration_like(const Rep2& r) : value(r) {}

  template<class Rep2,
           enable_if_t<!is_convertible<const duration_like<Rep2>&, rep>::value, int> = 0>
  duration_like(const duration_like<Rep2>& other) : value(other.value) {}
};

int main() {
  duration_like<long long> src(7LL);
  duration_like<int> dst(src);
  return dst.value;
}
