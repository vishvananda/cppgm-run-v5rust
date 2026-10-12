// Retrying the name as a type must still reject a missing expression.
using G = Unknown;
void f() {
  foo<bar<G::V +> > invalid;
}
