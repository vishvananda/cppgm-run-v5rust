typedef signed char V __attribute__((vector_size(8)));constexpr int f(){V v;v[0]=1;(void)(true?((V)v)[1]:1);return 3;}static_assert(f()==3,"");
