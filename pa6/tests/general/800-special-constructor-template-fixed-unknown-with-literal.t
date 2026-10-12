// Frozen types-mode special-member declaration responsibility regression.
struct C{template<class T>C(T)noexcept(unknown+1);};
