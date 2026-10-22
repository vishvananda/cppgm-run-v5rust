// N3485 7.2, 5.19; the frozen wide-enumerator constant projection.
// expect reject
constexpr int enumerate()
{
    enum E : __int128 { value = 18446744073709551615ULL };
    return 0;
}
static_assert(enumerate() == 0, "demand the constexpr body");
