typedef __complex__ int C;typedef __complex__ long D;constexpr int f(){C c;__real__ c=1;D d=(D)(C)c;return 3;}static_assert(f()==3,"");
