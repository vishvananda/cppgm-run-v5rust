template<class T, class U> struct same { static const bool value = false; };
template<class T> struct same<T, T> { static const bool value = true; };

template<bool B, class T, class F> struct conditional;
template<class T, class F> struct conditional<true, T, F> { typedef T type; };
template<class T, class F> struct conditional<false, T, F> { typedef F type; };
template<bool B, class T, class F> using conditional_t = typename conditional<B, T, F>::type;

template<class T> struct decay { typedef T type; };
template<class T> using decay_t = typename decay<T>::type;

template<class T, class U> struct imp {};

template<class T, class U>
struct common_type : conditional_t<same<T, decay_t<T> >::value &&
                                   same<U, decay_t<U> >::value,
                                   imp<T, U>,
                                   common_type<decay_t<T>, decay_t<U> > > {};

struct D {};

typedef common_type<D, D> X;

int main() { return 0; }
