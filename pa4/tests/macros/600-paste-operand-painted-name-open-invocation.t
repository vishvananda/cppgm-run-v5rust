// N3485 16.3.4p2: a macro name painted during rescanning stays painted when it
// is a ## operand; O inside the open G( 1 + O ) is not replaced again (full
// program: exit 7)
int O = 4; int x1 = 3;
#define G(a) x ## a
#define O G ( 1 + O )
int main() { return O; } // 7
