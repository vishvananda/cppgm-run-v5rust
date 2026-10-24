// GNU hosted types keep their type identity separately from their machine format.
#define FLOATING(T, N) static_assert(__is_floating_point(T), "floating"); static_assert(sizeof(T) == N, "size"); static_assert(alignof(T) == N, "alignment")
FLOATING(_Float16, 2);
FLOATING(__bf16, 2);
FLOATING(_Float32, 4);
FLOATING(_Float64, 8);
FLOATING(_Float32x, 8);
FLOATING(_Float64x, 16);
FLOATING(_Float128, 16);
FLOATING(__float128, 16);
static_assert(!__is_same(float, _Float32), "distinct");
static_assert(!__is_same(double, _Float64), "distinct");
static_assert(!__is_same(double, _Float32x), "distinct");
static_assert(!__is_same(_Float128, __float128), "distinct");
static_assert(!__is_same(long double, _Float64x), "distinct");
static_assert(__is_same(decltype(1.0f32), _Float32), "suffix");
static_assert(__is_same(decltype(2.0F64), _Float64), "suffix");
static_assert(__is_same(decltype(3.0F32x), _Float32x), "suffix");
static_assert(__is_same(decltype(4.0f64x), _Float64x), "suffix");
void overload(float); void overload(_Float32);
void overload(double); void overload(_Float64); void overload(_Float32x);
void overload(long double); void overload(_Float64x);
void overload(_Float16); void overload(__bf16);
void overload(_Float128); void overload(__float128);
int anchor;
