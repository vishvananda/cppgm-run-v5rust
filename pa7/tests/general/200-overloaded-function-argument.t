int g(int x) { return x; }
long g(long x) { return x; }
int call(int (*fp)(int)) { return fp(0); }
int f() { return call(g); }
