// N3485 14.7.3p16, 14.5.6.1p5-6: template<> template<class U> auto
// A<int>::g(U) -> decltype(u + int()) matches member template g whose type is
// decltype(u + C()) with C = int; expect exit 0
template <class C> struct A {
	template <class U> auto g(U u) -> decltype(u + C()) { return 1; }
};
template <> template <class U> auto A<int>::g(U u) -> decltype(u + int()) { return 4; }
int main() { A<int> a; A<char> b; return a.g(1) * 10 + b.g(1) == 41 ? 0 : 1; }
