// VALIDATION: compile-pass
// N3485 focus: 14.5.5 [temp.class.spec]

template<class T, class U, class = void>
struct c;

template<class A, class B, class D>
struct x {};

template<class A, class B, class D>
struct c<x<A, B, D>, x<A, B, D> > {};

int main()
{
  return 0;
}
