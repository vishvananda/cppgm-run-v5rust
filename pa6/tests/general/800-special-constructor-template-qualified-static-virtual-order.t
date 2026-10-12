// Frozen types-mode special-member declaration responsibility regression.
struct C{template<class T>C(T);};template<class T>static virtual C::C(T){}
