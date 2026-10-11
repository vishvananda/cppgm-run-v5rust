struct A { int a; };
struct B { int b; };
struct C { int c; };
struct D : virtual A, virtual B, virtual C {};

template<class E, class V>
E const & set_info(E const & x, V const &)
{
  (void)x.a;
  return x;
}

template<class E, class V>
E const & operator<<(E const & x, V const & v)
{
  return set_info(x, v);
}

int main()
{
  D d;
  d.a = 5;
  return (d << 1).a == 5 ? 0 : 1;
}
