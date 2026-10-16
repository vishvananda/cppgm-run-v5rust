// N3485 10.2, 14.6.1p5: an alias in another base does not join the common
// template designated by two injected specialization names. expect reject
template<class T> struct Base {};
struct Both : Base<int>, Base<char> {};
struct Alias { using Base = ::Base<int>; };
struct Derived : Both, Alias {};
using Ambiguous = Derived::Base<int>;
