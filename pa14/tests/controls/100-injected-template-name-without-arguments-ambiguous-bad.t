// N3485 14.6.1p5: injected names from different specializations name their
// common template only when followed by template arguments. expect reject
template<class T> struct Base {};
struct Derived : Base<int>, Base<char> {};
using Ambiguous = Derived::Base;
