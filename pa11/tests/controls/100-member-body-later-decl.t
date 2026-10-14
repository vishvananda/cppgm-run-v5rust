// N3485 3.4.1p8: a name in an in-class member function body of a non-template
// class sees only namespace declarations before the class; expect exit 0
int f(long) { return 1; }
struct K { int m() { return f(0); } };
int f(int) { return 2; }
int main() { K k; return k.m() == 1 ? 0 : 1; }
