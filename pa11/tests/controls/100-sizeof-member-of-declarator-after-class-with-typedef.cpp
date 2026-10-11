// N3485 5.3.3/1: x is an object (declarator of a class-specifier whose class
// has a member typedef), so sizeof(x.k) is sizeof of an expression;
// expect exit 0
struct X { typedef int t; int k; } x;
int main() { return sizeof(x.k) == sizeof(int) ? 0 : 1; }
