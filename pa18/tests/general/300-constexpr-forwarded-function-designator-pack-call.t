template<class T> struct remove_reference
{
  typedef T type;
};

template<class T> struct remove_reference<T &>
{
  typedef T type;
};

template<class T> struct remove_reference<T &&>
{
  typedef T type;
};

template<class T>
using remove_reference_t = typename remove_reference<T>::type;

template<class T>
constexpr T && forward(remove_reference_t<T> & value) noexcept
{
  return static_cast<T &&>(value);
}

template<class T>
constexpr T && forward(remove_reference_t<T> && value) noexcept
{
  return static_cast<T &&>(value);
}

template<class T>
T && declval() noexcept;

template<class F, class... A>
constexpr auto invoke(F && function, A &&... argument)
  noexcept(noexcept(forward<F>(function)(forward<A>(argument)...)))
  -> decltype(forward<F>(function)(forward<A>(argument)...))
{
  return forward<F>(function)(forward<A>(argument)...);
}

template<class F, class... A>
using invoke_result_t = decltype(invoke(declval<F>(), declval<A>()...));

template<class...> struct make_void
{
  typedef void type;
};

template<class... T>
using void_t = typename make_void<T...>::type;

template<class, class F, class... A> struct is_invocable
{
  static constexpr bool value = false;
};

template<class F, class... A>
struct is_invocable<void_t<invoke_result_t<F, A...>>, F, A...>
{
  static constexpr bool value = true;
};

constexpr int zero_arguments()
{
  return -1;
}

constexpr int one_argument(int value) noexcept
{
  return value;
}

static_assert(is_invocable<void, int (&)(int), int>::value,
              "dependent invoke result remains viable");
static_assert(invoke(zero_arguments) == -1,
              "forwarded zero-argument function designator");
static_assert(invoke(one_argument, 4) == 4,
              "forwarded function designator and argument pack");

int main()
{
  return 0;
}
