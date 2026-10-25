typedef __complex__ __float128 C;constexpr int f(){C a{};C b=a/a;return (int)__real__ b;}static_assert(f()==0,"");
