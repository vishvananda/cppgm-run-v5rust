// VALIDATION: compile-pass
// N3485 focus: 14.8.2 [temp.deduct], 14.8.3 [temp.over],
// 14.6.4.1 [temp.point]
// A dependent function-template result containing a non-type sizeof argument
// must be revalidated after substitution. If the sizeof target is incomplete,
// the candidate is a substitution failure, and an inherited trait base that
// carries the expression must resolve after the class template argument is
// substituted.

template<typename T, T V>
struct integral_constant
{
  static const T value = V;
  typedef integral_constant<T, V> type;
};

template<bool V>
struct bool_constant
{
  static const bool value = V;
  typedef bool_constant<V> type;
};

template<unsigned long N>
struct ok_tag
{
  char payload[N + 1];
};

template<typename T>
ok_tag<sizeof(T)> check_complete(int);

template<typename T>
char check_complete(...);

struct incomplete;

struct complete
{};

template<typename T>
struct is_complete
    : bool_constant<(sizeof(check_complete<T>(0)) != sizeof(char))>
{};

template<typename T>
struct nested_type_wknd
    : T::type
{};

static_assert(!is_complete<incomplete>::value, "incomplete is not complete");
static_assert(!nested_type_wknd<is_complete<incomplete> >::value,
              "nested type path preserves the fallback result");
static_assert(is_complete<complete>::value, "complete is complete");

int main()
{
  return nested_type_wknd<is_complete<incomplete> >::value ? 1 : 0;
}
