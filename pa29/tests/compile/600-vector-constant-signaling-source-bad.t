typedef float F __attribute__((vector_size(8))); typedef int I __attribute__((vector_size(8))); constexpr F f{__builtin_nansf("1"),0}; constexpr I i=(I)f;
