// N3485 8.5.4p3/p7, 5.3.4p15: narrowing double->int in the braced list of a
// run-time-count array new is ill-formed; must be rejected
int main() { int n = 2; int* p = new int[n]{n, 2.5}; return p[0]; }
