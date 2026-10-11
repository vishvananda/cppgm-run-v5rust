template<class T, T v>
struct integral_constant {
  typedef integral_constant type;
  typedef T value_type;
  static const T value = v;
};

template<class T, class... Args>
struct is_constructible
    : integral_constant<bool, __is_constructible(T, Args...)> {};

template<class B, class D>
struct is_base_of : integral_constant<bool, __is_base_of(B, D)> {};

struct B {};
struct D : B {};

static_assert(is_constructible<int>::type::value, "");
static_assert(is_base_of<B, D>::type::value, "");

int main()
{
  return 0;
}
