// Frozen types-mode special-member declaration responsibility regression.
namespace N{}struct C{template<class T>C(T)noexcept(N::missing);};
