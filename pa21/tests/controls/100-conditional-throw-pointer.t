// N3485 5.16p2: if one operand is a throw-expression the result has the other
// operand's type (int*), not void; expect exit 0
int* f(int* p) { return p ? p : throw 1; }
int main() { int x; return f(&x) == &x ? 0 : 1; }
