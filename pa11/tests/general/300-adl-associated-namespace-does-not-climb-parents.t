namespace boost_no_adl_barrier
{
  int marker = 0;

  namespace xxx
  {
    namespace nested
    {
      struct aaa
      {
      };
    }

    void begin(nested::aaa)
    {
      marker = 1;
    }
  }

  namespace nnn
  {
    void begin(xxx::nested::aaa)
    {
      marker = 2;
    }
  }

  int test()
  {
    using namespace nnn;
    xxx::nested::aaa a;
    begin(a);
    return marker;
  }
}

int main()
{
  return boost_no_adl_barrier::test();
}
