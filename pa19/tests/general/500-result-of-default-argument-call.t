template<class T> T&& declval();
template<class> struct result_of;
template<class T, class... A> struct result_of<T(A...)> {
  typedef decltype(declval<T>()(declval<A>()...)) type;
};
struct H { void operator()(int = 0); };
template<class T> struct B {
  template<class... A> typename result_of<T(A...)>::type operator()(A&&...);
};
void use(B<H>& value) { value(); }
