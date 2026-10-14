// N3485 12.2p5, 5.19p3: a temporary bound to a reference member of an
// aggregate that the initializer of a variable of static storage duration
// initializes persists with the variable, so it has static storage duration
// and the reference member is an address constant of a constexpr aggregate,
// also nested and as an array element (CWG 1634); expect exit 50
struct R { const int& r; int x; };
struct Q { R a; int y; };
constexpr R rr = { 5, 6 };
constexpr Q q = { { 7, 8 }, 9 };
constexpr R arr[2] = { { 1, 2 }, { 3, 4 } };
constexpr const int* p = &rr.r;
static_assert(rr.x == 6 && q.a.x == 8 && arr[1].x == 4, "");
int main() { return rr.r + rr.x + q.a.r + q.y + arr[0].r + arr[1].r + *p + 14; }
