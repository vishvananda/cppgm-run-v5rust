// Template exception expression equivalence uses structure and typed identities.
struct C{template<class T>C(T a,T b)noexcept(missing+sizeof(T));};template<class U>C::C(U x,U y)noexcept(other+sizeof(U)){}
