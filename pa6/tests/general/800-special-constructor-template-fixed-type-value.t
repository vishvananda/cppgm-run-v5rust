// Frozen types-mode special-member declaration responsibility regression.
namespace N{typedef int I;}struct C{template<class T>C(T)noexcept(N::I);};
