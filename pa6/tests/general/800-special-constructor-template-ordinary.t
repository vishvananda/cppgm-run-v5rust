// Frozen types-mode special-member declaration responsibility regression.
struct C{C(int);template<class T>C(int p){}};
