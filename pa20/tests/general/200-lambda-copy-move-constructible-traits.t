template <bool Value>
struct bool_constant
{
  static const bool value = Value;
};

typedef bool_constant<false> false_type;
typedef bool_constant<true> true_type;

template <bool Condition, class T>
struct enable_if;

template <class T>
struct enable_if<true, T>
{
  typedef T type;
};

template <bool Condition, class T>
using enable_if_t = typename enable_if<Condition, T>::type;

namespace traits
{
  template <class Target, class... Sources>
  struct constructible : true_type
  {};
}

template <class... Types>
struct type_list
{};

template <class... Types>
struct tuple
{};

template <class Tuple>
struct make_tuple_types;

template <class... Types>
struct make_tuple_types<tuple<Types...> >
{
  typedef type_list<Types...> type;
};

template <bool... Values>
struct all : bool_constant<true>
{};

struct sfinae_base
{
  template <template <class, class...> class Trait,
            class... Targets,
            class... Sources>
  static auto test(type_list<Targets...>, type_list<Sources...>)
      -> all<enable_if_t<Trait<Targets, Sources>::value, bool>{true}...>;

  template <template <class...> class>
  static auto test(...) -> false_type;

  template <class FromArgs, class ToArgs>
  using tuple_constructible = decltype(
      test<traits::constructible>(ToArgs{}, FromArgs{}));
};

template <class From, class To, bool = true, bool = true>
struct tuple_constructible : false_type
{};

template <class From, class To>
struct tuple_constructible<From, To, true, true>
    : sfinae_base::tuple_constructible<
          typename make_tuple_types<From>::type,
          typename make_tuple_types<To>::type>
{};

int main()
{
  int captured = 7;
  auto fn = [&]() { return captured; };
  typedef decltype(fn) closure;

  typedef decltype(sfinae_base::test<traits::constructible>(
      type_list<closure>{}, type_list<closure&&>{})) sfinae_result;
  static_assert(sfinae_result::value,
                "lambda constructibility must survive pack SFINAE");
  static_assert(tuple_constructible<
                    tuple<closure&&>, tuple<closure> >::value,
                "lambda tuple elements must survive layered pack SFINAE");
  return fn() - 7;
}
