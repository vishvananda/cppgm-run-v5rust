// Frozen types-mode special-member declaration responsibility regression.
struct C{template<class T>operator T(){return 1;}};
