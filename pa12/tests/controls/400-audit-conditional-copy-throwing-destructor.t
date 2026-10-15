// Throwing default destructors retain the completed local on unwind; expect exit 0
int live = 0;
int peak = 0;
struct T {
  T() { ++live; if (live > peak) peak = live; }
  ~T() noexcept(false) { --live; }
};
struct V {
  int n;
  V() : n(7) {}
  V(const V& v, const T& = T()) : n(v.n) {}
  ~V() {}
};
int copy(bool c, V& v) {
  V w = c ? v : V();
  return w.n;
}
int main() {
  V v;
  int a = copy(true, v);
  if (live != 0 || peak != 2 || a != 7) return 1;
  peak = 0;
  int b = copy(false, v);
  return live == 0 && peak == 1 && b == 7 ? 0 : 2;
}
