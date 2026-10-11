// N3485 9.2p2, 14.2p3: in its own (complete-class-context) body, f<N - 1>()
// names member template f; expect exit 0
struct S { template<int N> static int f() { return N + f<N - 1>(); } };
template<> int S::f<0>() { return 0; }
int main() { return S::f<2>() == 3 ? 0 : 1; }
