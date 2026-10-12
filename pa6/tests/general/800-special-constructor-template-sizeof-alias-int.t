// Frozen types-mode special-member declaration responsibility regression.
typedef int I;struct C{template<class T>C(T)noexcept(sizeof(I)==4);};
