// N3485 4.5, 5.19: a narrow extended-integer field promotes to int.
// expect reject
struct Field { __uint128_t value : 31; };
constexpr int promoted()
{
    Field field = {1};
    return field.value + 2147483647;
}
constexpr int result = promoted();
