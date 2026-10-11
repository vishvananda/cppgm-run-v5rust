// Interning a function template's result identity asks whether a piece of its
// syntax happens to name a concrete type, and falls through when it does not.
// The set of names it treats as dependent holds that template's own
// parameters, so a member template whose parameter type is written over the
// *enclosing class's* parameter is not recognised as dependent and the syntax
// is built as a type -- libc++ writes
// `enable_if_t<__is_allocator<_Allocator>::value, int>`, where resolving
// `__is_allocator<_Allocator>::value` as a type fails on the `::value`, which
// is exactly the evidence that it is a value.  Asking must not be fatal.

template<bool B, class T = void>
struct enable_if
{
};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<class A>
struct is_alloc
{
  static const bool value = true;
};

template<class C, class A>
struct str
{
  // Over the enclosing class's parameter: the failing shape.
  template<enable_if_t<is_alloc<A>::value, int> = 0>
  str(const C *)
  {
  }

  // Over the member template's own parameter: already dependent, and must
  // keep working.
  template<class X, enable_if_t<is_alloc<X>::value, int> = 0>
  void take(X)
  {
  }
};

int main()
{
  str<char, int> made(0);
  made.take(0);
  return 0;
}
