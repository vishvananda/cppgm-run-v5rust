// Frozen types-mode special-member declaration responsibility regression.
struct C{template<class T>C(int p)noexcept(p==0);};
