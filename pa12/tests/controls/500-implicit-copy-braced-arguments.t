// expect exit 0
// Braced argument ranking includes implicit copy declarations, without requiring
// user-declared copy constructors. The list owns its parameter temporary.
struct A {
  int n;
  A() : n(7) {}
  ~A() {}
};
int use(A a) { return a.n; }
int main() {
  A a;
  A b({a});
  A c{{a}};
  return b.n + c.n + use({a}) == 21 ? 0 : 1;
}
