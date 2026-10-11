// Boundary value stress test for constexpr evaluation.
// Exercises bit-boundary edge cases: LLONG_MIN, UINT_MAX,
// unsigned wrapping, and signed/unsigned comparison semantics.

// Long long literal classification
static_assert(1LL != 0, "long long literal");
static_assert(1LL + 0 == 1LL, "long long preserves type through addition");

// LLONG_MIN region: (1LL << 63) is the minimum long long value
static_assert((1LL << 63) < 0, "LLONG_MIN is negative");
static_assert(((1LL << 63) + 1) < 0, "LLONG_MIN + 1 is still negative");
static_assert(((1LL << 63) | 1) < 0, "LLONG_MIN | 1 is still negative");
static_assert(((1LL << 63) | 1) != 1, "LLONG_MIN | 1 is not 1");

// ULLONG_MAX wrapping
static_assert(0xFFFFFFFFFFFFFFFFuLL + 1u == 0uLL, "ULLONG_MAX + 1 wraps to 0");
static_assert(0uLL - 1uLL == 0xFFFFFFFFFFFFFFFFuLL, "0 - 1 wraps to ULLONG_MAX");

// Signed/unsigned comparison: usual arithmetic conversions
static_assert(((-1) < 0u) == false, "-1 converts to UINT_MAX before comparison");
static_assert((-1LL) < 0, "long long -1 is negative");

// Shift result type preserves LHS type
static_assert((1u << 16) == 65536u, "unsigned shift stays unsigned");
static_assert((1LL << 32) == 4294967296LL, "long long shift preserves width");

// Multiplication type preservation
static_assert((1LL * 1) == 1LL, "long long * int stays long long");
static_assert((1u * 1) == 1u, "unsigned * int stays unsigned");

// UINT_MAX wrapping
static_assert((0u - 1u) == 4294967295u, "unsigned 0 - 1 wraps to UINT_MAX");

int main() { return 0; }
