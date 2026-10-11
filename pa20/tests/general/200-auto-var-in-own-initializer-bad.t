// N3485 7.1.6.4p3: a variable declared with auto shall not appear in its own
// initializer; expect reject
auto fact = [](int n) -> int { return n <= 1 ? 1 : n * fact(n - 1); };
int main() { return fact(4) - 24; }
