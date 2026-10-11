template<class T, int N, bool C> struct P {};
template<class A, class B, class E = void> struct X;
template<class A, int N, bool C, class B> struct X<P<A, N, C>, B> { static const int value = 1; };
template<class A, int N, class B> struct X<P<A, N, false>, B> { static const int value = 2; };
int main() { return X<P<int, 0, false>, int>::value - 2; }
