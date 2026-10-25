typedef int V __attribute__((vector_size(8)));
constexpr int f(){V v;v[0]=1;switch(((V)v)[1]){case 0:return 3;default:return 4;}}static_assert(f()==3,"");
