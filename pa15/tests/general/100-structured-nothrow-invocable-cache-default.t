namespace std {

template<class T, T V>
struct integral_constant {
  static constexpr T value = V;
};

typedef integral_constant<bool, true> true_type;
typedef integral_constant<bool, false> false_type;

template<class A, class B>
struct __and_ : integral_constant<bool, A::value && B::value> {};

template<class P>
struct __not_ : integral_constant<bool, !P::value> {};

template<class>
struct __is_fast_hash : true_type {};

template<class Fn, class Arg>
struct __is_nothrow_invocable : false_type {};

template<class T>
struct hash {
  unsigned long operator()(T) const noexcept { return 0; }
};

template<class T, class Hash>
struct cache_default
    : __not_<__and_<__is_fast_hash<Hash>, __is_nothrow_invocable<const Hash &, T const &> > > {};

}

struct X {};

int main()
{
  std::hash<X *> h;
  X * p = 0;
  unsigned long v = h(p);
  return std::cache_default<X *, std::hash<X *> >::value ? 1 : (v == 0 ? 0 : 2);
}
