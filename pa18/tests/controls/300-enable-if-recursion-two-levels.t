// N3485 14.8.2p8, 14.7.1: each recursive specialization sum<N-1> redoes
// overload resolution with its own N (enable_if picks the base case at N==0);
// expect exit 0
template <bool B, class T = void> struct enable_if {};
template <class T> struct enable_if<true, T> { typedef T type; };
template <int N> typename enable_if<(N == 0), int>::type sum() { return 0; }
template <int N> typename enable_if<(N > 0), int>::type sum() { return N + sum<N - 1>(); }
int main() { return sum<2>() == 3 ? 0 : 1; }
