// expect exit 0
// PA18: failed invocation is immediate-context substitution failure, and a
// successful decltype query demands no callable body or result construction.
template<class T> T&& declval();
template<class...> struct make_void { typedef void type; };
template<class... T> using void_t = typename make_void<T...>::type;

template<class F, class = void> struct detected { static const int value = 0; };
template<class F> struct detected<F, void_t<decltype(__builtin_invoke(declval<F>(), 1))> >
{ static const int value = 1; };

struct result { result() = delete; };
struct callable
{
  template<class T> result operator()(T) { typename T::missing value; return value; }
};
struct deleted { int operator()(int) = delete; };
struct hidden { private: int operator()(int); };

static_assert(detected<int>::value == 0, "failed before success");
static_assert(detected<callable>::value == 1, "signature without body");
static_assert(detected<deleted>::value == 0, "deleted call");
static_assert(detected<hidden>::value == 0, "inaccessible call");
static_assert(detected<callable&>::value == 1, "distinct reference argument");
static_assert(detected<int&>::value == 0, "failed after success");

int main() { return 0; }
