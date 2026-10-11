template <class T> struct is_lvalue_reference { static constexpr bool value = false; };
template <class T> struct is_lvalue_reference<T&> { static constexpr bool value = true; };

struct X {
  template<class U> X(U&) {}
  template<class U> X(U&&) { static_assert(!is_lvalue_reference<U>::value, "bad"); }
};

X g(unsigned long& x) {
  return X(x);
}
