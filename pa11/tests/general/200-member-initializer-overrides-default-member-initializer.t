struct X { int m = 1; X() : m(2) {} };
int main() { X x; return x.m == 2 ? 0 : 1; }
