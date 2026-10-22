// N3485 5.19, 5.9: scoped enumeration comparisons use the entire value.
// expect exit 0
enum class E : __int128 { zero = 0, one = 1 };
constexpr E high = static_cast<E>(__int128(1) << 100);
constexpr E negative = static_cast<E>(-(__int128(1) << 100));
static_assert(high != E::zero && high > E::one && E::one < high, "");
static_assert(!(high == E::zero) && !(high <= E::one), "");
static_assert(negative < E::zero && negative <= E::one && E::zero >= negative, "");
constexpr bool compare(E a, E b) { return a < b; }
static_assert(compare(negative, high) && !compare(high, negative), "");
int main() { return high > negative && high != E::zero ? 0 : 1; }
