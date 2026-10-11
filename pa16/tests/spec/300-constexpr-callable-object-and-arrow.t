// N3485 focus: 5.2.2 [expr.call], 13.5.6 [over.ref]
struct X { int value; constexpr int operator()() const { return value; } };
struct P { X x; constexpr X const* operator->() const { return &x; } };
constexpr X function = {3};
constexpr P pointer = {{4}};
static_assert(function() == 3 && pointer->value == 4, "constexpr call and arrow");
int main() {}
