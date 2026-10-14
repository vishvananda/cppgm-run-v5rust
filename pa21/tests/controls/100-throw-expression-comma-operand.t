// N3485 5p1 grammar (expression: assignment-expression | expression,
// assignment-expression; throw-expression is an assignment-expression): (throw
// t, 1) is a comma expression; expect exit 0
static int g(int t) { return t ? (throw t, 1) : 0; }
int main() { return g(0); }
