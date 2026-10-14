// N3485 12.8p31, 8.5p16, 13.3.1.3: A a = A() may elide only a copy/move
// constructor; A(A&) cannot bind the prvalue, so A(R) via operator R() must
// run (v == 11); expect exit 0
struct A {
  int v;
  struct R { int v; };
  A() : v(1) {}
  A(A& o) : v(o.v + 100) {}
  A(R r) : v(r.v + 10) {}
  operator R() { R r; r.v = v; return r; }
};
int take(A a) { return a.v; }
int main() { A a = A(); return (a.v == 11 && take(A()) == 11) ? 0 : 1; }
