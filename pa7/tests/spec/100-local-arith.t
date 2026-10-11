// N3485 focus: 5.7 [expr.add], 5.17 [expr.ass]
int f(int x) { int y = x + 1; return y = y + 2; }
