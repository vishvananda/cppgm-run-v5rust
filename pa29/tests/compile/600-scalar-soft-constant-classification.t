constexpr __float128 q{};static_assert(__builtin_fpclassify(1,2,3,4,5,q)==5,"");
constexpr _Float16 h{};static_assert(__builtin_isfinite(h),"");static_assert(__builtin_fpclassify(1,2,3,4,5,h)==5,"");

namespace quad_step_global {
constexpr __float128 f(){__float128 x{};++x;return x;}constexpr __float128 q=f();static_assert((int)q==1,"");static_assert(__builtin_constant_p(q),"");__float128 g=q;
}

namespace half_step_global {
constexpr _Float16 f(){_Float16 x{};++x;return x;}constexpr _Float16 q=f();static_assert((int)q==1,"");static_assert(__builtin_constant_p(q),"");_Float16 g=q;
}

namespace quad_literal_query {
static_assert(!__builtin_constant_p(1.0Q),"");
}
