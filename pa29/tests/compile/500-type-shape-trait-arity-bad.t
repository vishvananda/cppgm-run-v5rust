// A unary type-shape trait rejects a second operand.
static_assert(__is_const(const int, int), "arity");

int main() { return 0; }
