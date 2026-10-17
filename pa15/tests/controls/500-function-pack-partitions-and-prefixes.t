// N3485 14.8.1p9, 14.8.2.1p1: explicit prefixes and canonical partitions;
// expect exit 0
template<class...> struct List {};
template<class... T> int count(T... t) { return sizeof...(t); }
template<class... T, int N = 7> int with_default() { return sizeof...(T) + N; }
template<class... U, class... T> int partition(List<U...>, T... t)
{
  return sizeof...(U) * 10 + sizeof...(t);
}
int (*address)(List<char, short>, int) = &partition<char, short>;
int main()
{
  return with_default<int, long, char>() != 10 || count() || count<int>('x', 2.0) != 2 ||
         partition(List<int, long>(), 1) != 21 ||
         partition(List<int>(), 1, 2) != 12 ||
         address(List<char, short>(), 4) != 21;
}
