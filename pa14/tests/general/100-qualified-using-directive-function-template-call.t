namespace lib
{
  namespace range_adl_barrier
  {
    template<class T>
    int begin(T &range)
    {
      return range.begin();
    }
  }

  using namespace range_adl_barrier;

  namespace algorithm
  {
    template<class T>
    int call_begin(T &range)
    {
      return lib::begin(range);
    }
  }
}

struct range_like
{
  int begin()
  {
    return 7;
  }
};

int main()
{
  range_like range;
  return lib::algorithm::call_begin(range) == 7 ? 0 : 1;
}
