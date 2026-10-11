// N3485 5.18, 15p1: a throw-expression may be the left operand of a
// parenthesized comma expression `(throw 1, 0)`; expect exit 0
int main() { int r = 3; try { (void)(r == 3 ? 0 : (throw 1, 0)); } catch (int) { return 1; } return 0; }
