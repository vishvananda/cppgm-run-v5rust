typedef float V __attribute__((vector_size(8)));
float bad();constexpr int f(){V v;v[0]=1;return __builtin_isgreater(((V)v)[1],bad());}static_assert(f()==0,"");
