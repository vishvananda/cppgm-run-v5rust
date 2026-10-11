namespace fusion
{
namespace extension
{
template<class Tag>
struct begin_impl;
}

struct mpl_sequence_tag
{
};

namespace extension
{
template<>
struct begin_impl<mpl_sequence_tag>
{
  template<class Sequence>
  struct apply
  {
    typedef int type;
  };
};

template<class Tag>
struct begin_impl
{
  template<class Sequence>
  struct apply
  {
  };
};

template<>
struct begin_impl<mpl_sequence_tag>;
}
}

int main()
{
  typedef typename fusion::extension::begin_impl<
      fusion::mpl_sequence_tag>::template apply<int>::type result;
  result value = 0;
  return value;
}
