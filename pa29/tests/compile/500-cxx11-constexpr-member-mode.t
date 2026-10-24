// The default C++11 mode keeps the implicit const on a constexpr member.
struct Cell { int n; constexpr int get() { return n; } };
int (Cell::*read_member)() const = &Cell::get;
constexpr Cell x = {4};
static_assert(x.get() == 4, "implicit const");
