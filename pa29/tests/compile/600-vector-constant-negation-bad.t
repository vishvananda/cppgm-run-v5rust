typedef int V __attribute__((vector_size(8)));
constexpr V a{1,2}; constexpr V b=-a;
