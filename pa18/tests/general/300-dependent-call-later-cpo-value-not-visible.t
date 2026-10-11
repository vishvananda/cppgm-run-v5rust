// VALIDATION: compile-pass
// A dependent expression-SFINAE probe must not find an ordinary callable
// object declared after the probe, even when later traits depend on its result.

template <bool Condition, class T = void>
struct enable_if
{
};

template <class T>
struct enable_if<true, T>
{
  typedef T type;
};

template <bool Condition, class T = void>
using enable_if_t = typename enable_if<Condition, T>::type;

template <class...>
struct void_type
{
  typedef void type;
};

template <class... T>
using void_t = typename void_type<T...>::type;

template <class T>
T&& declval();

namespace lib {

struct executor
{
};

struct property
{
};

namespace traits {

template <class T, class P, class = void>
struct prefer_free_default;

template <class T, class P, class = void>
struct prefer_free;

} // namespace traits

namespace detail {

struct no_prefer_free
{
  static constexpr bool is_valid = false;
};

template <class T, class P, class = void>
struct prefer_free_trait : no_prefer_free
{
};

template <class T, class P>
struct prefer_free_trait<
    T,
    P,
    void_t<decltype(prefer(declval<T>(), declval<P>()))> >
{
  static constexpr bool is_valid = true;
};

} // namespace detail

namespace traits {

template <class T, class P, class Enable>
struct prefer_free_default : detail::prefer_free_trait<T, P>
{
};

template <class T, class P, class Enable>
struct prefer_free : prefer_free_default<T, P>
{
};

} // namespace traits
} // namespace lib

namespace prefer_fn {

template <class T, class P, class = void>
struct call_traits
{
  static constexpr int overload = 7;
};

template <class T, class P>
struct call_traits<
    T,
    P,
    enable_if_t<!lib::traits::prefer_free<T, P>::is_valid> >
{
  static constexpr int overload = 0;
};

struct impl
{
  template <class T, class P>
  auto operator()(T&&, P&&) const
      -> enable_if_t<call_traits<T, P>::overload == 0, T&&>;
};

} // namespace prefer_fn

namespace lib {
namespace {

static constexpr prefer_fn::impl prefer = {};

} // namespace
} // namespace lib

static_assert(
    !lib::traits::prefer_free<lib::executor&, lib::property>::is_valid,
    "the later ordinary callable is not visible to the dependent probe");

static_assert(
    prefer_fn::call_traits<lib::executor&, lib::property>::overload == 0,
    "the false trait selects the fallback call path");

int main()
{
  return 0;
}
