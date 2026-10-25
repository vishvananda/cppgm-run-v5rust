typedef int V __attribute__((vector_size(8)));
constexpr int f(){V v{1,2};return v[2];} static_assert(f()==0,"range");
