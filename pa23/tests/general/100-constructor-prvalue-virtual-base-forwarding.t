struct A { int a; A(int value = 0) : a(value) {} };
struct B { int b; B(int value = 0) : b(value) {} };
struct C { int c; C(int value = 0) : c(value) {} };
struct D : virtual A, virtual B, virtual C {};

struct EmptyRoot { int value() { return 7; } };
struct EmptyDerived : virtual EmptyRoot {};

template<class E>
struct W : E {
  W(E const & e, int const & loc) : A(loc), B(e.b), C(e.c), E(e) {}
};

template<class E>
W<E> make_wrap(E const & e, int const & loc)
{
  return W<E>(e, loc);
}

int main()
{
  EmptyDerived source;
  EmptyDerived copy(source);
  EmptyDerived moved(static_cast<EmptyDerived&&>(source));
  if ((copy.*&EmptyRoot::value)() != 7 || (moved.*&EmptyRoot::value)() != 7)
    return 2;

  D d;
  d.a = 7;
  d.b = 8;
  d.c = 9;
  W<D> w = make_wrap(d, d.a);
  return w.a == 7 && w.b == 8 && w.c == 9 ? 0 : 1;
}
