// N3485 8.3.6p4 (default arguments may be added in later declarations only for
// non-template functions), 8.3.6p6: f(0) has no default for b;
// must be rejected
template<class T> int f(T, int);
template<class T> int f(T a, int b = 2) { return b; }
int main() { return f(0) == 2 ? 0 : 1; }
