// A dependent T&&... type-id must not share the class-template cache key used
// by an earlier bare T... pack.
namespace N {

template<class... T>
struct tuple {};

template<class T>
struct tuple<T &&>
{
  int value;
};

template<class... T>
tuple<T...> make_tuple(T...);

template<class... T>
tuple<T&&...> forward_as_tuple(T&&...)
{
  return tuple<T&&...>();
}

}

int main()
{
  N::tuple<int &&> value = N::forward_as_tuple(0);
  (void)value;
  return 0;
}
