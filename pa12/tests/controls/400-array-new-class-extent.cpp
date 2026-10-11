// N3485 5.3.4p6: the expression of a noptr-new-declarator may have class type
// with a single non-explicit conversion to an integral type; expect exit 0
struct C { int n; operator int() const { return n; } };
int main() { C c = { 3 }; int* p = new int[c]; p[2] = 5; return p[2] == 5 ? 0 : 1; }
