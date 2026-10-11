// N3485 focus: 5.19 [expr.const], 8.5.4 [dcl.init.list], 14.8.2 [temp.deduct]
struct optional {
  template<class U = int>
  constexpr int value_or(U&& value) const { return value; }
};
constexpr optional value = optional();
static_assert(value.value_or({}) == 0, "typed empty braced argument");
int main() {}
