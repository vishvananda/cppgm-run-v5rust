namespace lib
{
  typedef decltype(sizeof(0)) size_type;

  template<typename T, size_type N>
  struct fixed_array;

  template<typename T, lib::size_type N>
  struct fixed_array
  {
    T data[N];
  };
}

int main()
{
  return 0;
}
