// A dependent decltype retained through a class-template member type must be
// resolved in the scope where its expression was formed, not the class scope.
namespace std {
template<class... T>
struct tuple
{
};
}

namespace leaf {
template<class E>
struct slot
{
};

template<class... T>
struct context
{
  std::tuple<slot<T>...> & tup();
};

namespace detail {
template<class... H>
struct context_type_from_handlers_impl
{
  typedef context<H...> type;
};
}

template<class... H>
using context_type_from_handlers =
    typename detail::context_type_from_handlers_impl<H...>::type;
}

namespace std {
template<class T>
T && declval();

template<class T>
struct decay
{
  typedef T type;
};

template<class T>
struct decay<T &>
{
  typedef T type;
};
}

template<class T>
struct unwrap_tuple;

template<template<class> class S, class... E>
struct unwrap_tuple<std::tuple<S<E>...> >
{
  typedef std::tuple<E...> type;
};

template<class... H>
typename unwrap_tuple<typename std::decay<
    decltype(std::declval<typename leaf::context_type_from_handlers<H...> >().tup())
  >::type>::type * expd(H && ...)
{
  return 0;
}

int main()
{
  return expd(1) ? 1 : 0;
}
