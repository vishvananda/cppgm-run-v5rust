typedef int V __attribute__((vector_size(8)));
constexpr int f(){V v;v[0]=1;return v[((V)v)[1]];} static_assert(f()==1,"index");
