struct B {}; struct D : B {}; struct M : D {};
template<class D, class B> void f(const D *, const B *) {}
void g()
{
  f<D, B>((D *)0, (B *)0);
  f<M, D>((M *)0, (D *)0);
}
