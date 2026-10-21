// expect exit 0
// N3485 4.9, 4.12: integral initializers use converted floating values.
constexpr int positive = 3.75;
constexpr long negative = -9.5L;
constexpr bool truth = 0.125f;
constexpr bool negative_zero = -0.0;
static_assert(positive == 3 && negative == -9, "truncation");
static_assert(truth && !negative_zero, "truth");
int main() { return 0; }
