namespace fusion
{
namespace extension
{
struct access
{
  template<class Seq, int N>
  struct struct_member;
};
}
}

namespace test_detail
{
struct adapted_sequence
{
  int data;
};
}

namespace fusion
{
namespace extension
{
template<>
struct access::struct_member<test_detail::adapted_sequence, 0>
{
  typedef int type;
};
}
}

static_assert(sizeof(typename fusion::extension::access::struct_member<
                  test_detail::adapted_sequence, 0>::type) == sizeof(int),
              "qualified explicit specialization type");

int main()
{
  return 0;
}
