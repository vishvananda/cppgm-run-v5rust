// N3485 5.1.2p21: an array captured by copy is direct-initialized element by
// element in the closure; expect exit 0
int main() { int a[3] = {1, 2, 3}; auto f = [a]() { return a[0] + a[1] + a[2]; }; a[0] = 100; return f() == 6 ? 0 : 1; }
