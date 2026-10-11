namespace test_support {
struct true_type { static const bool value = true; };
struct false_type { static const bool value = false; };

template<class T> T && declval();

struct destructible_probe {
  template<class T, class = decltype(declval<T&>().~T())>
  static true_type test(int);

  template<class>
  static false_type test(...);
};

template<class T>
struct is_destructible : decltype(destructible_probe::test<T>(0)) {};
}

namespace semantic_expression {
int f()
{
  const auto fn = []()
  {
    struct Candidate {};
    static_assert(test_support::is_destructible<Candidate>::value, "");
    return 0;
  };
  return fn();
}
}

int main()
{
  return semantic_expression::f();
}
