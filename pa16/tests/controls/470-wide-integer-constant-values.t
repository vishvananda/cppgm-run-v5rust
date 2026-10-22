// N3485 4.7, 4.9, 5.19, 7.1.5; supported __int128 extension.
// expect exit 0
using U = __uint128_t;
using I = __int128_t;
constexpr U high = U(1) << 100;
constexpr I negative = -I(high) + 9;
constexpr U maximum = ~U(0);
constexpr U from_double = U(1e30);
constexpr double round_trip = double(from_double);
constexpr U values[] = { high + 9, maximum };
struct Pair { U first; I second; };
constexpr Pair pair = {high, negative};
constexpr U mutate(U n)
{
    n += high;
    ++n;
    n *= 3;
    n /= 3;
    n <<= 2;
    n >>= 2;
    return n;
}
struct Narrow { I value : 5; };
constexpr I narrow_mutation()
{
    Narrow field = {};
    field.value -= 1;
    return field.value;
}
struct Promoted { U value : 31; };
constexpr bool negated_field()
{
    Promoted field = {1};
    return -field.value < 0;
}
static_assert(negated_field(), "unary operation consumes the promoted type");
static_assert(narrow_mutation() == -1, "promoted operation converts to the declared width");
static_assert((high >> 100) == 1 && bool(high) && !!high, "complete truth value");
static_assert((negative >> 100) == -1, "signed value");
static_assert(!(I(-1) < U(1)) && U(1) < I(-1), "common unsigned type");
static_assert((from_double >> 64) == 0xc9f2c9cd0ull && round_trip == 1e30, "floating conversion");
static_assert((values[0] >> 100) == 1 && (values[1] >> 127) == 1, "array image");
static_assert(pair.first == high && pair.second == negative, "class image");
static_assert(mutate(4) == high + 5, "full-width local mutation");
int main()
{
    return high == (U(1) << 100) && negative < 0 && mutate(4) == high + 5 ? 0 : 1;
}
