typedef int V __attribute__((vector_size(8)));
constexpr int f(){V v;v[0]=1;int a[2]={1,2};int*p=a;p+=((V)v)[1];return *p;}static_assert(f()==1,"");
