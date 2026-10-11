// VALIDATION: compile-pass
// N3485 focus: 5.1.1 [expr.prim.general], 14.2 [temp.names]

namespace m { template<bool> struct x; }
namespace n {
int x;
template<class T> struct q : m::x<sizeof(T) && true> {};
}
