typedef signed char V __attribute__((vector_size(8)));int bad();constexpr int f(){V v;v[0]=1;return ((V)v)[1]+bad();} static_assert(f()==0,"order");
