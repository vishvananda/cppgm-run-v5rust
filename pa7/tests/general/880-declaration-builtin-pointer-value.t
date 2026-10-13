// Storage, linkage and declaration identity regression; frozen C++ behavior.
constexpr void(*p)()=__builtin_abort;int f(){return __builtin_constant_p(p);}
