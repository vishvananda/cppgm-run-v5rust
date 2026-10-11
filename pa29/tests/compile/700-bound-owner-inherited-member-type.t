// N3485 focus: [temp.dep.type] and [temp.arg.type].
// A dependent qualified member type whose owner is a bound template argument
// must still use class member lookup, including inherited typedefs.
template<bool B>
struct bool_constant {
  static const bool value = B;
};

template<class A, class B>
struct is_same : bool_constant<__is_same(A, B)> {};

template<class T, class I, I E>
struct traits_base {
  typedef T char_type;
};

template<class T>
struct traits;

template<>
struct traits<wchar_t> : traits_base<wchar_t, int, -1> {};

template<class C, class traits_type>
struct holder {
  static_assert(is_same<C, typename traits_type::char_type>::value, "match");
};

holder<wchar_t, traits<wchar_t> > value;

int main()
{
  (void)value;
  return 0;
}
