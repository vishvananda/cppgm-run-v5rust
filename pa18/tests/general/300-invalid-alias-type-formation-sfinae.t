namespace m {
struct true_ { static const bool value = true; };
struct false_ { static const bool value = false; };
template<template<class...> class F, class... T> struct valid_impl {
  template<template<class...> class G, class = G<T...> > static true_ check(int);
  template<template<class...> class> static false_ check(...);
  using type = decltype(check<F>(0));
};
template<template<class...> class F, class... T>
using valid = typename valid_impl<F, T...>::type;
}
template<class T> using pointer = T*;
template<class T> using reference = T&;
template<class T> using array = T[];
static_assert(!m::valid<pointer, int&>::value, "");
static_assert(!m::valid<reference, void>::value, "");
static_assert(!m::valid<array, void>::value, "");
static_assert(!m::valid<array, int&>::value, "");
namespace h { template<class T> void use(void (*)(T)) {} }
int main() { h::use((void (*)(m::valid<pointer, int&>))0); }
