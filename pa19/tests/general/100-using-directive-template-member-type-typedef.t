namespace boost
{
  namespace move_detail
  {
    template<unsigned N>
    struct aligned_storage
    {
      struct type
      {
        char data[N];
      };
    };
  }
}

namespace boost
{
  namespace container
  {
    namespace dtl
    {
      using ::boost::move_detail::aligned_storage;
    }
  }
}

using namespace boost::container;

struct X
{
  int value;
};

int f()
{
  typedef dtl::aligned_storage<sizeof(X)>::type storage_t;
  storage_t storage;
  storage.data[0] = 7;
  return storage.data[0];
}

int main()
{
  return f();
}
