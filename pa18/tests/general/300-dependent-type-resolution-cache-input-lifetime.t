template<class T> T &&declval();
struct W { int *operator->() const; };
template<bool> struct enable {};
template<> struct enable<true> { using type = int; };
template<bool B> using enable_t = typename enable<B>::type;
template<class> struct fancy;
template<> struct fancy<W> { static const bool value = true; };
template<class P> struct helper;
template<class T> T *address(T *);
template<class P, enable_t<fancy<P>::value> = 0>
auto address(P const &) -> decltype(helper<P>::call(declval<P>()));
template<class P> struct helper {
  static auto call(P const &p) -> decltype(address(p.operator->()));
};
template<class I> struct unwrap_impl {
  static auto unwrap(I) -> decltype(address(declval<I>()));
};
template<class I, class M = unwrap_impl<I> >
auto unwrap(I) -> decltype(M::unwrap(declval<I>()));
using R = decltype(unwrap(W()));
int main() {}
