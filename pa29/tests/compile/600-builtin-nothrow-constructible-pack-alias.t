template<bool B>
struct integral_constant {
  static constexpr bool value = B;
};

template<bool B>
using bool_constant = integral_constant<B>;

template<class T, class... Args>
using nothrow_constructible_impl =
    bool_constant<__is_nothrow_constructible(T, Args...)>;

template<class T, class... Args>
struct is_nothrow_constructible : nothrow_constructible_impl<T, Args...> {};

template<class... T>
struct tuple {};

struct P {
  P(int, tuple<int &&>, tuple<>) noexcept {}
};

static_assert(is_nothrow_constructible<P, int, tuple<int &&>, tuple<> >::value, "");

int main()
{
  return 0;
}
