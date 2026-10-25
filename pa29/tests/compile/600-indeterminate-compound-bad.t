typedef int V __attribute__((vector_size(8)));
constexpr int f(){V v;v[0]=1;int x=1;x+=((V)v)[1];return x;} static_assert(f()==1,"compound");
