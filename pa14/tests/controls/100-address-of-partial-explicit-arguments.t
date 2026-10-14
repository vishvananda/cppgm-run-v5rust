// N3485 13.4p1-p2 (target: a function parameter), 14.8.1p3, 14.8.2.2: the
// argument f<short> for parameter int(*)(int) deduces U = int from the target;
// expect exit 0
template<class T, class U> int f(U) { return sizeof(T); }
int call(int (*p)(int)) { return p(0); }
int main() { return call(f<short>) == sizeof(short) ? 0 : 1; }
