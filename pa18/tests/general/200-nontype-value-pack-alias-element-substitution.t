template<class T, T N> struct constant {};
template<class...> struct list {};

template<class T, T... N>
using list_c = list<constant<T, N>...>;

template<class, class> struct same { static const bool value = false; };
template<class T> struct same<T, T> { static const bool value = true; };

static_assert(same<list_c<int, 1, 3>,
                   list<constant<int, 1>, constant<int, 3> > >::value, "");

int main() { return 0; }
