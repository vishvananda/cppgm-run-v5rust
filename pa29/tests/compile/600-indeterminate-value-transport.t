namespace discard {
typedef int V __attribute__((vector_size(8)));
constexpr int f(){V v;v[0]=1;(void)((V)v)[1];return 0;} static_assert(f()==0,"discard");
}

namespace unused_initializer {
typedef int V __attribute__((vector_size(8)));
constexpr int f(){V v;v[0]=1;int x=((V)v)[1];(void)x;return 3;} static_assert(f()==3,"unused");
}

namespace overwrite {
typedef int V __attribute__((vector_size(8)));
constexpr int f(){V v;v[0]=1;int x=((V)v)[1];x=4;return x;} static_assert(f()==4,"overwrite");
}

namespace unused_parameter {
typedef int V __attribute__((vector_size(8)));
constexpr int ignore(int){return 3;} constexpr int f(){V v;v[0]=1;return ignore(((V)v)[1]);} static_assert(f()==3,"parameter");
}

namespace discard_call {
typedef int V __attribute__((vector_size(8)));
constexpr int f(){V v;v[0]=1;return ((V)v)[1];} constexpr int g(){(void)f();return 4;} static_assert(g()==4,"call");
}

namespace reference_unused {
typedef int V __attribute__((vector_size(8)));
constexpr int f(){V v;v[0]=1;const int& x=((V)v)[1];(void)x;return 3;} static_assert(f()==3,"reference");
}

namespace float_discard {
typedef float V __attribute__((vector_size(8)));constexpr int f(){V v;v[0]=1;(void)((V)v)[1];return 3;} static_assert(f()==3,"discard");
}

namespace float_store {
typedef float V __attribute__((vector_size(8)));constexpr int f(){V v;v[0]=1;float x=((V)v)[1];(void)x;return 3;} static_assert(f()==3,"store");
}

namespace float_compare {
typedef float V __attribute__((vector_size(8)));constexpr float f(){V v;v[0]=1;return ((V)v)[1];} static_assert(f()==0.f,"compare");
}

namespace float_arithmetic {
typedef float V __attribute__((vector_size(8)));constexpr float f(){V v;v[0]=1;return ((V)v)[1];} static_assert(f()+1.f==1.f,"arithmetic");
}

namespace complex_discard {
typedef __complex__ int C;constexpr int f(){C c;__real__ c=1;(void)(__imag__ ((C)c));return 3;} static_assert(f()==3,"complex");
}
