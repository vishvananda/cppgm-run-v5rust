typedef int V __attribute__((vector_size(8)));
constexpr V f(){V v;v[0]=1;return v;} constexpr V a=f();
