template<class...>
using void_t = void;

struct false_type {};
struct true_type {};

template<class T, class U, class = void>
struct less_than : false_type {};

template<class T, class U>
struct less_than<T, U, void_t<decltype((0) < (0))> > : true_type {};
