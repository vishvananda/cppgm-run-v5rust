// Frozen types-mode special-member declaration responsibility regression.
struct C{template<class T>operator T();};template<class U>C::operator U(){return 1;}
