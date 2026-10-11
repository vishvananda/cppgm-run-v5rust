struct yes { static const bool value = true; };
struct no { static const bool value = false; };
template<class...> struct list {};
template<class, class> struct starts;
template<class... A, class... B>
struct starts<list<A...>, list<B...> > {
  template<class L> static no check(L);
  template<class... T> static yes check(list<B..., T...>);
  using type = decltype(check(list<A...>()));
};
static_assert(starts<list<int>, list<> >::type::value, "");
static_assert(starts<list<int, void>, list<int, void> >::type::value, "");
int main() { return 0; }
