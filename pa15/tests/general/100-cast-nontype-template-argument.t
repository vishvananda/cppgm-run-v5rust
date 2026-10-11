// HHC-142
typedef unsigned short uint_least16_t;
template<class C, class I, I V> struct X {};
using Y = X<char16_t, uint_least16_t, static_cast<uint_least16_t>(0xFFFF)>;
