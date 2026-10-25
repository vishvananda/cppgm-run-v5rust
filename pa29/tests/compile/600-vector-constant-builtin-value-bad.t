typedef int V __attribute__((vector_size(8)));
constexpr V v=__builtin_convertvector(V{1,2},V);
