// N3485 14.8.2.5p5, 14.8.2p8: a result operand is non-deduced;
// invalid substituted expressions reject candidates without demanding bodies; expect exit 0
template<class T> auto choose(T x) -> decltype(x.missing());
long choose(...);
template<class T> auto choose(T* x) -> decltype(*x) { return *x; }
template<class T> auto named(int x) -> decltype(T()) { return x; }
int main()
{
  int n = 7;
  return sizeof(choose(1)) == sizeof(long) && choose(&n) == 7
    && named<int>(3) == 3 ? 0 : 1;
}
