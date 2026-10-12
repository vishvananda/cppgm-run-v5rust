// Template exception expression equivalence uses structure and typed identities.
namespace N{const int n=1;}namespace Alias=N;struct C{template<class T>C(T a,T b)noexcept(N::n+sizeof(T));};template<class U>C::C(U x,U y)noexcept(Alias::n+sizeof(U)){}
