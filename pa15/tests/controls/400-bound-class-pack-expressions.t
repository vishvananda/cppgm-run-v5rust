// Each nested expansion owns its packs; sizeof... retains the whole pack. expect exit 0
int sum(int a, int b) { return a + b; }
int pair(int a, int b) { return a * 10 + b; }
template<int... N> struct Values {
  static int data[sizeof...(N)];
  int items[sizeof...(N)];
  Values() : items{N...} {}
  int run() { return sum(pair(N, sum(N...))...); }
  template<int K> int shifted() { return sum((N + K)...); }
  void unused() { static_assert(sizeof...(N) == 0, "unused body"); }
};
template<int... N> int Values<N...>::data[] = {N...};
int main() {
  Values<1, 2> value;
  return value.run() == 36 && value.shifted<3>() == 9 && value.items[0] == 1 && Values<1, 2>::data[1] == 2 ? 0 : 1;
}
