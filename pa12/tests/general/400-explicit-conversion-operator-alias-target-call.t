struct runtime_traits
{
  int value;

  int make() const
  {
    return value;
  }
};

struct compile_traits
{
  typedef runtime_traits adaptor_type;

  operator adaptor_type() const
  {
    adaptor_type result = {7};
    return result;
  }
};

struct test_traits
{
  typedef compile_traits ct_traits_type;
  typedef runtime_traits rt_traits_type;

  static int make_runtime()
  {
    return ct_traits_type().operator rt_traits_type().make();
  }
};

int main()
{
  return test_traits::make_runtime() == 7 ? 0 : 1;
}
