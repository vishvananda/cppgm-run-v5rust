// N3485 focus: 14.8.2 substitution of default template arguments and
// converted constant expressions for non-type template arguments.
template<bool B, class T>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T>
using enable_if_t = typename enable_if<B, T>::type;

template<class T, class U>
struct check {
  template<class A, class B>
  static constexpr bool f() { return true; }
};

template<class T, class U>
struct pair_like {
  template<class C = check<T, U>,
           enable_if_t<C::template f<T const&, U const&>(), int> = 0>
  pair_like(T const&, U const&) {}
};

int main()
{
  int x = 1;
  pair_like<int, int> p(x, x);
  return 0;
}
