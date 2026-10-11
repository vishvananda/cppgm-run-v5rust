// Reduced from hosted std::forward_as_tuple: after deduction, a dependent
// class-template return type using T&&... must resolve from carried syntax.
namespace N {

template<class... T>
struct tuple {};

template<class... T>
tuple<T&&...> forward_as_tuple(T&&...)
{
  return tuple<T&&...>();
}

}

int main()
{
  int x = 0;
  N::tuple<int &> t = N::forward_as_tuple(x);
  (void)t;
  return 0;
}
