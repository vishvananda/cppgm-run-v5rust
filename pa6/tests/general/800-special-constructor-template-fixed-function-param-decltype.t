// Frozen types-mode special-member declaration responsibility regression.
struct C{template<class T>C(void f())noexcept(sizeof(decltype(f))==8);};
