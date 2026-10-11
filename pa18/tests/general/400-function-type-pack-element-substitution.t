template<class> struct W;
template<> struct W<long &> { typedef int type; };
template<> struct W<short> { typedef int type; };
template<class> struct R;
template<> struct R<void(int, int)> { typedef int type; };
struct F { template<class... A> typename R<void(typename W<A>::type...)>::type operator()(A &&...) const; };
template<class T> T && source();
decltype(source<F>()(source<long &>(), source<short>())) value;
