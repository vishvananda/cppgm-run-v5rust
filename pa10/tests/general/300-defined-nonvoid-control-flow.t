// Syntactic non-void ends need not be reachable on any evaluated path.
int forever() { while (1 == 1) {} }
int constant_return() { if ((1 + 2) * 3 == 9) return 4; }
// A goto can enter a nested label independently of the enclosing condition.
int label_return() { goto L; if (false) { L: return 5; } }
int maybe_value(bool enabled) { if (enabled) return 1; }
int main() { return constant_return() + label_return() + maybe_value(true) - 10; }
