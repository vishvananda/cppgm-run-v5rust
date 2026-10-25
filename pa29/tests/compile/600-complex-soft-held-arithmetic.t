namespace quad_add {
typedef __complex__ __float128 C;constexpr int f(){C c{};++__real__ c;C d=c+c;return (int)__real__ d;}static_assert(f()==2,"");
}

namespace half_add {
typedef __complex__ _Float16 C;constexpr int f(){C one{};++__real__ one;C c=one;for(int i=0;i<11;++i)c=c+c;C d=c+one;return (int)__real__ d;}static_assert(f()==2049,"");
}

namespace quad_subtract {
typedef __complex__ __float128 C;constexpr int f(){C a{};++__real__ a;C b=a+a+a;return (int)__real__(b-a);}static_assert(f()==2,"");
}

namespace quad_multiply {
typedef __complex__ __float128 C;constexpr int f(){C a{};++__real__ a;++__imag__ a;C b=a*a;return (int)__real__ b*10+(int)__imag__ b;}static_assert(f()==2,"");
}

namespace half_multiply {
typedef __complex__ _Float16 C;constexpr int f(){C a{};++__real__ a;C b=a;for(int i=0;i<11;++i)b=b+b;C c=b+a;return (int)__real__(c*(a+a));}static_assert(f()==4098,"");
}

namespace half_real_divisor {
typedef __complex__ _Float16 C;constexpr int f(){C a{};++__real__ a;C b=a;for(int i=0;i<11;++i)b=b+b;C c=b+a;_Float16 two=__real__(a+a);C d=c/two;return (int)(__real__ d*two);}static_assert(f()==2049,"");
}

namespace quad_real_divisor {
typedef __complex__ __float128 C;constexpr int f(){C a{};++__real__ a;C b=a+a;__float128 two=__real__ b;C d=b/two;return (int)__real__ d;}static_assert(f()==1,"");
}

namespace half_range {
typedef __complex__ _Float16 C;constexpr int f(){C a{};++__real__ a;for(int i=0;i<16;++i)a=a+a;return (int)__real__ a;}static_assert(f()==65536,"");
}

namespace quad_indeterminate_arithmetic {
typedef __complex__ __float128 C;constexpr int f(){C a;C b=a+a;return (int)__real__ b;}static_assert(f()==0,"");
}
