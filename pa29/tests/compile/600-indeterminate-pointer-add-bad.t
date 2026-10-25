typedef int V __attribute__((vector_size(8)));
constexpr int f(){V v;v[0]=1;int a[2]={1,2};return *(a+((V)v)[1]);}static_assert(f()==1,"");
