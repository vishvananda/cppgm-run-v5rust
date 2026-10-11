// N3485 focus: 13.3 [over.match] overload resolution
int g(int x) { return x; }
long g(long x) { return x; }
int f() { return g(0); }
