// N3485 14.7.3p5, p16: a member template of an explicitly specialized class is
// explicitly specialized with template<>; expect exit 0
template<class T> struct A { };
template<> struct A<int> { template<class U> int f(U) { return 1; } };
template<> int A<int>::f<char>(char) { return 2; }
int main() { A<int> a; return a.f(1) * 10 + a.f('c') == 12 ? 0 : 1; }
