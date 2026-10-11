struct Point { int x; int y; };

constexpr Point p = {3, 4};
static_assert(p.x == 3, "");
static_assert(p.y == 4, "");

struct BitFields { unsigned value:2; signed negative:3; bool flag:1; };
constexpr BitFields bits = {5, -1, true};
static_assert(bits.value == 1, "bit-field initializer is truncated");
static_assert(bits.negative == -1, "signed bit-field retains its sign");
static_assert(bits.flag, "one-bit bool retains true");

constexpr int arr[] = {10, 20, 30};
static_assert(arr[0] == 10, "");
static_assert(arr[1] == 20, "");
static_assert(arr[2] == 30, "");

static_assert("hello"[0] == 'h', "");
static_assert("hello"[4] == 'o', "");

int main() { return 0; }
