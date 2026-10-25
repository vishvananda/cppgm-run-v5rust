typedef __float128 V __attribute__((vector_size(32)));constexpr __float128 f(){V v;v[0]=1;return ((V)v)[1];}constexpr __float128 x=f();
