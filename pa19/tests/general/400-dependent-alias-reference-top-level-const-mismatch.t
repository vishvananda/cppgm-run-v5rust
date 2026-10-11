template<class A, class B> struct is_same { static constexpr bool value = false; };
template<class A> struct is_same<A, A> { static constexpr bool value = true; };
template<class T> using add_lvalue_reference_t = T&;

template<class T> struct check {
  static constexpr bool value =
      is_same<add_lvalue_reference_t<T>, add_lvalue_reference_t<const T>>::value;
};

static_assert(!check<char*>::value, "char*");
static_assert(!check<const char*>::value, "const char*");

int main() { return 0; }
