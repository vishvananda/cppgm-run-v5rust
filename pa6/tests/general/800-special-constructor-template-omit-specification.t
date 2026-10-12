// Frozen types-mode special-member declaration responsibility regression.
struct C{template<class T>C(T)noexcept(sizeof(T)==4);};template<class U>C::C(U p){}
