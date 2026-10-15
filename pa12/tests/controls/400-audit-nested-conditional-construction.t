// Nested conditional sources retain the declared outer call boundary; expect exit 0
struct V {
  int n;
  V(int x) : n(x) {}
  V(const V& v) : n(v.n) {}
  V(V&& v) : n(v.n) { v.n = 0; }
  ~V() {}
};
int copy(bool a, bool b, V& v) {
  V w = a ? (b ? v : V(2)) : V(3);
  return w.n;
}
int direct(bool a, bool b, V& v) {
  V w(a ? (b ? v : V(2)) : V(3));
  return w.n;
}
int main() {
  V v(7);
  return copy(true, true, v) == 7 && copy(true, false, v) == 2 &&
         copy(false, true, v) == 3 && direct(true, true, v) == 7 &&
         direct(true, false, v) == 2 && direct(false, false, v) == 3 ? 0 : 1;
}
