// N3485 5.19: repeated calls and overlapping recursive calls retain their
// argument-dependent results in every constant-expression context.
// expect exit 0
constexpr int fib(int n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }
constexpr int first = fib(18);
constexpr int second = fib(18) + fib(17);
static_assert(first == 2584 && second == 4181, "recursive values");
int main() { return first == 2584 && second == 4181 ? 0 : 1; }
