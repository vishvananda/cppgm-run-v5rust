// N3485 14.8.2.4p2-p4, 14.8.2.5p8: f(T(&)[3]) deduces N = 3 for f(T(&)[N]) but
// not vice versa, so it is more specialized; expect exit 0
template<class T, int N> int f(T (&)[N]) { return 1; }
template<class T> int f(T (&)[3]) { return 2; }
int main() { int a[3]; return f(a) == 2 ? 0 : 1; }
