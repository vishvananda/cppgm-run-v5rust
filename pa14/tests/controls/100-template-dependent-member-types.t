// N3485 14.6.2.1, 14.8.2.5: dependent member types resolve in the substituted
// class; their qualifiers are non-deduced contexts; expect exit 0
struct base { typedef int value_type; };
struct policy : base {
  struct nested { typedef short value_type; };
};
template<class T> struct deferred_fields { typename T::value_type n : 1; };
template<class T> typename T::value_type identity(T, typename T::value_type);
template<class U> typename U::value_type identity(U, typename U::value_type x) {
  using result = typename U::value_type;
  result value = x;
  return value;
}
template<class T> typename T::nested::value_type nested(T) { return 5; }
template<class T> int array(T, const typename T::value_type (&a)[3]) {
  return a[0] + a[1] + a[2];
}
template<class T, class U = typename T::value_type> int width(T) {
  return sizeof(U);
}
int main() {
  policy p;
  int values[3] = {1, 2, 3};
  int (*function)(policy, int) = identity;
  return function(p, 4) == 4 && nested(p) == 5 && array(p, values) == 6
      && width(p) == sizeof(int) ? 0 : 1;
}
