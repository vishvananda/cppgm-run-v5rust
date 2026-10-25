typedef int V __attribute__((vector_size(8)));
constexpr int f(){volatile V v{1,2};return ((V)v)[0];} static_assert(f()==1,"volatile");
