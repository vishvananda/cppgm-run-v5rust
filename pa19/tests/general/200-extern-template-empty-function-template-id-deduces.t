namespace boost_no_cxx11_extern_template
{
  template<class T, class U>
  void f(T const* p, U const* q)
  {
    p = q;
  }

  extern template void f<>(int const*, float const*);
}

int main()
{
  return 0;
}
