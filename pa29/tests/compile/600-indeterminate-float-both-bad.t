typedef float V __attribute__((vector_size(8)));constexpr float f(){V v;v[0]=1;return ((V)v)[1];} static_assert(f()==f(),"compare");
