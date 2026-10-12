// Template exception expression equivalence uses structure and typed identities.
struct C{template<class T>C(T a,T b)noexcept(new T[a][3]);};template<class U>C::C(U x,U y)noexcept(new U[x][3]){}
