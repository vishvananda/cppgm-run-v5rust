// Frozen types-mode special-member declaration responsibility regression.
struct C{template<class T=int>C(T);};template<class U=int>C::C(U p){}
