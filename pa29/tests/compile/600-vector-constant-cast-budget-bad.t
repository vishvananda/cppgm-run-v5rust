typedef char V __attribute__((vector_size(512))); typedef short W __attribute__((vector_size(512))); constexpr V a{}; constexpr W b=(W)a;
