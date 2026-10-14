// N3485 14.8.2p8, 8.3.4p1: forming char[N] with N == 0 during substitution is
// a substitution failure; expect exit 0
template <int N> char f(char (*)[N]) { return 0; }
template <int N> long f(...) { return 0; }
int main() { return sizeof(f<0>(0)) == sizeof(long) ? 0 : 1; }
