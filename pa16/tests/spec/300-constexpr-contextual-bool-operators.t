// N3485 focus: 5.3.1 [expr.unary.op], 5.19 [expr.const]
struct B {
  constexpr explicit operator bool() const { return true; }
};
constexpr bool both(B value) { return value && true; }
static_assert(!(!B()) && bool(B()) && both(B()), "constexpr bool conversions");
int main() {}
