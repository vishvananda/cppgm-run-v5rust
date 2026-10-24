template<int... I> struct sequence {};
template<int N> using indices = sequence<__integer_pack(N)...>;
using invalid = indices<-1>;
