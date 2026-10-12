// Frozen types-mode special-member declaration responsibility regression.
struct C{template<class T>C(int);template<class T,class U>C(int);};
