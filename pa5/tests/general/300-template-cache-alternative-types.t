// Syntax-only: the alias's class is unknown. A failed value reading of
// the argument must not suppress the later type reading of the same '<'.
using G = Unknown;
void f() {
  foo<G::V*> pointer;
  foo<G::V&> reference;
  foo<G::V * value> expression;
}
