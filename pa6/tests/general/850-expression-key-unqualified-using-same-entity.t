// Template exception expression equivalence uses structure and typed identities.
namespace N{const int n=1;}using N::n;struct C{template<class T>C(T a,T b)noexcept(n+sizeof(T));};template<class U>C::C(U x,U y)noexcept(N::n+sizeof(U)){}
