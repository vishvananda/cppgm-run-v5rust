// Unevaluated is still typed, including an unreachable operand.
int main() { return noexcept(false && missing()); }
