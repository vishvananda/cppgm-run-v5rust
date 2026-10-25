namespace quad_local_nonzero {
constexpr __float128 one(){__float128 q{};++q;return q;}constexpr int f(){constexpr __float128 q=one();return (int)q;}static_assert(f()==1,"");
}

namespace quad_local_emitted {
constexpr __float128 one(){__float128 q{};++q;return q;}constexpr int f(){constexpr __float128 q=one();return (int)q;}static_assert(f()==1,"");int g(){return f();}
}

namespace half_local_nonzero {
constexpr _Float16 one(){_Float16 q{};++q;return q;}constexpr int f(){constexpr _Float16 q=one();return (int)q;}static_assert(f()==1,"");int g(){return f();}
}

namespace quad_class_global {
struct S{constexpr operator __float128() const{__float128 q{};++q;return q;}};__float128 q=S();
}

namespace half_class_global {
struct S{constexpr operator _Float16() const{_Float16 q{};++q;return q;}};_Float16 q=S();
}

namespace indeterminate_class_integer {
typedef int V __attribute__((vector_size(8)));struct S{constexpr operator int() const{V v;v[0]=1;return ((V)v)[1];}};int q=S();
}

namespace indeterminate_class_floating {
typedef float V __attribute__((vector_size(8)));struct S{constexpr operator float() const{V v;v[0]=1;return ((V)v)[1];}};float q=S();
}
