// Frozen types-mode special-member declaration responsibility regression.
typedef void I;struct C{template<class T>C(T)noexcept(sizeof(I));};
