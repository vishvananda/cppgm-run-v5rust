// VALIDATION: compile-fail

constexpr int zero() { return 0; }
void take(int *);
void test() { take(zero()); }
