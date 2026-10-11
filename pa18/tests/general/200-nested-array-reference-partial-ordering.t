template<class T, int M, int N> int depth(T (&)[M][N]) { return 2; }
template<class T, int N> int depth(T (&)[N]) { return 1; }
int main() { int a[1][1] = {}; return depth(a) == 2 ? 0 : 1; }
