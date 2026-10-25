typedef unsigned int V __attribute__((vector_size(8)));
constexpr int f(){V v;v[0]=1;return __builtin_popcountg(((V)v)[1]);}static_assert(f()==0,"");
