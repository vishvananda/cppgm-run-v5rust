// N3485 3.3.2p7: struct Hidden* in a member declaration declares Hidden in the
// smallest enclosing namespace, not in Outer (full program: exit 7)
struct Outer { struct Hidden* p; };
struct Hidden { int v; };
int main() { Hidden h; h.v = 7; Outer o; o.p = &h; return o.p->v; } // 7
