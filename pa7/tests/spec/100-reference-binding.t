// N3485 focus: 8.5.3 [dcl.init.ref] reference initialization
int inc(int &x) { return x; }
int keep(const int &x) { return x; }
int f() { int x = 0; return inc(x) + keep(0); }
