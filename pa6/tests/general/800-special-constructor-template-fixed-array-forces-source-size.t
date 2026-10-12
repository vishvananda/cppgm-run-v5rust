// Frozen types-mode special-member declaration responsibility regression.
struct C{template<class T>C(int a[3])noexcept(1/(sizeof(a)==12));};
