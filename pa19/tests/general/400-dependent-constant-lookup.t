template<class T, T V>
struct integral_constant {
  static const T value = V;
  typedef T value_type;
  typedef integral_constant type;
};

template<bool B>
using bool_constant = integral_constant<bool, B>;

template<class T>
struct is_const : bool_constant<false> {};

template<class T>
struct alloc {
  static const bool ok = !is_const<T>::value;
};

template<bool B, class T, class U>
struct choose {
  typedef T type;
};

template<class T>
struct wrap {
  typedef typename choose<alloc<T>::ok, T, int>::type type;
};

template<class A, class B>
struct is_same : bool_constant<false> {};

template<class A>
struct is_same<A, A> : bool_constant<true> {};

template<class T>
struct Base : bool_constant<alloc<T>::ok> {};

template<class T>
struct Derived : Base<T> {};

static_assert(alloc<char>::ok, "");
static_assert(Derived<char>::value, "");
static_assert(is_same<char, char>::value, "");

int main() {
  return 0;
}
