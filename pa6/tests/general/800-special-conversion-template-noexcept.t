// Frozen types-mode special-member declaration responsibility regression.
struct C{template<class T>operator T()noexcept{return 1;}};
