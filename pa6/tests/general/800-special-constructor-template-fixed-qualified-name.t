// Frozen types-mode special-member declaration responsibility regression.
namespace N{const int n=1;}struct C{template<class T>C(T)noexcept(N::n);};
