typedef int V __attribute__((vector_size(8)));
constexpr int f(){V v;v[0]=1;return __builtin_fpclassify(0,1,2,3,((V)v)[1],0.f);}static_assert(f()==0,"");
