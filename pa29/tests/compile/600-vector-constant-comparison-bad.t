typedef int V __attribute__((vector_size(8)));
constexpr V v{1,2}; constexpr V b=v==v;
