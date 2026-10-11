template<class T>
T&& declval();

struct true_type
{
  static const bool value = true;
};

struct false_type
{
  static const bool value = false;
};

template<class R, class C, class A>
auto call_test(C&& callable, int, A&& arg)
    -> decltype(callable(arg), true_type());

template<class R, class C, class A>
false_type call_test(C&&, long, A&&);

template<class C, class F>
struct is_invocable : false_type
{
};

template<class C, class R, class A>
struct is_invocable<C, R(A)>
    : decltype(call_test<R>(declval<C>(), 1, declval<A>()))
{
};

struct callable
{
  int operator()(int);
};

static_assert(is_invocable<callable, int(int)>::value,
              "mutable callable should be invocable");
static_assert(!is_invocable<callable const, int(int)>::value,
              "const callable must reject a non-const call operator");

int main()
{
  return 0;
}
