// N3485 5.3.1p1, 4.3, 5.2.2p1: (*p)(3) with p a function pointer (or *r with r
// a function reference) designates the function and calls it; expect exit 0
int twice(int x) { return 2 * x; }
int main() { int (*p)(int) = twice; return (*p)(3) == 6 ? 0 : 1; }
