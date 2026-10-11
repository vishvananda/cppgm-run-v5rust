// N3485 9.4.2p2/7.1.1p1 (static is part of the member's declaration in the
// class, not of its namespace-scope definition; g++ and clang reject): `static
// int S::m = 1;` must be rejected
struct S { static int m; };
static int S::m = 1;
int main() { return S::m - 1; }
