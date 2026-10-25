typedef int V __attribute__((vector_size(8)));
constexpr int f(){V v;v[0]=1;return ((V)v)[1]+2;} static_assert(f()==0,"arithmetic");
