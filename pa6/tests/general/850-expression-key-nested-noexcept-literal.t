// Template exception expression equivalence uses structure and typed identities.
struct C{template<class T>C(T a,T b)noexcept(noexcept(1)+sizeof(T));};template<class U>C::C(U x,U y)noexcept(8ul+sizeof(U)){}
