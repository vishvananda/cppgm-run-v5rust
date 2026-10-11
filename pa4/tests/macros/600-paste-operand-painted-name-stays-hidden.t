// N3485 16.3.4p2: a macro name painted during rescanning is never again
// replaced, also when the invocation closes outside the replacement (full
// program: exit 7)
int O = 4; int x1 = 3;
#define G(a) x ## a
#define O G ( 1 + O
int main() { return O ); } // 7
