// N3485 5.3.3p5: sizeof... retains function-pack identity in signature operands.
// expect exit 0
template<class... T> auto array(T... t) -> int (*)[sizeof...(t) + 1];
template<class... U> auto array(U... u) -> int (*)[sizeof...(u) + 1]
{
  return nullptr;
}
template<class... T> auto count(T... t) -> decltype(sizeof...(t))
{
  return sizeof...(t);
}
template<int... N> decltype(sizeof...(N)) values() { return sizeof...(N); }
int main()
{
  return sizeof(*array()) != 4 || sizeof(*array(1, 2L)) != 12 ||
         count(1, 2L) != 2 || values<3, 4>() != 2;
}
