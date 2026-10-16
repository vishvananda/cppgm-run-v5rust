// Typing checks skipped assertion operands before constant evaluation.
int function(int);
static_assert(true || function(), "invalid call in skipped operand");
