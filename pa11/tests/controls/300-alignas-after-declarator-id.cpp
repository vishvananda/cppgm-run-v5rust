// N3485 8.3p1, 7.6.2p1: an alignment-specifier after the declarator-id
// appertains to the declared variable/member; expect exit 0
struct S { char c; int x alignas(16); };
int y alignas(16) = 1;
int main() { int z alignas(8) = 1; return (alignof(S) == 16 && y + z == 2) ? 0 : 1; }
