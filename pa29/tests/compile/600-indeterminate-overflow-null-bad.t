typedef int V __attribute__((vector_size(8)));
constexpr int f(){V v;v[0]=1;return __builtin_add_overflow(((V)v)[1],1,(int*)0);}static_assert(f()==0,"");
