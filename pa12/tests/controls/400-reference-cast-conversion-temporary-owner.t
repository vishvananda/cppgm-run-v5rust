// N3485 5.2.9p4, 12.2p5: a reference cast's constructed temporary is
// owned by the local reference that binds its result; expect exit 0
int destroyed;
struct X { int n; };
struct A {
  int n;
  explicit A(const X& x) : n(x.n) {}
  ~A() { ++destroyed; }
};
int main() {
  X x;
  x.n = 9;
  {
    const A& a = static_cast<const A&>(x);
    if (destroyed || a.n != 9) return 1;
  }
  return destroyed == 1 ? 0 : 2;
}
