// Frozen types-mode special-member declaration responsibility regression.
struct C{template<class T>C(T)noexcept;};template<class U>C::C(U p)noexcept(sizeof(U)==4){}
