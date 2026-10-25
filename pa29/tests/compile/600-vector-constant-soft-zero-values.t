namespace half_zero_values {
typedef _Float16 V __attribute__((vector_size(8))); constexpr V a{}; constexpr V b{a[0]}; static_assert((int)a[3]==0 && (int)b[0]==0,"zero"); V g=a;
}

namespace bfloat_zero_values {
typedef __bf16 V __attribute__((vector_size(8))); constexpr V a{}; constexpr V b{a[0]}; static_assert((int)b[1]==0,"zero"); V g=a;
}

namespace quad_zero {
typedef __float128 V __attribute__((vector_size(32))); constexpr V a{}; static_assert((int)a[0]==0,"zero"); V g=a;
}

namespace half_steps {
typedef _Float16 V __attribute__((vector_size(8))); constexpr int f(){V v{}; ++v[0]; return (int)v[0];} static_assert(f()==1,"step");
}

namespace quad_steps {
typedef __float128 V __attribute__((vector_size(32))); constexpr int f(){V v{}; ++v[0]; return (int)v[0];} static_assert(f()==1,"step"); constexpr V z{}; V g=z;
}
