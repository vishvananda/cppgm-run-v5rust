typedef int V __attribute__((vector_size(8)));
constexpr long f(){V v;v[0]=1;return (long)((V)v)[1];} static_assert(f()==0,"convert");
