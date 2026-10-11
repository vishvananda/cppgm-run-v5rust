namespace boost {
namespace fusion {
template<class K, class T>
struct pair {};

template<class... P>
struct map {
  map() {}
  template<class... T>
  map(T const&...) {}
};

namespace detail {
template<class T>
struct as_element {
  typedef T type;
};
}

template<class... Key, class... T>
map<pair<Key, typename detail::as_element<T>::type>...> make_map(T const&... arg)
{
  typedef map<pair<Key, typename detail::as_element<T>::type>...> result_type;
  return result_type(arg...);
}
}
}

template<class Map>
void test_map(Map const& map_value)
{
  (void)map_value;
}

struct A {};
struct B {};

int main()
{
  using namespace boost::fusion;
  test_map(make_map<A, B>(1, 'x'));
  return 0;
}
