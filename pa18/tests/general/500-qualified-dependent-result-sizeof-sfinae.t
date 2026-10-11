// VALIDATION: compile-pass
// N3485 focus: 14.8.2 [temp.deduct], 14.8.3 [temp.over]
// A qualified explicit-template call must discard a function-template
// candidate whose dependent result type becomes ill-formed, then select the
// viable fallback overload in an unevaluated sizeof operand.

template<typename T, T V>
struct integral_constant
{
  static const T value = V;
};

template<unsigned long N>
struct ok_tag
{
  double d;
  char payload[N];
};

namespace detail
{

template<typename T>
ok_tag<sizeof(T)> check_complete(int);

template<typename T>
char check_complete(...);

}

struct incomplete;

struct complete
{};

template<typename T>
struct is_complete
    : integral_constant<bool,
                        (sizeof(detail::check_complete<T>(0)) != sizeof(char))>
{};

static_assert(!is_complete<incomplete>::value, "incomplete is not complete");
static_assert(is_complete<complete>::value, "complete is complete");

int main()
{
  return is_complete<incomplete>::value || !is_complete<complete>::value;
}
