typedef int V __attribute__((vector_size(8)));
constexpr V v{1,2}; static_assert(v[-1]==0,"range");
