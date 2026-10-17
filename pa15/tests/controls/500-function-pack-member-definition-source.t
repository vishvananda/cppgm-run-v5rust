// N3485 14.5.3p4-5, 14.7.1p3, 15.4: retained member clauses and body demand;
// expect exit 0
int sum(int a, long b) noexcept { return a + b; }
template<class... T> struct Owner
{
  int inline_call(T... t) noexcept(noexcept(sum(t...))) { return sum(t...); }
  template<int N> int inner(T... t) { return sum(t...) + N; }
  int outside(T...);
};
template<class... U> int Owner<U...>::outside(U... u) { return sum(u...); }
int main()
{
  Owner<int, long> o;
  return !noexcept(o.inline_call(1, 2)) || o.inline_call(1, 2) != 3 ||
         o.inner<4>(1, 2) != 7 || o.outside(1, 2) != 3;
}
