// N3485 7.2p1/7p1: an unnamed enum-specifier followed by a declarator (enum {
// E1 = 3 } e = E1;) declares a variable; expect exit 0
int main() { enum { E1 = 3 } e = E1; return e - 3; }
