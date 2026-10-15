// N3485 12.8p16: list-selected trivial copies retain the storage transfer
// form, including a prvalue source; expect exit 0
struct A { int n; A() : n(7) {} A(const A&) = default; };
int main() {
  A source;
  A a{source};
  A b = {source};
  A c{A()};
  A d = {A()};
  return a.n + b.n + c.n + d.n == 28 ? 0 : 1;
}
