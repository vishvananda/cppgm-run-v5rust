// Template exception expression equivalence uses structure and typed identities.
struct C{template<class T>C(int a,T b)noexcept(sizeof(a)+sizeof(T));};template<class U>C::C(int x,U y)noexcept(sizeof((x))+sizeof(U)){}
