// Frozen types-mode special-member declaration responsibility regression.
template<class T>struct C{template<class U>C(U)noexcept(missing);};
