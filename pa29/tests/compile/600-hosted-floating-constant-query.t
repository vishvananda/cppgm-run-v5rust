#define CHECK(T, V) \
  static_assert(__builtin_constant_p(V), "literal"); \
  static_assert(__builtin_constant_p(V + V), "arithmetic"); \
  constexpr T constant_##T = V; \
  static_assert(__builtin_constant_p(constant_##T), "named constant"); \
  int query_##T(T parameter) { \
    static_assert(!__builtin_constant_p(parameter), "parameter"); \
    return __builtin_constant_p(parameter++); \
  }
CHECK(_Float32, 1.5f32)
CHECK(_Float64, 2.5f64)
CHECK(_Float32x, 3.5f32x)
CHECK(_Float64x, 4.5f64x)
static_assert(__builtin_constant_p(1.0f + 2.0f), "ordinary float");
static_assert(__builtin_constant_p(1.0 + 2.0), "ordinary double");
static_assert(__builtin_constant_p(1.0L + 2.0L), "ordinary extended");
constexpr _Float32 identity(_Float32 value) { return value; }
static_assert(__builtin_constant_p(identity(2.0f32)), "constant call");
_Float64 dynamic();
static_assert(!__builtin_constant_p(dynamic()), "runtime call");
int anchor;
