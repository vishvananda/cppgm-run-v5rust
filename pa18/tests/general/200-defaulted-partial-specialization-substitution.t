template<class V> struct vec_traits;
template<class T, int D> struct vec {};
template<class T> class proxy { proxy(); };
template<class T> struct vec_traits<proxy<T> > { typedef T scalar_type; enum { dim = 2 }; };
template<class V, int D = vec_traits<V>::dim, class S = typename vec_traits<V>::scalar_type> struct deduce { typedef V type; };
template<class T, int D> struct deduce<proxy<T>, D> { typedef vec<T, D> type; };
template<class A> typename deduce<A>::type f(A const &) { typename deduce<A>::type x; return x; }
proxy<float> const & value();
int main() { f(value()); }
