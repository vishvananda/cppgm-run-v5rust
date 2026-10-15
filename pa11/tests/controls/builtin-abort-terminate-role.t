// Abort forwards to the sole terminate owner when noexcept cleanup needs it.
// expect exit 0
int alive;
void check(int n) { if (n) __builtin_abort(); }
struct Value {
  Value() { ++alive; }
  ~Value() { check(alive != 1); --alive; }
};
void run() noexcept { Value value; check(alive != 1); }
int main() { run(); return alive; }
