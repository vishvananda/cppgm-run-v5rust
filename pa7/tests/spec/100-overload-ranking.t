// N3485 focus: 13.3.3.2 [over.ics.rank] conversion ranking
int g(int x) { return x; }
long g(long x) { return x; }
int f() { short x = 0; return g(x); }
