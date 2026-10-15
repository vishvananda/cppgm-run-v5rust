// N3485 8.5p16, 12.8p31: the selected conversion result constructs its
// member, base or allocated destination with normal cleanup; expect exit 0
int destroyed;
struct A { int n; A() : n(5) {} A(const A& a) : n(a.n) {} ~A() { ++destroyed; } };
struct S { operator A() { A a; return a; } };
struct Member { A a; Member(S& s) : a(s) {} };
struct Base : A { Base(S& s) : A(s) {} };
int main() {
  S s;
  {
    Member m(s);
    Base b(s);
    A *a = new A(s);
    if (destroyed || m.a.n + b.n + a->n != 15) return 1;
    delete a;
  }
  return destroyed == 3 ? 0 : 2;
}
