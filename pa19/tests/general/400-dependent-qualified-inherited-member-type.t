template<class T> struct base_type { typedef T type; };
template<class T> struct identity { typedef T type; };
template<class T> using identity_t = typename identity<T>::type;

template<class...> struct common_type;
template<class T> struct common_type<T> : common_type<T, T> {};
template<class T, class U> struct common_type<T, U> : identity_t<base_type<T> > {};

template<class Rep, class Period> struct duration;

template<class R1, class P1, class R2, class P2>
struct common_type<duration<R1, P1>, duration<R2, P2> > {
  typedef duration<typename common_type<R1, R2>::type, P1> type;
};

template<class Rep, class Period>
struct duration {};

typedef typename common_type<duration<int, int>, duration<int, int> >::type result;

int main() { return 0; }
