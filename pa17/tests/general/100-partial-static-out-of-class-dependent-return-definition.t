template<class> struct V {};
template<class> struct T;
template<class A> struct T<V<A> > { typedef int R; static R f(V<A> const &); };
template<class A> typename T<V<A> >::R T<V<A> >::f(V<A> const &) { return 0; }
int main() { return T<V<int> >::f(V<int>()); }
