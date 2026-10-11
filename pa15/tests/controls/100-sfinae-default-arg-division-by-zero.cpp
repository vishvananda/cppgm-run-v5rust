// N3485 14.8.2p8: forming D<0> whose default argument 10 / N divides by zero
// is a substitution failure, not a hard error; f<0>(0) picks f(...);
// expect exit 0
template<int N, int M = 10 / N> struct D { static const int v = M; };
template<int N> char f(D<N>*);
template<int N> long f(...);
int main() { return sizeof(f<0>(0)) == sizeof(long) && sizeof(f<2>(0)) == 1 ? 0 : 1; }
