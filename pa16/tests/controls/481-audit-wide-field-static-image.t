// N3485 5.19, 9.6: completed field values retain both target integer halves.
// expect exit 0
using U = __uint128_t;
struct Fields { U first : 100; U second : 90; unsigned tail; };
constexpr Fields direct = { (U(1) << 90) + 7, (U(1) << 80) + 11, 19 };
constexpr Fields make()
{
    Fields result{};
    result.first = (U(1) << 93) + 3;
    result.second = (U(1) << 78) + 5;
    result.tail = 23;
    return result;
}
constexpr Fields copied = make();
static_assert((direct.first >> 90) == 1 && (copied.second >> 78) == 1, "");
int check(const Fields* p, U a, U b, unsigned tail)
{
    return p->first == a && p->second == b && p->tail == tail;
}
int main()
{
    return check(&direct, (U(1) << 90) + 7, (U(1) << 80) + 11, 19)
        && check(&copied, (U(1) << 93) + 3, (U(1) << 78) + 5, 23) ? 0 : 1;
}
