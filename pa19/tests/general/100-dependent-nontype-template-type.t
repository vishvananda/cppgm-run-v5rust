// HHC-074
template<class Cp> struct Holder { typedef int __storage_type; };

template<class Cp, bool IsConst, typename Cp::__storage_type = 0>
class bit_iterator;
