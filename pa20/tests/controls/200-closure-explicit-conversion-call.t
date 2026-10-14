// N3485 5.1.2p6, 12.3.2: a captureless closure's conversion function to
// pointer to function may be called explicitly; expect exit 0
typedef int (*FP)(int);
int main() { auto l = [](int x) { return x + 4; }; FP p = l.operator FP(); return p(1) == 5 ? 0 : 1; }
