namespace boost
{
namespace mpl
{
template<class T>
struct identity
{
typedef T type;
};
}

namespace fusion
{
namespace vector_detail
{
template<int I, class T>
struct store
{
  T elem;
};

template<int N, class U>
static mpl::identity<U> value_at_impl(const volatile store<N, U>*)
{
  return mpl::identity<U>();
}
}

template<class T>
struct vector : vector_detail::store<0, T>
{
};
}
}

typedef const boost::fusion::vector<int>* vector_ptr;

vector_ptr&& move_ptr(vector_ptr& p)
{
  return static_cast<vector_ptr&&>(p);
}

int main()
{
  boost::fusion::vector<int> v;
  vector_ptr p = &v;
  boost::mpl::identity<int> x =
      boost::fusion::vector_detail::value_at_impl<0>(move_ptr(p));
  return sizeof(x) == sizeof(boost::mpl::identity<int>) ? 0 : 1;
}
