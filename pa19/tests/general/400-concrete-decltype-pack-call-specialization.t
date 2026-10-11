// VALIDATION: compile-pass
// A concrete pack element must not retain its dependent expansion wrapper.
template<class T> T && declval();
template<class> struct voider { typedef void type; };
template<class, class = void> struct result {};
template<class F, class... A>
struct result<F(A...), typename voider<
    decltype(declval<F>()(declval<A>()...))>::type> {
  typedef decltype(declval<F>()(declval<A>()...)) type;
};
struct fn { int operator()(float); };
result<fn(float)>::type value;
