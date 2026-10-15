// N3485 13.3.1.4p1 (CWG 899): a single direct argument admits explicit
// conversions to the copy constructor's class reference; expect exit 0
int copies;
int destroyed;
struct A {
  int n;
  A() : n(11) {}
  A(const A& a) : n(a.n) { ++copies; }
  ~A() { ++destroyed; }
};
struct S { explicit operator A() const { A a; return a; } };
int main() {
  S s;
  {
    A a(s);
    A b{s};
    A c = A(s);
    A d = static_cast<A>(s);
    if (copies != 4 || a.n + b.n + c.n + d.n != 44) return 1;
  }
  return destroyed == 8 ? 0 : 2;
}
