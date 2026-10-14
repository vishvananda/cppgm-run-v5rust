// N3485 14.3.2p1, 5.10p1-p2: a function-pointer non-type parameter bound to f
// compares with nullptr; expect exit 0
int f(int) { return 0; }
template<int (*F)(int)> struct A { static bool null() { return F == nullptr; } };
int main() { return !A<f>::null() ? 0 : 1; }
