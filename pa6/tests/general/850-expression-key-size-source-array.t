// Template exception expression equivalence uses structure and typed identities.
struct C{template<class T>C(int a[3],T b)noexcept(sizeof(a)+sizeof(T));};template<class U>C::C(int x[4],U y)noexcept(12ul+sizeof(U)){}
