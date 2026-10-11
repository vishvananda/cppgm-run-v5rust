// VALIDATION: compile-pass
// N3485 focus: 14.6.4.1 [temp.point]
namespace a {
namespace n { template<class> struct w {}; struct x {}; }
namespace b {
template<class> struct s { typedef char type; };
template<class T> struct s<n::w<T> > { typedef int type; };
namespace n {}
typedef s<a::n::w<a::n::x> >::type result;
}}
static_assert(sizeof(a::b::result) == sizeof(int), "");
int main() {}
