// N3485 8.3.6p9: a non-static member shall not be used in a default argument;
// must be rejected
struct S { int m; int f(int x = m); };
int main() { return 0; }
