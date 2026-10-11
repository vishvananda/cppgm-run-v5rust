// HHC-290
int g(int* p) { return p == 0; }
int f() { return g(0); }
