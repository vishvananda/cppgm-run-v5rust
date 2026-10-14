// N3485 12.3.2p1, 8.3.3 (conversion-type-id with ptr-operator S::*): operator
// int S::*() declares a conversion to a pointer to member; expect exit 0
struct S { int m; };
struct M { operator int S::*() const { return &S::m; } };
int main() { M m; int S::* q = m; return q == &S::m ? 0 : 1; }
