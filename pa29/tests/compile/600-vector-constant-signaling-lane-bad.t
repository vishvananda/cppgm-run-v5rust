typedef unsigned int I __attribute__((vector_size(8))); typedef float F __attribute__((vector_size(8))); constexpr I a{0x7f800001,0}; constexpr F b=(F)a;
