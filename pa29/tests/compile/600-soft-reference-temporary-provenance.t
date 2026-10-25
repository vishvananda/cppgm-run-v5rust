constexpr const __bf16& bfloat_ref=__bf16{};
static_assert((int)bfloat_ref==0,"bfloat temporary");
struct S{constexpr operator __float128()const{return __float128{};}};
constexpr const __float128& class_ref=(__float128)S();
static_assert((int)class_ref==0,"converted temporary");
constexpr _Float16 one(){_Float16 q{};++q;return q;}
constexpr const _Float16& call_ref=one();
static_assert((int)call_ref==1,"called temporary");
