// N3485 5.1.2p5, 15.4p1: a lambda-declarator may carry
// noexcept(constant-expression); expect exit 0
int main() { auto f = []() noexcept(true) { return 0; }; return noexcept(f()) ? f() : 1; }
