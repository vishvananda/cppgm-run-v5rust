// N3485 5.8p3, 4.7p2: (unsigned)x >> 1 of x = -37 shifts the unsigned value
// 4294967259, giving 2147483629, also when the shift is folded at compile time
int f(int x) { return (unsigned)x >> 1; }
int main() { return f(-37) == 2147483629 ? 0 : 1; }
