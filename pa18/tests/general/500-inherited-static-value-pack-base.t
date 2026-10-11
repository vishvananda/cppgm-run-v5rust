// VALIDATION: compile-pass
// N3485 focus: 14.5.3 [temp.variadic], 14.6.4.1 [temp.point]
// A class-template base containing a pack expansion of inherited static
// member values must substitute each concrete pack element before resolving
// the base specialization.

template<typename T, T V>
struct integral_constant
{
  static const T value = V;
};

typedef integral_constant<bool, false> false_type;
typedef integral_constant<bool, true> true_type;

namespace traits
{

template<typename From, typename To>
struct trait : true_type
{};

}

template<typename... Conditions>
struct all_impl : false_type
{};

template<typename... Types>
struct all_impl<integral_constant<Types, true>...> : true_type
{};

template<bool... Conditions>
struct all_values : all_impl<integral_constant<bool, Conditions>...>
{};

template<typename... Conditions>
struct all : all_values<Conditions::value...>
{};

static_assert(all<traits::trait<int, int>>::value,
              "inherited static values expand through the base pack");

int main()
{
  return all<traits::trait<int, int>>::value ? 0 : 1;
}
