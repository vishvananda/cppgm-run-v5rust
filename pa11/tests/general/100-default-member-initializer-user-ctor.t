struct X { int m = 1; X() {} };
int main() { X x; return x.m == 1 ? 0 : 1; }
