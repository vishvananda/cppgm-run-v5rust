// Frozen types-mode special-member declaration responsibility regression.
template<class T>struct C{C(T)noexcept(missing);};
