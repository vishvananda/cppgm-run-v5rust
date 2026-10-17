// N3485 8.3.6p3, 14.7.1p1: expansion moves trailing defaults; each is demanded once.
// expect exit 0
template<class... T> int count(T... t, int n = sizeof...(T), int bias = 7)
{
  return sizeof...(t) + n + bias;
}
template<class... T> int unused(T... t, int n = T::bad) { return n; }
template<class... T> struct Invalid
{
  static_assert(sizeof...(T) == 99, "unneeded return type");
  using type = int;
};
template<class... T> int arity(...) { return 5; }
template<class... T> typename Invalid<T...>::type arity(int, T...);
int main()
{
  return count<>() != 7 || count<int>(1) != 9 ||
         count<int, long>(1, 2) != 11 || count<int>(2) != 9 ||
         count<int, long>(1, 2, 3) != 12 || unused<int, long>(1, 2, 4) != 4 ||
         arity<char>() != 5;
}
