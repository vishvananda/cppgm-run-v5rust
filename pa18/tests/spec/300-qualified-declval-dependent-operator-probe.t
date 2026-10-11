// VALIDATION: compile-pass
// N3485 focus: 14.6.2 [temp.dep], 14.8.2 [temp.deduct]
// A qualified declval template-id retains its dependent type argument while a
// trailing-return operator probe is parsed, then resolves during substitution.

namespace local
{
  template<class T>
  T && declval();
}

template<class Char>
struct stream
{
  stream & operator>>(bool &);
  stream & operator>>(int &);
};

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

struct token
{
};

template<class T>
struct stream_probe
{
  template<class U>
  static auto select(long)
      -> decltype(local::declval<stream<char> &>() >> local::declval<U &>(),
                  char());

  template<class U>
  static void select(...);

  typedef decltype(select<T>(1L)) type;
};

static_assert(is_same<stream_probe<int>::type, char>::value,
              "the dependent operator probe should resolve after substitution");
static_assert(is_same<stream_probe<token>::type, void>::value,
              "an invalid concrete operator probe should remain SFINAE");

int main()
{
  return 0;
}
