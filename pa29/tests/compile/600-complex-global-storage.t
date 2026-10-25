namespace half_global {
typedef __complex__ _Float16 C;constexpr C f(){C a{};++__real__ a;C b=a;for(int i=0;i<11;++i)b=b+b;return b+a;}constexpr C h=f();static_assert((int)__real__ h==2049,"");C global=h;
}

namespace quad_global {
typedef __complex__ __float128 C;constexpr C f(){C a{};++__real__ a;return a+a;}constexpr C q=f();static_assert((int)__real__ q==2,"");C global=q;
}

namespace complex_uninitialized_global {
typedef __complex__ int C;C uninitialized;
}

namespace complex_dynamic_global {
typedef __complex__ double C;C f();C dynamic=f();
}
