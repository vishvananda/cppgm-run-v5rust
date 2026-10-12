// Failure in an inner speculative list abandons its enclosing lists.
// Every list still needs a fresh attempt under a different name category.
using G = Unknown;
void f() {
  foo<bar<G::V*> > pointer;
  foo<bar<G::V&> > reference;
  foo<bar<G::V * value> > expression;
}
