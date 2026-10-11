// N3485 7p1, 7.1.6, 6.7: a block-scope simple-declaration may define an
// enumeration and declare a variable of it (enum E { P = 1 } p = P;);
// expect exit 0
int main() { enum E { P = 1 } p = P; return p == P ? 0 : 1; }
