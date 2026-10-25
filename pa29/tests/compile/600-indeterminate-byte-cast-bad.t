typedef int V __attribute__((vector_size(8)));
typedef short W __attribute__((vector_size(8))); constexpr int f(){V v;v[0]=1;W w=(W)v;return w[0];} static_assert(f()==1,"cast");
