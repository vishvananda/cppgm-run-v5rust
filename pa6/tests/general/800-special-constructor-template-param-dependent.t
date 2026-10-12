// Frozen types-mode special-member declaration responsibility regression.
struct C{template<class T>C(T p)noexcept(sizeof(p)==4);};
