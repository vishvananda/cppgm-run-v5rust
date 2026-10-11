// N3485 focus: 14.2 [temp.names], 14.5.5 [temp.class.spec]

template <bool B>
struct bool_constant
{
  static const bool value = B;
};

namespace execution
{
namespace detail
{
template <int N>
struct tracked_t {};

template <int N>
struct untracked_t {};
}

template <class T>
struct prefer_only {};

namespace detail
{
template <unsigned long I, class Props>
struct supportable_properties;

template <unsigned long I, class Prop>
struct supportable_properties<I, void(Prop)>
{
  template <class T>
  struct is_valid_target : bool_constant<true> {};
};

template <unsigned long I, class Head, class... Tail>
struct supportable_properties<I, void(Head, Tail...)>
{
  template <class T>
  struct is_valid_target
    : bool_constant<
          supportable_properties<I, void(Head)>::template
              is_valid_target<T>::value &&
          supportable_properties<I + 1, void(Tail...)>::template
              is_valid_target<T>::value>
  {
  };
};
}
}

struct target {};

typedef void props(
    execution::prefer_only<execution::detail::tracked_t<0> >,
    execution::prefer_only<execution::detail::untracked_t<0> >);

static_assert(
    execution::detail::supportable_properties<0, props>::template
        is_valid_target<target>::value,
    "qualified member template-id value should evaluate");

int main()
{
  return 0;
}
