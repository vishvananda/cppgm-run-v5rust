template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<class A, class B>
struct is_same {
  static constexpr bool value = false;
};

template<class A>
struct is_same<A, A> {
  static constexpr bool value = true;
};

template<class Alloc>
struct holder {
  template<class _Tp, enable_if_t<is_same<_Tp, int>::value, int> = 0>
  static void construct(_Tp *);
};

template<class _Tp>
void outer() {
  holder<int> value;
  (void)value;
}

int main() {
  outer<double>();
}
