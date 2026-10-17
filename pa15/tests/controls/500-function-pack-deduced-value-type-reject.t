// N3485 14.8.1p9, 14.8.2.5p17: explicit prefix conversion does not convert a
// newly deduced value's type; expect reject
template<int N> struct A {};
template<short... N> int f(A<N>...) { return 0; }
int main() { return f<1>(A<1>(), A<2>()); }
