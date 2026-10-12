// Template exception expression equivalence uses structure and typed identities.
struct C{template<class T>C(T a,T b)noexcept(sizeof(int)==4);};template<class U>C::C(U x,U y)noexcept(sizeof(int)==5){}
