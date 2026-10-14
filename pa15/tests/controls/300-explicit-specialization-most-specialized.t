// N3485 14.7.3p? / 14.8.2.6, 14.5.6.2p1: template<> int f(int*) specializes
// the more specialized f(T*), not f(T); expect exit 0
template<class T> int f(T) { return 1; }
template<class T> int f(T*) { return 2; }
template<> int f(int*) { return 5; }
int main() { int i; return f(&i) == 5 ? 0 : 1; }
