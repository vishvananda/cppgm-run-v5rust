typedef float V __attribute__((vector_size(8)));constexpr float f(){V v;return ((V)v)[1];}static_assert(!__builtin_constant_p(f()),"");
