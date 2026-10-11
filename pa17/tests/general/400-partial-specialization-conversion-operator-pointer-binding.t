// VALIDATION: compile-pass
// A conversion operator declared in a class-template partial specialization
// must substitute the partial specialization's parameter binding, not the
// primary template argument. The pointer specialization below binds
// Formatter to the function type, so operator Formatter* returns a function
// pointer and participates in surrogate-call overload resolution.

typedef char no_type;
typedef char (&unary_type)[2];
typedef char (&binary_type)[3];
typedef char (&ternary_type)[4];

struct any_type
{
  any_type(...);
};

no_type check_is_formatter(unary_type, binary_type, ternary_type);

template<class T>
unary_type check_is_formatter(T const &, binary_type, ternary_type);

template<class T>
binary_type check_is_formatter(unary_type, T const &, ternary_type);

template<class T, class U>
binary_type check_is_formatter(T const &, U const &, ternary_type);

template<class T>
ternary_type check_is_formatter(unary_type, binary_type, T const &);

template<class T, class U>
ternary_type check_is_formatter(T const &, binary_type, U const &);

template<class T, class U>
ternary_type check_is_formatter(unary_type, T const &, U const &);

template<class T, class U, class V>
ternary_type check_is_formatter(T const &, U const &, V const &);

struct unary_binary_ternary
{
  typedef unary_type (*unary_fun)(any_type);
  typedef binary_type (*binary_fun)(any_type, any_type);
  typedef ternary_type (*ternary_fun)(any_type, any_type, any_type);
  operator unary_fun();
  operator binary_fun();
  operator ternary_fun();
};

template<class Formatter>
struct formatter_wrapper;

template<class Formatter>
struct formatter_wrapper<Formatter *> : unary_binary_ternary
{
  operator Formatter *();
};

struct what_type {};
struct out_type {};
struct string_like {};

template<class Formatter, class What, class Out>
struct formatter_arity
{
  static formatter_wrapper<Formatter> &formatter;
  static What &what;
  static Out &out;

  enum {
    value = sizeof(check_is_formatter(
      formatter(what),
      formatter(what, out),
      formatter(what, out, 0)
    )) - 1
  };
};

typedef string_like formatter_function(what_type const &);

static_assert(formatter_arity<formatter_function *, what_type, out_type>::value == 1,
              "partial specialization converted to the wrong callable type");

int main()
{
  return 0;
}
