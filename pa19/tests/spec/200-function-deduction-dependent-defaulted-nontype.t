// VALIDATION: compile-pass
// N3485 focus: 14.8.2.5 [temp.deduct.type], 14.8.2.4 [temp.deduct.call]

struct C { typedef int I; };
template<class T, bool, typename T::I = 0> struct X {};
template<bool, class T> void f(X<T, false>) {}
int main() { f<true>(X<C, false>()); }
