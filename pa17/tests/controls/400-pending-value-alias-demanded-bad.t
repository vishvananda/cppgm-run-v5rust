// expect reject
template<bool, class T> struct select { typedef T type; };
template<class C, class T> using pick = typename select<C::value, T>::type;
struct bad {};
template<class C> struct outer { template<class T> using type = pick<C, T>; };
outer<bad>::type<int> value;
