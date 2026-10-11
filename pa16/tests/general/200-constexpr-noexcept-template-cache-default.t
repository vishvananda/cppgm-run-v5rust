template<class T, T Value>
struct integral_constant {
  static const T value = Value;
};

template<class T, T Value>
const T integral_constant<T, Value>::value;

template<class T> T&& declval() noexcept;

template<class F, class Arg>
struct __is_nothrow_invocable
  : integral_constant<bool, noexcept(declval<F>()(declval<Arg>()))> {};

template<class T>
struct __not_ : integral_constant<bool, !T::value> {};

template<class Value, class Hash>
struct __cache_default
  : __not_<__is_nothrow_invocable<const Hash &, const Value &> > {};

template<class Value>
struct Hash {
  int operator()(Value) const noexcept { return 0; }
};

static_assert(!__cache_default<int *, Hash<int *> >::value, "cache");

int main()
{
  return 0;
}
