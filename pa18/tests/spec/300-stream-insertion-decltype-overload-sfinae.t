// VALIDATION: compile-pass
// N3485 focus: 14.8.2 [temp.deduct], expression SFINAE in a dependent decltype.
// A failed stream-insertion overload probe in a trailing return type must drop
// that candidate instead of falling through to builtin operator analysis.

namespace local
{
  template<class Char>
  struct basic_ostream
  {
  };

  typedef basic_ostream<char> ostream;
  typedef basic_ostream<wchar_t> wostream;

  template<class T>
  T && declval();

  template<class A, class B>
  struct is_same
  {
    static const bool value = false;
  };

  template<class A>
  struct is_same<A, A>
  {
    static const bool value = true;
  };
}

struct token
{
};

local::ostream & operator<<(local::ostream &, const token &);

template<class T>
struct stream_probe
{
  template<class U>
  static auto left_shift_type(long)
      -> decltype(local::declval<local::ostream &>() << local::declval<const U &>(),
                  char());

  template<class U>
  static auto left_shift_type(int)
      -> decltype(local::declval<local::wostream &>() << local::declval<const U &>(),
                  wchar_t());

  template<class U>
  static void left_shift_type(...);

  typedef decltype(left_shift_type<T>(1L)) type;
};

static_assert(local::is_same<stream_probe<token>::type, char>::value,
              "the invalid wide insertion probe must be a substitution failure");

int main()
{
  return 0;
}
